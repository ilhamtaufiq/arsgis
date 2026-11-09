//! Aturan penulisan aset (tambah, ubah): atribut wajib per jenis, rentang nilai, geometri, dan kondisi.
//! Murni, tanpa database. Pemeriksaan batas desa ada di pipa-store karena membutuhkan poligon.

use serde::{Deserialize, Deserializer};
use serde_json::{json, Value};

use crate::{FeatureType, Kondisi, Material};

/// Pelanggaran saat menulis aset. `Masukan` berarti isian tidak well-formed (400). `Aturan` berarti
/// aturan bisnis dilanggar, misalnya atribut wajib kosong atau di luar batas desa (422).
#[derive(Debug, Clone, PartialEq)]
pub enum AsetError {
    Masukan(String),
    Aturan(Vec<String>),
}

pub const TAHUN_MIN: i16 = 1900;
pub const TAHUN_MAKS: i16 = 2100;

/// Atribut yang bisa wajib diisi per jenis aset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Atribut {
    Nama,
    DiameterMm,
    Material,
    KapasitasM3,
    DebitLps,
}

impl Atribut {
    /// Nama kolom di database dan di API. Dipakai juga dalam pesan kesalahan.
    pub fn nama_kolom(self) -> &'static str {
        match self {
            Atribut::Nama => "nama",
            Atribut::DiameterMm => "diameter_mm",
            Atribut::Material => "material",
            Atribut::KapasitasM3 => "kapasitas_m3",
            Atribut::DebitLps => "debit_lps",
        }
    }
}

/// Atribut wajib per jenis. Keputusan ini didokumentasikan di sini supaya mudah diubah dalam satu tempat.
///
/// - `pipa`: diameter dan material.
/// - `reservoir`, `bak_penahan_tekanan`, `menara_air`: kapasitas_m3 (volume tampung).
/// - `intake`, `pompa`, `valve`: debit_lps (debit air yang dilayani atau dilewatkan).
/// - `hidran_umum`, `meter_induk`, `sr`, `bronkaptering`, `tapping`: nama.
pub fn atribut_wajib(t: FeatureType) -> &'static [Atribut] {
    match t {
        FeatureType::Pipa => &[Atribut::DiameterMm, Atribut::Material],
        FeatureType::Reservoir | FeatureType::BakPenahanTekanan | FeatureType::MenaraAir => {
            &[Atribut::KapasitasM3]
        }
        FeatureType::Intake | FeatureType::Pompa | FeatureType::Valve => &[Atribut::DebitLps],
        FeatureType::HidranUmum
        | FeatureType::MeterInduk
        | FeatureType::Sr
        | FeatureType::Bronkaptering
        | FeatureType::Tapping => {
            &[Atribut::Nama]
        }
    }
}

/// Bentuk geometri yang diterima per jenis. Pipa berupa garis, aset lain berupa titik.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JenisGeometri {
    Titik,
    Garis,
}

pub fn jenis_geometri(t: FeatureType) -> JenisGeometri {
    match t {
        FeatureType::Pipa => JenisGeometri::Garis,
        _ => JenisGeometri::Titik,
    }
}

/// Atribut aset yang diisi pengguna. Nama kolom sama dengan kolom `features`.
/// Teks yang kosong (setelah dipangkas) dianggap tidak diisi.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct AtributAset {
    pub nama: Option<String>,
    pub diameter_mm: Option<i32>,
    pub material: Option<String>,
    pub tahun_pasang: Option<i16>,
    pub kondisi: Option<String>,
    pub status: Option<String>,
    pub kapasitas_m3: Option<f64>,
    pub debit_lps: Option<f64>,
    pub elevasi_m: Option<f64>,
    pub tahun_bangun: Option<i16>,
    pub sumber_dana: Option<String>,
}

