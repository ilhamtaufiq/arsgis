//! Kelola batch impor: batal impor, pencarian berkas ganda, dan referensi pekerjaan.
//! Setiap perubahan fitur ditulis bersama baris riwayatnya dalam satu transaksi.

use pipa_core::impor::{periksa_batal, KandidatDuplikat, KelolaError};
use sqlx::Row;

use crate::aset::{fitur_dari_baris, fitur_geojson, FITUR_COLS};
use crate::{Store, StoreError};

/// Hasil pembatalan. `Berhasil.dibatalkan` adalah jumlah fitur yang dihapus dari peta (0 untuk batch staged).
#[derive(Debug, PartialEq, Eq)]
pub enum BatalHasil {
    Berhasil { dibatalkan: u64 },
    TidakAda,
    SudahDibatalkan,
}

impl Store {
    /// Batalkan satu impor utuh.
    /// - Batch `committed`: semua fitur dengan `import_batch_id` ini dan belum terhapus di-soft-delete.
    ///   Setiap fitur dicatat di riwayat dengan aksi `batal_impor`. Aset yang sudah diubah tetap ikut,
    ///   karena keterkaitan dihitung dari `import_batch_id`.
    /// - Batch `staged`: baris stagingnya dihapus. Tidak ada fitur yang berubah.
    pub async fn batalkan_impor(&self, id: i64, oleh: &str) -> Result<BatalHasil, StoreError> {
        let mut tx = self.pool.begin().await?;
        let Some(status): Option<String> =
            sqlx::query_scalar("SELECT status FROM import_batches WHERE id = $1 FOR UPDATE")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?
        else {
            return Ok(BatalHasil::TidakAda);
        };
        match periksa_batal(&status) {
            Ok(()) => {}
            Err(KelolaError::SudahDibatalkan) => return Ok(BatalHasil::SudahDibatalkan),
            Err(e) => return Err(StoreError::Data(e.to_string())),
        }

        let dibatalkan = if status == "committed" {
            let rows = sqlx::query(&format!(
                "UPDATE features SET deleted_at = now() \
                 WHERE import_batch_id = $1 AND deleted_at IS NULL \
                 RETURNING {FITUR_COLS}"
            ))
            .bind(id)
            .fetch_all(&mut *tx)
            .await?;
            for row in &rows {
                let fitur = fitur_dari_baris(row)?;
                sqlx::query(
                    "INSERT INTO feature_riwayat (feature_id, aksi, diubah_oleh, sebelum) VALUES ($1, 'batal_impor', $2, $3)",
                )
                .bind(fitur.id)
                .bind(oleh)
                .bind(fitur_geojson(&fitur))
                .execute(&mut *tx)
                .await?;
            }
            rows.len() as u64
        } else {
            sqlx::query("DELETE FROM import_staging WHERE batch_id = $1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            0
        };

        sqlx::query(
            "UPDATE import_batches SET status = 'dibatalkan', dibatalkan_oleh = $2, dibatalkan_pada = now() WHERE id = $1",
        )
        .bind(id)
        .bind(oleh)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(BatalHasil::Berhasil { dibatalkan })
    }

    /// Batch committed yang berkas aslinya ber-hash `hash`, urut id menaik. Dipakai untuk mendeteksi impor ganda.
    pub async fn batch_committed_dengan_hash(&self, hash: &str) -> Result<Vec<KandidatDuplikat>, StoreError> {
        let rows = sqlx::query(
            "SELECT id, nama_file, file_sha256, to_char(created_at, 'YYYY-MM-DD\"T\"HH24:MI:SSOF') AS created_at \
             FROM import_batches WHERE status = 'committed' AND file_sha256 = $1 ORDER BY id",
        )
        .bind(hash)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| KandidatDuplikat {
                batch_id: r.get("id"),
                nama_file: r.get("nama_file"),
                created_at: r.get("created_at"),
                file_sha256: r.get("file_sha256"),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    /// Jaringan, satu batch (committed atau staged), dan `jumlah` fitur yang menempel pada batch itu.
    async fn batch_dengan_fitur(pool: &PgPool, status: &str, jumlah: usize) -> (i64, Vec<i64>) {
        let network: i64 = sqlx::query_scalar("INSERT INTO networks (nama) VALUES ('uji') RETURNING id")
            .fetch_one(pool)
            .await
            .unwrap();
        let batch: i64 = sqlx::query_scalar(
            "INSERT INTO import_batches (nama_file, status, network_id, laporan, dibuat_oleh) \
             VALUES ('uji.kml', $1, $2, '{}', 'operator') RETURNING id",
        )
        .bind(status)
        .bind(network)
        .fetch_one(pool)
        .await
        .unwrap();
        let mut ids = Vec::new();
        for i in 0..jumlah {
            let id: i64 = sqlx::query_scalar(
                "INSERT INTO features (network_id, feature_type, geom, nama, import_batch_id) \
                 VALUES ($1, 'pipa', ST_SetSRID(ST_GeomFromText('LINESTRING(107.1 -6.8, 107.11 -6.8)'), 4326), $2, $3) \
                 RETURNING id",
            )
            .bind(network)
            .bind(format!("pipa {i}"))
            .bind(batch)
            .fetch_one(pool)
            .await
            .unwrap();
            ids.push(id);
        }
        (batch, ids)
    }

    /// Batch staged dengan satu baris staging. Mengembalikan id batch.
    async fn batch_staged_dengan_staging(pool: &PgPool) -> i64 {
        let (batch, _) = batch_dengan_fitur(pool, "staged", 0).await;
        sqlx::query(
            "INSERT INTO import_staging (batch_id, feature_type, nama, geom) \
             VALUES ($1, 'pipa', 'calon', ST_SetSRID(ST_GeomFromText('LINESTRING(107.1 -6.8, 107.11 -6.8)'), 4326))",
        )
        .bind(batch)
        .execute(pool)
        .await
        .unwrap();
        batch
    }

    #[ignore = "butuh database PostgreSQL"]
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn impor_batal_committed_menghapus_fitur_dan_mencatat_riwayat(pool: PgPool) {
        let (batch, ids) = batch_dengan_fitur(&pool, "committed", 2).await;
        let store = Store::from_pool(pool.clone());

        let hasil = store.batalkan_impor(batch, "operator").await.unwrap();
        assert_eq!(hasil, BatalHasil::Berhasil { dibatalkan: 2 });

        let terhapus: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM features WHERE import_batch_id = $1 AND deleted_at IS NOT NULL",
        )
        .bind(batch)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(terhapus, 2);

        let riwayat: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM feature_riwayat WHERE aksi = 'batal_impor' AND feature_id = ANY($1)",
        )
        .bind(&ids)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(riwayat, 2);

        let status: String = sqlx::query_scalar("SELECT status FROM import_batches WHERE id = $1")
            .bind(batch)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status, "dibatalkan");

        assert_eq!(store.batalkan_impor(batch, "operator").await.unwrap(), BatalHasil::SudahDibatalkan);
    }

    #[ignore = "butuh database PostgreSQL"]
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn impor_batal_staged_menghapus_staging(pool: PgPool) {
        let batch = batch_staged_dengan_staging(&pool).await;
        let store = Store::from_pool(pool.clone());

        let hasil = store.batalkan_impor(batch, "operator").await.unwrap();
        assert_eq!(hasil, BatalHasil::Berhasil { dibatalkan: 0 });

        let sisa: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM import_staging WHERE batch_id = $1")
            .bind(batch)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(sisa, 0);
    }

    #[ignore = "butuh database PostgreSQL"]
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn impor_batal_batch_tidak_ada(pool: PgPool) {
        let store = Store::from_pool(pool);
        assert_eq!(store.batalkan_impor(999_999, "operator").await.unwrap(), BatalHasil::TidakAda);
    }
}
