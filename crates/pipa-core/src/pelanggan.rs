//! Aturan isian pelanggan: nama, alamat, nomor meter, golongan tarif, dan status. Setiap pelanggan terhubung
//! ke satu titik SR (fitur `sr`). Pemeriksaan bahwa SR itu ada dan berjenis `sr` dilakukan di pipa-store.

use serde::{Deserialize, Serialize};

pub const MAKS_NAMA_PELANGGAN: usize = 120;
pub const MAKS_ALAMAT: usize = 255;
pub const MAKS_NOMOR_METER: usize = 32;

/// Golongan tarif. Nilai teks sama dengan CHECK `pelanggan.golongan_tarif` di migrasi 0015.
/// Daftar ini masih perlu dikonfirmasi dengan tarif resmi PDAM setempat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GolonganTarif {
    Sosial,
    RumahTangga,
    Niaga,
    Industri,
    Khusus,
}

impl GolonganTarif {
    pub const ALL: [GolonganTarif; 5] = [
        GolonganTarif::Sosial,
        GolonganTarif::RumahTangga,
        GolonganTarif::Niaga,
        GolonganTarif::Industri,
        GolonganTarif::Khusus,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            GolonganTarif::Sosial => "sosial",
            GolonganTarif::RumahTangga => "rumah_tangga",
            GolonganTarif::Niaga => "niaga",
            GolonganTarif::Industri => "industri",
            GolonganTarif::Khusus => "khusus",
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        Self::ALL.into_iter().find(|g| g.as_str() == s).ok_or_else(|| format!("golongan tarif tidak dikenal: {s}"))
    }
}

/// Status pelanggan. `putus` berarti sambungan diputus. Data pelanggan tidak dihapus, sehingga riwayat tetap ada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusPelanggan {
    Aktif,
    Putus,
}

impl StatusPelanggan {
    pub fn as_str(self) -> &'static str {
        match self {
            StatusPelanggan::Aktif => "aktif",
            StatusPelanggan::Putus => "putus",
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "aktif" => Ok(StatusPelanggan::Aktif),
            "putus" => Ok(StatusPelanggan::Putus),
            other => Err(format!("status pelanggan tidak dikenal: {other}")),
        }
    }
}

/// Isian pelanggan dari body HTTP, sebelum divalidasi.
#[derive(Debug, Clone, Deserialize)]
pub struct PelangganMasukan {
    pub nama: String,
    #[serde(default)]
    pub alamat: Option<String>,
    #[serde(default)]
    pub nomor_meter: Option<String>,
    pub golongan_tarif: String,
    /// Bila tidak diisi, status dianggap `aktif`.
    #[serde(default)]
    pub status: Option<String>,
    pub sr_feature_id: i64,
}

/// Isian pelanggan yang sudah lolos validasi bentuk. Keberadaan SR diperiksa di database.
#[derive(Debug, Clone, PartialEq)]
pub struct PelangganIsian {
    pub nama: String,
    pub alamat: Option<String>,
    pub nomor_meter: Option<String>,
    pub golongan_tarif: GolonganTarif,
    pub status: StatusPelanggan,
    pub sr_feature_id: i64,
}

fn teks_opsional(v: &Option<String>, maks: usize, label: &str) -> Result<Option<String>, String> {
    let Some(raw) = v else { return Ok(None) };
    let t = raw.trim();
    if t.is_empty() {
        return Ok(None);
    }
    if t.chars().count() > maks {
        return Err(format!("{label} maksimal {maks} karakter"));
    }
    if t.chars().any(char::is_control) {
        return Err(format!("{label} tidak boleh berisi karakter kontrol"));
    }
    Ok(Some(t.to_string()))
}

/// Nomor meter: huruf, angka, tanda hubung, garis miring, dan titik. Spasi di ujung dihapus.
fn periksa_nomor_meter(s: &str) -> Result<(), String> {
    if s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '/' | '.')) {
        Ok(())
    } else {
        Err("nomor meter hanya boleh berisi huruf, angka, '-', '/', atau '.'".into())
    }
}