fn bersih(s: Option<String>) -> Option<String> {
    s.map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

impl AtributAset {
    /// Memangkas teks dan mengubah teks kosong menjadi tidak diisi.
    pub fn normalisasi(mut self) -> Self {
        self.nama = bersih(self.nama);
        self.material = bersih(self.material);
        self.kondisi = bersih(self.kondisi);
        self.status = bersih(self.status);
        self.sumber_dana = bersih(self.sumber_dana);
        self
    }

    fn terisi(&self, a: Atribut) -> bool {
        match a {
            Atribut::Nama => self.nama.is_some(),
            Atribut::DiameterMm => self.diameter_mm.is_some(),
            Atribut::Material => self.material.is_some(),
            Atribut::KapasitasM3 => self.kapasitas_m3.is_some(),
            Atribut::DebitLps => self.debit_lps.is_some(),
        }
    }
}

/// Isian ubah parsial. Bidang yang tidak ada di JSON tidak diubah. Bidang yang bernilai `null`
/// dikosongkan. Karena itu setiap bidang berbentuk `Option<Option<T>>`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AtributPatch {
    #[serde(default, deserialize_with = "isi_atau_kosong")]
    pub nama: Option<Option<String>>,
    #[serde(default, deserialize_with = "isi_atau_kosong")]
    pub diameter_mm: Option<Option<i32>>,
    #[serde(default, deserialize_with = "isi_atau_kosong")]
    pub material: Option<Option<String>>,
    #[serde(default, deserialize_with = "isi_atau_kosong")]
    pub tahun_pasang: Option<Option<i16>>,
    #[serde(default, deserialize_with = "isi_atau_kosong")]
    pub kondisi: Option<Option<String>>,
    #[serde(default, deserialize_with = "isi_atau_kosong")]
    pub status: Option<Option<String>>,
    #[serde(default, deserialize_with = "isi_atau_kosong")]
    pub kapasitas_m3: Option<Option<f64>>,
    #[serde(default, deserialize_with = "isi_atau_kosong")]
    pub debit_lps: Option<Option<f64>>,
    #[serde(default, deserialize_with = "isi_atau_kosong")]
    pub elevasi_m: Option<Option<f64>>,
    #[serde(default, deserialize_with = "isi_atau_kosong")]
    pub tahun_bangun: Option<Option<i16>>,
    #[serde(default, deserialize_with = "isi_atau_kosong")]
    pub sumber_dana: Option<Option<String>>,
}

/// Membedakan "tidak ada" (None di luar) dari "null" (Some(None)).
fn isi_atau_kosong<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

impl AtributPatch {
    /// Menerapkan bidang yang dikirim ke atribut yang sudah ada.
    pub fn terapkan(self, a: &mut AtributAset) {
        if let Some(v) = self.nama {
            a.nama = v;
        }
        if let Some(v) = self.diameter_mm {
            a.diameter_mm = v;
        }
        if let Some(v) = self.material {
            a.material = v;
        }
        if let Some(v) = self.tahun_pasang {
            a.tahun_pasang = v;
        }
        if let Some(v) = self.kondisi {
            a.kondisi = v;
        }
        if let Some(v) = self.status {
            a.status = v;
        }
        if let Some(v) = self.kapasitas_m3 {
            a.kapasitas_m3 = v;
        }
        if let Some(v) = self.debit_lps {
            a.debit_lps = v;
        }
        if let Some(v) = self.elevasi_m {
            a.elevasi_m = v;
        }
        if let Some(v) = self.tahun_bangun {
            a.tahun_bangun = v;
        }
        if let Some(v) = self.sumber_dana {
            a.sumber_dana = v;
        }
    }

    /// True bila patch menyentuh kondisi. Kondisi lama yang berbentuk teks bebas hanya diperiksa
    /// ketika ikut diubah, sama seperti pemicu di database.
    pub fn menyentuh_kondisi(&self) -> bool {
        self.kondisi.is_some()
    }
}

