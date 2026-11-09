//! Riwayat perbaikan per aset, jadwal perawatan berulang, dan daftar jatuh tempo.

use pipa_core::om::{periksa_teks, periksa_teks_opsional, OmError};
use pipa_core::perawatan::{
    periksa_interval, periksa_pencatatan, selisih_hari, status_jatuh_tempo, tanggal_berikutnya, JenisPerawatan,
    StatusJatuhTempo,
};
use pipa_core::tanggal::Tanggal;
use serde::Serialize;
use serde_json::Value;
use sqlx::postgres::PgRow;
use sqlx::{Postgres, Row, Transaction};

use crate::om_riwayat::catat;
use crate::{Store, StoreError};

const PERBAIKAN_COLS: &str = "p.id, p.feature_id, p.gangguan_id, to_char(p.tanggal, 'YYYY-MM-DD') AS tanggal, \
    p.tindakan, p.teknisi, p.biaya::float8 AS biaya, p.catatan, p.dicatat_oleh, \
    to_char(p.dicatat_pada, 'YYYY-MM-DD\"T\"HH24:MI:SSOF') AS dicatat_pada";

const JADWAL_COLS: &str = "j.id, j.feature_id, j.jenis, j.interval_hari::bigint AS interval_hari, \
    to_char(j.terakhir_dilakukan, 'YYYY-MM-DD') AS terakhir_dilakukan, \
    to_char(j.tanggal_berikutnya, 'YYYY-MM-DD') AS tanggal_berikutnya, j.aktif, j.diubah_oleh, \
    to_char(j.diubah_pada, 'YYYY-MM-DD\"T\"HH24:MI:SSOF') AS diubah_pada";

#[derive(Debug, Clone, Serialize)]
pub struct Perbaikan {
    pub id: i64,
    pub feature_id: i64,
    pub gangguan_id: Option<i64>,
    pub tanggal: String,
    pub tindakan: String,
    pub teknisi: String,
    pub biaya: Option<f64>,
    pub catatan: Option<String>,
    pub dicatat_oleh: String,
    pub dicatat_pada: String,
}

