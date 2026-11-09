//! Data pelanggan: daftar, detail, tambah, ubah, dan riwayat. Setiap tambah dan ubah menulis baris
//! `pelanggan_riwayat` dalam transaksi yang sama. Pelanggan tidak dihapus; pemutusan memakai status `putus`.
//! Aturan isian ada di pipa-core (`validasi_pelanggan`) dan dipanggil oleh API sebelum data sampai ke sini.

use pipa_core::{PelangganIsian, StatusPelanggan};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::{Postgres, Row, Transaction};

use crate::{RiwayatAset, Store, StoreError};

/// Pelanggan beserta SR, dusun, dan desa tempat SR berada. Kolom SR ikut supaya pencarian dan tampilan
/// tidak perlu lookup kedua.
const PELANGGAN_SELECT: &str = "SELECT p.id, p.nama, p.alamat, p.nomor_meter, p.golongan_tarif, p.status, \
    p.sr_feature_id, f.kode_aset AS kode_aset_sr, f.dusun_id, du.nama AS dusun_nama, n.desa_id, \
    to_char(p.created_at, 'YYYY-MM-DD') AS created_at \
    FROM pelanggan p \
    JOIN features f ON f.id = p.sr_feature_id \
    JOIN networks n ON n.id = f.network_id \
    LEFT JOIN dusun du ON du.id = f.dusun_id";

/// Batas baris per permintaan daftar. Dibatasi di sini supaya tidak ada permintaan yang mengembalikan seluruh tabel.
pub const MAKS_BARIS_DAFTAR: i64 = 500;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PelangganRow {
    pub id: i64,
    pub nama: String,
    pub alamat: Option<String>,
    pub nomor_meter: Option<String>,
    pub golongan_tarif: String,
    pub status: String,
    pub sr_feature_id: i64,
    pub kode_aset_sr: String,
    pub dusun_id: Option<i64>,
    pub dusun_nama: Option<String>,
    pub desa_id: Option<i64>,
    pub created_at: String,
}

/// Filter daftar pelanggan. Semua opsional. `q` mencari nama dan nomor meter, tidak peka huruf besar-kecil.
/// Titik SR untuk pilihan di form pelanggan.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SrPilihan {
    pub id: i64,
    pub kode_aset: String,
    pub nama: Option<String>,
    pub dusun_id: Option<i64>,
    pub dusun_nama: Option<String>,
    pub desa_id: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct FilterPelanggan {
    pub q: Option<String>,
    pub desa_id: Option<i64>,
    pub dusun_id: Option<i64>,
    pub status: Option<StatusPelanggan>,
    pub limit: i64,
}

#[derive(Debug, PartialEq)]
pub enum PelangganTulis {
    Berhasil(PelangganRow),
    TidakAda,
    /// SR tidak ada, sudah dihapus, atau bukan jenis `sr`.
    SrTidakValid,
    NomorMeterDipakai,
}

const CONSTRAINT_NOMOR_METER: &str = "pelanggan_nomor_meter_uniq";

/// Keadaan pelanggan yang dibandingkan untuk riwayat. Bentuknya sama untuk sebelum dan sesudah.
fn snapshot(
    nama: &str,
    alamat: Option<&str>,
    nomor_meter: Option<&str>,
    golongan_tarif: &str,
    status: &str,
    sr_feature_id: i64,
) -> Value {
    json!({
        "nama": nama,
        "alamat": alamat,
        "nomor_meter": nomor_meter,
        "golongan_tarif": golongan_tarif,
        "status": status,
        "sr_feature_id": sr_feature_id,
    })
}

fn snapshot_isian(i: &PelangganIsian) -> Value {
    snapshot(
        &i.nama,
        i.alamat.as_deref(),
        i.nomor_meter.as_deref(),
        i.golongan_tarif.as_str(),
        i.status.as_str(),
        i.sr_feature_id,
    )
}

/// Escape karakter khusus LIKE supaya pencarian `100%` tidak dianggap wildcard.
fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

fn nomor_meter_dipakai(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(db) if db.code().as_deref() == Some("23505")
        && db.constraint() == Some(CONSTRAINT_NOMOR_METER))
}

