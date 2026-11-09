//! Model domain WebGIS perpipaan: jenis fitur, bbox, dan validasi input.
//! Tidak bergantung pada HTTP atau database.

use serde::{Deserialize, Serialize};
use thiserror::Error;

mod aset;
pub use aset::{
    atribut_wajib, jenis_geometri, normalisasi_geometri, periksa_atribut, periksa_kondisi, AsetError, AtributAset,
    AtributPatch, Atribut, JenisGeometri, TAHUN_MAKS, TAHUN_MIN,
};

mod cakupan;
pub use cakupan::{cakupan_json, hitung_cakupan, validasi_dusun, Cakupan, DusunIsian, DusunMasukan};

mod kode;
pub use kode::{format_kode_aset, kode_desa};

mod pelanggan;
pub use pelanggan::{validasi_pelanggan, GolonganTarif, PelangganIsian, PelangganMasukan, StatusPelanggan};
// Modul O&M (Tahap C). Dipisah dari aset agar tidak bercampur dengan aturan model aset.
pub mod gangguan;
pub mod impor;
pub mod om;
pub mod perawatan;
pub mod rehab;
pub mod tanggal;

#[derive(Debug, Error, PartialEq)]
pub enum CoreError {
    #[error("jenis fitur tidak dikenal: {0}")]
    UnknownFeatureType(String),
    #[error("bbox tidak valid: {0}")]
    InvalidBbox(String),
    #[error("peran tidak dikenal: {0}")]
    UnknownRole(String),
    #[error("material tidak dikenal: {0}")]
    UnknownMaterial(String),
    #[error("kondisi tidak dikenal: {0}")]
    UnknownKondisi(String),
}

/// Jenis aset jaringan. Nilainya disimpan sebagai teks di kolom `features.feature_type`.
/// Nama teks harus sama dengan `as_str` dan dengan CHECK di migrasi 0010.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureType {
    Pipa,
    Valve,
    Reservoir,
    Sr,
    Bronkaptering,
    /// Bak penahan tekanan (BPT).
    BakPenahanTekanan,
    MenaraAir,
    HidranUmum,
    /// Sumur atau mata air.
    Intake,
    Pompa,
    MeterInduk,
    /// Titik sambungan pengambilan air dari pipa (tapping). Bukan data pelanggan; pelanggan tetap lewat SR.
    Tapping,
}

impl FeatureType {
    pub const ALL: [FeatureType; 12] = [
        FeatureType::Pipa,
        FeatureType::Valve,
        FeatureType::Reservoir,
        FeatureType::Sr,
        FeatureType::Bronkaptering,
        FeatureType::BakPenahanTekanan,
        FeatureType::MenaraAir,
        FeatureType::HidranUmum,
        FeatureType::Intake,
        FeatureType::Pompa,
        FeatureType::MeterInduk,
        FeatureType::Tapping,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            FeatureType::Pipa => "pipa",
            FeatureType::Valve => "valve",
            FeatureType::Reservoir => "reservoir",
            FeatureType::Sr => "sr",
            FeatureType::Bronkaptering => "bronkaptering",
            FeatureType::BakPenahanTekanan => "bak_penahan_tekanan",
            FeatureType::MenaraAir => "menara_air",
            FeatureType::HidranUmum => "hidran_umum",
            FeatureType::Intake => "intake",
            FeatureType::Pompa => "pompa",
            FeatureType::MeterInduk => "meter_induk",
            FeatureType::Tapping => "tapping",
        }
    }

    pub fn parse(s: &str) -> Result<Self, CoreError> {
        Self::ALL
            .into_iter()
            .find(|t| t.as_str() == s)
            .ok_or_else(|| CoreError::UnknownFeatureType(s.to_string()))
    }

    /// Jenis yang termasuk data pelanggan. Hanya boleh dibaca oleh peran internal.
    pub fn is_customer_data(self) -> bool {
        matches!(self, FeatureType::Sr)
    }
}

/// Material pipa dan aset lain. Nilai sama dengan CHECK `features.material` di migrasi 0010.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Material {
    Pvc,
    Hdpe,
    Gip,
    Dci,
    Steel,
    Acp,
    Lainnya,
}

impl Material {
    pub const ALL: [Material; 7] = [
        Material::Pvc,
        Material::Hdpe,
        Material::Gip,
        Material::Dci,
        Material::Steel,
        Material::Acp,
        Material::Lainnya,
    ];

