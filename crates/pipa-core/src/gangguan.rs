//! Pelaporan gangguan layanan: jenis, lokasi, status, dan aturan foto. Tidak bergantung pada database.

use crate::om::{periksa_teks, periksa_titik, OmError};

pub const MAKS_DESKRIPSI: usize = 2000;
pub const MAKS_PELAPOR: usize = 100;
/// Batas foto per laporan.
pub const MAKS_FOTO_PER_LAPORAN: i64 = 5;
/// Batas ukuran satu foto: 5 MB.
pub const MAKS_FOTO_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JenisGangguan {
    PipaBocor,
    AirMati,
    KualitasKeruh,
    Lainnya,
}

impl JenisGangguan {
    pub const ALL: [JenisGangguan; 4] = [
        JenisGangguan::PipaBocor,
        JenisGangguan::AirMati,
        JenisGangguan::KualitasKeruh,
        JenisGangguan::Lainnya,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            JenisGangguan::PipaBocor => "pipa_bocor",
            JenisGangguan::AirMati => "air_mati",
            JenisGangguan::KualitasKeruh => "kualitas_keruh",
            JenisGangguan::Lainnya => "lainnya",
        }
    }

    pub fn parse(s: &str) -> Result<Self, OmError> {
        Self::ALL
            .into_iter()
            .find(|j| j.as_str() == s)
            .ok_or_else(|| OmError::Masukan(format!("jenis gangguan tidak dikenal: {s}")))
    }
}

/// Status laporan. Urutannya maju: dilaporkan, ditangani, lalu selesai.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StatusGangguan {
    Dilaporkan,
    Ditangani,
    Selesai,
}

impl StatusGangguan {
    pub fn as_str(self) -> &'static str {
        match self {
            StatusGangguan::Dilaporkan => "dilaporkan",
            StatusGangguan::Ditangani => "ditangani",
            StatusGangguan::Selesai => "selesai",
        }
    }

    pub fn parse(s: &str) -> Result<Self, OmError> {
        match s {
            "dilaporkan" => Ok(StatusGangguan::Dilaporkan),
            "ditangani" => Ok(StatusGangguan::Ditangani),
            "selesai" => Ok(StatusGangguan::Selesai),
            other => Err(OmError::Masukan(format!("status laporan tidak dikenal: {other}"))),
        }
    }
}

/// Aturan transisi status. Status hanya boleh maju. Melompat dari dilaporkan langsung ke selesai
/// diizinkan untuk gangguan yang selesai ditangani saat itu juga. Mengirim status yang sama adalah no-op.
pub fn periksa_transisi(dari: StatusGangguan, ke: StatusGangguan) -> Result<(), OmError> {
    if dari == ke || ke > dari {
        Ok(())
    } else {
        Err(OmError::Aturan(vec![format!(
            "status laporan tidak boleh mundur dari {} ke {}",
            dari.as_str(),
            ke.as_str()
        )]))
    }
}

/// Lokasi laporan: titik koordinat, aset terkait, atau keduanya. Minimal salah satu harus ada.
pub fn periksa_lokasi(feature_id: Option<i64>, titik: Option<(f64, f64)>) -> Result<(), OmError> {
    if let Some(id) = feature_id {
        if id <= 0 {
            return Err(OmError::Masukan("id aset tidak valid".into()));
        }
    }
    if let Some((lon, lat)) = titik {
        periksa_titik(lon, lat)?;
    }
    if feature_id.is_none() && titik.is_none() {
        return Err(OmError::Masukan("lokasi wajib: isi titik koordinat atau aset terkait".into()));
    }
    Ok(())
}

/// Isian laporan yang sudah dipangkas: deskripsi dan nama pelapor.
pub fn periksa_laporan(deskripsi: &str, pelapor: &str) -> Result<(String, String), OmError> {
    Ok((
        periksa_teks(deskripsi, "deskripsi", MAKS_DESKRIPSI)?,
        periksa_teks(pelapor, "pelapor", MAKS_PELAPOR)?,
    ))
}

/// Tipe foto dari tanda awal berkas, bukan dari header yang dikirim klien.
/// Hanya JPEG dan PNG yang diterima.
pub fn deteksi_tipe_foto(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("image/png")
    } else {
        None
    }
}

pub fn ekstensi_foto(mime: &str) -> Option<&'static str> {
    match mime {
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        _ => None,
    }
}