/// Validasi bentuk isian pelanggan. Nama wajib, golongan wajib dari daftar, SR harus positif.
pub fn validasi_pelanggan(m: &PelangganMasukan) -> Result<PelangganIsian, String> {
    let nama = m.nama.trim();
    if nama.chars().count() < 2 {
        return Err("nama pelanggan minimal 2 karakter".into());
    }
    if nama.chars().count() > MAKS_NAMA_PELANGGAN {
        return Err(format!("nama pelanggan maksimal {MAKS_NAMA_PELANGGAN} karakter"));
    }
    if nama.chars().any(char::is_control) {
        return Err("nama pelanggan tidak boleh berisi karakter kontrol".into());
    }
    let alamat = teks_opsional(&m.alamat, MAKS_ALAMAT, "alamat")?;
    let nomor_meter = teks_opsional(&m.nomor_meter, MAKS_NOMOR_METER, "nomor meter")?;
    if let Some(n) = &nomor_meter {
        periksa_nomor_meter(n)?;
    }
    let golongan_tarif = GolonganTarif::parse(&m.golongan_tarif)?;
    let status = match m.status.as_deref() {
        None => StatusPelanggan::Aktif,
        Some(s) => StatusPelanggan::parse(s)?,
    };
    if m.sr_feature_id <= 0 {
        return Err("titik SR wajib dipilih".into());
    }
    Ok(PelangganIsian {
        nama: nama.to_string(),
        alamat,
        nomor_meter,
        golongan_tarif,
        status,
        sr_feature_id: m.sr_feature_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn masukan() -> PelangganMasukan {
        PelangganMasukan {
            nama: "  Siti Aminah ".into(),
            alamat: Some(" Jl. Melati 3 ".into()),
            nomor_meter: Some("MTR-0042/A".into()),
            golongan_tarif: "rumah_tangga".into(),
            status: None,
            sr_feature_id: 17,
        }
    }

    #[test]
    fn isian_normal_lolos_dan_dirapikan() {
        let p = validasi_pelanggan(&masukan()).unwrap();
        assert_eq!(p.nama, "Siti Aminah");
        assert_eq!(p.alamat.as_deref(), Some("Jl. Melati 3"));
        assert_eq!(p.golongan_tarif, GolonganTarif::RumahTangga);
        assert_eq!(p.status, StatusPelanggan::Aktif);
    }

    #[test]
    fn nomor_meter_dan_alamat_boleh_kosong() {
        let mut m = masukan();
        m.nomor_meter = Some("   ".into());
        m.alamat = None;
        let p = validasi_pelanggan(&m).unwrap();
        assert_eq!(p.nomor_meter, None);
        assert_eq!(p.alamat, None);
    }

    #[test]
    fn nama_wajib_dan_batasnya() {
        let mut m = masukan();
        m.nama = " A ".into();
        assert!(validasi_pelanggan(&m).is_err());
        m.nama = "a".repeat(MAKS_NAMA_PELANGGAN + 1);
        assert!(validasi_pelanggan(&m).is_err());
        m.nama = "Budi\u{0000}".into();
        assert!(validasi_pelanggan(&m).is_err());
    }

    #[test]
    fn golongan_harus_dari_daftar() {
        let mut m = masukan();
        m.golongan_tarif = "Rumah Tangga".into();
        assert!(validasi_pelanggan(&m).is_err());
        for g in GolonganTarif::ALL {
            m.golongan_tarif = g.as_str().into();
            assert!(validasi_pelanggan(&m).is_ok(), "{g:?}");
        }
    }

    #[test]
    fn status_default_aktif_dan_hanya_dua_nilai() {
        let mut m = masukan();
        m.status = Some("putus".into());
        assert_eq!(validasi_pelanggan(&m).unwrap().status, StatusPelanggan::Putus);
        m.status = Some("nonaktif".into());
        assert!(validasi_pelanggan(&m).is_err());
    }

    #[test]
    fn nomor_meter_hanya_karakter_aman() {
        let mut m = masukan();
        m.nomor_meter = Some("12 34".into());
        assert!(validasi_pelanggan(&m).is_err());
        m.nomor_meter = Some("'; DROP".into());
        assert!(validasi_pelanggan(&m).is_err());
        m.nomor_meter = Some("x".repeat(MAKS_NOMOR_METER + 1));
        assert!(validasi_pelanggan(&m).is_err());
    }

    #[test]
    fn sr_wajib_positif() {
        let mut m = masukan();
        m.sr_feature_id = 0;
        assert!(validasi_pelanggan(&m).is_err());
    }

    #[test]
    fn golongan_dan_status_roundtrip_teks() {
        for g in GolonganTarif::ALL {
            assert_eq!(GolonganTarif::parse(g.as_str()).unwrap(), g);
        }
        assert_eq!(StatusPelanggan::parse("aktif").unwrap().as_str(), "aktif");
    }
}
