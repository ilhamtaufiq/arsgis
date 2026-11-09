//! Penulisan aset (tambah, ubah, hapus) beserta riwayatnya. Setiap perubahan dan baris riwayatnya
//! ditulis dalam satu transaksi. Aturan atribut dan geometri ada di pipa-core. Pemeriksaan batas desa
//! dilakukan di sini karena memerlukan poligon dari `ref_batas_desa`.

use pipa_core::impor::periksa_pulih;
use pipa_core::{
    normalisasi_geometri, periksa_atribut, periksa_kondisi, AsetError, AtributAset, AtributPatch, FeatureType,
};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::postgres::PgRow;
use sqlx::{Postgres, Row, Transaction};

use crate::{Store, StoreError};

/// Kolom fitur dalam urutan yang dibaca `fitur_dari_baris`. Geometri diambil sebagai GeoJSON.
pub(crate) const FITUR_COLS: &str = "id, network_id, feature_type, nama, diameter_mm, material, tahun_pasang, kondisi, \
    status, fungsi, kapasitas_m3::float8 AS kapasitas_m3, debit_lps::float8 AS debit_lps, \
    elevasi_m::float8 AS elevasi_m, tahun_bangun, sumber_dana, kode_aset, pekerjaan_id, \
    (SELECT nama_paket FROM ref_pekerjaan rp WHERE rp.id = features.pekerjaan_id) AS pekerjaan_nama, \
    ST_AsGeoJSON(geom)::text AS geometry";

/// Satu fitur lengkap dengan atributnya, seperti yang tersimpan di database.
#[derive(Debug, Clone)]
pub struct FiturLengkap {
    pub id: i64,
    pub network_id: i64,
    pub feature_type: String,
    /// Kode aset unik, tidak pernah berubah setelah dibuat.
    pub kode_aset: String,
    pub pekerjaan_id: Option<i64>,
    pub pekerjaan_nama: Option<String>,
    pub geometry: Value,
    pub fungsi: Option<String>,
    pub atribut: AtributAset,
}

/// Isian tambah aset yang sudah dipisah dari body HTTP.
#[derive(Debug, Clone)]
pub struct AsetBaru {
    pub network_id: i64,
    pub feature_type: FeatureType,
    pub geometry: Value,
    pub atribut: AtributAset,
}

/// Isian ubah aset. `geometry` None berarti geometri tidak dipindah.
#[derive(Debug, Clone, Default)]
pub struct AsetUbah {
    pub geometry: Option<Value>,
    pub atribut: AtributPatch,
}

/// Hasil penulisan. `Ditolak` berisi alasan dari aturan (400 untuk masukan, 422 untuk aturan bisnis).
#[derive(Debug, PartialEq)]
pub enum TulisHasil<T> {
    Berhasil(T),
    TidakAda,
    Ditolak(AsetError),
}

/// Satu baris riwayat. `sebelum` dan `sesudah` berbentuk GeoJSON Feature (lihat `fitur_geojson`).
#[derive(Debug, Clone, Serialize)]
pub struct RiwayatAset {
    pub aksi: String,
    pub diubah_oleh: String,
    pub diubah_pada: String,
    pub sebelum: Option<Value>,
    pub sesudah: Option<Value>,
}

/// Bentuk fitur yang dikirim ke API dan disimpan di riwayat: GeoJSON Feature dengan properti yang sama
/// dengan endpoint GeoJSON untuk peta.
pub fn fitur_geojson(f: &FiturLengkap) -> Value {
    let a = &f.atribut;
    json!({
        "type": "Feature",
        "id": f.id,
        "geometry": f.geometry,
        "properties": {
            "network_id": f.network_id,
            "feature_type": f.feature_type,
            "nama": a.nama,
            "diameter_mm": a.diameter_mm,
            "material": a.material,
            "tahun_pasang": a.tahun_pasang,
            "kondisi": a.kondisi,
            "status": a.status,
            "fungsi": f.fungsi,
            "kapasitas_m3": a.kapasitas_m3,
            "debit_lps": a.debit_lps,
            "elevasi_m": a.elevasi_m,
            "tahun_bangun": a.tahun_bangun,
            "sumber_dana": a.sumber_dana,
            "kode_aset": f.kode_aset,
            "pekerjaan_id": f.pekerjaan_id,
            "pekerjaan_nama": f.pekerjaan_nama,
        },
    })
}

