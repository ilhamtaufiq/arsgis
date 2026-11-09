//! Aturan kelola impor yang tidak butuh database: batal impor, pulihkan aset, dan deteksi berkas ganda.
//! Fungsi di sini murni (tanpa SQL atau HTTP). Pemanggilnya ada di pipa-store dan pipa-api.

use thiserror::Error;

/// Status batch impor. Nilainya sama dengan CHECK di migrasi 0020.
pub const STATUS_STAGED: &str = "staged";
pub const STATUS_COMMITTED: &str = "committed";
pub const STATUS_DIBATALKAN: &str = "dibatalkan";

/// Pelanggaran aturan kelola impor. Pemetaan ke HTTP ada di pipa-api.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum KelolaError {
    #[error("batch impor sudah dibatalkan")]
    SudahDibatalkan,
    #[error("status batch impor tidak dikenal: {0}")]
    StatusTidakDikenal(String),
    #[error("aset tidak sedang dihapus, jadi tidak bisa dipulihkan")]
    TidakTerhapus,
}

/// Batch boleh dibatalkan bila masih `staged` atau `committed`. Batch yang sudah dibatalkan tidak boleh lagi.
pub fn periksa_batal(status: &str) -> Result<(), KelolaError> {
    match status {
        STATUS_STAGED | STATUS_COMMITTED => Ok(()),
        STATUS_DIBATALKAN => Err(KelolaError::SudahDibatalkan),
        lain => Err(KelolaError::StatusTidakDikenal(lain.to_string())),
    }
}

/// Aset hanya bisa dipulihkan bila sedang terhapus (soft delete).
pub fn periksa_pulih(terhapus: bool) -> Result<(), KelolaError> {
    if terhapus {
        Ok(())
    } else {
        Err(KelolaError::TidakTerhapus)
    }
}

/// Batch yang sudah di-commit dan punya hash berkas yang sama dengan berkas baru.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KandidatDuplikat {
    pub batch_id: i64,
    pub nama_file: String,
    /// Waktu batch dibuat, sudah diformat untuk API.
    pub created_at: String,
    /// sha256 berkas asli dalam hex.
    pub file_sha256: String,
}

/// Pilih batch committed pertama yang berkas aslinya sama dengan `hash`.
/// Batch `sendiri` dilewati (saat commit, supaya batch tidak dianggap duplikat dirinya sendiri).
/// Perbandingan hex tidak peka huruf besar-kecil. Yang dipilih adalah batch dengan id terkecil.
pub fn cari_duplikat<'a>(
    hash: &str,
    sendiri: Option<i64>,
    kandidat: &'a [KandidatDuplikat],
) -> Option<&'a KandidatDuplikat> {
    kandidat
        .iter()
        .filter(|k| Some(k.batch_id) != sendiri)
        .filter(|k| k.file_sha256.eq_ignore_ascii_case(hash))
        .min_by_key(|k| k.batch_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kandidat(id: i64, hash: &str) -> KandidatDuplikat {
        KandidatDuplikat {
            batch_id: id,
            nama_file: format!("impor-{id}.kml"),
            created_at: "2026-10-01T08:00:00+07:00".into(),
            file_sha256: hash.into(),
        }
    }

    #[test]
    fn batal_boleh_untuk_staged_dan_committed() {
        assert_eq!(periksa_batal("staged"), Ok(()));
        assert_eq!(periksa_batal("committed"), Ok(()));
    }

    #[test]
    fn batal_ditolak_bila_sudah_dibatalkan() {
        assert_eq!(periksa_batal("dibatalkan"), Err(KelolaError::SudahDibatalkan));
    }

    #[test]
    fn batal_menolak_status_tidak_dikenal() {
        assert_eq!(
            periksa_batal("entah"),
            Err(KelolaError::StatusTidakDikenal("entah".into()))
        );
    }

    #[test]
    fn pulih_hanya_untuk_aset_terhapus() {
        assert_eq!(periksa_pulih(true), Ok(()));
        assert_eq!(periksa_pulih(false), Err(KelolaError::TidakTerhapus));
    }

    #[test]
    fn duplikat_cocok_hanya_bila_hash_sama() {
        let daftar = [kandidat(4, "aa11"), kandidat(7, "bb22")];
        assert_eq!(cari_duplikat("bb22", None, &daftar).map(|k| k.batch_id), Some(7));
        assert_eq!(cari_duplikat("cc33", None, &daftar), None);
    }

    #[test]
    fn duplikat_tidak_peka_huruf_besar_kecil_hex() {
        let daftar = [kandidat(3, "abcdef")];
        assert_eq!(cari_duplikat("ABCDEF", None, &daftar).map(|k| k.batch_id), Some(3));
    }

    #[test]
    fn batch_sendiri_tidak_dihitung_duplikat() {
        let daftar = [kandidat(9, "aa11")];
        assert_eq!(cari_duplikat("aa11", Some(9), &daftar), None);
        // Batch lain dengan hash sama tetap terdeteksi walau batch sendiri ikut dalam daftar.
        let daftar = [kandidat(9, "aa11"), kandidat(2, "aa11")];
        assert_eq!(cari_duplikat("aa11", Some(9), &daftar).map(|k| k.batch_id), Some(2));
    }

    #[test]
    fn duplikat_yang_dipilih_adalah_batch_tertua() {
        let daftar = [kandidat(12, "aa11"), kandidat(5, "aa11"), kandidat(8, "aa11")];
        assert_eq!(cari_duplikat("aa11", None, &daftar).map(|k| k.batch_id), Some(5));
    }
}
