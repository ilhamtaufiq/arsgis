//! Jadwal perawatan aset: interval, tanggal berikutnya, dan status jatuh tempo.
//! Semua perhitungan memakai `Tanggal` dan tidak membaca jam sistem, supaya mudah diuji.

use crate::om::OmError;
use crate::tanggal::Tanggal;

pub const INTERVAL_MIN_HARI: i64 = 1;
pub const INTERVAL_MAKS_HARI: i64 = 3650;
/// Jendela daftar jatuh tempo: paling jauh 365 hari ke depan.
pub const JENDELA_MAKS_HARI: i64 = 365;
pub const JENDELA_DEFAULT_HARI: i64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JenisPerawatan {
    PembersihanBak,
    CekValve,
    KalibrasiMeter,
    Lainnya,
}

impl JenisPerawatan {
    pub const ALL: [JenisPerawatan; 4] = [
        JenisPerawatan::PembersihanBak,
        JenisPerawatan::CekValve,
        JenisPerawatan::KalibrasiMeter,
        JenisPerawatan::Lainnya,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            JenisPerawatan::PembersihanBak => "pembersihan_bak",
            JenisPerawatan::CekValve => "cek_valve",
            JenisPerawatan::KalibrasiMeter => "kalibrasi_meter",
            JenisPerawatan::Lainnya => "lainnya",
        }
    }

    pub fn parse(s: &str) -> Result<Self, OmError> {
        Self::ALL
            .into_iter()
            .find(|j| j.as_str() == s)
            .ok_or_else(|| OmError::Masukan(format!("jenis perawatan tidak dikenal: {s}")))
    }
}

pub fn periksa_interval(hari: i64) -> Result<(), OmError> {
    if (INTERVAL_MIN_HARI..=INTERVAL_MAKS_HARI).contains(&hari) {
        Ok(())
    } else {
        Err(OmError::Masukan(format!(
            "interval harus {INTERVAL_MIN_HARI}–{INTERVAL_MAKS_HARI} hari, bukan {hari}"
        )))
    }
}

/// Jendela daftar jatuh tempo dalam hari, 0 sampai 365.
pub fn periksa_jendela(hari: i64) -> Result<i64, OmError> {
    if (0..=JENDELA_MAKS_HARI).contains(&hari) {
        Ok(hari)
    } else {
        Err(OmError::Masukan(format!("jendela hari harus 0–{JENDELA_MAKS_HARI}")))
    }
}

/// Tanggal perawatan berikutnya: tanggal terakhir dilakukan ditambah interval.
pub fn tanggal_berikutnya(terakhir_dilakukan: Tanggal, interval_hari: i64) -> Tanggal {
    terakhir_dilakukan.tambah_hari(interval_hari)
}

