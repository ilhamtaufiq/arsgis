//! Akses data PostGIS: migrasi, daftar jaringan, dan fitur dalam bbox.

use pipa_core::{Bbox, FeatureType};
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};
use sqlx::Row;
use thiserror::Error;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Batas jumlah fitur per permintaan. Di atas ini, gunakan vector tiles (fase lanjutan).
pub const MAX_FEATURES: i64 = 50_000;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database: {0}")]
    Db(#[from] sqlx::Error),
    #[error("migrasi: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("hasil melebihi {MAX_FEATURES} fitur; persempit bbox atau jenis")]
    TooManyFeatures,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Network {
    pub id: i64,
    pub nama: String,
    pub unit_spam_id: Option<i64>,
    pub desa_id: Option<i64>,
    pub deskripsi: Option<String>,
}

/// Satu fitur dengan geometri sudah dalam bentuk GeoJSON.
#[derive(Debug, Clone)]
pub struct FeatureRow {
    pub id: i64,
    pub network_id: i64,
    pub feature_type: String,
    pub nama: Option<String>,
    pub diameter_mm: Option<i32>,
    pub material: Option<String>,
    pub tahun_pasang: Option<i16>,
    pub kondisi: Option<String>,
    pub status: String,
    pub fungsi: Option<String>,
    pub geometry: serde_json::Value,
}

#[derive(Clone)]
pub struct Store {
    pool: PgPool,
}

impl Store {
    pub async fn connect(database_url: &str) -> Result<Self, StoreError> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn migrate(&self) -> Result<(), StoreError> {
        MIGRATOR.run(&self.pool).await?;
        Ok(())
    }

    pub async fn ping(&self) -> Result<(), StoreError> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }

    pub async fn list_networks(&self) -> Result<Vec<Network>, StoreError> {
        let rows = sqlx::query(
            "SELECT id, nama, unit_spam_id, desa_id, deskripsi FROM networks ORDER BY nama",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| Network {
                id: r.get("id"),
                nama: r.get("nama"),
                unit_spam_id: r.get("unit_spam_id"),
                desa_id: r.get("desa_id"),
                deskripsi: r.get("deskripsi"),
            })
            .collect())
    }

    /// Fitur jaringan dalam bbox, dengan filter jenis opsional. Fitur terhapus (soft delete) dilewati.
    pub async fn features_in_bbox(
        &self,
        network_id: i64,
        bbox: Bbox,
        types: Option<&[FeatureType]>,
    ) -> Result<Vec<FeatureRow>, StoreError> {
        let type_names: Option<Vec<String>> =
            types.map(|ts| ts.iter().map(|t| t.as_str().to_string()).collect());

        let rows = sqlx::query(
            "SELECT id, network_id, feature_type, nama, diameter_mm, material, tahun_pasang, \
                    kondisi, status, fungsi, ST_AsGeoJSON(geom)::text AS geometry \
             FROM features \
             WHERE network_id = $1 \
               AND deleted_at IS NULL \
               AND geom && ST_MakeEnvelope($2, $3, $4, $5, 4326) \
               AND ($6::text[] IS NULL OR feature_type = ANY($6)) \
             ORDER BY id \
             LIMIT $7",
        )
        .bind(network_id)
        .bind(bbox.west)
        .bind(bbox.south)
        .bind(bbox.east)
        .bind(bbox.north)
        .bind(type_names)
        .bind(MAX_FEATURES + 1)
        .fetch_all(&self.pool)
        .await?;

        if rows.len() as i64 > MAX_FEATURES {
            return Err(StoreError::TooManyFeatures);
        }
        rows.iter().map(feature_from_row).collect()
    }
}

fn feature_from_row(r: &PgRow) -> Result<FeatureRow, StoreError> {
    let geometry_text: String = r.try_get("geometry")?;
    let geometry = serde_json::from_str(&geometry_text)
        .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
    Ok(FeatureRow {
        id: r.try_get("id")?,
        network_id: r.try_get("network_id")?,
        feature_type: r.try_get("feature_type")?,
        nama: r.try_get("nama")?,
        diameter_mm: r.try_get("diameter_mm")?,
        material: r.try_get("material")?,
        tahun_pasang: r.try_get("tahun_pasang")?,
        kondisi: r.try_get("kondisi")?,
        status: r.try_get("status")?,
        fungsi: r.try_get("fungsi")?,
        geometry,
    })
}

#[derive(Debug, Clone)]
pub struct UserRow {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub role: String,
    pub active: bool,
}

impl Store {
    pub async fn find_user(&self, username: &str) -> Result<Option<UserRow>, StoreError> {
        let row = sqlx::query(
            "SELECT id, username, password_hash, role, active FROM users WHERE username = $1",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| UserRow {
            id: r.get("id"),
            username: r.get("username"),
            password_hash: r.get("password_hash"),
            role: r.get("role"),
            active: r.get("active"),
        }))
    }

    pub async fn create_user(
        &self,
        username: &str,
        password_hash: &str,
        role: &str,
    ) -> Result<i64, StoreError> {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO users (username, password_hash, role) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(username)
        .bind(password_hash)
        .bind(role)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }
}

/// Satu baris staging yang siap ditulis. Geometri dalam bentuk teks GeoJSON.
#[derive(Debug, Clone)]
pub struct StagingRow {
    pub feature_type: String,
    pub nama: Option<String>,
    pub diameter_mm: Option<i32>,
    pub geometry: String,
}