pub(crate) fn fitur_dari_baris(r: &PgRow) -> Result<FiturLengkap, StoreError> {
    let geometry_text: String = r.try_get("geometry")?;
    let geometry = serde_json::from_str(&geometry_text).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
    Ok(FiturLengkap {
        id: r.try_get("id")?,
        network_id: r.try_get("network_id")?,
        feature_type: r.try_get("feature_type")?,
        kode_aset: r.try_get("kode_aset")?,
        pekerjaan_id: r.try_get("pekerjaan_id")?,
        pekerjaan_nama: r.try_get("pekerjaan_nama")?,
        geometry,
        fungsi: r.try_get("fungsi")?,
        atribut: AtributAset {
            nama: r.try_get("nama")?,
            diameter_mm: r.try_get("diameter_mm")?,
            material: r.try_get("material")?,
            tahun_pasang: r.try_get("tahun_pasang")?,
            kondisi: r.try_get("kondisi")?,
            status: Some(r.try_get("status")?),
            kapasitas_m3: r.try_get("kapasitas_m3")?,
            debit_lps: r.try_get("debit_lps")?,
            elevasi_m: r.try_get("elevasi_m")?,
            tahun_bangun: r.try_get("tahun_bangun")?,
            sumber_dana: r.try_get("sumber_dana")?,
        },
    })
}

/// Hasil pemeriksaan batas desa untuk satu unit.
enum BatasCek {
    TidakAda,
    BukanUnit,
    TanpaBatas(String),
    Dalam,
    Luar,
}

/// Geometri harus tertutup (`ST_Covers`) oleh poligon desa unit. Titik di tepi batas dihitung dalam.
/// Garis harus seluruhnya di dalam, tidak cukup separuh. Berbeda dengan impor, yang masih menerima pipa
/// transmisi yang melintasi desa tetangga.
async fn cek_batas(
    tx: &mut Transaction<'_, Postgres>,
    network_id: i64,
    geometry: &Value,
) -> Result<BatasCek, StoreError> {
    let row = sqlx::query(
        "SELECT n.unit_spam_id IS NOT NULL AND n.desa_id IS NOT NULL AS unit, d.nama AS desa, \
                b.kode_bps IS NOT NULL AS punya_batas, \
                COALESCE(ST_Covers(b.geom, g.geom), false) AS dalam \
         FROM networks n \
         LEFT JOIN ref_desa d ON d.id = n.desa_id \
         LEFT JOIN ref_batas_desa b ON b.kode_bps = d.kode_bps \
         CROSS JOIN (SELECT ST_SetSRID(ST_GeomFromGeoJSON($2), 4326) AS geom) g \
         WHERE n.id = $1",
    )
    .bind(network_id)
    .bind(geometry.to_string())
    .fetch_optional(&mut **tx)
    .await?;

    let Some(row) = row else {
        return Ok(BatasCek::TidakAda);
    };
    if !row.try_get::<bool, _>("unit")? {
        return Ok(BatasCek::BukanUnit);
    }
    if !row.try_get::<bool, _>("punya_batas")? {
        let desa: Option<String> = row.try_get("desa")?;
        return Ok(BatasCek::TanpaBatas(desa.unwrap_or_default()));
    }
    Ok(if row.try_get::<bool, _>("dalam")? { BatasCek::Dalam } else { BatasCek::Luar })
}

/// Menerjemahkan hasil `cek_batas` menjadi penolakan, atau None bila geometri boleh disimpan.
fn tolak_bila_di_luar(cek: BatasCek) -> Option<AsetError> {
    match cek {
        BatasCek::Dalam => None,
        BatasCek::TidakAda | BatasCek::BukanUnit => Some(AsetError::Aturan(vec![
            "jaringan ini bukan unit SPAM yang punya desa, sehingga batas wilayah tidak bisa diperiksa".into(),
        ])),
        BatasCek::TanpaBatas(desa) => Some(AsetError::Aturan(vec![format!(
            "desa {desa} belum punya batas wilayah, sehingga aset tidak bisa ditambah atau dipindah"
        )])),
        BatasCek::Luar => Some(AsetError::Aturan(vec![
            "geometri berada di luar batas desa unit".into(),
        ])),
    }
}

/// Pelanggaran CHECK di database (SQLSTATE 23514) dianggap aturan bisnis, bukan kesalahan internal.
fn is_check_violation(e: &StoreError) -> bool {
    matches!(e, StoreError::Db(sqlx::Error::Database(db)) if db.code().as_deref() == Some("23514"))
}