/// Memeriksa nilai dan atribut wajib untuk jenis `t`. `a` harus sudah dinormalisasi.
/// Nilai di luar rentang lebih dulu dilaporkan sebagai `Masukan`, lalu atribut wajib yang kosong sebagai `Aturan`.
pub fn periksa_atribut(t: FeatureType, a: &AtributAset) -> Result<(), AsetError> {
    let mut masukan = Vec::new();
    if let Some(m) = a.material.as_deref() {
        if Material::parse(m).is_err() {
            masukan.push(format!("material tidak dikenal: {m}"));
        }
    }
    if let Some(d) = a.diameter_mm {
        if d <= 0 {
            masukan.push("diameter_mm harus lebih dari 0".to_string());
        }
    }
    for (nilai, nama) in [(a.kapasitas_m3, "kapasitas_m3"), (a.debit_lps, "debit_lps")] {
        if let Some(v) = nilai {
            if !v.is_finite() || v < 0.0 {
                masukan.push(format!("{nama} harus bilangan tidak negatif"));
            }
        }
    }
    for (nilai, nama) in [(a.tahun_pasang, "tahun_pasang"), (a.tahun_bangun, "tahun_bangun")] {
        if let Some(v) = nilai {
            if !(TAHUN_MIN..=TAHUN_MAKS).contains(&v) {
                masukan.push(format!("{nama} di luar rentang {TAHUN_MIN}-{TAHUN_MAKS}"));
            }
        }
    }
    if let Some(e) = a.elevasi_m {
        if !e.is_finite() {
            masukan.push("elevasi_m harus bilangan".to_string());
        }
    }
    if !masukan.is_empty() {
        return Err(AsetError::Masukan(masukan.join("; ")));
    }

    let kurang: Vec<String> = atribut_wajib(t)
        .iter()
        .filter(|atr| !a.terisi(**atr))
        .map(|atr| format!("{} wajib untuk {}", atr.nama_kolom(), t.as_str()))
        .collect();
    if kurang.is_empty() {
        Ok(())
    } else {
        Err(AsetError::Aturan(kurang))
    }
}

/// Kondisi harus salah satu nilai baru. Teks lama di database tidak diperiksa di sini.
pub fn periksa_kondisi(k: &str) -> Result<(), AsetError> {
    Kondisi::parse(k).map(|_| ()).map_err(|_| {
        AsetError::Aturan(vec![format!(
            "kondisi tidak valid: {k}. Pilih baik, rusak_ringan, rusak_berat, atau tidak_berfungsi"
        )])
    })
}

/// Memeriksa bentuk GeoJSON (Point, LineString, atau MultiLineString) dan kesesuaiannya dengan jenis aset.
/// Mengembalikan geometri 2D yang sudah dinormalisasi, siap dikirim ke database.
pub fn normalisasi_geometri(t: FeatureType, v: &Value) -> Result<Value, AsetError> {
    let tipe = v
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| AsetError::Masukan("geometry harus objek GeoJSON dengan bidang type".into()))?;
    let koordinat = v
        .get("coordinates")
        .ok_or_else(|| AsetError::Masukan("geometry harus memiliki bidang coordinates".into()))?;

    let normal = match tipe {
        "Point" => json!(posisi(koordinat)?),
        "LineString" => json!(garis(koordinat)?),
        "MultiLineString" => {
            let daftar = koordinat
                .as_array()
                .filter(|l| !l.is_empty())
                .ok_or_else(|| AsetError::Masukan("MultiLineString harus berisi minimal satu garis".into()))?;
            let garis_normal: Vec<Vec<[f64; 2]>> = daftar.iter().map(garis).collect::<Result<_, _>>()?;
            json!(garis_normal)
        }
        other => {
            return Err(AsetError::Masukan(format!(
                "tipe geometri tidak didukung: {other}. Pilih Point, LineString, atau MultiLineString"
            )))
        }
    };

    let cocok = match (jenis_geometri(t), tipe) {
        (JenisGeometri::Titik, "Point") => true,
        (JenisGeometri::Garis, "LineString" | "MultiLineString") => true,
        _ => false,
    };
    if !cocok {
        let harus = match jenis_geometri(t) {
            JenisGeometri::Titik => "Point",
            JenisGeometri::Garis => "LineString atau MultiLineString",
        };
        return Err(AsetError::Masukan(format!("geometri {} harus {harus}, bukan {tipe}", t.as_str())));
    }
    Ok(json!({ "type": tipe, "coordinates": normal }))
}

/// Satu posisi [lon, lat]. Ketinggian (koordinat ketiga) dibuang.
fn posisi(v: &Value) -> Result<[f64; 2], AsetError> {
    let salah = || AsetError::Masukan("koordinat harus pasangan [bujur, lintang] dalam derajat".into());
    let arr = v.as_array().filter(|a| a.len() >= 2).ok_or_else(salah)?;
    let lon = arr[0].as_f64().filter(|x| x.is_finite()).ok_or_else(salah)?;
    let lat = arr[1].as_f64().filter(|x| x.is_finite()).ok_or_else(salah)?;
    if !(-180.0..=180.0).contains(&lon) || !(-90.0..=90.0).contains(&lat) {
        return Err(AsetError::Masukan(format!("koordinat di luar rentang derajat: [{lon}, {lat}]")));
    }
    Ok([lon, lat])
}