async fn sr_ada(tx: &mut Transaction<'_, Postgres>, feature_id: i64) -> Result<bool, StoreError> {
    let ada: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM features WHERE id = $1 AND feature_type = 'sr' AND deleted_at IS NULL)",
    )
    .bind(feature_id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(ada)
}

fn baris_pelanggan(r: &sqlx::postgres::PgRow) -> Result<PelangganRow, StoreError> {
    Ok(PelangganRow {
        id: r.try_get("id")?,
        nama: r.try_get("nama")?,
        alamat: r.try_get("alamat")?,
        nomor_meter: r.try_get("nomor_meter")?,
        golongan_tarif: r.try_get("golongan_tarif")?,
        status: r.try_get("status")?,
        sr_feature_id: r.try_get("sr_feature_id")?,
        kode_aset_sr: r.try_get("kode_aset_sr")?,
        dusun_id: r.try_get("dusun_id")?,
        dusun_nama: r.try_get("dusun_nama")?,
        desa_id: r.try_get("desa_id")?,
        created_at: r.try_get("created_at")?,
    })
}

impl Store {
    /// Daftar pelanggan, diurutkan menurut nama. Filter dan batas baris dari `FilterPelanggan`.
    pub async fn list_pelanggan(&self, f: &FilterPelanggan) -> Result<Vec<PelangganRow>, StoreError> {
        let pola = f.q.as_deref().map(str::trim).filter(|q| !q.is_empty()).map(|q| format!("%{}%", escape_like(q)));
        let limit = f.limit.clamp(1, MAKS_BARIS_DAFTAR);
        let rows = sqlx::query(&format!(
            "{PELANGGAN_SELECT} \
             WHERE ($1::text IS NULL OR p.nama ILIKE $1 OR p.nomor_meter ILIKE $1) \
               AND ($2::bigint IS NULL OR n.desa_id = $2) \
               AND ($3::bigint IS NULL OR f.dusun_id = $3) \
               AND ($4::text IS NULL OR p.status = $4) \
             ORDER BY p.nama, p.id LIMIT $5"
        ))
        .bind(pola)
        .bind(f.desa_id)
        .bind(f.dusun_id)
        .bind(f.status.map(StatusPelanggan::as_str))
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(baris_pelanggan).collect()
    }

    pub async fn get_pelanggan(&self, id: i64) -> Result<Option<PelangganRow>, StoreError> {
        let row =
            sqlx::query(&format!("{PELANGGAN_SELECT} WHERE p.id = $1")).bind(id).fetch_optional(&self.pool).await?;
        row.as_ref().map(baris_pelanggan).transpose()
    }