impl Store {
    /// Tambah aset baru ke unit. Menulis riwayat `tambah` dalam transaksi yang sama.
    pub async fn tambah_aset(&self, baru: AsetBaru, oleh: &str) -> Result<TulisHasil<FiturLengkap>, StoreError> {
        let geometry = match normalisasi_geometri(baru.feature_type, &baru.geometry) {
            Ok(g) => g,
            Err(e) => return Ok(TulisHasil::Ditolak(e)),
        };
        let atribut = baru.atribut.clone().normalisasi();
        if let Some(k) = atribut.kondisi.as_deref() {
            if let Err(e) = periksa_kondisi(k) {
                return Ok(TulisHasil::Ditolak(e));
            }
        }
        if let Err(e) = periksa_atribut(baru.feature_type, &atribut) {
            return Ok(TulisHasil::Ditolak(e));
        }

        match self.tambah_dalam_tx(&baru, &geometry, &atribut, oleh).await {
            Err(e) if is_check_violation(&e) => {
                Ok(TulisHasil::Ditolak(AsetError::Aturan(vec!["isian melanggar aturan tabel (kondisi atau nilai lain)".into()])))
            }
            hasil => hasil,
        }
    }

    async fn tambah_dalam_tx(
        &self,
        baru: &AsetBaru,
        geometry: &Value,
        a: &AtributAset,
        oleh: &str,
    ) -> Result<TulisHasil<FiturLengkap>, StoreError> {
        let mut tx = self.pool.begin().await?;
        let cek = cek_batas(&mut tx, baru.network_id, geometry).await?;
        if matches!(cek, BatasCek::TidakAda) {
            return Ok(TulisHasil::TidakAda);
        }
        if let Some(e) = tolak_bila_di_luar(cek) {
            return Ok(TulisHasil::Ditolak(e));
        }

        // kode_aset tidak diisi di sini. Trigger `features_isi_kode_aset` (migrasi 0014) membuatnya dalam statement ini.
        let row = sqlx::query(&format!(
            "INSERT INTO features (network_id, feature_type, geom, nama, diameter_mm, material, tahun_pasang, \
                kondisi, status, kapasitas_m3, debit_lps, elevasi_m, tahun_bangun, sumber_dana) \
             VALUES ($1, $2, ST_SetSRID(ST_GeomFromGeoJSON($3), 4326), $4, $5, $6, $7, $8, COALESCE($9, 'aktif'), \
                $10, $11, $12, $13, $14) \
             RETURNING {FITUR_COLS}"
        ))
        .bind(baru.network_id)
        .bind(baru.feature_type.as_str())
        .bind(geometry.to_string())
        .bind(&a.nama)
        .bind(a.diameter_mm)
        .bind(&a.material)
        .bind(a.tahun_pasang)
        .bind(&a.kondisi)
        .bind(&a.status)
        .bind(a.kapasitas_m3)
        .bind(a.debit_lps)
        .bind(a.elevasi_m)
        .bind(a.tahun_bangun)
        .bind(&a.sumber_dana)
        .fetch_one(&mut *tx)
        .await?;
        let fitur = fitur_dari_baris(&row)?;

        sqlx::query(
            "INSERT INTO feature_riwayat (feature_id, aksi, diubah_oleh, sesudah) VALUES ($1, 'tambah', $2, $3)",
        )
        .bind(fitur.id)
        .bind(oleh)
        .bind(fitur_geojson(&fitur))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(TulisHasil::Berhasil(fitur))
    }

    /// Ubah atribut dan/atau geometri. Atribut digabung dengan keadaan sekarang, lalu seluruhnya diperiksa.
    /// Baris fitur dikunci (`FOR UPDATE`) sejak dibaca sampai riwayat ditulis, supaya `sebelum` akurat.
    /// Bila tidak ada yang berubah, tidak ada baris riwayat yang ditulis.
    pub async fn ubah_aset(&self, id: i64, ubah: AsetUbah, oleh: &str) -> Result<TulisHasil<FiturLengkap>, StoreError> {
        match self.ubah_dalam_tx(id, ubah, oleh).await {
            Err(e) if is_check_violation(&e) => {
                Ok(TulisHasil::Ditolak(AsetError::Aturan(vec!["isian melanggar aturan tabel (kondisi atau nilai lain)".into()])))
            }
            hasil => hasil,
        }
    }

