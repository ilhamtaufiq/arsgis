//! Log perubahan O&M (`om_riwayat`) dan pembacaan gabungan per aset untuk panel detail aset.
//! Setiap penulisan O&M memanggil `catat` dalam transaksi yang sama dengan perubahan datanya.

use serde::Serialize;
use serde_json::Value;
use sqlx::{Postgres, Row, Transaction};

use crate::gangguan::{FilterGangguan, Gangguan};
use crate::perawatan::{JadwalPerawatan, Perbaikan};
use crate::rehab::AsetRehab;
use crate::{Store, StoreError};

/// Satu baris log O&M. `sebelum` dan `sesudah` berbentuk objek JSON dari entitas terkait.
#[derive(Debug, Clone, Serialize)]
pub struct RiwayatOm {
    pub sumber: String,
    pub sumber_id: i64,
    pub aksi: String,
    pub diubah_oleh: String,
    pub diubah_pada: String,
    pub sebelum: Option<Value>,
    pub sesudah: Option<Value>,
}

/// Panel detail O&M untuk satu aset.
#[derive(Debug, Clone, Serialize)]
pub struct OmAset {
    pub feature_id: i64,
    pub perbaikan: Vec<Perbaikan>,
    pub jadwal: Vec<JadwalPerawatan>,
    pub rehab: Option<AsetRehab>,
    pub gangguan: Vec<Gangguan>,
    pub riwayat: Vec<RiwayatOm>,
}

/// Menulis satu baris log. Dipanggil dari dalam transaksi penulisan.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn catat(
    tx: &mut Transaction<'_, Postgres>,
    sumber: &str,
    sumber_id: i64,
    feature_id: Option<i64>,
    aksi: &str,
    oleh: &str,
    sebelum: Option<Value>,
    sesudah: Option<Value>,
) -> Result<(), StoreError> {
    sqlx::query(
        "INSERT INTO om_riwayat (sumber, sumber_id, feature_id, aksi, diubah_oleh, sebelum, sesudah) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(sumber)
    .bind(sumber_id)
    .bind(feature_id)
    .bind(aksi)
    .bind(oleh)
    .bind(sebelum)
    .bind(sesudah)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

impl Store {
    /// Jaringan tempat aset berada. None bila aset tidak ada atau sudah dihapus.
    pub(crate) async fn jaringan_aset(&self, feature_id: i64) -> Result<Option<i64>, StoreError> {
        let id = sqlx::query_scalar(
            "SELECT network_id FROM features WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(feature_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(id)
    }

    /// Jenis aset (teks dari `features.feature_type`). None bila aset tidak ada atau sudah dihapus.
    pub async fn tipe_aset(&self, feature_id: i64) -> Result<Option<String>, StoreError> {
        let tipe = sqlx::query_scalar("SELECT feature_type FROM features WHERE id = $1 AND deleted_at IS NULL")
            .bind(feature_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(tipe)
    }

    /// Panel O&M satu aset: perbaikan, jadwal, rehabilitasi, laporan gangguan terbaru, dan log.
    /// None bila aset tidak ada atau sudah dihapus.
    pub async fn om_aset(&self, feature_id: i64) -> Result<Option<OmAset>, StoreError> {
        if self.jaringan_aset(feature_id).await?.is_none() {
            return Ok(None);
        }
        let perbaikan = self.daftar_perbaikan(feature_id, 50).await?;
        let jadwal = self.daftar_jadwal(feature_id).await?;
        let rehab = self.ambil_rehab(feature_id).await?;
        let gangguan = self
            .daftar_gangguan(FilterGangguan { network_id: None, feature_id: Some(feature_id), status: None }, 20)
            .await?;
        let riwayat = self.riwayat_om_aset(feature_id, 50).await?;
        Ok(Some(OmAset { feature_id, perbaikan, jadwal, rehab, gangguan, riwayat }))
    }

    async fn riwayat_om_aset(&self, feature_id: i64, batas: i64) -> Result<Vec<RiwayatOm>, StoreError> {
        let rows = sqlx::query(
            "SELECT sumber, sumber_id, aksi, diubah_oleh, \
                    to_char(diubah_pada, 'YYYY-MM-DD\"T\"HH24:MI:SSOF') AS diubah_pada, sebelum, sesudah \
             FROM om_riwayat WHERE feature_id = $1 ORDER BY diubah_pada DESC, id DESC LIMIT $2",
        )
        .bind(feature_id)
        .bind(batas)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|r| {
                Ok(RiwayatOm {
                    sumber: r.try_get("sumber")?,
                    sumber_id: r.try_get("sumber_id")?,
                    aksi: r.try_get("aksi")?,
                    diubah_oleh: r.try_get("diubah_oleh")?,
                    diubah_pada: r.try_get("diubah_pada")?,
                    sebelum: r.try_get("sebelum")?,
                    sesudah: r.try_get("sesudah")?,
                })
            })
            .collect()
    }
}