/// Pencatatan perawatan tidak boleh di masa depan. Tanggal mundur diizinkan untuk pencatatan yang terlambat masuk.
pub fn periksa_pencatatan(dilakukan: Tanggal, hari_ini: Tanggal) -> Result<(), OmError> {
    if dilakukan > hari_ini {
        Err(OmError::Aturan(vec!["tanggal pelaksanaan belum boleh di masa depan".into()]))
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusJatuhTempo {
    /// Sudah lewat dari tanggal berikutnya.
    Terlambat,
    /// Jatuh tempo hari ini.
    HariIni,
    /// Jatuh tempo dalam jendela yang diminta.
    Segera,
    /// Di luar jendela.
    Nanti,
}

impl StatusJatuhTempo {
    pub fn as_str(self) -> &'static str {
        match self {
            StatusJatuhTempo::Terlambat => "terlambat",
            StatusJatuhTempo::HariIni => "hari_ini",
            StatusJatuhTempo::Segera => "segera",
            StatusJatuhTempo::Nanti => "nanti",
        }
    }
}

/// Selisih hari dari hari ini ke tanggal berikutnya. Negatif berarti sudah lewat.
pub fn selisih_hari(berikutnya: Tanggal, hari_ini: Tanggal) -> i64 {
    berikutnya.hari() - hari_ini.hari()
}

pub fn status_jatuh_tempo(berikutnya: Tanggal, hari_ini: Tanggal, jendela_hari: i64) -> StatusJatuhTempo {
    let selisih = selisih_hari(berikutnya, hari_ini);
    if selisih < 0 {
        StatusJatuhTempo::Terlambat
    } else if selisih == 0 {
        StatusJatuhTempo::HariIni
    } else if selisih <= jendela_hari {
        StatusJatuhTempo::Segera
    } else {
        StatusJatuhTempo::Nanti
    }
}

/// Masuk daftar "jatuh tempo dalam N hari" bila sudah lewat, hari ini, atau dalam jendela.
pub fn masuk_daftar_jatuh_tempo(berikutnya: Tanggal, hari_ini: Tanggal, jendela_hari: i64) -> bool {
    selisih_hari(berikutnya, hari_ini) <= jendela_hari
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> Tanggal {
        Tanggal::parse(s).unwrap()
    }

    #[test]
    fn jenis_perawatan_roundtrip() {
        for j in JenisPerawatan::ALL {
            assert_eq!(JenisPerawatan::parse(j.as_str()).unwrap(), j);
        }
        assert!(JenisPerawatan::parse("ganti_pipa").is_err());
    }

    #[test]
    fn interval_dibatasi_1_sampai_3650() {
        assert!(periksa_interval(1).is_ok());
        assert!(periksa_interval(3650).is_ok());
        assert!(periksa_interval(0).is_err());
        assert!(periksa_interval(-7).is_err());
        assert!(periksa_interval(3651).is_err());
    }

    #[test]
    fn jendela_dibatasi_0_sampai_365() {
        assert_eq!(periksa_jendela(0).unwrap(), 0);
        assert_eq!(periksa_jendela(365).unwrap(), 365);
        assert!(periksa_jendela(366).is_err());
        assert!(periksa_jendela(-1).is_err());
    }

    #[test]
    fn tanggal_berikutnya_menambah_interval() {
        // Bak dibersihkan 2026-01-15 dengan interval 90 hari.
        assert_eq!(tanggal_berikutnya(t("2026-01-15"), 90).format(), "2026-04-15");
        // Melewati tahun kabisat dan pergantian tahun.
        assert_eq!(tanggal_berikutnya(t("2027-12-20"), 30).format(), "2028-01-19");
    }

    #[test]
    fn pencatatan_tidak_boleh_masa_depan() {
        let hari_ini = t("2026-11-09");
        assert!(periksa_pencatatan(t("2026-11-09"), hari_ini).is_ok());
        assert!(periksa_pencatatan(t("2026-10-01"), hari_ini).is_ok());
        assert!(matches!(periksa_pencatatan(t("2026-11-10"), hari_ini), Err(OmError::Aturan(_))));
    }

    #[test]
    fn status_jatuh_tempo_per_selisih() {
        let hari_ini = t("2026-11-09");
        assert_eq!(status_jatuh_tempo(t("2026-11-08"), hari_ini, 30), StatusJatuhTempo::Terlambat);
        assert_eq!(status_jatuh_tempo(t("2026-11-09"), hari_ini, 30), StatusJatuhTempo::HariIni);
        assert_eq!(status_jatuh_tempo(t("2026-11-10"), hari_ini, 30), StatusJatuhTempo::Segera);
        assert_eq!(status_jatuh_tempo(t("2026-12-09"), hari_ini, 30), StatusJatuhTempo::Segera);
        assert_eq!(status_jatuh_tempo(t("2026-12-10"), hari_ini, 30), StatusJatuhTempo::Nanti);
    }

    #[test]
    fn daftar_mencakup_terlambat_dan_jendela() {
        let hari_ini = t("2026-11-09");
        // Terlambat tetap masuk daftar, berapa pun lamanya.
        assert!(masuk_daftar_jatuh_tempo(t("2025-01-01"), hari_ini, 7));
        assert!(masuk_daftar_jatuh_tempo(t("2026-11-16"), hari_ini, 7));
        assert!(!masuk_daftar_jatuh_tempo(t("2026-11-17"), hari_ini, 7));
        // Jendela 0 hanya menampilkan yang sudah lewat dan hari ini.
        assert!(masuk_daftar_jatuh_tempo(t("2026-11-09"), hari_ini, 0));
        assert!(!masuk_daftar_jatuh_tempo(t("2026-11-10"), hari_ini, 0));
    }
}