    /// Ejaan resmi yang disimpan di database. Huruf besar-kecil dibedakan.
    pub fn as_str(self) -> &'static str {
        match self {
            Material::Pvc => "PVC",
            Material::Hdpe => "HDPE",
            Material::Gip => "GIP",
            Material::Dci => "DCI",
            Material::Steel => "Steel",
            Material::Acp => "ACP",
            Material::Lainnya => "Lainnya",
        }
    }

    pub fn parse(s: &str) -> Result<Self, CoreError> {
        Self::ALL
            .into_iter()
            .find(|m| m.as_str() == s)
            .ok_or_else(|| CoreError::UnknownMaterial(s.to_string()))
    }
}

/// Kondisi fisik aset. Data lama boleh berisi teks bebas (mis. `perlu cek`). Validasi ini hanya dipakai
/// untuk nilai yang ditulis setelah migrasi 0010, sesuai pemicu di database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kondisi {
    Baik,
    RusakRingan,
    RusakBerat,
    TidakBerfungsi,
}

impl Kondisi {
    pub const ALL: [Kondisi; 4] = [Kondisi::Baik, Kondisi::RusakRingan, Kondisi::RusakBerat, Kondisi::TidakBerfungsi];

    pub fn as_str(self) -> &'static str {
        match self {
            Kondisi::Baik => "baik",
            Kondisi::RusakRingan => "rusak_ringan",
            Kondisi::RusakBerat => "rusak_berat",
            Kondisi::TidakBerfungsi => "tidak_berfungsi",
        }
    }

    pub fn parse(s: &str) -> Result<Self, CoreError> {
        Self::ALL
            .into_iter()
            .find(|k| k.as_str() == s)
            .ok_or_else(|| CoreError::UnknownKondisi(s.to_string()))
    }
}

/// Peran pengguna. Nilai teks sama dengan CHECK `users.role` (migrasi 0013) dan klaim `role` di token.
///
/// Pengunjung tanpa login (publik) tidak punya peran: `Option<Role>` = `None`.
/// Urutan variant tidak dipakai untuk memeriksa izin. Setiap izin ditulis eksplisit di `Role::punya`,
/// karena peran seperti bendahara tidak lebih tinggi dari editor, hanya berbeda aksesnya.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Pengguna login umum, hanya membaca data yang tidak pribadi (sama dengan `viewer` lama).
    Viewer,
    /// Kepala desa: hanya membaca, dan hanya ringkasan cakupan. Tidak melihat data pelanggan perorangan.
    KepalaDesa,
    /// Bendahara: membaca data pelanggan (nama, nomor meter, status). Tidak mengubah aset atau pelanggan.
    Bendahara,
    /// Operator atau teknisi (nilai teks tetap `editor`): mengelola aset, impor, Pokmas, dan pelanggan.
    Editor,
    Admin,
}

/// Izin per tindakan. Setiap endpoint memeriksa salah satu izin ini, bukan peran secara langsung.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Izin {
    /// Membaca data pelanggan (titik SR, daftar dan detail pelanggan, riwayat pelanggan).
    BacaDataPelanggan,
    /// Mengubah data pelanggan dan menghubungkan SR ke dusun.
    KelolaPelanggan,
    /// Mengubah aset, impor, koreksi Pokmas, dan jumlah rumah per dusun.
    KelolaData,
    /// Membaca nama pengurus Pokmas dan riwayat koreksinya.
    BacaNamaPokmas,
    /// Membaca ringkasan cakupan per dusun dan desa (tanpa data pelanggan perorangan).
    BacaCakupan,
    /// Mengelola pengguna dan katalog layer.
    KelolaPengguna,
}

impl Role {
    pub const ALL: [Role; 5] = [Role::Viewer, Role::KepalaDesa, Role::Bendahara, Role::Editor, Role::Admin];

