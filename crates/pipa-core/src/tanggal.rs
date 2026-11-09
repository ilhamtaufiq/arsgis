//! Tanggal kalender tanpa dependensi waktu. Disimpan sebagai jumlah hari sejak 1970-01-01, sehingga
//! selisih dan penjumlahan hari cukup aritmetika bilangan bulat. Teks yang dipakai API dan database
//! adalah `YYYY-MM-DD`.

use crate::om::OmError;

/// Tanggal dalam kalender Gregorian, disimpan sebagai hari sejak 1970-01-01.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tanggal(i64);

impl Tanggal {
    pub fn dari_hari(hari: i64) -> Self {
        Self(hari)
    }

    pub fn hari(self) -> i64 {
        self.0
    }

    pub fn tambah_hari(self, n: i64) -> Self {
        Self(self.0 + n)
    }

    /// Tanggal lokal dari detik Unix. `offset_menit` mengikuti zona waktu, misalnya 420 untuk WIB.
    pub fn dari_unix(detik: i64, offset_menit: i64) -> Self {
        Self((detik + offset_menit * 60).div_euclid(86_400))
    }

    /// Parse `YYYY-MM-DD` yang valid di kalender, termasuk tahun kabisat.
    pub fn parse(s: &str) -> Result<Self, OmError> {
        let galat = || OmError::Masukan(format!("tanggal harus berformat YYYY-MM-DD: {s}"));
        let b = s.as_bytes();
        if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
            return Err(galat());
        }
        let angka = |r: std::ops::Range<usize>| -> Option<i64> {
            let bagian = s.get(r)?;
            if bagian.bytes().all(|c| c.is_ascii_digit()) {
                bagian.parse().ok()
            } else {
                None
            }
        };
        let (Some(y), Some(m), Some(d)) = (angka(0..4), angka(5..7), angka(8..10)) else {
            return Err(galat());
        };
        if !(1900..=2200).contains(&y) || !(1..=12).contains(&m) || d < 1 || d > dari_bulan(y, m) {
            return Err(galat());
        }
        Ok(Self(hari_dari_sipil(y, m as u32, d as u32)))
    }

    pub fn format(self) -> String {
        let (y, m, d) = sipil_dari_hari(self.0);
        format!("{y:04}-{m:02}-{d:02}")
    }
}

fn kabisat(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn dari_bulan(y: i64, m: i64) -> i64 {
    match m {
        2 if kabisat(y) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Algoritma hari dari tanggal sipil (Howard Hinnant), dengan hari 0 = 1970-01-01.
fn hari_dari_sipil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn sipil_dari_hari(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_adalah_nol() {
        assert_eq!(Tanggal::parse("1970-01-01").unwrap().hari(), 0);
        assert_eq!(Tanggal::dari_hari(0).format(), "1970-01-01");
    }

    #[test]
    fn roundtrip_teks_dan_hari() {
        for s in ["2026-11-09", "2000-02-29", "1999-12-31", "2100-03-01", "1900-01-01"] {
            assert_eq!(Tanggal::parse(s).unwrap().format(), s);
        }
    }

    #[test]
    fn tahun_kabisat_dihitung_benar() {
        assert!(Tanggal::parse("2024-02-29").is_ok());
        assert!(Tanggal::parse("2000-02-29").is_ok());
        assert!(Tanggal::parse("2026-02-29").is_err());
        assert!(Tanggal::parse("2100-02-29").is_err());
    }

    #[test]
    fn tambah_hari_melewati_bulan_dan_tahun() {
        assert_eq!(Tanggal::parse("2026-12-31").unwrap().tambah_hari(1).format(), "2027-01-01");
        assert_eq!(Tanggal::parse("2026-01-31").unwrap().tambah_hari(30).format(), "2026-03-02");
        assert_eq!(Tanggal::parse("2026-03-01").unwrap().tambah_hari(-1).format(), "2026-02-28");
    }

    #[test]
    fn selisih_hari_konsisten() {
        let a = Tanggal::parse("2026-11-09").unwrap();
        let b = Tanggal::parse("2026-12-01").unwrap();
        assert_eq!(b.hari() - a.hari(), 22);
    }

    #[test]
    fn format_salah_ditolak() {
        for s in ["2026/11/09", "09-11-2026", "2026-13-01", "2026-00-10", "2026-11-31", "2026-1-09", "", "abcd-ef-gh"] {
            assert!(Tanggal::parse(s).is_err(), "{s} seharusnya ditolak");
        }
    }

    #[test]
    fn dari_unix_memakai_zona_waktu() {
        // 2026-11-08 17:30 UTC adalah 2026-11-09 00:30 WIB.
        let detik_utc = Tanggal::parse("2026-11-08").unwrap().hari() * 86_400 + 17 * 3600 + 30 * 60;
        assert_eq!(Tanggal::dari_unix(detik_utc, 7 * 60).format(), "2026-11-09");
        assert_eq!(Tanggal::dari_unix(detik_utc, 0).format(), "2026-11-08");
    }

    #[test]
    fn dari_unix_sebelum_epoch_tetap_benar() {
        assert_eq!(Tanggal::dari_unix(-1, 0).format(), "1969-12-31");
    }
}
