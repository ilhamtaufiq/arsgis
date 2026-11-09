//! Dasar bersama untuk operasi dan pemeliharaan (O&M): galat domain dan pemeriksaan teks.
//! Tidak bergantung pada HTTP atau database.

use thiserror::Error;

/// Galat O&M. Pemetaan ke HTTP ada di pipa-api: 400 untuk masukan, 422 untuk aturan, 404 untuk tidak ditemukan.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OmError {
    #[error("{0}")]
    Masukan(String),
    #[error("{}", .0.join("; "))]
    Aturan(Vec<String>),
    #[error("{0} tidak ditemukan")]
    TidakDitemukan(&'static str),
}

/// Teks wajib: dipangkas, tidak boleh kosong, dan tidak lebih dari `maks` karakter.
pub fn periksa_teks(nilai: &str, label: &str, maks: usize) -> Result<String, OmError> {
    let bersih = nilai.trim();
    if bersih.is_empty() {
        return Err(OmError::Masukan(format!("{label} wajib diisi")));
    }
    if bersih.chars().count() > maks {
        return Err(OmError::Masukan(format!("{label} maksimal {maks} karakter")));
    }
    Ok(bersih.to_string())
}

/// Teks opsional: kosong atau spasi saja dianggap tidak ada.
pub fn periksa_teks_opsional(nilai: Option<&str>, label: &str, maks: usize) -> Result<Option<String>, OmError> {
    match nilai {
        Some(v) if !v.trim().is_empty() => periksa_teks(v, label, maks).map(Some),
        _ => Ok(None),
    }
}

/// Titik WGS84 dalam urutan `(lon, lat)`.
pub fn periksa_titik(lon: f64, lat: f64) -> Result<(), OmError> {
    let valid = lon.is_finite()
        && lat.is_finite()
        && (-180.0..=180.0).contains(&lon)
        && (-90.0..=90.0).contains(&lat);
    if valid {
        Ok(())
    } else {
        Err(OmError::Masukan(format!("koordinat tidak valid: lon {lon}, lat {lat}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn teks_wajib_dipangkas_dan_dibatasi() {
        assert_eq!(periksa_teks("  pipa bocor  ", "Deskripsi", 20).unwrap(), "pipa bocor");
        assert!(periksa_teks("   ", "Deskripsi", 20).is_err());
        assert!(periksa_teks(&"a".repeat(21), "Deskripsi", 20).is_err());
        assert!(periksa_teks(&"a".repeat(20), "Deskripsi", 20).is_ok());
    }

    #[test]
    fn teks_opsional_kosong_menjadi_none() {
        assert_eq!(periksa_teks_opsional(None, "Catatan", 10).unwrap(), None);
        assert_eq!(periksa_teks_opsional(Some("  "), "Catatan", 10).unwrap(), None);
        assert_eq!(periksa_teks_opsional(Some(" ok "), "Catatan", 10).unwrap(), Some("ok".into()));
        assert!(periksa_teks_opsional(Some("terlalu panjang"), "Catatan", 5).is_err());
    }

    #[test]
    fn titik_harus_dalam_rentang() {
        assert!(periksa_titik(107.1, -6.8).is_ok());
        assert!(periksa_titik(181.0, 0.0).is_err());
        assert!(periksa_titik(0.0, 91.0).is_err());
        assert!(periksa_titik(f64::NAN, 0.0).is_err());
    }
}
