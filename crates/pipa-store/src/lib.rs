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
                    kondisi, status, ST_AsGeoJSON(geom)::text AS geometry \
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
        geometry,
    })
}
