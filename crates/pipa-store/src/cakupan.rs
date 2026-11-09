//! Dusun per desa, jumlah rumah dan jiwa, serta penghubungan SR ke dusun. Hasilnya data mentah;
//! rumus persen dan estimasi jiwa ada di pipa-core (`hitung_cakupan`).

use pipa_core::DusunIsian;
use serde::Serialize;
use sqlx::Row;

use crate::{Store, StoreError};

const CONSTRAINT_NAMA_DUSUN: &str = "dusun_desa_nama_uniq";

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DusunRow {
    pub id: i64,
    pub desa_id: i64,
    pub nama: String,
    pub jumlah_rumah: i64,
    pub jumlah_jiwa: i64,
    pub diperbarui_pada: String,
}

#[derive(Debug, PartialEq)]
pub enum DusunTulis {
    Berhasil(DusunRow),
    TidakAda,
    DesaTidakAda,
    NamaDipakai,
}

/// Angka mentah cakupan satu dusun. `dusun_id` None berarti SR di desa yang belum dipasang ke dusun mana pun.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DusunCakupan {
    pub dusun_id: Option<i64>,
    pub nama: String,
    pub jumlah_rumah: i64,
    pub jumlah_jiwa: i64,
    pub jumlah_sr: i64,
    pub sr_terlayani: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CakupanDesa {
    pub per_dusun: Vec<DusunCakupan>,
    pub tanpa_dusun: DusunCakupan,
}

#[derive(Debug, PartialEq)]
pub enum SetDusunHasil {
    Berhasil,
    /// Fitur tidak ada, sudah dihapus, atau bukan jenis sr.
    SrTidakAda,
    /// Dusun tidak ada, atau bukan dusun dari desa tempat SR berada.
    DusunBedaDesa,
}

/// SQL yang menghitung SR terlayani: SR yang punya minimal satu pelanggan aktif.
const EXISTS_AKTIF: &str = "EXISTS (SELECT 1 FROM pelanggan p WHERE p.sr_feature_id = f.id AND p.status = 'aktif')";

fn baris_dusun(r: &sqlx::postgres::PgRow) -> Result<DusunRow, StoreError> {
    Ok(DusunRow {
        id: r.try_get("id")?,
        desa_id: r.try_get("desa_id")?,
        nama: r.try_get("nama")?,
        jumlah_rumah: r.try_get::<i32, _>("jumlah_rumah")?.into(),
        jumlah_jiwa: r.try_get::<i32, _>("jumlah_jiwa")?.into(),
        diperbarui_pada: r.try_get("diperbarui_pada")?,
    })
}

const DUSUN_SELECT: &str = "SELECT id, desa_id, nama, jumlah_rumah, jumlah_jiwa, \
    to_char(diperbarui_pada, 'YYYY-MM-DD HH24:MI') AS diperbarui_pada FROM dusun";

impl Store {
    /// Daftar dusun satu desa, menurut nama.
    pub async fn list_dusun(&self, desa_id: i64) -> Result<Vec<DusunRow>, StoreError> {
        let rows = sqlx::query(&format!("{DUSUN_SELECT} WHERE desa_id = $1 ORDER BY lower(nama)"))
            .bind(desa_id)
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(baris_dusun).collect()
    }

    /// Tambah dusun ke desa aktif. Nama unik per desa, tanpa membedakan huruf besar-kecil.
    pub async fn tambah_dusun(&self, desa_id: i64, isian: &DusunIsian, oleh: &str) -> Result<DusunTulis, StoreError> {
        if !self.desa_aktif(desa_id).await? {
            return Ok(DusunTulis::DesaTidakAda);
        }
        let row = sqlx::query(&format!(
            "INSERT INTO dusun (desa_id, nama, jumlah_rumah, jumlah_jiwa, diperbarui_oleh) \
             VALUES ($1, $2, $3, $4, $5) \
             RETURNING id, desa_id, nama, jumlah_rumah, jumlah_jiwa, \
                       to_char(diperbarui_pada, 'YYYY-MM-DD HH24:MI') AS diperbarui_pada"
        ))
        .bind(desa_id)
        .bind(&isian.nama)
        .bind(isian.jumlah_rumah as i32)
        .bind(isian.jumlah_jiwa as i32)
        .bind(oleh)
        .fetch_one(&self.pool)
        .await;
        match row {
            Ok(r) => Ok(DusunTulis::Berhasil(baris_dusun(&r)?)),
            Err(e) if constraint_terlanggar(&e, CONSTRAINT_NAMA_DUSUN) => Ok(DusunTulis::NamaDipakai),
            Err(e) => Err(e.into()),
        }
    }

