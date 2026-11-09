//! Pokmas efektif per desa. Data hasil sinkron Arumanis (`ref_pokmas`) bisa digantikan koreksi
//! yang diisi langsung di arsgis (`pokmas_override`). Modul ini tidak menyentuh database.

use serde::{Deserialize, Serialize};

use crate::PokmasDesa;

pub const MAKS_NAMA: usize = 200;
pub const MAKS_PENGURUS: usize = 120;
pub const MAKS_PERDES: usize = 200;
pub const MAKS_CATATAN: usize = 1000;

/// Asal data Pokmas yang ditampilkan. Nilainya ikut dikirim ke klien sebagai `override` atau `arumanis`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SumberPokmas {
    Override,
    Arumanis,
}

/// Koreksi Pokmas yang tersimpan di `pokmas_override`, lengkap dengan jejak pengubahnya.
#[derive(Debug, Clone, Serialize)]
pub struct PokmasOverride {
    pub nama: String,
    pub perdes: Option<String>,
    pub kepala: Option<String>,
    pub bendahara: Option<String>,
    pub sekretaris: Option<String>,
    pub catatan: Option<String>,
    pub diperbarui_oleh: String,
    /// Waktu lokal server, format `YYYY-MM-DD HH24:MI:SS`.
    pub diperbarui_pada: String,
}

/// Hasil fungsi `efektifkan`: Pokmas yang berlaku untuk desa beserta asalnya.
#[derive(Debug, Clone, Serialize)]
pub struct PokmasEfektif {
    pub sumber: SumberPokmas,
    pub pokmas: Option<PokmasDesa>,
}

/// Pokmas efektif satu desa beserta koreksi mentahnya (bila ada), untuk layar edit.
#[derive(Debug, Clone)]
pub struct PokmasDetail {
    pub efektif: PokmasEfektif,
    pub koreksi: Option<PokmasOverride>,
}

/// Isian koreksi dari klien. Semua bidang opsional agar kesalahan bisa dijelaskan dengan pesan Indonesia.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PokmasInput {
    pub nama: Option<String>,
    pub perdes: Option<String>,
    pub kepala: Option<String>,
    pub bendahara: Option<String>,
    pub sekretaris: Option<String>,
    pub catatan: Option<String>,
}

/// Isian yang sudah dirapikan (spasi di tepi dibuang, teks kosong menjadi None) dan lolos batas panjang.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PokmasIsian {
    pub nama: String,
    pub perdes: Option<String>,
    pub kepala: Option<String>,
    pub bendahara: Option<String>,
    pub sekretaris: Option<String>,
    pub catatan: Option<String>,
}

impl PokmasInput {
    pub fn validated(&self) -> Result<PokmasIsian, String> {
        let nama = bersihkan("nama Pokmas", &self.nama, MAKS_NAMA)?
            .ok_or_else(|| "nama Pokmas wajib diisi".to_string())?;
        Ok(PokmasIsian {
            nama,
            perdes: bersihkan("perdes", &self.perdes, MAKS_PERDES)?,
            kepala: bersihkan("nama kepala", &self.kepala, MAKS_PENGURUS)?,
            bendahara: bersihkan("nama bendahara", &self.bendahara, MAKS_PENGURUS)?,
            sekretaris: bersihkan("nama sekretaris", &self.sekretaris, MAKS_PENGURUS)?,
            catatan: bersihkan("catatan", &self.catatan, MAKS_CATATAN)?,
        })
    }
}

fn bersihkan(label: &str, nilai: &Option<String>, maks: usize) -> Result<Option<String>, String> {
    let Some(teks) = nilai.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    if teks.chars().count() > maks {
        return Err(format!("{label} maksimal {maks} karakter"));
    }
    if teks.chars().any(|c| c.is_control() && c != '\n') {
        return Err(format!("{label} tidak boleh berisi karakter kontrol"));
    }
    Ok(Some(teks.to_string()))
}

