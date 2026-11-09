//! Referensi pekerjaan (nama paket) dari Arumanis: sinkron, daftar untuk dropdown, dan pengecekan pemilihan.
//! Tabel `ref_pekerjaan` hanya ditulis oleh sinkron. Baris yang tidak lagi ada di Arumanis ditandai tidak aktif.

use serde::Serialize;
use sqlx::Row;

use crate::{Store, StoreError};

/// Satu pekerjaan dari sinkron. `desa_id` dikosongkan bila desanya tidak dikenal di `ref_desa`.
#[derive(Debug, Clone)]
pub struct PekerjaanRefInput {
    pub id: i64,
    pub nama_paket: Option<String>,
    pub tahun: Option<i16>,
    pub desa_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PekerjaanRow {
    pub id: i64,
    pub nama_paket: String,
    pub tahun: Option<i16>,
    pub desa_id: Option<i64>,
}

impl Store {
    /// Terapkan daftar pekerjaan dalam satu transaksi. Pekerjaan yang tidak ada lagi ditandai tidak aktif.
    /// Mengembalikan jumlah pekerjaan yang diterapkan.
    pub async fn apply_pekerjaan(&self, rows: &[PekerjaanRefInput]) -> Result<i64, StoreError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE ref_pekerjaan SET aktif = false").execute(&mut *tx).await?;
        for p in rows {
            // Subquery mengembalikan NULL bila desa tidak ada di ref_desa, jadi FK tidak gagal dan sinkron tetap lanjut.
            sqlx::query(
                "INSERT INTO ref_pekerjaan (id, nama_paket, tahun, desa_id, aktif, synced_at) \
                 VALUES ($1, $2, $3, (SELECT id FROM ref_desa WHERE id = $4), true, now()) \
                 ON CONFLICT (id) DO UPDATE SET nama_paket = EXCLUDED.nama_paket, tahun = EXCLUDED.tahun, \
                   desa_id = EXCLUDED.desa_id, aktif = true, synced_at = now()",
            )
            .bind(p.id)
            .bind(p.nama_paket.as_deref().unwrap_or(&format!("Pekerjaan #{}", p.id)))
            .bind(p.tahun)
            .bind(p.desa_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(rows.len() as i64)
    }

    /// Pekerjaan yang aktif, urut nama paket. Untuk dropdown pemilihan pekerjaan saat impor.
    pub async fn list_pekerjaan(&self) -> Result<Vec<PekerjaanRow>, StoreError> {
        let rows = sqlx::query(
            "SELECT id, nama_paket, tahun, desa_id FROM ref_pekerjaan WHERE aktif ORDER BY nama_paket, id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| PekerjaanRow {
                id: r.get("id"),
                nama_paket: r.get("nama_paket"),
                tahun: r.get("tahun"),
                desa_id: r.get("desa_id"),
            })
            .collect())
    }

    /// True bila pekerjaan ada dan aktif. Dipakai saat upload untuk menolak pekerjaan yang tidak dikenal.
    pub async fn pekerjaan_aktif(&self, id: i64) -> Result<bool, StoreError> {
        Ok(sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM ref_pekerjaan WHERE id = $1 AND aktif)")
            .bind(id)
            .fetch_one(&self.pool)
            .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CommitOutcome;
    use pipa_core::{Bbox, FeatureType};
    use sqlx::PgPool;

    fn pekerjaan(id: i64, nama: Option<&str>, desa_id: Option<i64>) -> PekerjaanRefInput {
        PekerjaanRefInput { id, nama_paket: nama.map(String::from), tahun: None, desa_id }
    }

    #[ignore = "butuh database PostgreSQL"]
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn pekerjaan_sync_upsert_dan_nonaktifkan_yang_hilang(pool: PgPool) {
        let store = Store::from_pool(pool.clone());
        // Desa 99 tidak ada di ref_desa, jadi desa_id harus kosong dan sinkron tetap berhasil.
        store
            .apply_pekerjaan(&[pekerjaan(1, Some("Pipa Bantar"), None), pekerjaan(2, Some("Air Ciranjang"), Some(99))])
            .await
            .unwrap();
        let daftar = store.list_pekerjaan().await.unwrap();
        let nama: Vec<&str> = daftar.iter().map(|p| p.nama_paket.as_str()).collect();
        assert_eq!(nama, vec!["Air Ciranjang", "Pipa Bantar"], "urut nama paket");
        assert_eq!(daftar[0].desa_id, None);

        // Sinkron berikutnya tidak lagi memuat pekerjaan 2 dan mengganti nama pekerjaan 1.
        store.apply_pekerjaan(&[pekerjaan(1, Some("Pipa Bantar Revisi"), None)]).await.unwrap();
        let daftar = store.list_pekerjaan().await.unwrap();
        assert_eq!(daftar.len(), 1);
        assert_eq!(daftar[0].nama_paket, "Pipa Bantar Revisi");
        assert!(store.pekerjaan_aktif(1).await.unwrap());
        assert!(!store.pekerjaan_aktif(2).await.unwrap());
    }

    #[ignore = "butuh database PostgreSQL"]
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn impor_commit_menerapkan_pekerjaan_ke_semua_aset(pool: PgPool) {
        let store = Store::from_pool(pool.clone());
        store.apply_pekerjaan(&[pekerjaan(5, Some("Jaringan Pipa Utama"), None)]).await.unwrap();
        let network: i64 = sqlx::query_scalar("INSERT INTO networks (nama) VALUES ('uji') RETURNING id")
            .fetch_one(&pool)
            .await
            .unwrap();
        let batch: i64 = sqlx::query_scalar(
            "INSERT INTO import_batches (nama_file, status, network_id, laporan, dibuat_oleh, pekerjaan_id) \
             VALUES ('uji.kml', 'staged', $1, '{}', 'operator', 5) RETURNING id",
        )
        .bind(network)
        .fetch_one(&pool)
        .await
        .unwrap();
        for i in 0..2 {
            sqlx::query(
                "INSERT INTO import_staging (batch_id, feature_type, nama, geom, dalam_batas) \
                 VALUES ($1, 'sr', $2, ST_SetSRID(ST_MakePoint(107.1 + $3::float8 * 0.01, -6.8), 4326), true)",
            )
            .bind(batch)
            .bind(format!("Rumah {i}"))
            .bind(i as f64)
            .execute(&pool)
            .await
            .unwrap();
        }

        assert_eq!(
            store.commit_batch(batch, network, false).await.unwrap(),
            CommitOutcome::Committed { ditulis: 2, dilewati: 0 }
        );
        let dengan_pekerjaan: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM features WHERE import_batch_id = $1 AND pekerjaan_id = 5",
        )
        .bind(batch)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(dengan_pekerjaan, 2);

        // Nama pekerjaan ikut terbaca di GeoJSON peta (query bbox).
        let bbox = Bbox { west: 106.0, south: -7.0, east: 108.0, north: -6.0 };
        let fitur = store.features_in_bbox(Some(network), bbox, Some(&[FeatureType::Sr])).await.unwrap();
        assert_eq!(fitur.len(), 2);
        assert!(fitur.iter().all(|f| f.pekerjaan_id == Some(5)
            && f.pekerjaan_nama.as_deref() == Some("Jaringan Pipa Utama")));
    }
}