    /// Daftar titik SR untuk memilih SR saat menambah pelanggan. Filter desa dan dusun opsional.
    /// `q` mencari kode aset dan nama SR. Hanya SR yang belum dihapus.
    pub async fn list_sr(
        &self,
        desa_id: Option<i64>,
        dusun_id: Option<i64>,
        q: Option<&str>,
    ) -> Result<Vec<SrPilihan>, StoreError> {
        let pola = q.map(str::trim).filter(|q| !q.is_empty()).map(|q| format!("%{}%", escape_like(q)));
        let rows = sqlx::query(
            "SELECT f.id, f.kode_aset, f.nama, f.dusun_id, du.nama AS dusun_nama, n.desa_id \
             FROM features f \
             JOIN networks n ON n.id = f.network_id \
             LEFT JOIN dusun du ON du.id = f.dusun_id \
             WHERE f.feature_type = 'sr' AND f.deleted_at IS NULL \
               AND ($1::bigint IS NULL OR n.desa_id = $1) \
               AND ($2::bigint IS NULL OR f.dusun_id = $2) \
               AND ($3::text IS NULL OR f.kode_aset ILIKE $3 OR f.nama ILIKE $3) \
             ORDER BY f.kode_aset LIMIT $4",
        )
        .bind(desa_id)
        .bind(dusun_id)
        .bind(pola)
        .bind(MAKS_BARIS_DAFTAR)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|r| {
                Ok(SrPilihan {
                    id: r.try_get("id")?,
                    kode_aset: r.try_get("kode_aset")?,
                    nama: r.try_get("nama")?,
                    dusun_id: r.try_get("dusun_id")?,
                    dusun_nama: r.try_get("dusun_nama")?,
                    desa_id: r.try_get("desa_id")?,
                })
            })
            .collect()
    }

    /// Tambah pelanggan baru. Riwayat `tambah` ditulis dalam transaksi yang sama.
    pub async fn tambah_pelanggan(&self, isian: &PelangganIsian, oleh: &str) -> Result<PelangganTulis, StoreError> {
        let mut tx = self.pool.begin().await?;
        if !sr_ada(&mut tx, isian.sr_feature_id).await? {
            return Ok(PelangganTulis::SrTidakValid);
        }
        let id: i64 = match sqlx::query_scalar(
            "INSERT INTO pelanggan (nama, alamat, nomor_meter, golongan_tarif, status, sr_feature_id) \
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
        )
        .bind(&isian.nama)
        .bind(&isian.alamat)
        .bind(&isian.nomor_meter)
        .bind(isian.golongan_tarif.as_str())
        .bind(isian.status.as_str())
        .bind(isian.sr_feature_id)
        .fetch_one(&mut *tx)
        .await
        {
            Ok(id) => id,
            Err(e) if nomor_meter_dipakai(&e) => return Ok(PelangganTulis::NomorMeterDipakai),
            Err(e) => return Err(e.into()),
        };

        sqlx::query(
            "INSERT INTO pelanggan_riwayat (pelanggan_id, aksi, diubah_oleh, sesudah) VALUES ($1, 'tambah', $2, $3)",
        )
        .bind(id)
        .bind(oleh)
        .bind(snapshot_isian(isian))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(match self.get_pelanggan(id).await? {
            Some(row) => PelangganTulis::Berhasil(row),
            None => return Err(StoreError::Data(format!("pelanggan {id} hilang setelah ditambah"))),
        })
    }

    /// Ubah pelanggan. Baris dikunci sejak dibaca sampai riwayat ditulis. Bila tidak ada yang berubah, tidak ada riwayat.
    pub async fn ubah_pelanggan(
        &self,
        id: i64,
        isian: &PelangganIsian,
        oleh: &str,
    ) -> Result<PelangganTulis, StoreError> {
        let mut tx = self.pool.begin().await?;
        let Some(now) = sqlx::query(
            "SELECT nama, alamat, nomor_meter, golongan_tarif, status, sr_feature_id \
             FROM pelanggan WHERE id = $1 FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(PelangganTulis::TidakAda);
        };
        let sr_lama: i64 = now.try_get("sr_feature_id")?;
        if isian.sr_feature_id != sr_lama && !sr_ada(&mut tx, isian.sr_feature_id).await? {
            return Ok(PelangganTulis::SrTidakValid);
        }
        let nama_lama: String = now.try_get("nama")?;
        let alamat_lama: Option<String> = now.try_get("alamat")?;
        let meter_lama: Option<String> = now.try_get("nomor_meter")?;
        let golongan_lama: String = now.try_get("golongan_tarif")?;
        let status_lama: String = now.try_get("status")?;
        let sebelum =
            snapshot(&nama_lama, alamat_lama.as_deref(), meter_lama.as_deref(), &golongan_lama, &status_lama, sr_lama);

        if let Err(e) = sqlx::query(
            "UPDATE pelanggan SET nama = $2, alamat = $3, nomor_meter = $4, golongan_tarif = $5, status = $6, \
             sr_feature_id = $7 WHERE id = $1",
        )
        .bind(id)
        .bind(&isian.nama)
        .bind(&isian.alamat)
        .bind(&isian.nomor_meter)
        .bind(isian.golongan_tarif.as_str())
        .bind(isian.status.as_str())
        .bind(isian.sr_feature_id)
        .execute(&mut *tx)
        .await
        {
            if nomor_meter_dipakai(&e) {
                return Ok(PelangganTulis::NomorMeterDipakai);
            }
            return Err(e.into());
        }

        let sesudah = snapshot_isian(isian);
        if sebelum != sesudah {
            sqlx::query(
                "INSERT INTO pelanggan_riwayat (pelanggan_id, aksi, diubah_oleh, sebelum, sesudah) \
                 VALUES ($1, 'ubah', $2, $3, $4)",
            )
            .bind(id)
            .bind(oleh)
            .bind(sebelum)
            .bind(sesudah)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(match self.get_pelanggan(id).await? {
            Some(row) => PelangganTulis::Berhasil(row),
            None => return Err(StoreError::Data(format!("pelanggan {id} hilang setelah diubah"))),
        })
    }

    /// Riwayat satu pelanggan, terbaru di atas. None bila pelanggan tidak ada.
    pub async fn riwayat_pelanggan(&self, id: i64) -> Result<Option<Vec<RiwayatAset>>, StoreError> {
        let ada: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pelanggan WHERE id = $1)")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
        if !ada {
            return Ok(None);
        }
        let rows = sqlx::query(
            "SELECT aksi, diubah_oleh, to_char(diubah_pada, 'YYYY-MM-DD\"T\"HH24:MI:SSOF') AS diubah_pada, \
                    sebelum, sesudah \
             FROM pelanggan_riwayat WHERE pelanggan_id = $1 ORDER BY diubah_pada DESC, id DESC",
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

#[cfg(test)]
mod tests {
    use super::*;
    use pipa_core::{validasi_pelanggan, DusunIsian, PelangganMasukan};
    use sqlx::PgPool;

    use crate::{CakupanDesa, DusunTulis, SetDusunHasil};

    async fn titik_sr(pool: &PgPool, network_id: i64, dusun_id: Option<i64>) -> i64 {
        sqlx::query_scalar(
            "INSERT INTO features (network_id, feature_type, geom, dusun_id) \
             VALUES ($1, 'sr', ST_SetSRID(ST_MakePoint(107.1, -6.8), 4326), $2) RETURNING id",
        )
        .bind(network_id)
        .bind(dusun_id)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    fn isian(nama: &str, meter: Option<&str>, sr: i64) -> PelangganIsian {
        validasi_pelanggan(&PelangganMasukan {
            nama: nama.into(),
            alamat: None,
            nomor_meter: meter.map(str::to_string),
            golongan_tarif: "sosial".into(),
            status: None,
            sr_feature_id: sr,
        })
        .unwrap()
    }

    /// Butuh DATABASE_URL ke PostgreSQL dengan PostGIS (sqlx membuat database uji sementara).
    /// Jalankan: `cargo test -p pipa-store -- --ignored`
    #[ignore = "butuh database PostgreSQL"]
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn kode_aset_dusun_pelanggan_dan_cakupan(pool: PgPool) {
        sqlx::query("INSERT INTO users (username, password_hash, role) VALUES ('op1', 'x', 'editor')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO ref_desa (id, nama, kecamatan) VALUES (12, 'Desa A', 'Kec'), (13, 'Desa B', 'Kec')")
            .execute(&pool)
            .await
            .unwrap();
        let network: i64 =
            sqlx::query_scalar("INSERT INTO networks (nama, desa_id) VALUES ('Unit A', 12) RETURNING id")
                .fetch_one(&pool)
                .await
                .unwrap();
        let store = Store::from_pool(pool.clone());

        // Kode dibuat trigger: <kode_desa>-<jenis>-<nomor>.
        let sr1 = titik_sr(&pool, network, None).await;
        let sr2 = titik_sr(&pool, network, None).await;
        let kode: String = sqlx::query_scalar("SELECT kode_aset FROM features WHERE id = $1")
            .bind(sr1)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(kode, "0012-sr-000001");
        let kode2: String = sqlx::query_scalar("SELECT kode_aset FROM features WHERE id = $1")
            .bind(sr2)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(kode2, "0012-sr-000002");

        // Dusun: nama unik per desa, tanpa membedakan huruf besar-kecil.
        let dusun = match store
            .tambah_dusun(12, &DusunIsian { nama: "Cibodas".into(), jumlah_rumah: 10, jumlah_jiwa: 40 }, "op1")
            .await
            .unwrap()
        {
            DusunTulis::Berhasil(d) => d,
            other => panic!("{other:?}"),
        };
        assert_eq!(
            store
                .tambah_dusun(12, &DusunIsian { nama: "cibodas".into(), jumlah_rumah: 1, jumlah_jiwa: 1 }, "op1")
                .await
                .unwrap(),
            DusunTulis::NamaDipakai
        );
        assert_eq!(
            store
                .tambah_dusun(99, &DusunIsian { nama: "X".into(), jumlah_rumah: 1, jumlah_jiwa: 1 }, "op1")
                .await
                .unwrap(),
            DusunTulis::DesaTidakAda
        );
        // SR desa lain tidak boleh dipasang ke dusun desa ini.
        let dusun_desa13 = match store
            .tambah_dusun(13, &DusunIsian { nama: "Lain".into(), jumlah_rumah: 1, jumlah_jiwa: 1 }, "op1")
            .await
            .unwrap()
        {
            DusunTulis::Berhasil(d) => d,
            other => panic!("{other:?}"),
        };
        assert_eq!(store.set_dusun_sr(sr1, Some(dusun_desa13.id)).await.unwrap(), SetDusunHasil::DusunBedaDesa);
        assert_eq!(store.set_dusun_sr(sr1, Some(dusun.id)).await.unwrap(), SetDusunHasil::Berhasil);
        assert_eq!(store.set_dusun_sr(sr2, Some(dusun.id)).await.unwrap(), SetDusunHasil::Berhasil);

        // Pelanggan: tambah, nomor meter unik, SR harus ada.
        let p1 = match store.tambah_pelanggan(&isian("Siti Aminah", Some("M-1"), sr1), "op1").await.unwrap() {
            PelangganTulis::Berhasil(p) => p,
            other => panic!("{other:?}"),
        };
        assert_eq!(p1.dusun_nama.as_deref(), Some("Cibodas"));
        assert_eq!(
            store.tambah_pelanggan(&isian("Budi", Some("M-1"), sr2), "op1").await.unwrap(),
            PelangganTulis::NomorMeterDipakai
        );
        assert_eq!(
            store.tambah_pelanggan(&isian("Budi", None, 999_999), "op1").await.unwrap(),
            PelangganTulis::SrTidakValid
        );

        // Cakupan: SR1 terlayani, SR2 belum.
        let CakupanDesa { per_dusun, tanpa_dusun } = store.cakupan_desa(12).await.unwrap();
        assert_eq!(per_dusun.len(), 1);
        assert_eq!((per_dusun[0].jumlah_sr, per_dusun[0].sr_terlayani), (2, 1));
        assert_eq!(tanpa_dusun.jumlah_sr, 0);

        // Putus: status putus tidak lagi dihitung sebagai terlayani, dan riwayat mencatat perubahan.
        let putus = PelangganIsian { status: StatusPelanggan::Putus, ..isian("Siti Aminah", Some("M-1"), sr1) };
        assert!(matches!(store.ubah_pelanggan(p1.id, &putus, "op1").await.unwrap(), PelangganTulis::Berhasil(_)));
        let per = store.cakupan_desa(12).await.unwrap().per_dusun;
        assert_eq!(per[0].sr_terlayani, 0);
        let riwayat = store.riwayat_pelanggan(p1.id).await.unwrap().unwrap();
        let aksi: Vec<&str> = riwayat.iter().map(|r| r.aksi.as_str()).collect();
        assert_eq!(aksi, vec!["ubah", "tambah"]);
        assert_eq!(riwayat[0].diubah_oleh, "op1");

        // Ubah tanpa perubahan tidak menambah riwayat.
        assert!(matches!(store.ubah_pelanggan(p1.id, &putus, "op1").await.unwrap(), PelangganTulis::Berhasil(_)));
        assert_eq!(store.riwayat_pelanggan(p1.id).await.unwrap().unwrap().len(), 2);

        // Pencarian nama dan filter status.
        let cari = FilterPelanggan { q: Some("siti".into()), limit: 10, ..Default::default() };
        assert_eq!(store.list_pelanggan(&cari).await.unwrap().len(), 1);
        let aktif = FilterPelanggan { status: Some(StatusPelanggan::Aktif), limit: 10, ..Default::default() };
        assert!(store.list_pelanggan(&aktif).await.unwrap().is_empty());
    }

    #[test]
    fn escape_like_menetralkan_wildcard() {
        assert_eq!(escape_like("100%_a\\b"), "100\\%\\_a\\\\b");
        assert_eq!(escape_like("Siti"), "Siti");
    }

    #[test]
    fn snapshot_stabil_untuk_perbandingan_riwayat() {
        let a = snapshot("Siti", None, Some("M1"), "sosial", "aktif", 5);
        let b = snapshot("Siti", None, Some("M1"), "sosial", "aktif", 5);
        let c = snapshot("Siti", None, Some("M1"), "sosial", "putus", 5);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