    async fn ubah_dalam_tx(&self, id: i64, ubah: AsetUbah, oleh: &str) -> Result<TulisHasil<FiturLengkap>, StoreError> {
        let mut tx = self.pool.begin().await?;
        let Some(row) = sqlx::query(&format!(
            "SELECT {FITUR_COLS} FROM features WHERE id = $1 AND deleted_at IS NULL FOR UPDATE"
        ))
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(TulisHasil::TidakAda);
        };
        let sebelum = fitur_dari_baris(&row)?;
        let jenis = FeatureType::parse(&sebelum.feature_type).map_err(|e| StoreError::Data(e.to_string()))?;

        let menyentuh_kondisi = ubah.atribut.menyentuh_kondisi();
        let mut gabungan = sebelum.atribut.clone();
        ubah.atribut.terapkan(&mut gabungan);
        let gabungan = gabungan.normalisasi();
        // Kondisi lama berbentuk teks bebas. Hanya nilai yang ikut diubah yang diperiksa, sama seperti pemicu.
        if menyentuh_kondisi {
            if let Some(k) = gabungan.kondisi.as_deref() {
                if let Err(e) = periksa_kondisi(k) {
                    return Ok(TulisHasil::Ditolak(e));
                }
            }
        }
        if let Err(e) = periksa_atribut(jenis, &gabungan) {
            return Ok(TulisHasil::Ditolak(e));
        }

        let geometry_baru = match ubah.geometry {
            Some(g) => {
                let normal = match normalisasi_geometri(jenis, &g) {
                    Ok(n) => n,
                    Err(e) => return Ok(TulisHasil::Ditolak(e)),
                };
                // Pindah fitur harus lewat pemeriksaan batas desa yang sama dengan tambah.
                let cek = cek_batas(&mut tx, sebelum.network_id, &normal).await?;
                if let Some(e) = tolak_bila_di_luar(cek) {
                    return Ok(TulisHasil::Ditolak(e));
                }
                Some(normal)
            }
            None => None,
        };

        let sesudah_baris = sqlx::query(&format!(
            "UPDATE features SET nama = $2, diameter_mm = $3, material = $4, tahun_pasang = $5, kondisi = $6, \
                status = COALESCE($7, 'aktif'), kapasitas_m3 = $8, debit_lps = $9, elevasi_m = $10, \
                tahun_bangun = $11, sumber_dana = $12, \
                geom = COALESCE(ST_SetSRID(ST_GeomFromGeoJSON($13), 4326), geom) \
             WHERE id = $1 \
             RETURNING {FITUR_COLS}"
        ))
        .bind(id)
        .bind(&gabungan.nama)
        .bind(gabungan.diameter_mm)
        .bind(&gabungan.material)
        .bind(gabungan.tahun_pasang)
        .bind(&gabungan.kondisi)
        .bind(&gabungan.status)
        .bind(gabungan.kapasitas_m3)
        .bind(gabungan.debit_lps)
        .bind(gabungan.elevasi_m)
        .bind(gabungan.tahun_bangun)
        .bind(&gabungan.sumber_dana)
        .bind(geometry_baru.as_ref().map(Value::to_string))
        .fetch_one(&mut *tx)
        .await?;
        let sesudah = fitur_dari_baris(&sesudah_baris)?;

        let snap_sebelum = fitur_geojson(&sebelum);
        let snap_sesudah = fitur_geojson(&sesudah);
        if snap_sebelum != snap_sesudah {
            sqlx::query(
                "INSERT INTO feature_riwayat (feature_id, aksi, diubah_oleh, sebelum, sesudah) \
                 VALUES ($1, 'ubah', $2, $3, $4)",
            )
            .bind(id)
            .bind(oleh)
            .bind(snap_sebelum)
            .bind(snap_sesudah)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(TulisHasil::Berhasil(sesudah))
    }

    /// Hapus lunak: mengisi `deleted_at`. Baris riwayat `hapus` menyimpan keadaan sebelum hapus.
    pub async fn hapus_aset(&self, id: i64, oleh: &str) -> Result<TulisHasil<()>, StoreError> {
        let mut tx = self.pool.begin().await?;
        let Some(row) = sqlx::query(&format!(
            "SELECT {FITUR_COLS} FROM features WHERE id = $1 AND deleted_at IS NULL FOR UPDATE"
        ))
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(TulisHasil::TidakAda);
        };
        let fitur = fitur_dari_baris(&row)?;

