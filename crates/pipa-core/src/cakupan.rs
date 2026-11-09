//! Perhitungan cakupan layanan per dusun. Data mentah (jumlah SR, SR yang terlayani, jumlah rumah dan jiwa)
//! diambil dari database oleh pipa-store. Di sini hanya rumus dan validasi isian dusun.
//!
//! Definisi:
//! - SR terlayani: titik SR yang punya minimal satu pelanggan berstatus `aktif`.
//! - SR belum: SR dalam dusun yang tidak punya pelanggan aktif.
//! - Persen cakupan: SR terlayani dibagi jumlah rumah dusun (dasar rumah, sesuai target SPM).
//! - Persen SR terlayani: SR terlayani dibagi jumlah SR dusun (dasar titik SR yang tercatat).
//! - Jiwa terlayani perkiraan: jumlah jiwa dusun dikali proporsi rumah terlayani. Ini estimasi, bukan
//!   hitungan langsung, karena pelanggan belum mencatat jumlah jiwa.

use serde::Deserialize;
use serde_json::{json, Value};

/// Batas nilai jumlah rumah dan jiwa per dusun. Mencegah salah ketik satu-dua digit menjadi angka tidak masuk akal.
pub const MAKS_RUMAH_JIWA: i64 = 10_000_000;
pub const MAKS_NAMA_DUSUN: usize = 120;

/// Isian dusun dari body HTTP.
#[derive(Debug, Clone, Deserialize)]
pub struct DusunMasukan {
    pub nama: String,
    pub jumlah_rumah: i64,
    pub jumlah_jiwa: i64,
}

/// Isian dusun yang sudah lolos validasi.
#[derive(Debug, Clone, PartialEq)]
pub struct DusunIsian {
    pub nama: String,
    pub jumlah_rumah: i64,
    pub jumlah_jiwa: i64,
}

/// Hasil hitung cakupan untuk satu dusun atau satu desa.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Cakupan {
    pub jumlah_sr: i64,
    pub sr_terlayani: i64,
    pub sr_belum: i64,
    /// Persen terhadap jumlah rumah, dibulatkan satu desimal. None bila jumlah rumah nol.
    pub persen_cakupan: Option<f64>,
    /// Persen SR terlayani terhadap jumlah SR. None bila belum ada SR.
    pub persen_sr_terlayani: Option<f64>,
    /// Estimasi jiwa terlayani. None bila jumlah rumah nol.
    pub jiwa_terlayani_perkiraan: Option<i64>,
}