    pub fn as_str(self) -> &'static str {
        match self {
            Role::Viewer => "viewer",
            Role::KepalaDesa => "kepala_desa",
            Role::Bendahara => "bendahara",
            Role::Editor => "editor",
            Role::Admin => "admin",
        }
    }

    pub fn parse(s: &str) -> Result<Self, CoreError> {
        Self::ALL
            .into_iter()
            .find(|r| r.as_str() == s)
            .ok_or_else(|| CoreError::UnknownRole(s.to_string()))
    }

    /// Matriks izin. Satu-satunya tempat yang menentukan siapa boleh apa.
    pub fn punya(self, izin: Izin) -> bool {
        use Izin::*;
        match (self, izin) {
            (Role::Admin, _) => true,
            (Role::Editor, KelolaPelanggan | KelolaData | BacaDataPelanggan | BacaNamaPokmas | BacaCakupan) => true,
            (Role::Bendahara, BacaDataPelanggan | BacaCakupan) => true,
            (Role::KepalaDesa | Role::Viewer, BacaCakupan) => true,
            _ => false,
        }
    }

    /// Menulis aset (tambah, ubah, hapus) hanya untuk operator dan admin.
    pub fn can_write(self) -> bool {
        self.punya(Izin::KelolaData)
    }

    /// Data pelanggan (sr) dibaca oleh peran yang punya `BacaDataPelanggan`.
    pub fn can_read(self, t: FeatureType) -> bool {
        !t.is_customer_data() || self.punya(Izin::BacaDataPelanggan)
    }
}

/// Peran `None` berarti pengunjung tanpa login.
pub fn can_read_types(role: Option<Role>, types: &[FeatureType]) -> bool {
    types.iter().all(|t| match role {
        Some(r) => r.can_read(*t),
        None => !t.is_customer_data(),
    })
}

/// Nilai `min_role` di katalog layer yang boleh dilihat peran ini. `None` berarti pengunjung tanpa login.
/// Layer dengan `min_role` di luar daftar ini tidak boleh dikirim ke pemanggil.
/// Bendahara dan kepala desa hanya melihat layer publik dan viewer, karena layer internal memuat data operasional.
pub fn catalog_roles(role: Option<Role>) -> &'static [&'static str] {
    match role {
        None => &["anon"],
        Some(Role::Viewer | Role::KepalaDesa | Role::Bendahara) => &["anon", "viewer"],
        Some(Role::Editor) => &["anon", "viewer", "editor"],
        Some(Role::Admin) => &["anon", "viewer", "editor", "admin"],
    }
}

/// Kotak batas dalam derajat WGS84, urutan `w,s,e,n` (kiri, bawah, kanan, atas).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bbox {
    pub west: f64,
    pub south: f64,
    pub east: f64,
    pub north: f64,
}

impl Bbox {
    /// Batas luas bbox dalam derajat. Mencegah permintaan seluruh dunia yang membebani server.
    pub const MAX_SPAN_DEG: f64 = 2.0;

    pub fn parse(s: &str) -> Result<Self, CoreError> {
        let bad = || CoreError::InvalidBbox(s.to_string());
        let parts: Vec<f64> = s
            .split(',')
            .map(|p| p.trim().parse::<f64>().map_err(|_| bad()))
            .collect::<Result<_, _>>()?;
        let [west, south, east, north] = parts[..] else {
            return Err(bad());
        };
        let finite = [west, south, east, north].iter().all(|v| v.is_finite());
        let in_range = (-180.0..=180.0).contains(&west)
            && (-180.0..=180.0).contains(&east)
            && (-90.0..=90.0).contains(&south)
            && (-90.0..=90.0).contains(&north);
        if !finite || !in_range || west >= east || south >= north {
            return Err(bad());
        }
        if east - west > Self::MAX_SPAN_DEG || north - south > Self::MAX_SPAN_DEG {
            return Err(CoreError::InvalidBbox(format!(
                "{s} (lebar atau tinggi melebihi {} derajat)",
                Self::MAX_SPAN_DEG
            )));
        }
        Ok(Self { west, south, east, north })
    }
}