/// Memeriksa satu foto sebelum disimpan. `jumlah_sekarang` adalah foto yang sudah ada pada laporan.
pub fn periksa_foto(bytes: &[u8], jumlah_sekarang: i64) -> Result<&'static str, OmError> {
    if bytes.is_empty() {
        return Err(OmError::Masukan("berkas foto kosong".into()));
    }
    if bytes.len() > MAKS_FOTO_BYTES {
        return Err(OmError::Masukan(format!("ukuran foto maksimal {} MB", MAKS_FOTO_BYTES / 1024 / 1024)));
    }
    let Some(mime) = deteksi_tipe_foto(bytes) else {
        return Err(OmError::Masukan("foto harus berformat JPEG atau PNG".into()));
    };
    if jumlah_sekarang >= MAKS_FOTO_PER_LAPORAN {
        return Err(OmError::Aturan(vec![format!(
            "laporan sudah punya {MAKS_FOTO_PER_LAPORAN} foto, batas maksimalnya"
        )]));
    }
    Ok(mime)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jenis_roundtrip_dan_tolak_yang_tidak_dikenal() {
        for j in JenisGangguan::ALL {
            assert_eq!(JenisGangguan::parse(j.as_str()).unwrap(), j);
        }
        assert!(JenisGangguan::parse("PIPA_BOCOR").is_err());
        assert!(JenisGangguan::parse("sumur").is_err());
    }

    #[test]
    fn status_hanya_tiga_nilai() {
        assert_eq!(StatusGangguan::parse("dilaporkan").unwrap(), StatusGangguan::Dilaporkan);
        assert_eq!(StatusGangguan::parse("ditangani").unwrap(), StatusGangguan::Ditangani);
        assert_eq!(StatusGangguan::parse("selesai").unwrap(), StatusGangguan::Selesai);
        assert!(StatusGangguan::parse("ditutup").is_err());
        assert!(StatusGangguan::parse("Selesai").is_err());
    }

    #[test]
    fn transisi_maju_diizinkan() {
        use StatusGangguan::*;
        assert!(periksa_transisi(Dilaporkan, Ditangani).is_ok());
        assert!(periksa_transisi(Ditangani, Selesai).is_ok());
        assert!(periksa_transisi(Dilaporkan, Selesai).is_ok());
        // Mengirim status yang sama tidak mengubah apa pun.
        assert!(periksa_transisi(Ditangani, Ditangani).is_ok());
    }

    #[test]
    fn transisi_mundur_ditolak() {
        use StatusGangguan::*;
        assert!(matches!(periksa_transisi(Selesai, Ditangani), Err(OmError::Aturan(_))));
        assert!(matches!(periksa_transisi(Selesai, Dilaporkan), Err(OmError::Aturan(_))));
        assert!(matches!(periksa_transisi(Ditangani, Dilaporkan), Err(OmError::Aturan(_))));
    }

    #[test]
    fn lokasi_wajib_salah_satu() {
        assert!(periksa_lokasi(None, None).is_err());
        assert!(periksa_lokasi(Some(42), None).is_ok());
        assert!(periksa_lokasi(None, Some((107.1, -6.8))).is_ok());
        assert!(periksa_lokasi(Some(42), Some((107.1, -6.8))).is_ok());
        assert!(periksa_lokasi(Some(0), None).is_err());
        assert!(periksa_lokasi(None, Some((200.0, 0.0))).is_err());
    }

    #[test]
    fn laporan_wajib_deskripsi_dan_pelapor() {
        let (d, p) = periksa_laporan("  Air keruh di RT 02 ", " Siti ").unwrap();
        assert_eq!(d, "Air keruh di RT 02");
        assert_eq!(p, "Siti");
        assert!(periksa_laporan("   ", "Siti").is_err());
        assert!(periksa_laporan("ok", "").is_err());
        assert!(periksa_laporan(&"x".repeat(MAKS_DESKRIPSI + 1), "Siti").is_err());
    }

    #[test]
    fn foto_dikenali_dari_tanda_awal() {
        assert_eq!(deteksi_tipe_foto(&[0xFF, 0xD8, 0xFF, 0xE0, 0x00]), Some("image/jpeg"));
        assert_eq!(
            deteksi_tipe_foto(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00]),
            Some("image/png")
        );
        // Berkas GIF atau teks biasa ditolak meskipun namanya .jpg.
        assert_eq!(deteksi_tipe_foto(b"GIF89a"), None);
        assert_eq!(deteksi_tipe_foto(b"<html>"), None);
        assert_eq!(deteksi_tipe_foto(&[]), None);
    }

    #[test]
    fn foto_memeriksa_ukuran_tipe_dan_jumlah() {
        let jpeg = [0xFF, 0xD8, 0xFF, 0x00];
        assert_eq!(periksa_foto(&jpeg, 0).unwrap(), "image/jpeg");
        assert!(matches!(periksa_foto(&[], 0), Err(OmError::Masukan(_))));
        assert!(matches!(periksa_foto(b"bukan foto", 0), Err(OmError::Masukan(_))));
        assert!(matches!(periksa_foto(&jpeg, MAKS_FOTO_PER_LAPORAN), Err(OmError::Aturan(_))));
        assert!(periksa_foto(&jpeg, MAKS_FOTO_PER_LAPORAN - 1).is_ok());

        let terlalu_besar = {
            let mut v = vec![0xFF, 0xD8, 0xFF];
            v.resize(MAKS_FOTO_BYTES + 1, 0);
            v
        };
        assert!(matches!(periksa_foto(&terlalu_besar, 0), Err(OmError::Masukan(_))));
    }

    #[test]
    fn ekstensi_hanya_untuk_tipe_yang_diterima() {
        assert_eq!(ekstensi_foto("image/jpeg"), Some("jpg"));
        assert_eq!(ekstensi_foto("image/png"), Some("png"));
        assert_eq!(ekstensi_foto("image/gif"), None);
    }
}
