//! Penandaan rehabilitasi per aset dan daftar prioritas rehabilitasi per jaringan.

use pipa_core::om::{periksa_teks_opsional, OmError};
use pipa_core::rehab::{bandingkan_prioritas, masuk_daftar, periksa_rehab, KandidatRehab, StatusRehab};
use serde::Serialize;
use sqlx::Row;

use crate::om_riwayat::catat;
use crate::{Store, StoreError};

/// Penandaan rehabilitasi satu aset, sebagaimana tersimpan.
#[derive(Debug, Clone, Serialize)]
pub struct AsetRehab {
    pub feature_id: i64,
    pub status_rehab: String,
    pub prioritas: Option<i64>,
    pub catatan: Option<String>,
    pub diubah_oleh: String,
    pub diubah_pada: String,
}

/// Satu baris daftar prioritas. `status_rehab` bernilai `belum_ditandai` untuk aset rusak yang belum pernah ditandai.
#[derive(Debug, Clone, Serialize)]
pub struct BarisPrioritas {
    pub feature_id: i64,
    pub feature_type: String,
    pub nama_aset: Option<String>,
    pub kondisi: Option<String>,
    pub status_rehab: &'static str,
    pub prioritas: Option<i64>,
    pub catatan: Option<String>,
}

const REHAB_COLS: &str = "r.feature_id, r.status_rehab, r.prioritas::bigint AS prioritas, r.catatan, \
    r.diubah_oleh, to_char(r.diubah_pada, 'YYYY-MM-DD\"T\"HH24:MI:SSOF') AS diubah_pada";

fn rehab_dari_baris(r: &sqlx::postgres::PgRow) -> Result<AsetRehab, sqlx::Error> {
    Ok(AsetRehab {
        feature_id: r.try_get("feature_id")?,
        status_rehab: r.try_get("status_rehab")?,
        prioritas: r.try_get("prioritas")?,
        catatan: r.try_get("catatan")?,
        diubah_oleh: r.try_get("diubah_oleh")?,
        diubah_pada: r.try_get("diubah_pada")?,
    })
}

impl Store {
    pub async fn ambil_rehab(&self, feature_id: i64) -> Result<Option<AsetRehab>, StoreError> {
        let row = sqlx::query(&format!("SELECT {REHAB_COLS} FROM aset_rehab r WHERE r.feature_id = $1"))
            .bind(feature_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.as_ref().map(rehab_dari_baris).transpose()?)
    }

