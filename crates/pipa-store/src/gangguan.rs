//! Pelaporan gangguan layanan dan metadata foto laporan. Berkas foto sendiri ditulis oleh pipa-api
//! ke disk; di sini hanya metadatanya yang disimpan.

use pipa_core::gangguan::{
    periksa_laporan, periksa_lokasi, periksa_transisi, JenisGangguan, StatusGangguan, MAKS_FOTO_PER_LAPORAN,
};
use pipa_core::om::OmError;
use serde::Serialize;
use serde_json::json;
use sqlx::postgres::PgRow;
use sqlx::{Postgres, Row, Transaction};

use crate::om_riwayat::catat;
use crate::{Store, StoreError};

const GANGGUAN_COLS: &str = "g.id, g.network_id, g.feature_id, g.jenis, \
    ST_X(g.geom)::float8 AS lon, ST_Y(g.geom)::float8 AS lat, g.deskripsi, g.pelapor, g.status, \
    to_char(g.dilaporkan_pada, 'YYYY-MM-DD\"T\"HH24:MI:SSOF') AS dilaporkan_pada, \
    to_char(g.diperbarui_pada, 'YYYY-MM-DD\"T\"HH24:MI:SSOF') AS diperbarui_pada, g.diperbarui_oleh";

/// Metadata satu foto laporan. Nama berkas di disk tidak ikut dikirim ke klien.
#[derive(Debug, Clone, Serialize)]
pub struct FotoGangguan {
    pub id: i64,
    pub gangguan_id: i64,
    pub mime: String,
    pub ukuran_byte: i32,
    pub diunggah_pada: String,
    #[serde(skip)]
    pub berkas: String,
}

/// Laporan gangguan lengkap dengan foto. `pelapor` tetap terisi di sini; pemanggil API yang
/// menyembunyikannya untuk peran di bawah editor.
#[derive(Debug, Clone, Serialize)]
pub struct Gangguan {
    pub id: i64,
    pub network_id: i64,
    pub feature_id: Option<i64>,
    pub jenis: String,
    pub lon: Option<f64>,
    pub lat: Option<f64>,
    pub deskripsi: String,
    pub pelapor: String,
    pub status: String,
    pub dilaporkan_pada: String,
    pub diperbarui_pada: String,
    pub diperbarui_oleh: String,
    pub foto: Vec<FotoGangguan>,
}

/// Isian laporan baru dari API. `jenis` dan lokasi divalidasi di pipa-core.
#[derive(Debug, Clone)]
pub struct GangguanBaru {
    pub network_id: i64,
    pub feature_id: Option<i64>,
    /// `(lon, lat)`.
    pub titik: Option<(f64, f64)>,
    pub jenis: String,
    pub deskripsi: String,
    pub pelapor: String,
}

#[derive(Debug, Clone, Default)]
pub struct FilterGangguan {
    pub network_id: Option<i64>,
    pub feature_id: Option<i64>,
    pub status: Option<StatusGangguan>,
}

/// Berkas foto beserta laporannya, untuk dibaca API saat foto diunduh.
#[derive(Debug, Clone)]
pub struct BerkasFoto {
    pub gangguan_id: i64,
    pub berkas: String,
    pub mime: String,
}

fn gangguan_dari_baris(r: &PgRow) -> Result<Gangguan, StoreError> {
    Ok(Gangguan {
        id: r.try_get("id")?,
        network_id: r.try_get("network_id")?,
        feature_id: r.try_get("feature_id")?,
        jenis: r.try_get("jenis")?,
        lon: r.try_get("lon")?,
        lat: r.try_get("lat")?,
        deskripsi: r.try_get("deskripsi")?,
        pelapor: r.try_get("pelapor")?,
        status: r.try_get("status")?,
        dilaporkan_pada: r.try_get("dilaporkan_pada")?,
        diperbarui_pada: r.try_get("diperbarui_pada")?,
        diperbarui_oleh: r.try_get("diperbarui_oleh")?,
        foto: Vec::new(),
    })
}

fn snapshot(g: &Gangguan) -> Result<serde_json::Value, StoreError> {
    serde_json::to_value(g).map_err(|e| StoreError::Data(e.to_string()))
}

/// Membaca satu laporan dalam transaksi, tanpa foto. Dipakai untuk snapshot riwayat.
async fn baca_dalam_tx(tx: &mut Transaction<'_, Postgres>, id: i64) -> Result<Option<Gangguan>, StoreError> {
    let row = sqlx::query(&format!("SELECT {GANGGUAN_COLS} FROM gangguan g WHERE g.id = $1"))
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?;
    row.as_ref().map(gangguan_dari_baris).transpose()
}

