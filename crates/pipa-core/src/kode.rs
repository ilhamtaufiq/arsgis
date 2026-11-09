//! Kode aset unik. Format yang didokumentasikan:
//!
//! `<kode_desa>-<jenis>-<nomor urut>`, contoh `0012-pipa-000045` atau `0012-bak_penahan_tekanan-000002`.
//!
//! - `kode_desa`: id desa di Arumanis, empat digit dengan nol di depan. Jaringan tanpa desa memakai
//!   `J` + id jaringan tiga digit (`J007`), supaya tidak bertabrakan dengan kode desa.
//! - `jenis`: nilai `feature_type` apa adanya (huruf kecil, garis bawah).
//! - `nomor urut`: enam digit, dihitung per pasangan (kode_desa, jenis). Nomor tidak pernah dipakai ulang,
//!   termasuk untuk aset yang sudah dihapus (hapus lunak).
//!
//! Nomor urut diambil dari tabel `kode_aset_urut` di database, dalam transaksi yang sama dengan penulisan
//! aset. Fungsi di sini hanya menyusun teks, sehingga format bisa diuji tanpa database.

/// Kode desa untuk satu jaringan. `desa_id` None berarti jaringan belum punya desa.
pub fn kode_desa(desa_id: Option<i64>, network_id: i64) -> String {
    match desa_id {
        Some(id) => format!("{id:04}"),
        None => format!("J{network_id:03}"),
    }
}

/// Menyusun kode aset dari bagian-bagiannya. `nomor` harus positif.
pub fn format_kode_aset(kode_desa: &str, jenis: &str, nomor: i64) -> String {
    format!("{kode_desa}-{jenis}-{nomor:06}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kode_desa_empat_digit() {
        assert_eq!(kode_desa(Some(12), 3), "0012");
        assert_eq!(kode_desa(Some(12345), 3), "12345");
    }

    #[test]
    fn jaringan_tanpa_desa_memakai_huruf_j() {
        assert_eq!(kode_desa(None, 7), "J007");
        assert!(kode_desa(None, 7).starts_with('J'));
    }

    #[test]
    fn format_sesuai_dokumentasi() {
        assert_eq!(format_kode_aset("0012", "pipa", 45), "0012-pipa-000045");
        assert_eq!(format_kode_aset("0012", "bak_penahan_tekanan", 2), "0012-bak_penahan_tekanan-000002");
        assert_eq!(format_kode_aset("J007", "sr", 1_000_000), "J007-sr-1000000");
    }
}