        sqlx::query("UPDATE features SET deleted_at = now() WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO feature_riwayat (feature_id, aksi, diubah_oleh, sebelum) VALUES ($1, 'hapus', $2, $3)",
        )
        .bind(id)
        .bind(oleh)
        .bind(fitur_geojson(&fitur))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(TulisHasil::Berhasil(()))
    }

    /// Riwayat satu fitur, terbaru di atas. None bila id fitur tidak ada. Fitur yang sudah dihapus tetap dibaca.
    pub async fn riwayat_aset(&self, id: i64) -> Result<Option<Vec<RiwayatAset>>, StoreError> {
        let ada: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM features WHERE id = $1)")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
        if !ada {
            return Ok(None);
        }
        let rows = sqlx::query(
            "SELECT aksi, diubah_oleh, to_char(diubah_pada, 'YYYY-MM-DD\"T\"HH24:MI:SSOF') AS diubah_pada, \
                    sebelum, sesudah \
             FROM feature_riwayat WHERE feature_id = $1 ORDER BY diubah_pada DESC, id DESC",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;
        Ok(Some(
            rows.iter()
                .map(|r| RiwayatAset {
                    aksi: r.get("aksi"),
                    diubah_oleh: r.get("diubah_oleh"),
                    diubah_pada: r.get("diubah_pada"),
                    sebelum: r.get("sebelum"),
                    sesudah: r.get("sesudah"),
                })
                .collect(),
        ))
    }
}

/// Hasil pemulihan aset yang terhapus.
#[derive(Debug)]
pub enum PulihHasil {
    Berhasil(FiturLengkap),
    TidakAda,
    /// Aset tidak sedang dihapus.
    TidakTerhapus,
    /// Aset tidak lagi memenuhi aturan (batas desa, atribut wajib, atau nilai). Selalu 422.
    Ditolak(AsetError),
}

/// Baris aset terhapus untuk daftar pemulihan.
#[derive(Debug, Clone, Serialize)]
pub struct TerhapusRow {
    pub id: i64,
    pub kode_aset: String,
    pub nama: Option<String>,
    pub jenis: String,
    pub deleted_at: String,
    pub network_id: i64,
}

/// Memeriksa ulang aturan tambah dan ubah terhadap keadaan aset yang akan dipulihkan. Mengembalikan
/// penolakan (selalu sebagai `Aturan`) atau None bila aset masih valid. Masukan yang tidak valid ikut menjadi 422.
async fn periksa_ulang_pulih(
    tx: &mut Transaction<'_, Postgres>,
    fitur: &FiturLengkap,
) -> Result<Option<AsetError>, StoreError> {
    let jenis = FeatureType::parse(&fitur.feature_type).map_err(|e| StoreError::Data(e.to_string()))?;
    let geometry = match normalisasi_geometri(jenis, &fitur.geometry) {
        Ok(g) => g,
        Err(e) => return Ok(Some(hanya_aturan(e))),
    };
    if let Some(e) = tolak_bila_di_luar(cek_batas(tx, fitur.network_id, &geometry).await?) {
        return Ok(Some(e));
    }
    let atribut = fitur.atribut.clone().normalisasi();
    if let Some(k) = atribut.kondisi.as_deref() {
        if let Err(e) = periksa_kondisi(k) {
            return Ok(Some(e));
        }
    }
    if let Err(e) = periksa_atribut(jenis, &atribut) {
        return Ok(Some(hanya_aturan(e)));
    }
    Ok(None)
}

/// Semua penolakan pemulihan dibuat 422, jadi kesalahan masukan dinaikkan menjadi aturan.
fn hanya_aturan(e: AsetError) -> AsetError {
    match e {
        AsetError::Masukan(m) => AsetError::Aturan(vec![m]),
        aturan => aturan,
    }
}