#[derive(Debug, Clone)]
pub struct BatchRow {
    pub id: i64,
    pub nama_file: String,
    pub status: String,
    pub network_id: Option<i64>,
    pub laporan: serde_json::Value,
    pub dibuat_oleh: String,
}

#[derive(Debug, PartialEq)]
pub enum CommitOutcome {
    Committed(u64),
    NotFound,
    AlreadyCommitted,
}

impl Store {
    pub async fn create_batch(
        &self,
        nama_file: &str,
        laporan: &serde_json::Value,
        dibuat_oleh: &str,
        rows: &[StagingRow],
    ) -> Result<i64, StoreError> {
        let mut tx = self.pool.begin().await?;
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO import_batches (nama_file, laporan, dibuat_oleh) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(nama_file)
        .bind(laporan)
        .bind(dibuat_oleh)
        .fetch_one(&mut *tx)
        .await?;

        for row in rows {
            sqlx::query(
                "INSERT INTO import_staging (batch_id, feature_type, nama, diameter_mm, geom) \
                 VALUES ($1, $2, $3, $4, ST_SetSRID(ST_GeomFromGeoJSON($5), 4326))",
            )
            .bind(id)
            .bind(&row.feature_type)
            .bind(&row.nama)
            .bind(row.diameter_mm)
            .bind(&row.geometry)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(id)
    }

    pub async fn get_batch(&self, id: i64) -> Result<Option<BatchRow>, StoreError> {
        let row = sqlx::query(
            "SELECT id, nama_file, status, network_id, laporan, dibuat_oleh FROM import_batches WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| BatchRow {
            id: r.get("id"),
            nama_file: r.get("nama_file"),
            status: r.get("status"),
            network_id: r.get("network_id"),
            laporan: r.get("laporan"),
            dibuat_oleh: r.get("dibuat_oleh"),
        }))
    }

    /// Memindahkan staging batch ke `features`. Batch yang sudah di-commit tidak bisa diulang.
    pub async fn commit_batch(&self, id: i64, network_id: i64) -> Result<CommitOutcome, StoreError> {
        let mut tx = self.pool.begin().await?;
        let status: Option<String> =
            sqlx::query_scalar("SELECT status FROM import_batches WHERE id = $1 FOR UPDATE")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
        match status.as_deref() {
            None => return Ok(CommitOutcome::NotFound),
            Some("committed") => return Ok(CommitOutcome::AlreadyCommitted),
            Some(_) => {}
        }

        let result = sqlx::query(
            "INSERT INTO features (network_id, feature_type, geom, nama, diameter_mm, import_batch_id) \
             SELECT $2, feature_type, geom, nama, diameter_mm, batch_id \
             FROM import_staging WHERE batch_id = $1",
        )
        .bind(id)
        .bind(network_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query("UPDATE import_batches SET status = 'committed', network_id = $2 WHERE id = $1")
            .bind(id)
            .bind(network_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(CommitOutcome::Committed(result.rows_affected()))
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FeatureCount {
    pub feature_type: String,
    pub jumlah: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PipaLength {
    pub fungsi: String,
    pub jumlah: i64,
    pub km: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ImportSummary {
    pub id: i64,
    pub nama_file: String,
    pub status: String,
    pub dibuat_oleh: String,
    pub jumlah_fitur: Option<i64>,
    pub waktu: String,
}

impl Store {
    /// Jumlah fitur aktif per jenis. Sambungan rumah hanya dihitung bila `include_customer` true.
    pub async fn feature_counts(&self, network_id: i64, include_customer: bool) -> Result<Vec<FeatureCount>, StoreError> {
        let rows = sqlx::query(
            "SELECT feature_type, COUNT(*) AS jumlah FROM features \
             WHERE network_id = $1 AND deleted_at IS NULL AND ($2 OR feature_type <> 'sr') \
             GROUP BY feature_type ORDER BY feature_type",
        )
        .bind(network_id)
        .bind(include_customer)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| FeatureCount { feature_type: r.get("feature_type"), jumlah: r.get("jumlah") })
            .collect())
    }

    /// Jumlah dan panjang geodetik pipa per fungsi. Pipa tanpa fungsi dihitung sebagai distribusi.
    pub async fn pipa_lengths(&self, network_id: i64) -> Result<Vec<PipaLength>, StoreError> {
        let rows = sqlx::query(
            "SELECT COALESCE(fungsi, 'distribusi') AS fungsi, COUNT(*) AS jumlah, \
                    COALESCE(SUM(ST_Length(geom::geography)), 0)::float8 / 1000.0 AS km \
             FROM features \
             WHERE network_id = $1 AND feature_type = 'pipa' AND deleted_at IS NULL \
             GROUP BY 1 ORDER BY 1",
        )
        .bind(network_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| PipaLength { fungsi: r.get("fungsi"), jumlah: r.get("jumlah"), km: r.get("km") })
            .collect())
    }

    pub async fn recent_imports(&self, limit: i64) -> Result<Vec<ImportSummary>, StoreError> {
        let rows = sqlx::query(
            "SELECT id, nama_file, status, dibuat_oleh, laporan->>'jumlah_fitur' AS jumlah_fitur, \
                    to_char(created_at, 'YYYY-MM-DD HH24:MI') AS waktu \
             FROM import_batches ORDER BY id DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| ImportSummary {
                id: r.get("id"),
                nama_file: r.get("nama_file"),
                status: r.get("status"),
                dibuat_oleh: r.get("dibuat_oleh"),
                jumlah_fitur: r.get::<Option<String>, _>("jumlah_fitur").and_then(|s| s.parse().ok()),
                waktu: r.get("waktu"),
            })
            .collect())
    }
}