/// Pokmas yang berlaku untuk satu desa. Bila ada koreksi, koreksi menggantikan seluruh data hasil sinkron:
/// tidak ada bidang yang diambil dari sinkron, dan tanda konflik serta daftar unit sumber dikosongkan.
pub fn efektifkan(synced: Option<PokmasDesa>, koreksi: Option<PokmasOverride>) -> PokmasEfektif {
    match koreksi {
        Some(k) => PokmasEfektif {
            sumber: SumberPokmas::Override,
            pokmas: Some(PokmasDesa {
                nama: Some(k.nama),
                perdes: k.perdes,
                konflik: false,
                kepala: k.kepala,
                bendahara: k.bendahara,
                sekretaris: k.sekretaris,
                sumber_unit_ids: Vec::new(),
            }),
        },
        None => PokmasEfektif { sumber: SumberPokmas::Arumanis, pokmas: synced },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synced() -> PokmasDesa {
        PokmasDesa {
            nama: Some("Tirta Lestari".into()),
            perdes: Some("Perdes 3/2020".into()),
            konflik: true,
            kepala: Some("Hasan".into()),
            bendahara: Some("Siti".into()),
            sekretaris: None,
            sumber_unit_ids: vec![11, 12],
        }
    }

    fn koreksi() -> PokmasOverride {
        PokmasOverride {
            nama: "Tirta Makmur".into(),
            perdes: None,
            kepala: Some("Budi".into()),
            bendahara: None,
            sekretaris: Some("Rina".into()),
            catatan: Some("Diperbarui dari berita acara".into()),
            diperbarui_oleh: "editor1".into(),
            diperbarui_pada: "2026-11-01 09:00:00".into(),
        }
    }

    #[test]
    fn tanpa_koreksi_memakai_data_arumanis_apa_adanya() {
        let hasil = efektifkan(Some(synced()), None);
        assert_eq!(hasil.sumber, SumberPokmas::Arumanis);
        let p = hasil.pokmas.expect("pokmas sinkron");
        assert_eq!(p.nama.as_deref(), Some("Tirta Lestari"));
        assert!(p.konflik);
        assert_eq!(p.sumber_unit_ids, vec![11, 12]);
    }

    #[test]
    fn tanpa_koreksi_dan_tanpa_sinkron_tidak_ada_pokmas() {
        let hasil = efektifkan(None, None);
        assert_eq!(hasil.sumber, SumberPokmas::Arumanis);
        assert!(hasil.pokmas.is_none());
    }

    #[test]
    fn koreksi_menggantikan_seluruh_data_sinkron() {
        let hasil = efektifkan(Some(synced()), Some(koreksi()));
        assert_eq!(hasil.sumber, SumberPokmas::Override);
        let p = hasil.pokmas.expect("pokmas koreksi");
        assert_eq!(p.nama.as_deref(), Some("Tirta Makmur"));
        assert_eq!(p.kepala.as_deref(), Some("Budi"));
        // Bidang yang kosong di koreksi tetap kosong, tidak diisi dari sinkron.
        assert_eq!(p.perdes, None);
        assert_eq!(p.bendahara, None);
        assert_eq!(p.sekretaris.as_deref(), Some("Rina"));
        // Tanda konflik dan daftar unit berasal dari sinkron, jadi tidak berlaku untuk koreksi.
        assert!(!p.konflik);
        assert!(p.sumber_unit_ids.is_empty());
    }

    #[test]
    fn koreksi_tetap_berlaku_walau_desa_belum_punya_pokmas_sinkron() {
        let hasil = efektifkan(None, Some(koreksi()));
        assert_eq!(hasil.sumber, SumberPokmas::Override);
        assert_eq!(hasil.pokmas.map(|p| p.nama).flatten().as_deref(), Some("Tirta Makmur"));
    }

    #[test]
    fn sumber_terserialisasi_sebagai_teks_huruf_kecil() {
        let override_ = serde_json::to_value(SumberPokmas::Override).unwrap();
        let arumanis = serde_json::to_value(SumberPokmas::Arumanis).unwrap();
        assert_eq!(override_, "override");
        assert_eq!(arumanis, "arumanis");
    }

    fn isian(nama: &str) -> PokmasInput {
        PokmasInput { nama: Some(nama.into()), ..Default::default() }
    }

    #[test]
    fn nama_wajib_dan_dirapikan() {
        assert!(PokmasInput::default().validated().is_err());
        assert!(isian("   ").validated().is_err());
        let ok = isian("  Tirta Makmur  ").validated().unwrap();
        assert_eq!(ok.nama, "Tirta Makmur");
    }

    #[test]
    fn teks_kosong_opsional_menjadi_none() {
        let input = PokmasInput {
            nama: Some("Tirta".into()),
            perdes: Some("  ".into()),
            kepala: Some(String::new()),
            ..Default::default()
        };
        let ok = input.validated().unwrap();
        assert_eq!(ok.perdes, None);
        assert_eq!(ok.kepala, None);
        assert_eq!(ok.catatan, None);
    }

    #[test]
    fn batas_panjang_dihitung_dalam_karakter() {
        let pas = "ä".repeat(MAKS_PENGURUS);
        let input = PokmasInput { nama: Some("Tirta".into()), kepala: Some(pas), ..Default::default() };
        assert!(input.validated().is_ok());

        let lewat = "ä".repeat(MAKS_PENGURUS + 1);
        let input = PokmasInput { nama: Some("Tirta".into()), kepala: Some(lewat), ..Default::default() };
        let err = input.validated().unwrap_err();
        assert!(err.contains("nama kepala"), "{err}");
    }

    #[test]
    fn karakter_kontrol_ditolak_kecuali_baris_baru_di_catatan() {
        let input = PokmasInput { nama: Some("Tirta\u{0}".into()), ..Default::default() };
        assert!(input.validated().is_err());

        let input = PokmasInput {
            nama: Some("Tirta".into()),
            catatan: Some("baris satu\nbaris dua".into()),
            ..Default::default()
        };
        assert!(input.validated().is_ok());
    }
}

#[cfg(test)]
mod db_tests {
    use crate::{PokmasInput, Store};
    use sqlx::postgres::PgPool;

    fn isian(nama: &str) -> crate::PokmasIsian {
        PokmasInput { nama: Some(nama.into()), ..Default::default() }.validated().unwrap()
    }

    /// Butuh DATABASE_URL ke PostgreSQL dengan PostGIS (sqlx membuat database uji sementara).
    /// Jalankan: `cargo test -p pipa-store -- --ignored`
    #[ignore = "butuh database PostgreSQL"]
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn koreksi_menggantikan_sinkron_dan_bertahan_saat_sinkron_ulang(pool: PgPool) {
        sqlx::query("INSERT INTO users (username, password_hash, role) VALUES ('editor1', 'x', 'editor')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO ref_desa (id, nama, kecamatan) VALUES (1, 'Desa A', 'Kec'), (2, 'Desa B', 'Kec')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO ref_pokmas (desa_id, nama, konflik, kepala, sumber_unit_ids) \
             VALUES (1, 'Tirta Lestari', true, 'Hasan', '{11,12}')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let store = Store::from_pool(pool.clone());

        // Hanya data sinkron.
        let d = store.pokmas_detail(1).await.unwrap().unwrap();
        assert_eq!(d.efektif.sumber, crate::SumberPokmas::Arumanis);
        assert!(d.koreksi.is_none());

        // Koreksi menggantikan data sinkron.
        assert!(store.simpan_pokmas_koreksi(1, &isian("Tirta Makmur"), "editor1").await.unwrap());
        let d = store.pokmas_detail(1).await.unwrap().unwrap();
        assert_eq!(d.efektif.sumber, crate::SumberPokmas::Override);
        let p = d.efektif.pokmas.unwrap();
        assert_eq!(p.nama.as_deref(), Some("Tirta Makmur"));
        assert!(!p.konflik);
        assert_eq!(d.koreksi.unwrap().diperbarui_oleh, "editor1");

        // Upsert: simpan lagi menimpa koreksi yang lama.
        assert!(store.simpan_pokmas_koreksi(1, &isian("Tirta Jaya"), "editor1").await.unwrap());
        let p = store.pokmas_detail(1).await.unwrap().unwrap().efektif.pokmas.unwrap();
        assert_eq!(p.nama.as_deref(), Some("Tirta Jaya"));

        // Sinkron ulang (DELETE + INSERT ref_pokmas) tidak menghapus koreksi.
        sqlx::query("DELETE FROM ref_pokmas").execute(&pool).await.unwrap();
        let p = store.pokmas_detail(1).await.unwrap().unwrap().efektif.pokmas.unwrap();
        assert_eq!(p.nama.as_deref(), Some("Tirta Jaya"));

        // Desa tanpa sinkron tetap bisa punya koreksi, dan daftar desa menandainya punya Pokmas.
        assert!(store.simpan_pokmas_koreksi(2, &isian("Mandiri"), "editor1").await.unwrap());
        let daftar = store.list_desa().await.unwrap();
        assert!(daftar.iter().find(|d| d.id == 2).unwrap().punya_pokmas);

        // Hapus koreksi: kembali ke data sinkron (yang sudah kosong untuk desa 1).
        store.hapus_pokmas_koreksi(1).await.unwrap();
        let d = store.pokmas_detail(1).await.unwrap().unwrap();
        assert_eq!(d.efektif.sumber, crate::SumberPokmas::Arumanis);
        assert!(d.efektif.pokmas.is_none());
    }

    #[ignore = "butuh database PostgreSQL"]
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn desa_tidak_aktif_atau_tidak_ada_ditolak(pool: PgPool) {
        sqlx::query("INSERT INTO users (username, password_hash, role) VALUES ('editor1', 'x', 'editor')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO ref_desa (id, nama, kecamatan, aktif) VALUES (3, 'Desa C', 'Kec', false)")
            .execute(&pool)
            .await
            .unwrap();
        let store = Store::from_pool(pool.clone());

        assert!(store.pokmas_detail(3).await.unwrap().is_none());
        assert!(store.pokmas_detail(404).await.unwrap().is_none());
        assert!(!store.simpan_pokmas_koreksi(3, &isian("X"), "editor1").await.unwrap());
        assert!(!store.simpan_pokmas_koreksi(404, &isian("X"), "editor1").await.unwrap());
        assert!(!store.desa_aktif(3).await.unwrap());
    }
}