impl Store {
    /// Pulihkan aset yang dihapus (soft delete). Batas desa dan atribut wajib diperiksa ulang dengan aturan yang sama
    /// dengan tambah dan ubah. Baris riwayat `pulihkan` menyimpan keadaan saat terhapus (`sebelum`) dan sesudah pulih.
    pub async fn pulihkan_aset(&self, id: i64, oleh: &str) -> Result<PulihHasil, StoreError> {
        let mut tx = self.pool.begin().await?;
        let Some(row) = sqlx::query(&format!(
            "SELECT {FITUR_COLS}, deleted_at IS NOT NULL AS terhapus FROM features WHERE id = $1 FOR UPDATE"
        ))
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(PulihHasil::TidakAda);
        };
        let terhapus: bool = row.try_get("terhapus")?;
        if periksa_pulih(terhapus).is_err() {
            return Ok(PulihHasil::TidakTerhapus);
        }
        let sebelum = fitur_dari_baris(&row)?;
        if let Some(e) = periksa_ulang_pulih(&mut tx, &sebelum).await? {
            return Ok(PulihHasil::Ditolak(e));
        }

        let sesudah_row = sqlx::query(&format!(
            "UPDATE features SET deleted_at = NULL WHERE id = $1 RETURNING {FITUR_COLS}"
        ))
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        let sesudah = fitur_dari_baris(&sesudah_row)?;
        sqlx::query(
            "INSERT INTO feature_riwayat (feature_id, aksi, diubah_oleh, sebelum, sesudah) \
             VALUES ($1, 'pulihkan', $2, $3, $4)",
        )
        .bind(id)
        .bind(oleh)
        .bind(fitur_geojson(&sebelum))
        .bind(fitur_geojson(&sesudah))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(PulihHasil::Berhasil(sesudah))
    }

    /// Aset yang sedang terhapus, terbaru dihapus di atas. Mengembalikan halaman dan jumlah seluruhnya.
    pub async fn daftar_terhapus(&self, limit: i64, offset: i64) -> Result<(Vec<TerhapusRow>, i64), StoreError> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM features WHERE deleted_at IS NOT NULL")
            .fetch_one(&self.pool)
            .await?;
        let rows = sqlx::query(
            "SELECT id, kode_aset, nama, feature_type AS jenis, network_id, \
                    to_char(deleted_at, 'YYYY-MM-DD\"T\"HH24:MI:SSOF') AS deleted_at \
             FROM features WHERE deleted_at IS NOT NULL \
             ORDER BY deleted_at DESC, id DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        let daftar = rows
            .iter()
            .map(|r| TerhapusRow {
                id: r.get("id"),
                kode_aset: r.get("kode_aset"),
                nama: r.get("nama"),
                jenis: r.get("jenis"),
                deleted_at: r.get("deleted_at"),
                network_id: r.get("network_id"),
            })
            .collect();
        Ok((daftar, total))
    }
}

#[cfg(test)]
mod pulih_tests {
    use super::*;
    use sqlx::PgPool;