fn garis(v: &Value) -> Result<Vec<[f64; 2]>, AsetError> {
    let titik: Vec<[f64; 2]> = v
        .as_array()
        .map(|a| a.iter().map(posisi).collect::<Result<_, _>>())
        .transpose()?
        .unwrap_or_default();
    if titik.len() < 2 {
        return Err(AsetError::Masukan("garis harus memiliki minimal dua titik".into()));
    }
    Ok(titik)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atribut_pipa() -> AtributAset {
        AtributAset { diameter_mm: Some(110), material: Some("PVC".into()), ..Default::default() }
    }

    #[test]
    fn pipa_wajib_diameter_dan_material() {
        let err = periksa_atribut(FeatureType::Pipa, &AtributAset::default()).unwrap_err();
        let AsetError::Aturan(daftar) = err else { panic!("harus Aturan") };
        assert_eq!(daftar.len(), 2);
        assert!(daftar[0].contains("diameter_mm"));
        assert!(daftar[1].contains("material"));
        assert!(periksa_atribut(FeatureType::Pipa, &atribut_pipa()).is_ok());
    }

    #[test]
    fn kapasitas_atau_debit_sesuai_jenis() {
        let kapasitas = AtributAset { kapasitas_m3: Some(50.0), ..Default::default() };
        let debit = AtributAset { debit_lps: Some(5.0), ..Default::default() };
        assert!(periksa_atribut(FeatureType::Reservoir, &kapasitas).is_ok());
        assert!(periksa_atribut(FeatureType::MenaraAir, &kapasitas).is_ok());
        assert!(periksa_atribut(FeatureType::BakPenahanTekanan, &kapasitas).is_ok());
        assert!(periksa_atribut(FeatureType::Intake, &debit).is_ok());
        assert!(periksa_atribut(FeatureType::Pompa, &debit).is_ok());
        assert!(periksa_atribut(FeatureType::Valve, &debit).is_ok());

        // Tertukar: reservoir butuh kapasitas, bukan debit.
        assert!(periksa_atribut(FeatureType::Reservoir, &debit).is_err());
        assert!(periksa_atribut(FeatureType::Pompa, &kapasitas).is_err());
        assert!(periksa_atribut(FeatureType::Valve, &kapasitas).is_err());
    }

    #[test]
    fn nama_wajib_untuk_titik_tertentu() {
        for t in [
            FeatureType::HidranUmum,
            FeatureType::MeterInduk,
            FeatureType::Sr,
            FeatureType::Bronkaptering,
            FeatureType::Tapping,
        ] {
            assert!(periksa_atribut(t, &AtributAset::default()).is_err(), "{t:?}");
            let ada = AtributAset { nama: Some("Hidran 1".into()), ..Default::default() };
            assert!(periksa_atribut(t, &ada).is_ok(), "{t:?}");
        }
    }

    #[test]
    fn nama_kosong_setelah_normalisasi_dianggap_tidak_diisi() {
        let a = AtributAset { nama: Some("   ".into()), ..Default::default() }.normalisasi();
        assert_eq!(a.nama, None);
        let err = periksa_atribut(FeatureType::Sr, &a).unwrap_err();
        assert!(matches!(err, AsetError::Aturan(_)));
    }

    #[test]
    fn rentang_nilai_ditolak_sebagai_masukan() {
        let mut a = atribut_pipa();
        a.diameter_mm = Some(0);
        assert!(matches!(periksa_atribut(FeatureType::Pipa, &a), Err(AsetError::Masukan(_))));

        let mut a = atribut_pipa();
        a.material = Some("besi".into());
        assert!(matches!(periksa_atribut(FeatureType::Pipa, &a), Err(AsetError::Masukan(_))));

        let mut a = atribut_pipa();
        a.tahun_pasang = Some(1800);
        assert!(matches!(periksa_atribut(FeatureType::Pipa, &a), Err(AsetError::Masukan(_))));

        let a = AtributAset { debit_lps: Some(-1.0), ..Default::default() };
        assert!(matches!(periksa_atribut(FeatureType::Pompa, &a), Err(AsetError::Masukan(_))));
    }

    #[test]
    fn material_baru_diterima() {
        let mut a = atribut_pipa();
        a.material = Some("Steel".into());
        assert!(periksa_atribut(FeatureType::Pipa, &a).is_ok());
    }

    #[test]
    fn kondisi_hanya_nilai_baru() {
        for k in ["baik", "rusak_ringan", "rusak_berat", "tidak_berfungsi"] {
            assert!(periksa_kondisi(k).is_ok());
        }
        assert!(matches!(periksa_kondisi("perlu cek"), Err(AsetError::Aturan(_))));
        assert!(matches!(periksa_kondisi("Baik"), Err(AsetError::Aturan(_))));
    }

    #[test]
    fn patch_membedakan_tidak_ada_dan_null() {
        let patch: AtributPatch = serde_json::from_value(json!({ "nama": null, "debit_lps": 7.5 })).unwrap();
        assert_eq!(patch.nama, Some(None));
        assert_eq!(patch.debit_lps, Some(Some(7.5)));
        assert_eq!(patch.diameter_mm, None);

        let mut a = AtributAset { nama: Some("Lama".into()), diameter_mm: Some(90), ..Default::default() };
        patch.terapkan(&mut a);
        assert_eq!(a.nama, None, "null mengosongkan");
        assert_eq!(a.diameter_mm, Some(90), "bidang yang tidak dikirim tetap");
        assert_eq!(a.debit_lps, Some(7.5));
    }

    #[test]
    fn patch_menolak_bidang_tidak_dikenal() {
        assert!(serde_json::from_value::<AtributPatch>(json!({ "sumur": 1 })).is_err());
    }

    #[test]
    fn point_diterima_untuk_aset_titik() {
        let v = json!({ "type": "Point", "coordinates": [107.15, -6.82, 120.0] });
        let n = normalisasi_geometri(FeatureType::Sr, &v).unwrap();
        assert_eq!(n, json!({ "type": "Point", "coordinates": [107.15, -6.82] }));
    }

    #[test]
    fn garis_diterima_untuk_pipa() {
        let ls = json!({ "type": "LineString", "coordinates": [[107.1, -6.8], [107.2, -6.8]] });
        assert!(normalisasi_geometri(FeatureType::Pipa, &ls).is_ok());
        let mls = json!({ "type": "MultiLineString", "coordinates": [[[107.1, -6.8], [107.2, -6.8]]] });
        assert!(normalisasi_geometri(FeatureType::Pipa, &mls).is_ok());
    }

    #[test]
    fn geometri_tidak_cocok_dengan_jenis() {
        let titik = json!({ "type": "Point", "coordinates": [107.1, -6.8] });
        let garis = json!({ "type": "LineString", "coordinates": [[107.1, -6.8], [107.2, -6.8]] });
        assert!(matches!(normalisasi_geometri(FeatureType::Pipa, &titik), Err(AsetError::Masukan(_))));
        assert!(matches!(normalisasi_geometri(FeatureType::Valve, &garis), Err(AsetError::Masukan(_))));
    }

    #[test]
    fn geometri_rusak_ditolak() {
        let satu_titik = json!({ "type": "LineString", "coordinates": [[107.1, -6.8]] });
        assert!(normalisasi_geometri(FeatureType::Pipa, &satu_titik).is_err());
        let lintang = json!({ "type": "Point", "coordinates": [107.1, -95.0] });
        assert!(normalisasi_geometri(FeatureType::Sr, &lintang).is_err());
        let poligon = json!({ "type": "Polygon", "coordinates": [] });
        assert!(normalisasi_geometri(FeatureType::Sr, &poligon).is_err());
        assert!(normalisasi_geometri(FeatureType::Sr, &json!({ "coordinates": [1, 2] })).is_err());
        let kosong = json!({ "type": "MultiLineString", "coordinates": [] });
        assert!(normalisasi_geometri(FeatureType::Pipa, &kosong).is_err());
    }
}