fn satu_desimal(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

/// Menghitung cakupan dari angka mentah. `sr_terlayani` dibatasi ke `jumlah_sr` supaya hasil tidak
/// pernah lebih dari 100 persen SR.
pub fn hitung_cakupan(jumlah_sr: i64, sr_terlayani: i64, jumlah_rumah: i64, jumlah_jiwa: i64) -> Cakupan {
    let jumlah_sr = jumlah_sr.max(0);
    let terlayani = sr_terlayani.clamp(0, jumlah_sr);
    let rumah = jumlah_rumah.max(0);
    let jiwa = jumlah_jiwa.max(0);

    let persen_cakupan = (rumah > 0).then(|| satu_desimal(terlayani as f64 * 100.0 / rumah as f64));
    let persen_sr_terlayani = (jumlah_sr > 0).then(|| satu_desimal(terlayani as f64 * 100.0 / jumlah_sr as f64));
    // Proporsi dibatasi ke satu, sebab rumah terlayani tidak mungkin melebihi jumlah rumah.
    let jiwa_terlayani_perkiraan = (rumah > 0).then(|| {
        let proporsi = (terlayani.min(rumah)) as f64 / rumah as f64;
        (jiwa as f64 * proporsi).round() as i64
    });

    Cakupan {
        jumlah_sr,
        sr_terlayani: terlayani,
        sr_belum: jumlah_sr - terlayani,
        persen_cakupan,
        persen_sr_terlayani,
        jiwa_terlayani_perkiraan,
    }
}

/// Validasi isian dusun. Nama wajib, tidak boleh berisi karakter kontrol, dan jumlah harus berada di rentang wajar.
pub fn validasi_dusun(m: &DusunMasukan) -> Result<DusunIsian, String> {
    let nama = m.nama.trim();
    if nama.is_empty() {
        return Err("nama dusun wajib diisi".into());
    }
    if nama.chars().count() > MAKS_NAMA_DUSUN {
        return Err(format!("nama dusun maksimal {MAKS_NAMA_DUSUN} karakter"));
    }
    if nama.chars().any(char::is_control) {
        return Err("nama dusun tidak boleh berisi karakter kontrol".into());
    }
    for (label, nilai) in [("jumlah rumah", m.jumlah_rumah), ("jumlah jiwa", m.jumlah_jiwa)] {
        if !(0..=MAKS_RUMAH_JIWA).contains(&nilai) {
            return Err(format!("{label} harus antara 0 dan {MAKS_RUMAH_JIWA}"));
        }
    }
    if m.jumlah_jiwa > 0 && m.jumlah_rumah == 0 {
        return Err("jumlah jiwa diisi tetapi jumlah rumah nol".into());
    }
    Ok(DusunIsian { nama: nama.to_string(), jumlah_rumah: m.jumlah_rumah, jumlah_jiwa: m.jumlah_jiwa })
}

/// Bentuk JSON cakupan untuk API. Dipisah dari struct supaya nama field tetap stabil.
pub fn cakupan_json(c: &Cakupan) -> Value {
    json!({
        "jumlah_sr": c.jumlah_sr,
        "sr_terlayani": c.sr_terlayani,
        "sr_belum": c.sr_belum,
        "persen_cakupan": c.persen_cakupan,
        "persen_sr_terlayani": c.persen_sr_terlayani,
        "jiwa_terlayani_perkiraan": c.jiwa_terlayani_perkiraan,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn masukan(nama: &str, rumah: i64, jiwa: i64) -> DusunMasukan {
        DusunMasukan { nama: nama.into(), jumlah_rumah: rumah, jumlah_jiwa: jiwa }
    }

    #[test]
    fn cakupan_dasar() {
        let c = hitung_cakupan(40, 30, 100, 420);
        assert_eq!(c.sr_belum, 10);
        assert_eq!(c.persen_cakupan, Some(30.0));
        assert_eq!(c.persen_sr_terlayani, Some(75.0));
        assert_eq!(c.jiwa_terlayani_perkiraan, Some(126));
    }

    #[test]
    fn cakupan_tanpa_rumah_tidak_punya_persen() {
        let c = hitung_cakupan(5, 2, 0, 0);
        assert_eq!(c.persen_cakupan, None);
        assert_eq!(c.jiwa_terlayani_perkiraan, None);
        assert_eq!(c.persen_sr_terlayani, Some(40.0));
    }

    #[test]
    fn cakupan_tanpa_sr_tidak_punya_persen_sr() {
        let c = hitung_cakupan(0, 0, 50, 200);
        assert_eq!(c.persen_sr_terlayani, None);
        assert_eq!(c.persen_cakupan, Some(0.0));
        assert_eq!(c.sr_belum, 0);
    }

    #[test]
    fn terlayani_tidak_melebihi_sr_maupun_rumah() {
        // Data tidak konsisten (lebih banyak terlayani dari SR) tetap menghasilkan angka wajar.
        let c = hitung_cakupan(10, 14, 8, 30);
        assert_eq!(c.sr_terlayani, 10);
        assert_eq!(c.sr_belum, 0);
        assert_eq!(c.persen_sr_terlayani, Some(100.0));
        assert_eq!(c.jiwa_terlayani_perkiraan, Some(30));
    }

    #[test]
    fn persen_dibulatkan_satu_desimal() {
        let c = hitung_cakupan(3, 1, 3, 0);
        assert_eq!(c.persen_cakupan, Some(33.3));
    }

    #[test]
    fn validasi_dusun_menerima_isian_normal_dan_merapikan_nama() {
        let d = validasi_dusun(&masukan("  Cibodas  ", 120, 480)).unwrap();
        assert_eq!(d.nama, "Cibodas");
        assert_eq!(d.jumlah_rumah, 120);
    }

    #[test]
    fn validasi_dusun_menolak_isian_tidak_masuk_akal() {
        assert!(validasi_dusun(&masukan("   ", 1, 1)).is_err());
        assert!(validasi_dusun(&masukan("Ci\u{0007}bodas", 1, 1)).is_err());
        assert!(validasi_dusun(&masukan(&"a".repeat(MAKS_NAMA_DUSUN + 1), 1, 1)).is_err());
        assert!(validasi_dusun(&masukan("X", -1, 0)).is_err());
        assert!(validasi_dusun(&masukan("X", MAKS_RUMAH_JIWA + 1, 0)).is_err());
        assert!(validasi_dusun(&masukan("X", 0, 5)).is_err());
        assert!(validasi_dusun(&masukan("X", 0, 0)).is_ok());
    }

    #[test]
    fn cakupan_json_memakai_nama_field_yang_stabil() {
        let v = cakupan_json(&hitung_cakupan(4, 2, 4, 16));
        assert_eq!(v["sr_belum"], 2);
        assert_eq!(v["persen_cakupan"], 50.0);
        assert_eq!(v["jiwa_terlayani_perkiraan"], 8);
    }
}