#[derive(Debug, Clone)]
pub struct PerbaikanBaru {
    /// `YYYY-MM-DD`.
    pub tanggal: String,
    pub tindakan: String,
    pub teknisi: String,
    pub biaya: Option<f64>,
    pub catatan: Option<String>,
    pub gangguan_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JadwalPerawatan {
    pub id: i64,
    pub feature_id: i64,
    pub jenis: String,
    pub interval_hari: i64,
    pub terakhir_dilakukan: Option<String>,
    pub tanggal_berikutnya: String,
    pub aktif: bool,
    pub diubah_oleh: String,
    pub diubah_pada: String,
}

#[derive(Debug, Clone)]
pub struct JadwalBaru {
    pub jenis: String,
    pub interval_hari: i64,
    /// Tanggal perawatan pertama. Bila kosong, dihitung mulai hari ini.
    pub tanggal_berikutnya: Option<String>,
}

/// Baris daftar jatuh tempo, lengkap dengan nama aset dan status hitungannya.
#[derive(Debug, Clone, Serialize)]
pub struct JatuhTempoBaris {
    pub jadwal_id: i64,
    pub feature_id: i64,
    pub feature_type: String,
    pub nama_aset: Option<String>,
    pub jenis: String,
    pub interval_hari: i64,
    pub terakhir_dilakukan: Option<String>,
    pub tanggal_berikutnya: String,
    pub selisih_hari: i64,
    pub status: &'static str,
}

fn perbaikan_dari_baris(r: &PgRow) -> Result<Perbaikan, sqlx::Error> {
    Ok(Perbaikan {
        id: r.try_get("id")?,
        feature_id: r.try_get("feature_id")?,
        gangguan_id: r.try_get("gangguan_id")?,
        tanggal: r.try_get("tanggal")?,
        tindakan: r.try_get("tindakan")?,
        teknisi: r.try_get("teknisi")?,
        biaya: r.try_get("biaya")?,
        catatan: r.try_get("catatan")?,
        dicatat_oleh: r.try_get("dicatat_oleh")?,
        dicatat_pada: r.try_get("dicatat_pada")?,
    })
}

fn jadwal_dari_baris(r: &PgRow) -> Result<JadwalPerawatan, sqlx::Error> {
    Ok(JadwalPerawatan {
        id: r.try_get("id")?,
        feature_id: r.try_get("feature_id")?,
        jenis: r.try_get("jenis")?,
        interval_hari: r.try_get("interval_hari")?,
        terakhir_dilakukan: r.try_get("terakhir_dilakukan")?,
        tanggal_berikutnya: r.try_get("tanggal_berikutnya")?,
        aktif: r.try_get("aktif")?,
        diubah_oleh: r.try_get("diubah_oleh")?,
        diubah_pada: r.try_get("diubah_pada")?,
    })
}

fn snapshot<T: Serialize>(v: &T) -> Result<Value, StoreError> {
    serde_json::to_value(v).map_err(|e| StoreError::Data(e.to_string()))
}

fn biaya_valid(biaya: Option<f64>) -> Result<(), OmError> {
    match biaya {
        Some(b) if !b.is_finite() || b < 0.0 => Err(OmError::Masukan("biaya tidak boleh negatif".into())),
        _ => Ok(()),
    }
}

async fn jadwal_dalam_tx(tx: &mut Transaction<'_, Postgres>, id: i64, kunci: bool) -> Result<Option<JadwalPerawatan>, StoreError> {
    let kunci_sql = if kunci { " FOR UPDATE OF j" } else { "" };
    let row = sqlx::query(&format!("SELECT {JADWAL_COLS} FROM jadwal_perawatan j WHERE j.id = $1{kunci_sql}"))
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?;
    row.as_ref().map(jadwal_dari_baris).transpose().map_err(StoreError::from)
}

impl Store {
    /// Catat perbaikan. Aset harus ada. Laporan gangguan opsional, dan bila ada harus terkait aset ini atau tanpa aset.
    pub async fn tambah_perbaikan(
        &self,
        feature_id: i64,
        baru: PerbaikanBaru,
        hari_ini: Tanggal,
        oleh: &str,
    ) -> Result<Perbaikan, StoreError> {
        let tanggal = Tanggal::parse(&baru.tanggal)?;
        periksa_pencatatan(tanggal, hari_ini)?;
        let tindakan = periksa_teks(&baru.tindakan, "tindakan", 500)?;
        let teknisi = periksa_teks(&baru.teknisi, "teknisi", 100)?;
        let catatan = periksa_teks_opsional(baru.catatan.as_deref(), "catatan", 2000)?;
        biaya_valid(baru.biaya)?;

        let mut tx = self.pool.begin().await?;
        let ada_aset: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM features WHERE id = $1 AND deleted_at IS NULL)",
        )
        .bind(feature_id)
        .fetch_one(&mut *tx)
        .await?;
        if !ada_aset {
            return Err(OmError::TidakDitemukan("aset").into());
        }
        if let Some(gid) = baru.gangguan_id {
            let laporan_feature: Option<Option<i64>> =
                sqlx::query_scalar("SELECT feature_id FROM gangguan WHERE id = $1")
                    .bind(gid)
                    .fetch_optional(&mut *tx)
                    .await?;
            match laporan_feature {
                None => return Err(OmError::TidakDitemukan("laporan gangguan").into()),
                Some(Some(f)) if f != feature_id => {
                    return Err(OmError::Aturan(vec!["laporan gangguan ini terkait aset lain".into()]).into())
                }
                Some(_) => {}
            }
        }