/// Parse daftar `types=pipa,valve`. Kosong berarti semua jenis.
pub fn parse_types(s: Option<&str>) -> Result<Option<Vec<FeatureType>>, CoreError> {
    match s.map(str::trim).filter(|v| !v.is_empty()) {
        None => Ok(None),
        Some(list) => list
            .split(',')
            .map(|t| FeatureType::parse(t.trim()))
            .collect::<Result<Vec<_>, _>>()
            .map(Some),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_type_roundtrip() {
        for t in FeatureType::ALL {
            assert_eq!(FeatureType::parse(t.as_str()).unwrap(), t);
        }
        assert!(FeatureType::parse("sumur").is_err());
    }

    #[test]
    fn new_feature_types_roundtrip_and_serde_match_as_str() {
        for t in [
            FeatureType::BakPenahanTekanan,
            FeatureType::MenaraAir,
            FeatureType::HidranUmum,
            FeatureType::Intake,
            FeatureType::Pompa,
            FeatureType::MeterInduk,
            FeatureType::Tapping,
        ] {
            assert_eq!(FeatureType::parse(t.as_str()).unwrap(), t);
            // Serde harus memakai nama teks yang sama dengan database dan API.
            let json = serde_json::to_string(&t).unwrap();
            assert_eq!(json, format!("\"{}\"", t.as_str()));
        }
        assert_eq!(FeatureType::parse("bak_penahan_tekanan").unwrap(), FeatureType::BakPenahanTekanan);
        assert!(FeatureType::parse("BPT").is_err());
        assert!(FeatureType::parse("sumur").is_err());
        assert_eq!(FeatureType::ALL.len(), 12);
    }

    #[test]
    fn tapping_is_its_own_type_not_customer_data() {
        assert_eq!(FeatureType::parse("tapping").unwrap(), FeatureType::Tapping);
        assert_eq!(FeatureType::Tapping.as_str(), "tapping");
        assert!(!FeatureType::Tapping.is_customer_data());
        assert_ne!(FeatureType::Tapping, FeatureType::Sr);
        // Pengunjung tanpa login boleh membaca tapping, sama seperti jenis jaringan lain.
        assert!(can_read_types(None, &[FeatureType::Tapping]));
        assert!(Role::Viewer.can_read(FeatureType::Tapping));
        assert!(!Role::Viewer.can_read(FeatureType::Sr));
    }

    #[test]
    fn only_sr_is_customer_data() {
        for t in FeatureType::ALL {
            assert_eq!(t.is_customer_data(), t == FeatureType::Sr, "{t:?}");
        }
    }

    #[test]
    fn material_validation() {
        for m in Material::ALL {
            assert_eq!(Material::parse(m.as_str()).unwrap(), m);
        }
        assert_eq!(Material::parse("Steel").unwrap(), Material::Steel);
        assert_eq!(Material::parse("ACP").unwrap(), Material::Acp);
        // Huruf besar-kecil dibedakan, sama dengan CHECK di database.
        assert!(Material::parse("steel").is_err());
        assert!(Material::parse("besi").is_err());
        assert!(Material::parse("").is_err());
    }

    #[test]
    fn kondisi_validation() {
        for k in Kondisi::ALL {
            assert_eq!(Kondisi::parse(k.as_str()).unwrap(), k);
        }
        assert_eq!(Kondisi::parse("rusak_berat").unwrap(), Kondisi::RusakBerat);
        // Teks bebas lama tidak lolos untuk penulisan baru.
        assert!(Kondisi::parse("perlu cek").is_err());
        assert!(Kondisi::parse("Baik").is_err());
    }

    #[test]
    fn catalog_roles_never_expose_higher_roles() {
        assert_eq!(catalog_roles(None), ["anon"]);
        assert_eq!(catalog_roles(Some(Role::Viewer)), ["anon", "viewer"]);
        assert!(!catalog_roles(Some(Role::Viewer)).contains(&"editor"));
        assert!(!catalog_roles(Some(Role::Editor)).contains(&"admin"));
        assert_eq!(catalog_roles(Some(Role::Admin)).len(), 4);
    }

    #[test]
    fn bbox_valid() {
        let b = Bbox::parse("107.10,-6.85,107.20,-6.78").unwrap();
        assert_eq!(b.west, 107.10);
        assert_eq!(b.north, -6.78);
    }

    #[test]
    fn bbox_rejects_bad_input() {
        assert!(Bbox::parse("107.2,-6.8,107.1,-6.7").is_err()); // west >= east
        assert!(Bbox::parse("107.1,-6.8,107.2").is_err()); // kurang komponen
        assert!(Bbox::parse("abc,1,2,3").is_err());
        assert!(Bbox::parse("0,0,200,10").is_err()); // di luar rentang
        assert!(Bbox::parse("100,-6,110,-4").is_err()); // lebih dari 2 derajat
    }

    #[test]
    fn role_gates_customer_data() {
        assert!(Role::Editor.can_read(FeatureType::Sr));
        assert!(Role::Admin.can_read(FeatureType::Sr));
        assert!(!Role::Viewer.can_read(FeatureType::Sr));
        assert!(Role::Viewer.can_read(FeatureType::Pipa));
        assert!(can_read_types(None, &[FeatureType::Pipa, FeatureType::Valve]));
        assert!(!can_read_types(None, &[FeatureType::Pipa, FeatureType::Sr]));
        assert!(can_read_types(Some(Role::Editor), &[FeatureType::Sr]));
        assert_eq!(Role::parse("admin").unwrap(), Role::Admin);
        assert!(Role::parse("root").is_err());
    }

    #[test]
    fn role_can_write_only_editor_and_admin() {
        assert!(!Role::Viewer.can_write());
        assert!(!Role::KepalaDesa.can_write());
        assert!(!Role::Bendahara.can_write());
        assert!(Role::Editor.can_write());
        assert!(Role::Admin.can_write());
    }

    #[test]
    fn matriks_izin_per_peran() {
        use Izin::*;
        // Admin punya semua izin.
        for izin in [BacaDataPelanggan, KelolaPelanggan, KelolaData, BacaNamaPokmas, BacaCakupan, KelolaPengguna] {
            assert!(Role::Admin.punya(izin), "{izin:?}");
        }
        // Operator mengelola data, tetapi tidak mengatur pengguna.
        for izin in [BacaDataPelanggan, KelolaPelanggan, KelolaData, BacaNamaPokmas, BacaCakupan] {
            assert!(Role::Editor.punya(izin), "{izin:?}");
        }
        assert!(!Role::Editor.punya(KelolaPengguna));
        // Bendahara membaca pelanggan dan cakupan, tidak mengubah apa pun.
        assert!(Role::Bendahara.punya(BacaDataPelanggan));
        assert!(Role::Bendahara.punya(BacaCakupan));
        assert!(!Role::Bendahara.punya(KelolaPelanggan));
        assert!(!Role::Bendahara.punya(KelolaData));
        assert!(!Role::Bendahara.punya(BacaNamaPokmas));
        // Kepala desa dan viewer hanya ringkasan cakupan.
        for r in [Role::KepalaDesa, Role::Viewer] {
            assert!(r.punya(BacaCakupan), "{r:?}");
            assert!(!r.punya(BacaDataPelanggan), "{r:?}");
            assert!(!r.punya(KelolaData), "{r:?}");
            assert!(!r.punya(BacaNamaPokmas), "{r:?}");
        }
    }

    #[test]
    fn peran_baru_tidak_membaca_data_pelanggan_selain_bendahara_dan_internal() {
        assert!(Role::Bendahara.can_read(FeatureType::Sr));
        assert!(!Role::KepalaDesa.can_read(FeatureType::Sr));
        assert!(!can_read_types(Some(Role::KepalaDesa), &[FeatureType::Sr]));
        assert!(can_read_types(Some(Role::KepalaDesa), &[FeatureType::Pipa]));
        assert!(Role::KepalaDesa.can_read(FeatureType::Pipa));
    }

    #[test]
    fn peran_teks_roundtrip_termasuk_peran_baru() {
        for r in Role::ALL {
            assert_eq!(Role::parse(r.as_str()).unwrap(), r);
            let json = serde_json::to_string(&r).unwrap();
            assert_eq!(json, format!("\"{}\"", r.as_str()));
        }
        assert_eq!(Role::parse("kepala_desa").unwrap(), Role::KepalaDesa);
        assert_eq!(Role::parse("editor").unwrap(), Role::Editor);
        assert!(Role::parse("Kepala Desa").is_err());
    }

    #[test]
    fn katalog_peran_baru_tidak_melihat_layer_internal() {
        assert_eq!(catalog_roles(Some(Role::Bendahara)), ["anon", "viewer"]);
        assert_eq!(catalog_roles(Some(Role::KepalaDesa)), ["anon", "viewer"]);
    }

    #[test]
    fn types_filter() {
        assert_eq!(parse_types(None).unwrap(), None);
        assert_eq!(parse_types(Some("")).unwrap(), None);
        assert_eq!(
            parse_types(Some("pipa, valve")).unwrap(),
            Some(vec![FeatureType::Pipa, FeatureType::Valve])
        );
        assert!(parse_types(Some("pipa,xyz")).is_err());
    }
}