    /// Simpan penandaan rehabilitasi. Aturan kondisi dan prioritas ada di `pipa_core::rehab::periksa_rehab`.
    /// Isian yang sama dengan yang sudah tersimpan tidak menulis riwayat.
    pub async fn simpan_rehab(
        &self,
        feature_id: i64,
        status: StatusRehab,
        prioritas: Option<i64>,
        catatan: Option<&str>,
        oleh: &str,
    ) -> Result<AsetRehab, StoreError> {
        let catatan = periksa_teks_opsional(catatan, "catatan", 2000)?;
        let mut tx = self.pool.begin().await?;
        let Some(aset) = sqlx::query("SELECT feature_type, kondisi FROM features WHERE id = $1 AND deleted_at IS NULL FOR UPDATE")
            .bind(feature_id)
            .fetch_optional(&mut *tx)
            .await?
        else {
            return Err(OmError::TidakDitemukan("aset").into());
        };
        let feature_type: String = aset.try_get("feature_type")?;
        if feature_type == "sr" {
            return Err(OmError::Aturan(vec!["data pelanggan tidak bisa ditandai rehabilitasi".into()]).into());
        }
        let kondisi: Option<String> = aset.try_get("kondisi")?;
        periksa_rehab(status, prioritas, kondisi.as_deref())?;

        let sebelum_row = sqlx::query(&format!("SELECT {REHAB_COLS} FROM aset_rehab r WHERE r.feature_id = $1 FOR UPDATE"))
            .bind(feature_id)
            .fetch_optional(&mut *tx)
            .await?;
        let sebelum = sebelum_row.as_ref().map(rehab_dari_baris).transpose()?;
        if let Some(s) = &sebelum {
            if s.status_rehab == status.as_str() && s.prioritas == prioritas && s.catatan == catatan {
                tx.rollback().await?;
                return Ok(s.clone());
            }
        }

        sqlx::query(
            "INSERT INTO aset_rehab (feature_id, status_rehab, prioritas, catatan, diubah_oleh) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (feature_id) DO UPDATE SET status_rehab = EXCLUDED.status_rehab, \
                prioritas = EXCLUDED.prioritas, catatan = EXCLUDED.catatan, \
                diubah_oleh = EXCLUDED.diubah_oleh, diubah_pada = now()",
        )
        .bind(feature_id)
        .bind(status.as_str())
        .bind(prioritas.map(|p| p as i16))
        .bind(&catatan)
        .bind(oleh)
        .execute(&mut *tx)
        .await?;

        let sesudah_row = sqlx::query(&format!("SELECT {REHAB_COLS} FROM aset_rehab r WHERE r.feature_id = $1"))
            .bind(feature_id)
            .fetch_one(&mut *tx)
            .await?;
        let sesudah = rehab_dari_baris(&sesudah_row)?;
        let snap = |v: &AsetRehab| serde_json::to_value(v).map_err(|e| StoreError::Data(e.to_string()));
        catat(
            &mut tx,
            "rehab",
            feature_id,
            Some(feature_id),
            if sebelum.is_some() { "ubah" } else { "tambah" },
            oleh,
            sebelum.as_ref().map(&snap).transpose()?,
            Some(snap(&sesudah)?),
        )
        .await?;
        tx.commit().await?;
        Ok(sesudah)
    }

    /// Daftar prioritas rehabilitasi satu jaringan. Berisi aset dengan status aktif, dan aset rusak yang belum
    /// pernah ditandai. Diurutkan dengan `pipa_core::rehab::bandingkan_prioritas`.
    pub async fn daftar_prioritas(&self, network_id: i64) -> Result<Vec<BarisPrioritas>, StoreError> {
        let rows = sqlx::query(
            "SELECT f.id AS feature_id, f.feature_type, f.nama AS nama_aset, f.kondisi, \
                    r.status_rehab, r.prioritas::bigint AS prioritas, r.catatan \
             FROM features f LEFT JOIN aset_rehab r ON r.feature_id = f.id \
             WHERE f.network_id = $1 AND f.deleted_at IS NULL AND f.feature_type <> 'sr' \
               AND (r.status_rehab IS NULL OR r.status_rehab <> 'tidak_ada') \
               AND (r.status_rehab IS NOT NULL OR f.kondisi IN ('rusak_ringan', 'rusak_berat', 'tidak_berfungsi'))",
        )
        .bind(network_id)
        .fetch_all(&self.pool)
        .await?;

        let mut daftar: Vec<(KandidatRehab, BarisPrioritas)> = Vec::with_capacity(rows.len());
        for r in &rows {
            let status_teks: Option<String> = r.try_get("status_rehab")?;
            let status = status_teks.as_deref().map(StatusRehab::parse).transpose()?;
            let kondisi: Option<String> = r.try_get("kondisi")?;
            let prioritas: Option<i64> = r.try_get("prioritas")?;
            if !masuk_daftar(status, kondisi.as_deref()) {
                continue;
            }
            let kandidat = KandidatRehab { status, prioritas, kondisi: kondisi.clone() };
            let baris = BarisPrioritas {
                feature_id: r.try_get("feature_id")?,
                feature_type: r.try_get("feature_type")?,
                nama_aset: r.try_get("nama_aset")?,
                kondisi,
                status_rehab: status.map_or("belum_ditandai", StatusRehab::as_str),
                prioritas,
                catatan: r.try_get("catatan")?,
            };
            daftar.push((kandidat, baris));
        }
        daftar.sort_by(|a, b| bandingkan_prioritas(&a.0, &b.0));
        Ok(daftar.into_iter().map(|(_, b)| b).collect())
    }
}