        let id: i64 = sqlx::query_scalar(
            "INSERT INTO perbaikan (feature_id, gangguan_id, tanggal, tindakan, teknisi, biaya, catatan, dicatat_oleh) \
             VALUES ($1, $2, $3::date, $4, $5, $6::numeric, $7, $8) RETURNING id",
        )
        .bind(feature_id)
        .bind(baru.gangguan_id)
        .bind(tanggal.format())
        .bind(&tindakan)
        .bind(&teknisi)
        .bind(baru.biaya)
        .bind(&catatan)
        .bind(oleh)
        .fetch_one(&mut *tx)
        .await?;

        let row = sqlx::query(&format!("SELECT {PERBAIKAN_COLS} FROM perbaikan p WHERE p.id = $1"))
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
        let hasil = perbaikan_dari_baris(&row)?;
        catat(&mut tx, "perbaikan", id, Some(feature_id), "tambah", oleh, None, Some(snapshot(&hasil)?)).await?;
        tx.commit().await?;
        Ok(hasil)
    }

    /// Riwayat perbaikan satu aset, tanggal terbaru di atas.
    pub async fn daftar_perbaikan(&self, feature_id: i64, batas: i64) -> Result<Vec<Perbaikan>, StoreError> {
        let rows = sqlx::query(&format!(
            "SELECT {PERBAIKAN_COLS} FROM perbaikan p WHERE p.feature_id = $1 \
             ORDER BY p.tanggal DESC, p.id DESC LIMIT $2"
        ))
        .bind(feature_id)
        .bind(batas)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(perbaikan_dari_baris).collect::<Result<_, _>>()?)
    }

    /// Tambah jadwal perawatan untuk satu aset.
    pub async fn tambah_jadwal(
        &self,
        feature_id: i64,
        baru: JadwalBaru,
        hari_ini: Tanggal,
        oleh: &str,
    ) -> Result<JadwalPerawatan, StoreError> {
        let jenis = JenisPerawatan::parse(&baru.jenis)?;
        periksa_interval(baru.interval_hari)?;
        let berikutnya = match baru.tanggal_berikutnya.as_deref() {
            Some(s) => Tanggal::parse(s)?,
            None => hari_ini,
        };

        let mut tx = self.pool.begin().await?;
        let ada_aset: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM features WHERE id = $1 AND deleted_at IS NULL)",
        )
        .bind(feature_id)
        .fetch_one(&mut *tx)
        .await?;
        if !ada_aset {
            return Err(OmError::TidakDitemukan("aset").into());
        }
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO jadwal_perawatan (feature_id, jenis, interval_hari, tanggal_berikutnya, diubah_oleh) \
             VALUES ($1, $2, $3, $4::date, $5) RETURNING id",
        )
        .bind(feature_id)
        .bind(jenis.as_str())
        .bind(baru.interval_hari as i32)
        .bind(berikutnya.format())
        .bind(oleh)
        .fetch_one(&mut *tx)
        .await?;
        let hasil = jadwal_dalam_tx(&mut tx, id, false)
            .await?
            .ok_or_else(|| StoreError::Data("jadwal baru hilang".into()))?;
        catat(&mut tx, "jadwal_perawatan", id, Some(feature_id), "tambah", oleh, None, Some(snapshot(&hasil)?))
            .await?;
        tx.commit().await?;
        Ok(hasil)
    }

    pub async fn daftar_jadwal(&self, feature_id: i64) -> Result<Vec<JadwalPerawatan>, StoreError> {
        let rows = sqlx::query(&format!(
            "SELECT {JADWAL_COLS} FROM jadwal_perawatan j WHERE j.feature_id = $1 ORDER BY j.aktif DESC, j.tanggal_berikutnya, j.id"
        ))
        .bind(feature_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(jadwal_dari_baris).collect::<Result<_, _>>()?)
    }

    /// Catat perawatan dilakukan. Tanggal terakhir diisi, dan tanggal berikutnya dihitung dari interval.
    pub async fn catat_perawatan_selesai(
        &self,
        jadwal_id: i64,
        tanggal: Option<&str>,
        hari_ini: Tanggal,
        oleh: &str,
    ) -> Result<JadwalPerawatan, StoreError> {
        let dilakukan = match tanggal {
            Some(s) => Tanggal::parse(s)?,
            None => hari_ini,
        };
        periksa_pencatatan(dilakukan, hari_ini)?;

        let mut tx = self.pool.begin().await?;
        let Some(sebelum) = jadwal_dalam_tx(&mut tx, jadwal_id, true).await? else {
            return Err(OmError::TidakDitemukan("jadwal perawatan").into());
        };
        if !sebelum.aktif {
            return Err(OmError::Aturan(vec!["jadwal ini nonaktif".into()]).into());
        }
        let berikutnya = tanggal_berikutnya(dilakukan, sebelum.interval_hari);
        sqlx::query(
            "UPDATE jadwal_perawatan SET terakhir_dilakukan = $2::date, tanggal_berikutnya = $3::date, \
                diubah_oleh = $4, diubah_pada = now() WHERE id = $1",
        )
        .bind(jadwal_id)
        .bind(dilakukan.format())
        .bind(berikutnya.format())
        .bind(oleh)
        .execute(&mut *tx)
        .await?;
        let sesudah = jadwal_dalam_tx(&mut tx, jadwal_id, false)
            .await?
            .ok_or_else(|| StoreError::Data("jadwal hilang".into()))?;
        catat(
            &mut tx,
            "jadwal_perawatan",
            jadwal_id,
            Some(sesudah.feature_id),
            "selesai",
            oleh,
            Some(snapshot(&sebelum)?),
            Some(snapshot(&sesudah)?),
        )
        .await?;
        tx.commit().await?;
        Ok(sesudah)
    }

    /// Jadwal aktif yang sudah lewat atau jatuh tempo dalam `jendela_hari` hari, untuk satu jaringan.
    /// Hitungan statusnya memakai `pipa_core::perawatan`.
    pub async fn jatuh_tempo(
        &self,
        network_id: i64,
        hari_ini: Tanggal,
        jendela_hari: i64,
    ) -> Result<Vec<JatuhTempoBaris>, StoreError> {
        let rows = sqlx::query(
            "SELECT j.id AS jadwal_id, f.id AS feature_id, f.feature_type, f.nama AS nama_aset, j.jenis, \
                    j.interval_hari::bigint AS interval_hari, to_char(j.terakhir_dilakukan, 'YYYY-MM-DD') AS terakhir_dilakukan, \
                    to_char(j.tanggal_berikutnya, 'YYYY-MM-DD') AS tanggal_berikutnya \
             FROM jadwal_perawatan j JOIN features f ON f.id = j.feature_id \
             WHERE j.aktif AND f.network_id = $1 AND f.deleted_at IS NULL \
               AND j.tanggal_berikutnya <= ($2::date + $3::int) \
             ORDER BY j.tanggal_berikutnya, j.id",
        )
        .bind(network_id)
        .bind(hari_ini.format())
        .bind(jendela_hari as i32)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(|r| {
                let berikutnya_teks: String = r.try_get("tanggal_berikutnya")?;
                let berikutnya = Tanggal::parse(&berikutnya_teks).map_err(|e| StoreError::Data(e.to_string()))?;
                let status: StatusJatuhTempo = status_jatuh_tempo(berikutnya, hari_ini, jendela_hari);
                Ok(JatuhTempoBaris {
                    jadwal_id: r.try_get("jadwal_id")?,
                    feature_id: r.try_get("feature_id")?,
                    feature_type: r.try_get("feature_type")?,
                    nama_aset: r.try_get("nama_aset")?,
                    jenis: r.try_get("jenis")?,
                    interval_hari: r.try_get("interval_hari")?,
                    terakhir_dilakukan: r.try_get("terakhir_dilakukan")?,
                    tanggal_berikutnya: berikutnya_teks,
                    selisih_hari: selisih_hari(berikutnya, hari_ini),
                    status: status.as_str(),
                })
            })
            .collect()
    }
}