    /// Unit SPAM dengan desa beserta poligon batasnya (kotak 107.0-107.5, -7.0 sampai -6.5).
    async fn unit_dengan_batas(pool: &PgPool) -> i64 {
        sqlx::query(
            "INSERT INTO ref_batas_desa (kode_bps, kecamatan, desa, geom) VALUES ('uji001', 'Uji', 'Uji', \
             ST_Multi(ST_GeomFromText('POLYGON((107 -7, 107.5 -7, 107.5 -6.5, 107 -6.5, 107 -7))', 4326)))",
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO ref_desa (id, nama, kecamatan, kode_bps) VALUES (1, 'Uji', 'Uji', 'uji001')")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query_scalar("INSERT INTO networks (nama, unit_spam_id, desa_id) VALUES ('Uji unit', 1, 1) RETURNING id")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    /// Satu pipa lengkap atributnya di dalam batas, lalu langsung dihapus lunak.
    async fn pipa_terhapus(pool: &PgPool, network: i64) -> i64 {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO features (network_id, feature_type, geom, nama, diameter_mm, material) \
             VALUES ($1, 'pipa', ST_SetSRID(ST_GeomFromText('LINESTRING(107.1 -6.8, 107.11 -6.8)'), 4326), \
                     'Pipa uji', 100, 'PVC') RETURNING id",
        )
        .bind(network)
        .fetch_one(pool)
        .await
        .unwrap();
        let store = Store::from_pool(pool.clone());
        assert!(matches!(store.hapus_aset(id, "operator").await.unwrap(), TulisHasil::Berhasil(())));
        id
    }

    #[ignore = "butuh database PostgreSQL"]
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn pulih_aset_terhapus_kembali_dan_mencatat_riwayat(pool: PgPool) {
        let network = unit_dengan_batas(&pool).await;
        let id = pipa_terhapus(&pool, network).await;
        let store = Store::from_pool(pool.clone());

        let hasil = store.pulihkan_aset(id, "editor").await.unwrap();
        assert!(matches!(hasil, PulihHasil::Berhasil(_)), "hasil: {hasil:?}");

        let deleted: Option<String> = sqlx::query_scalar("SELECT deleted_at::text FROM features WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(deleted.is_none());

        let aksi: String = sqlx::query_scalar(
            "SELECT aksi FROM feature_riwayat WHERE feature_id = $1 ORDER BY id DESC LIMIT 1",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(aksi, "pulihkan");
    }

    #[ignore = "butuh database PostgreSQL"]
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn pulih_aset_yang_tidak_terhapus_ditolak(pool: PgPool) {
        let network = unit_dengan_batas(&pool).await;
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO features (network_id, feature_type, geom, nama) \
             VALUES ($1, 'sr', ST_SetSRID(ST_MakePoint(107.1, -6.8), 4326), 'Rumah 1') RETURNING id",
        )
        .bind(network)
        .fetch_one(&pool)
        .await
        .unwrap();
        let store = Store::from_pool(pool);
        assert!(matches!(store.pulihkan_aset(id, "editor").await.unwrap(), PulihHasil::TidakTerhapus));
    }

    #[ignore = "butuh database PostgreSQL"]
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn pulih_aset_di_luar_batas_desa_ditolak_422(pool: PgPool) {
        let network = unit_dengan_batas(&pool).await;
        let id = pipa_terhapus(&pool, network).await;
        // Batas desa berubah setelah aset dihapus: geometri tersimpan kini berada di luar poligon.
        sqlx::query("UPDATE features SET geom = ST_SetSRID(ST_GeomFromText('LINESTRING(108.1 -6.8, 108.11 -6.8)'), 4326) WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
        let store = Store::from_pool(pool.clone());

        match store.pulihkan_aset(id, "editor").await.unwrap() {
            PulihHasil::Ditolak(AsetError::Aturan(daftar)) => {
                assert!(daftar.iter().any(|m| m.contains("luar batas")), "pesan: {daftar:?}");
            }
            hasil => panic!("seharusnya ditolak sebagai aturan, hasil: {hasil:?}"),
        }
        let masih_terhapus: bool =
            sqlx::query_scalar("SELECT deleted_at IS NOT NULL FROM features WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(masih_terhapus, "aset yang ditolak tidak boleh ikut pulih");
    }

    #[ignore = "butuh database PostgreSQL"]
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn pulih_aset_atribut_wajib_kosong_ditolak_422(pool: PgPool) {
        let network = unit_dengan_batas(&pool).await;
        let id = pipa_terhapus(&pool, network).await;
        // Diameter hilang setelah dihapus. Pipa wajib punya diameter.
        sqlx::query("UPDATE features SET diameter_mm = NULL WHERE id = $1").bind(id).execute(&pool).await.unwrap();
        let store = Store::from_pool(pool);
        assert!(matches!(
            store.pulihkan_aset(id, "editor").await.unwrap(),
            PulihHasil::Ditolak(AsetError::Aturan(_))
        ));
    }

    #[ignore = "butuh database PostgreSQL"]
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn pulih_aset_tidak_ada(pool: PgPool) {
        let store = Store::from_pool(pool);
        assert!(matches!(store.pulihkan_aset(999_999, "editor").await.unwrap(), PulihHasil::TidakAda));
    }

    #[ignore = "butuh database PostgreSQL"]
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn daftar_terhapus_hanya_memuat_aset_terhapus(pool: PgPool) {
        let network = unit_dengan_batas(&pool).await;
        let id = pipa_terhapus(&pool, network).await;
        let _aktif: i64 = sqlx::query_scalar(
            "INSERT INTO features (network_id, feature_type, geom, nama) \
             VALUES ($1, 'sr', ST_SetSRID(ST_MakePoint(107.2, -6.8), 4326), 'Rumah 2') RETURNING id",
        )
        .bind(network)
        .fetch_one(&pool)
        .await
        .unwrap();
        let store = Store::from_pool(pool);
        let (daftar, total) = store.daftar_terhapus(50, 0).await.unwrap();
        assert_eq!(total, 1);
        assert_eq!(daftar.len(), 1);
        assert_eq!(daftar[0].id, id);
        assert_eq!(daftar[0].jenis, "pipa");
        assert_eq!(daftar[0].network_id, network);
    }
}