    /// Ubah nama dan jumlah dusun. Desa dusun tidak bisa diubah.
    pub async fn ubah_dusun(&self, id: i64, isian: &DusunIsian, oleh: &str) -> Result<DusunTulis, StoreError> {
        let row = sqlx::query(&format!(
            "UPDATE dusun SET nama = $2, jumlah_rumah = $3, jumlah_jiwa = $4, diperbarui_oleh = $5, \
                diperbarui_pada = now() \
             WHERE id = $1 \
             RETURNING id, desa_id, nama, jumlah_rumah, jumlah_jiwa, \
                       to_char(diperbarui_pada, 'YYYY-MM-DD HH24:MI') AS diperbarui_pada"
        ))
        .bind(id)
        .bind(&isian.nama)
        .bind(isian.jumlah_rumah as i32)
        .bind(isian.jumlah_jiwa as i32)
        .bind(oleh)
        .fetch_optional(&self.pool)
        .await;
        match row {
            Ok(None) => Ok(DusunTulis::TidakAda),
            Ok(Some(r)) => Ok(DusunTulis::Berhasil(baris_dusun(&r)?)),
            Err(e) if constraint_terlanggar(&e, CONSTRAINT_NAMA_DUSUN) => Ok(DusunTulis::NamaDipakai),
            Err(e) => Err(e.into()),
        }
    }

    /// Pasang SR ke dusun (atau lepaskan bila `dusun_id` None). Dusun harus dari desa yang sama dengan SR.
    pub async fn set_dusun_sr(&self, feature_id: i64, dusun_id: Option<i64>) -> Result<SetDusunHasil, StoreError> {
        let Some(row) = sqlx::query(
            "SELECT f.feature_type, n.desa_id FROM features f JOIN networks n ON n.id = f.network_id \
             WHERE f.id = $1 AND f.deleted_at IS NULL",
        )
        .bind(feature_id)
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(SetDusunHasil::SrTidakAda);
        };
        let jenis: String = row.try_get("feature_type")?;
        if jenis != "sr" {
            return Ok(SetDusunHasil::SrTidakAda);
        }
        let desa_sr: Option<i64> = row.try_get("desa_id")?;
        if let Some(d) = dusun_id {
            let desa_dusun: Option<i64> = sqlx::query_scalar("SELECT desa_id FROM dusun WHERE id = $1")
                .bind(d)
                .fetch_optional(&self.pool)
                .await?;
            if desa_dusun.is_none() || desa_dusun != desa_sr {
                return Ok(SetDusunHasil::DusunBedaDesa);
            }
        }
        sqlx::query("UPDATE features SET dusun_id = $2 WHERE id = $1")
            .bind(feature_id)
            .bind(dusun_id)
            .execute(&self.pool)
            .await?;
        Ok(SetDusunHasil::Berhasil)
    }

    /// Angka mentah cakupan semua dusun di satu desa, plus SR desa yang belum punya dusun.
    /// Hanya menghitung SR yang belum dihapus.
    pub async fn cakupan_desa(&self, desa_id: i64) -> Result<CakupanDesa, StoreError> {
        let rows = sqlx::query(&format!(
            "SELECT du.id AS dusun_id, du.nama, du.jumlah_rumah, du.jumlah_jiwa, \
                    COUNT(f.id) AS jumlah_sr, \
                    COUNT(f.id) FILTER (WHERE {EXISTS_AKTIF}) AS sr_terlayani \
             FROM dusun du \
             LEFT JOIN features f ON f.dusun_id = du.id AND f.feature_type = 'sr' AND f.deleted_at IS NULL \
             WHERE du.desa_id = $1 \
             GROUP BY du.id, du.nama, du.jumlah_rumah, du.jumlah_jiwa \
             ORDER BY lower(du.nama)"
        ))
        .bind(desa_id)
        .fetch_all(&self.pool)
        .await?;
        let per_dusun = rows
            .iter()
            .map(|r| -> Result<DusunCakupan, StoreError> {
                Ok(DusunCakupan {
                    dusun_id: Some(r.try_get("dusun_id")?),
                    nama: r.try_get("nama")?,
                    jumlah_rumah: r.try_get::<i32, _>("jumlah_rumah")?.into(),
                    jumlah_jiwa: r.try_get::<i32, _>("jumlah_jiwa")?.into(),
                    jumlah_sr: r.try_get("jumlah_sr")?,
                    sr_terlayani: r.try_get("sr_terlayani")?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;

        let tanpa = sqlx::query(&format!(
            "SELECT COUNT(f.id) AS jumlah_sr, COUNT(f.id) FILTER (WHERE {EXISTS_AKTIF}) AS sr_terlayani \
             FROM features f JOIN networks n ON n.id = f.network_id \
             WHERE n.desa_id = $1 AND f.feature_type = 'sr' AND f.deleted_at IS NULL AND f.dusun_id IS NULL"
        ))
        .bind(desa_id)
        .fetch_one(&self.pool)
        .await?;
        let tanpa_dusun = DusunCakupan {
            dusun_id: None,
            nama: "Tanpa dusun".into(),
            jumlah_rumah: 0,
            jumlah_jiwa: 0,
            jumlah_sr: tanpa.try_get("jumlah_sr")?,
            sr_terlayani: tanpa.try_get("sr_terlayani")?,
        };
        Ok(CakupanDesa { per_dusun, tanpa_dusun })
    }
}

fn constraint_terlanggar(e: &sqlx::Error, nama: &str) -> bool {
    matches!(e, sqlx::Error::Database(db) if db.code().as_deref() == Some("23505") && db.constraint() == Some(nama))
}