impl Store {
    /// Laporan baru. Lokasi harus titik atau aset yang berada di jaringan yang sama. Riwayat `tambah` ditulis.
    pub async fn buat_gangguan(&self, baru: GangguanBaru, oleh: &str) -> Result<Gangguan, StoreError> {
        let jenis = JenisGangguan::parse(&baru.jenis)?;
        periksa_lokasi(baru.feature_id, baru.titik)?;
        let (deskripsi, pelapor) = periksa_laporan(&baru.deskripsi, &baru.pelapor)?;

        let mut tx = self.pool.begin().await?;
        let ada_jaringan: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM networks WHERE id = $1)")
                .bind(baru.network_id)
                .fetch_one(&mut *tx)
                .await?;
        if !ada_jaringan {
            return Err(OmError::TidakDitemukan("jaringan").into());
        }
        if let Some(fid) = baru.feature_id {
            let jaringan_aset: Option<i64> = sqlx::query_scalar(
                "SELECT network_id FROM features WHERE id = $1 AND deleted_at IS NULL",
            )
            .bind(fid)
            .fetch_optional(&mut *tx)
            .await?;
            match jaringan_aset {
                None => return Err(OmError::TidakDitemukan("aset").into()),
                Some(n) if n != baru.network_id => {
                    return Err(OmError::Aturan(vec!["aset bukan bagian jaringan ini".into()]).into())
                }
                Some(_) => {}
            }
        }

