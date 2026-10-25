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

    /// Jumlah dan panjang pipa per fungsi (dari kolom `panjang_m`). Pipa tanpa fungsi dihitung sebagai distribusi.
    pub async fn pipa_lengths(&self, network_id: i64) -> Result<Vec<PipaLength>, StoreError> {
        let rows = sqlx::query(
            "SELECT COALESCE(fungsi, 'distribusi') AS fungsi, COUNT(*) AS jumlah, \
                    COALESCE(SUM(panjang_m), 0)::float8 / 1000.0 AS km \
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

#[derive(Debug, Clone, serde::Serialize)]
pub struct UserSummary {
    pub id: i64,
    pub username: String,
    pub role: String,
    pub active: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ImportAdminRow {
    pub id: i64,
    pub nama_file: String,
    pub status: String,
    pub network_id: Option<i64>,
    pub dibuat_oleh: String,
    pub jumlah_fitur: Option<i64>,
    pub waktu: String,
}

impl Store {
    pub async fn list_users(&self) -> Result<Vec<UserSummary>, StoreError> {
        let rows = sqlx::query(
            "SELECT id, username, role, active, to_char(created_at, 'YYYY-MM-DD HH24:MI') AS created_at \
             FROM users ORDER BY username",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| UserSummary {
                id: r.get("id"),
                username: r.get("username"),
                role: r.get("role"),
                active: r.get("active"),
                created_at: r.get("created_at"),
            })
            .collect())
    }

    /// Ubah peran dan/atau status aktif. Field yang None tidak diubah. Mengembalikan false bila pengguna tidak ada.
    pub async fn update_user(&self, id: i64, role: Option<&str>, active: Option<bool>) -> Result<bool, StoreError> {
        let result = sqlx::query(
            "UPDATE users SET role = COALESCE($2, role), active = COALESCE($3, active) WHERE id = $1",
        )
        .bind(id)
        .bind(role)
        .bind(active)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn create_network(&self, nama: &str, deskripsi: Option<&str>) -> Result<i64, StoreError> {
        let id: i64 = sqlx::query_scalar("INSERT INTO networks (nama, deskripsi) VALUES ($1, $2) RETURNING id")
            .bind(nama)
            .bind(deskripsi)
            .fetch_one(&self.pool)
            .await?;
        Ok(id)
    }

    /// Semua batch impor, termasuk yang belum disimpan, terbaru di atas.
    pub async fn list_imports(&self, limit: i64) -> Result<Vec<ImportAdminRow>, StoreError> {
        let rows = sqlx::query(
            "SELECT id, nama_file, status, network_id, dibuat_oleh, laporan->>'jumlah_fitur' AS jumlah_fitur, \
                    to_char(created_at, 'YYYY-MM-DD HH24:MI') AS waktu \
             FROM import_batches ORDER BY id DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| ImportAdminRow {
                id: r.get("id"),
                nama_file: r.get("nama_file"),
                status: r.get("status"),
                network_id: r.get("network_id"),
                dibuat_oleh: r.get("dibuat_oleh"),
                jumlah_fitur: r.get::<Option<String>, _>("jumlah_fitur").and_then(|s| s.parse().ok()),
                waktu: r.get("waktu"),
            })
            .collect())
    }
}

// ---------------------------------------------------------------------------
// Referensi SPM air minum (hasil sinkron Arumanis dan batas desa)
// ---------------------------------------------------------------------------

/// Satu poligon batas desa. `geometry` dalam teks GeoJSON (Polygon atau MultiPolygon).
#[derive(Debug, Clone)]
pub struct BatasInput {
    pub kode_bps: String,
    pub kecamatan: String,
    pub desa: String,
    pub geometry: String,
}

#[derive(Debug, Clone)]
pub struct DesaRefInput {
    pub id: i64,
    pub nama: String,
    pub kecamatan: String,
    pub target_kk: i64,
    pub jp_kk: i64,
    pub bjp_master: i64,
    pub bjp_unit: i64,
    pub sr: i64,
    pub jiwa: i64,
    pub unit_count: i64,
}

#[derive(Debug, Clone)]
pub struct UnitRefInput {
    pub id: i64,
    pub desa_id: i64,
    pub nama: Option<String>,
    pub sistem_layanan: Option<String>,
    pub sumber_dana: Option<String>,
    pub is_simspam: bool,
    pub sr: i64,
    pub kk: i64,
    pub jiwa: i64,
    pub bjp_kk: i64,
}

#[derive(Debug, Clone)]
pub struct PokmasRefInput {
    pub desa_id: i64,
    pub nama: Option<String>,
    pub perdes: Option<String>,
    pub kepala: Option<String>,
    pub bendahara: Option<String>,
    pub sekretaris: Option<String>,
    pub konflik: bool,
    pub sumber_unit_ids: Vec<i64>,
}

/// Hasil penerapan sinkron.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ApplyResult {
    pub jumlah_desa: i64,
    pub jumlah_unit: i64,
    pub jumlah_pokmas: i64,
    /// Unit yang desanya tidak ada di daftar desa Arumanis, sehingga dilewati.
    pub unit_dilewati: i64,
    pub desa_tanpa_batas: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DesaSummary {
    pub id: i64,
    pub nama: String,
    pub kecamatan: String,
    pub kode_bps: Option<String>,
    pub punya_batas: bool,
    pub target_kk: i64,
    pub jp_kk: i64,
    pub bjp_kk: i64,
    pub sr: i64,
    pub jiwa: i64,
    pub unit_count: i64,
    /// (JP + BJP) / target × 100. Null bila target 0.
    pub cakupan_persen: Option<f64>,
    pub punya_pokmas: bool,
    pub pokmas_konflik: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SyncRunRow {
    pub id: i64,
    pub dipicu_oleh: String,
    pub mulai_at: String,
    pub selesai_at: Option<String>,
    pub status: String,
    pub jumlah_desa: Option<i32>,
    pub jumlah_unit: Option<i32>,
    pub jumlah_pokmas: Option<i32>,
    pub desa_tanpa_batas: Option<i32>,
    pub pesan: Option<String>,
}

/// Cocokkan desa ke poligon: koreksi manual dulu, lalu nama kecamatan + desa yang dinormalisasi.
const MATCH_DESA_SQL: &str = "\
UPDATE ref_desa d SET kode_bps = m.kode FROM (
    SELECT d2.id,
           CASE WHEN o.desa_id IS NOT NULL
                THEN (SELECT b2.kode_bps FROM ref_batas_desa b2 WHERE b2.kode_bps = o.kode_bps)
                ELSE (SELECT b.kode_bps FROM ref_batas_desa b
                      WHERE norm_wilayah(b.kecamatan) = norm_wilayah(d2.kecamatan)
                        AND norm_wilayah(b.desa) = norm_wilayah(d2.nama)
                      ORDER BY b.kode_bps LIMIT 1)
           END AS kode
    FROM ref_desa d2
    LEFT JOIN ref_desa_kode_override o ON o.desa_id = d2.id
) m
WHERE m.id = d.id AND d.kode_bps IS DISTINCT FROM m.kode";

async fn count_desa_tanpa_batas(conn: &mut sqlx::PgConnection) -> Result<i64, StoreError> {
    Ok(sqlx::query_scalar("SELECT COUNT(*) FROM ref_desa WHERE aktif AND kode_bps IS NULL")
        .fetch_one(conn)
        .await?)
}

impl Store {
    /// Ganti seluruh batas desa, lalu cocokkan ulang desa ke poligon. Mengembalikan (jumlah poligon, desa tanpa batas).
    pub async fn replace_batas_desa(&self, rows: &[BatasInput], sumber: &str) -> Result<(i64, i64), StoreError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM ref_batas_desa").execute(&mut *tx).await?;
        for r in rows {
            sqlx::query(
                "INSERT INTO ref_batas_desa (kode_bps, kecamatan, desa, geom, sumber) \
                 VALUES ($1, $2, $3, ST_Multi(ST_SetSRID(ST_GeomFromGeoJSON($4), 4326)), $5)",
            )
            .bind(&r.kode_bps)
            .bind(&r.kecamatan)
            .bind(&r.desa)
            .bind(&r.geometry)
            .bind(sumber)
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query(MATCH_DESA_SQL).execute(&mut *tx).await?;
        let tanpa = count_desa_tanpa_batas(&mut tx).await?;
        tx.commit().await?;
        Ok((rows.len() as i64, tanpa))
    }

    /// Terapkan hasil sinkron dalam satu transaksi. `units` None berarti unit dan Pokmas tidak disentuh
    /// (misalnya token Arumanis belum diatur). Baris yang tidak lagi ada di Arumanis ditandai tidak aktif.
    pub async fn apply_sync(
        &self,
        desa: &[DesaRefInput],
        units: Option<(&[UnitRefInput], &[PokmasRefInput])>,
    ) -> Result<ApplyResult, StoreError> {
        let mut tx = self.pool.begin().await?;
        let mut result = ApplyResult::default();

        sqlx::query("UPDATE ref_desa SET aktif = false").execute(&mut *tx).await?;
        for d in desa {
            sqlx::query(
                "INSERT INTO ref_desa (id, nama, kecamatan, target_kk, jp_kk, bjp_master, bjp_unit, sr, jiwa, unit_count, aktif, synced_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, true, now()) \
                 ON CONFLICT (id) DO UPDATE SET nama = EXCLUDED.nama, kecamatan = EXCLUDED.kecamatan, \
                   target_kk = EXCLUDED.target_kk, jp_kk = EXCLUDED.jp_kk, bjp_master = EXCLUDED.bjp_master, \
                   bjp_unit = EXCLUDED.bjp_unit, sr = EXCLUDED.sr, jiwa = EXCLUDED.jiwa, \
                   unit_count = EXCLUDED.unit_count, aktif = true, synced_at = now()",
            )
            .bind(d.id)
            .bind(&d.nama)
            .bind(&d.kecamatan)
            .bind(d.target_kk as i32)
            .bind(d.jp_kk as i32)
            .bind(d.bjp_master as i32)
            .bind(d.bjp_unit as i32)
            .bind(d.sr as i32)
            .bind(d.jiwa as i32)
            .bind(d.unit_count as i32)
            .execute(&mut *tx)
            .await?;
        }
        result.jumlah_desa = desa.len() as i64;

        if let Some((units, pokmas)) = units {
            let known: std::collections::HashSet<i64> = desa.iter().map(|d| d.id).collect();
            sqlx::query("UPDATE ref_unit_spam SET aktif = false").execute(&mut *tx).await?;
            for u in units {
                if !known.contains(&u.desa_id) {
                    result.unit_dilewati += 1;
                    continue;
                }
                sqlx::query(
                    "INSERT INTO ref_unit_spam (id, desa_id, nama, sistem_layanan, sumber_dana, is_simspam, sr, kk, jiwa, bjp_kk, aktif, synced_at) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, true, now()) \
                     ON CONFLICT (id) DO UPDATE SET desa_id = EXCLUDED.desa_id, nama = EXCLUDED.nama, \
                       sistem_layanan = EXCLUDED.sistem_layanan, sumber_dana = EXCLUDED.sumber_dana, \
                       is_simspam = EXCLUDED.is_simspam, sr = EXCLUDED.sr, kk = EXCLUDED.kk, jiwa = EXCLUDED.jiwa, \
                       bjp_kk = EXCLUDED.bjp_kk, aktif = true, synced_at = now()",
                )
                .bind(u.id)
                .bind(u.desa_id)
                .bind(&u.nama)
                .bind(&u.sistem_layanan)
                .bind(&u.sumber_dana)
                .bind(u.is_simspam)
                .bind(u.sr as i32)
                .bind(u.kk as i32)
                .bind(u.jiwa as i32)
                .bind(u.bjp_kk as i32)
                .execute(&mut *tx)
                .await?;
                result.jumlah_unit += 1;
            }

            sqlx::query("DELETE FROM ref_pokmas").execute(&mut *tx).await?;
            for p in pokmas.iter().filter(|p| known.contains(&p.desa_id)) {
                sqlx::query(
                    "INSERT INTO ref_pokmas (desa_id, nama, perdes, kepala, bendahara, sekretaris, konflik, sumber_unit_ids, synced_at) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now())",
                )
                .bind(p.desa_id)
                .bind(&p.nama)
                .bind(&p.perdes)
                .bind(&p.kepala)
                .bind(&p.bendahara)
                .bind(&p.sekretaris)
                .bind(p.konflik)
                .bind(&p.sumber_unit_ids)
                .execute(&mut *tx)
                .await?;
                result.jumlah_pokmas += 1;
            }
        }

        sqlx::query(MATCH_DESA_SQL).execute(&mut *tx).await?;
        result.desa_tanpa_batas = count_desa_tanpa_batas(&mut tx).await?;
        tx.commit().await?;
        Ok(result)
    }

    pub async fn start_sync_run(&self, dipicu_oleh: &str) -> Result<i64, StoreError> {
        Ok(sqlx::query_scalar("INSERT INTO sync_runs (dipicu_oleh) VALUES ($1) RETURNING id")
            .bind(dipicu_oleh)
            .fetch_one(&self.pool)
            .await?)
    }

    pub async fn finish_sync_run(
        &self,
        id: i64,
        status: &str,
        result: Option<&ApplyResult>,
        pesan: Option<&str>,
    ) -> Result<(), StoreError> {
        sqlx::query(
            "UPDATE sync_runs SET selesai_at = now(), status = $2, jumlah_desa = $3, jumlah_unit = $4, \
             jumlah_pokmas = $5, desa_tanpa_batas = $6, pesan = $7 WHERE id = $1",
        )
        .bind(id)
        .bind(status)
        .bind(result.map(|r| r.jumlah_desa as i32))
        .bind(result.map(|r| r.jumlah_unit as i32))
        .bind(result.map(|r| r.jumlah_pokmas as i32))
        .bind(result.map(|r| r.desa_tanpa_batas as i32))
        .bind(pesan)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_sync_runs(&self, limit: i64) -> Result<Vec<SyncRunRow>, StoreError> {
        let rows = sqlx::query(
            "SELECT id, dipicu_oleh, to_char(mulai_at, 'YYYY-MM-DD HH24:MI:SS') AS mulai_at, \
                    to_char(selesai_at, 'YYYY-MM-DD HH24:MI:SS') AS selesai_at, status, jumlah_desa, jumlah_unit, \
                    jumlah_pokmas, desa_tanpa_batas, pesan \
             FROM sync_runs ORDER BY id DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| SyncRunRow {
                id: r.get("id"),
                dipicu_oleh: r.get("dipicu_oleh"),
                mulai_at: r.get("mulai_at"),
                selesai_at: r.get("selesai_at"),
                status: r.get("status"),
                jumlah_desa: r.get("jumlah_desa"),
                jumlah_unit: r.get("jumlah_unit"),
                jumlah_pokmas: r.get("jumlah_pokmas"),
                desa_tanpa_batas: r.get("desa_tanpa_batas"),
                pesan: r.get("pesan"),
            })
            .collect())
    }

    /// Sinkron yang masih tercatat berjalan lebih dari 30 menit dianggap gagal (misalnya server mati di tengah).
    pub async fn close_stale_sync_runs(&self) -> Result<u64, StoreError> {
        Ok(sqlx::query(
            "UPDATE sync_runs SET status = 'gagal', selesai_at = now(), pesan = 'terhenti (server berhenti sebelum selesai)' \
             WHERE status = 'berjalan' AND mulai_at < now() - interval '30 minutes'",
        )
        .execute(&self.pool)
        .await?
        .rows_affected())
    }

    /// Daftar desa aktif dengan capaian SPM. Tidak memuat nama pengurus Pokmas.
    pub async fn list_desa(&self) -> Result<Vec<DesaSummary>, StoreError> {
        let rows = sqlx::query(
            "SELECT d.id, d.nama, d.kecamatan, d.kode_bps, d.target_kk, d.jp_kk, d.bjp_master + d.bjp_unit AS bjp_kk, \
                    d.sr, d.jiwa, d.unit_count, \
                    (p.nama IS NOT NULL) AS punya_pokmas, COALESCE(p.konflik, false) AS pokmas_konflik \
             FROM ref_desa d LEFT JOIN ref_pokmas p ON p.desa_id = d.id \
             WHERE d.aktif ORDER BY d.kecamatan, d.nama",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| {
                let target: i32 = r.get("target_kk");
                let jp: i32 = r.get("jp_kk");
                let bjp: i32 = r.get("bjp_kk");
                let kode: Option<String> = r.get("kode_bps");
                DesaSummary {
                    id: r.get("id"),
                    nama: r.get("nama"),
                    kecamatan: r.get("kecamatan"),
                    punya_batas: kode.is_some(),
                    kode_bps: kode,
                    target_kk: target.into(),
                    jp_kk: jp.into(),
                    bjp_kk: bjp.into(),
                    sr: r.get::<i32, _>("sr").into(),
                    jiwa: r.get::<i32, _>("jiwa").into(),
                    unit_count: r.get::<i32, _>("unit_count").into(),
                    cakupan_persen: (target > 0)
                        .then(|| ((jp + bjp) as f64 / target as f64 * 10_000.0).round() / 100.0),
                    punya_pokmas: r.get("punya_pokmas"),
                    pokmas_konflik: r.get("pokmas_konflik"),
                }
            })
            .collect())
    }
}