        let (lon, lat) = match baru.titik {
            Some((lon, lat)) => (Some(lon), Some(lat)),
            None => (None, None),
        };
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO gangguan (network_id, feature_id, jenis, geom, deskripsi, pelapor, diperbarui_oleh) \
             VALUES ($1, $2, $3, \
                CASE WHEN $4::float8 IS NULL THEN NULL ELSE ST_SetSRID(ST_MakePoint($4, $5), 4326) END, \
                $6, $7, $8) \
             RETURNING id",
        )
        .bind(baru.network_id)
        .bind(baru.feature_id)
        .bind(jenis.as_str())
        .bind(lon)
        .bind(lat)
        .bind(&deskripsi)
        .bind(&pelapor)
        .bind(oleh)
        .fetch_one(&mut *tx)
        .await?;

        let g = baca_dalam_tx(&mut tx, id).await?.ok_or_else(|| StoreError::Data("laporan baru hilang".into()))?;
        catat(&mut tx, "gangguan", id, g.feature_id, "tambah", oleh, None, Some(snapshot(&g)?)).await?;
        tx.commit().await?;
        Ok(g)
    }

    /// Daftar laporan, terbaru di atas, lengkap dengan foto.
    pub async fn daftar_gangguan(&self, filter: FilterGangguan, batas: i64) -> Result<Vec<Gangguan>, StoreError> {
        let rows = sqlx::query(&format!(
            "SELECT {GANGGUAN_COLS} FROM gangguan g \
             WHERE ($1::bigint IS NULL OR g.network_id = $1) \
               AND ($2::bigint IS NULL OR g.feature_id = $2) \
               AND ($3::text IS NULL OR g.status = $3) \
             ORDER BY g.dilaporkan_pada DESC, g.id DESC LIMIT $4"
        ))
        .bind(filter.network_id)
        .bind(filter.feature_id)
        .bind(filter.status.map(|s| s.as_str()))
        .bind(batas)
        .fetch_all(&self.pool)
        .await?;
        let mut daftar = rows.iter().map(gangguan_dari_baris).collect::<Result<Vec<_>, _>>()?;
        let ids: Vec<i64> = daftar.iter().map(|g| g.id).collect();
        let foto = self.foto_untuk(&ids).await?;
        for g in &mut daftar {
            g.foto = foto.iter().filter(|f| f.gangguan_id == g.id).cloned().collect();
        }
        Ok(daftar)
    }

    /// Satu laporan lengkap dengan foto. None bila tidak ada.
    pub async fn ambil_gangguan(&self, id: i64) -> Result<Option<Gangguan>, StoreError> {
        let row = sqlx::query(&format!("SELECT {GANGGUAN_COLS} FROM gangguan g WHERE g.id = $1"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        let Some(row) = row else { return Ok(None) };
        let mut g = gangguan_dari_baris(&row)?;
        g.foto = self.foto_untuk(&[id]).await?;
        Ok(Some(g))
    }

    async fn foto_untuk(&self, gangguan_ids: &[i64]) -> Result<Vec<FotoGangguan>, StoreError> {
        if gangguan_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query(
            "SELECT id, gangguan_id, berkas, mime, ukuran_byte, \
                    to_char(diunggah_pada, 'YYYY-MM-DD\"T\"HH24:MI:SSOF') AS diunggah_pada \
             FROM gangguan_foto WHERE gangguan_id = ANY($1) ORDER BY id",
        )
        .bind(gangguan_ids)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|r| {
                Ok(FotoGangguan {
                    id: r.try_get("id")?,
                    gangguan_id: r.try_get("gangguan_id")?,
                    mime: r.try_get("mime")?,
                    ukuran_byte: r.try_get("ukuran_byte")?,
                    diunggah_pada: r.try_get("diunggah_pada")?,
                    berkas: r.try_get("berkas")?,
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()
            .map_err(StoreError::from)
    }

    /// Ubah status laporan. Status hanya boleh maju (lihat `periksa_transisi`). Status yang sama tidak menulis riwayat.
    pub async fn ubah_status_gangguan(&self, id: i64, ke: &str, oleh: &str) -> Result<Gangguan, StoreError> {
        let ke = StatusGangguan::parse(ke)?;
        let mut tx = self.pool.begin().await?;
        let Some(sebelum) = baca_dalam_tx_kunci(&mut tx, id).await? else {
            return Err(OmError::TidakDitemukan("laporan").into());
        };
        let dari = StatusGangguan::parse(&sebelum.status)?;
        periksa_transisi(dari, ke)?;
        if dari == ke {
            tx.rollback().await?;
            return self.ambil_gangguan(id).await?.ok_or_else(|| OmError::TidakDitemukan("laporan").into());
        }

        sqlx::query("UPDATE gangguan SET status = $2, diperbarui_pada = now(), diperbarui_oleh = $3 WHERE id = $1")
            .bind(id)
            .bind(ke.as_str())
            .bind(oleh)
            .execute(&mut *tx)
            .await?;
        let sesudah = baca_dalam_tx(&mut tx, id).await?.ok_or_else(|| StoreError::Data("laporan hilang".into()))?;
        catat(
            &mut tx,
            "gangguan",
            id,
            sesudah.feature_id,
            "ubah",
            oleh,
            Some(snapshot(&sebelum)?),
            Some(snapshot(&sesudah)?),
        )
        .await?;
        tx.commit().await?;
        self.ambil_gangguan(id).await?.ok_or_else(|| StoreError::Data("laporan hilang".into()))
    }

    /// Menyimpan metadata foto setelah berkasnya ditulis. Jumlah foto dicek ulang di sini, di bawah kunci baris,
    /// supaya dua unggahan bersamaan tidak melewati batas.
    pub async fn tambah_foto(
        &self,
        gangguan_id: i64,
        berkas: &str,
        mime: &str,
        ukuran_byte: i32,
        oleh: &str,
    ) -> Result<FotoGangguan, StoreError> {
        let mut tx = self.pool.begin().await?;
        let Some(induk) = sqlx::query("SELECT feature_id FROM gangguan WHERE id = $1 FOR UPDATE")
            .bind(gangguan_id)
            .fetch_optional(&mut *tx)
            .await?
        else {
            return Err(OmError::TidakDitemukan("laporan").into());
        };
        let feature_id: Option<i64> = induk.try_get("feature_id")?;
        let jumlah: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM gangguan_foto WHERE gangguan_id = $1")
            .bind(gangguan_id)
            .fetch_one(&mut *tx)
            .await?;
        if jumlah >= MAKS_FOTO_PER_LAPORAN {
            return Err(OmError::Aturan(vec![format!(
                "laporan sudah punya {MAKS_FOTO_PER_LAPORAN} foto, batas maksimalnya"
            )])
            .into());
        }

        let row = sqlx::query(
            "INSERT INTO gangguan_foto (gangguan_id, berkas, mime, ukuran_byte, diunggah_oleh) \
             VALUES ($1, $2, $3, $4, $5) \
             RETURNING id, to_char(diunggah_pada, 'YYYY-MM-DD\"T\"HH24:MI:SSOF') AS diunggah_pada",
        )
        .bind(gangguan_id)
        .bind(berkas)
        .bind(mime)
        .bind(ukuran_byte)
        .bind(oleh)
        .fetch_one(&mut *tx)
        .await?;
        let foto = FotoGangguan {
            id: row.try_get("id")?,
            gangguan_id,
            mime: mime.to_string(),
            ukuran_byte,
            diunggah_pada: row.try_get("diunggah_pada")?,
            berkas: berkas.to_string(),
        };
        catat(
            &mut tx,
            "gangguan",
            gangguan_id,
            feature_id,
            "foto",
            oleh,
            None,
            Some(json!({ "foto_id": foto.id, "mime": mime, "ukuran_byte": ukuran_byte })),
        )
        .await?;
        tx.commit().await?;
        Ok(foto)
    }

    /// Berkas foto untuk diunduh. None bila foto tidak ada.
    pub async fn ambil_berkas_foto(&self, foto_id: i64) -> Result<Option<BerkasFoto>, StoreError> {
        let row = sqlx::query("SELECT gangguan_id, berkas, mime FROM gangguan_foto WHERE id = $1")
            .bind(foto_id)
            .fetch_optional(&self.pool)
            .await?;
        row.map(|r| {
            Ok(BerkasFoto {
                gangguan_id: r.try_get("gangguan_id")?,
                berkas: r.try_get("berkas")?,
                mime: r.try_get("mime")?,
            })
        })
        .transpose()
    }

}

/// Membaca laporan dengan kunci baris, untuk perubahan status.
async fn baca_dalam_tx_kunci(tx: &mut Transaction<'_, Postgres>, id: i64) -> Result<Option<Gangguan>, StoreError> {
    let row = sqlx::query(&format!("SELECT {GANGGUAN_COLS} FROM gangguan g WHERE g.id = $1 FOR UPDATE OF g"))
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?;
    row.as_ref().map(gangguan_dari_baris).transpose()
}
