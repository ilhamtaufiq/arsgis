//! Isian admin untuk katalog layer, beserta validasinya. Murni (tanpa database) supaya mudah diuji.
//! Aturan ini melengkapi CHECK di migrasi 0008; di sini pesannya bisa dijelaskan ke admin.

use serde::Deserialize;
use serde_json::Value;

pub const KINDS: [&str; 6] = ["internal", "feature", "arcgis", "vector", "wms", "pmtiles"];
pub const ROLES: [&str; 4] = ["anon", "viewer", "editor", "admin"];

const MAX_JSON_BYTES: usize = 20_000;

fn default_role() -> String {
    "anon".into()
}

fn default_true() -> bool {
    true
}

/// Badan POST /api/v1/admin/catalog. Bidang yang tidak dikenal ditolak supaya salah ketik tidak diam-diam hilang.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogInput {
    pub slug: String,
    pub nama: String,
    pub kategori: String,
    pub sumber: String,
    pub kind: String,
    pub url: Option<String>,
    pub params: Option<Value>,
    pub legend: Option<Value>,
    pub style: Option<Value>,
    pub attribution: Option<String>,
    pub min_zoom: Option<i16>,
    #[serde(default = "default_role")]
    pub min_role: String,
    #[serde(default)]
    pub chip: bool,
    pub chip_urut: Option<i16>,
    pub baru_sampai: Option<String>,
    #[serde(default = "default_true")]
    pub aktif: bool,
}

/// Badan PATCH /api/v1/admin/catalog/{id}. `slug` dan `kind` sengaja tidak bisa diubah: klien memakainya sebagai kunci.
/// Bidang yang tidak dikirim tidak diubah; nilai tidak bisa dikosongkan lewat PATCH.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogPatch {
    pub nama: Option<String>,
    pub kategori: Option<String>,
    pub sumber: Option<String>,
    pub url: Option<String>,
    pub attribution: Option<String>,
    pub min_role: Option<String>,
    pub aktif: Option<bool>,
    pub chip: Option<bool>,
    pub chip_urut: Option<i16>,
    pub baru_sampai: Option<String>,
}

fn text(label: &str, v: &str, max: usize) -> Result<String, String> {
    let t = v.trim();
    if t.is_empty() || t.chars().count() > max {
        return Err(format!("{label} 1–{max} karakter"));
    }
    Ok(t.to_string())
}

fn optional_text(label: &str, v: Option<&String>, max: usize) -> Result<Option<String>, String> {
    match v.map(|s| s.trim()) {
        None | Some("") => Ok(None),
        Some(t) if t.chars().count() > max => Err(format!("{label} maksimal {max} karakter")),
        Some(t) => Ok(Some(t.to_string())),
    }
}

/// Hanya https. Layer http ditolak karena diblokir peramban di halaman https (mixed content).
fn url(v: &str) -> Result<String, String> {
    let t = v.trim();
    if t.len() > 2000 || !t.starts_with("https://") || t.len() == "https://".len() {
        return Err("url harus diawali https:// dan maksimal 2000 karakter".into());
    }
    if t.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err("url tidak boleh berisi spasi atau karakter kontrol".into());
    }
    Ok(t.to_string())
}

fn role(v: &str) -> Result<String, String> {
    if ROLES.contains(&v) {
        Ok(v.to_string())
    } else {
        Err(format!("min_role harus salah satu dari {}", ROLES.join(", ")))
    }
}

fn json_size(label: &str, v: &Value) -> Result<(), String> {
    if v.to_string().len() > MAX_JSON_BYTES {
        return Err(format!("{label} terlalu besar (maksimal {MAX_JSON_BYTES} byte)"));
    }
    Ok(())
}

/// Tanggal ISO `YYYY-MM-DD` yang benar menurut kalender (termasuk tahun kabisat).
pub fn valid_iso_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    if !s.bytes().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit()) {
        return false;
    }
    let (Ok(y), Ok(m), Ok(d)) = (s[0..4].parse::<u32>(), s[5..7].parse::<u32>(), s[8..10].parse::<u32>()) else {
        return false;
    };
    if y < 1900 {
        return false;
    }
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let days = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&d)
}

fn date(v: &str) -> Result<String, String> {
    if valid_iso_date(v.trim()) {
        Ok(v.trim().to_string())
    } else {
        Err("baru_sampai harus tanggal YYYY-MM-DD yang valid".into())
    }
}

fn chip_order(v: i16) -> Result<i16, String> {
    if (0..=1000).contains(&v) {
        Ok(v)
    } else {
        Err("chip_urut harus 0–1000".into())
    }
}

impl CatalogInput {
    /// Memangkas spasi dan memeriksa semua aturan. Mengembalikan salinan yang siap disimpan, atau pesan untuk admin.
    pub fn validated(&self) -> Result<CatalogInput, String> {
        let slug = self.slug.trim();
        if slug.is_empty()
            || slug.len() > 60
            || !slug.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
        {
            return Err("slug 1–60 karakter: huruf kecil, angka, atau tanda hubung".into());
        }
        if !KINDS.contains(&self.kind.as_str()) {
            return Err(format!("kind harus salah satu dari {}", KINDS.join(", ")));
        }
        let url_value = match self.url.as_deref().map(str::trim).filter(|u| !u.is_empty()) {
            Some(u) => Some(url(u)?),
            None => None,
        };
        if self.kind != "internal" && url_value.is_none() {
            return Err("layer selain internal wajib punya url".into());
        }
        let params = self.params.clone().unwrap_or_else(|| Value::Object(Default::default()));
        if !params.is_object() {
            return Err("params harus berupa objek JSON".into());
        }
        json_size("params", &params)?;
        if self.kind == "internal" && !params.get("internalKey").is_some_and(|k| k.as_str().is_some_and(|s| !s.is_empty())) {
            return Err("layer internal wajib punya params.internalKey".into());
        }
        for (label, v) in [("legend", &self.legend), ("style", &self.style)] {
            if let Some(v) = v {
                json_size(label, v)?;
            }
        }
        if let Some(z) = self.min_zoom {
            if !(0..=22).contains(&z) {
                return Err("min_zoom harus 0–22".into());
            }
        }
        let chip_urut = self.chip_urut.map(chip_order).transpose()?;
        if self.chip && chip_urut.is_none() {
            return Err("layer chip wajib punya chip_urut".into());
        }
        Ok(CatalogInput {
            slug: slug.to_string(),
            nama: text("nama", &self.nama, 120)?,
            kategori: text("kategori", &self.kategori, 80)?,
            sumber: text("sumber", &self.sumber, 120)?,
            kind: self.kind.clone(),
            url: url_value,
            params: Some(params),
            legend: self.legend.clone(),
            style: self.style.clone(),
            attribution: optional_text("attribution", self.attribution.as_ref(), 500)?,
            min_zoom: self.min_zoom,
            min_role: role(&self.min_role)?,
            chip: self.chip,
            chip_urut,
            baru_sampai: self.baru_sampai.as_deref().map(date).transpose()?,
            aktif: self.aktif,
        })
    }
}

impl CatalogPatch {
    pub fn is_empty(&self) -> bool {
        self.nama.is_none()
            && self.kategori.is_none()
            && self.sumber.is_none()
            && self.url.is_none()
            && self.attribution.is_none()
            && self.min_role.is_none()
            && self.aktif.is_none()
            && self.chip.is_none()
            && self.chip_urut.is_none()
            && self.baru_sampai.is_none()
    }

    /// Memeriksa bidang yang dikirim. Aturan antarbidang (mis. chip butuh chip_urut, selain internal butuh url)
    /// dijaga CHECK di database dan dilaporkan sebagai 400.
    pub fn validated(&self) -> Result<CatalogPatch, String> {
        if self.is_empty() {
            return Err("tidak ada bidang yang diubah".into());
        }
        Ok(CatalogPatch {
            nama: self.nama.as_deref().map(|v| text("nama", v, 120)).transpose()?,
            kategori: self.kategori.as_deref().map(|v| text("kategori", v, 80)).transpose()?,
            sumber: self.sumber.as_deref().map(|v| text("sumber", v, 120)).transpose()?,
            url: self.url.as_deref().map(url).transpose()?,
            attribution: optional_text("attribution", self.attribution.as_ref(), 500)?,
            min_role: self.min_role.as_deref().map(role).transpose()?,
            aktif: self.aktif,
            chip: self.chip,
            chip_urut: self.chip_urut.map(chip_order).transpose()?,
            baru_sampai: self.baru_sampai.as_deref().map(date).transpose()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn input() -> CatalogInput {
        serde_json::from_value(json!({
            "slug": "batas-provinsi",
            "nama": "Batas provinsi",
            "kategori": "Administrasi",
            "sumber": "BIG",
            "kind": "wms",
            "url": "https://contoh.go.id/wms"
        }))
        .unwrap()
    }

    #[test]
    fn input_valid_dengan_default() {
        let v = input().validated().unwrap();
        assert_eq!(v.min_role, "anon");
        assert!(v.aktif);
        assert_eq!(v.params, Some(json!({})));
    }

    #[test]
    fn bidang_tak_dikenal_ditolak() {
        let r: Result<CatalogInput, _> = serde_json::from_value(json!({
            "slug": "a", "nama": "a", "kategori": "a", "sumber": "a", "kind": "internal", "min_rol": "editor"
        }));
        assert!(r.is_err(), "salah ketik min_rol tidak boleh diabaikan diam-diam");
    }

    #[test]
    fn slug_hanya_huruf_kecil_angka_hubung() {
        for buruk in ["", "Besar", "ada spasi", "a_b", "a/b", &"x".repeat(61)] {
            let mut i = input();
            i.slug = buruk.into();
            assert!(i.validated().is_err(), "slug {buruk:?} harus ditolak");
        }
    }

    #[test]
    fn url_harus_https_tanpa_spasi() {
        for buruk in ["http://x.id/a", "javascript:alert(1)", "https://", "https://a b", "ftp://x"] {
            let mut i = input();
            i.url = Some(buruk.into());
            assert!(i.validated().is_err(), "url {buruk:?} harus ditolak");
        }
    }

    #[test]
    fn selain_internal_wajib_url_dan_internal_wajib_kunci() {
        let mut i = input();
        i.url = None;
        assert!(i.validated().is_err());

        let mut j = input();
        j.kind = "internal".into();
        j.url = None;
        assert!(j.validated().is_err(), "internal tanpa internalKey");
        j.params = Some(json!({ "internalKey": "cakupan-spm" }));
        assert!(j.validated().is_ok());
    }

    #[test]
    fn peran_dan_kind_harus_dikenal() {
        let mut i = input();
        i.min_role = "superadmin".into();
        assert!(i.validated().is_err());
        let mut j = input();
        j.kind = "shapefile".into();
        assert!(j.validated().is_err());
    }

    #[test]
    fn chip_butuh_urutan_dan_zoom_dibatasi() {
        let mut i = input();
        i.chip = true;
        assert!(i.validated().is_err());
        i.chip_urut = Some(1);
        assert!(i.validated().is_ok());
        i.min_zoom = Some(23);
        assert!(i.validated().is_err());
    }

    #[test]
    fn params_harus_objek_dan_tidak_terlalu_besar() {
        let mut i = input();
        i.params = Some(json!([1, 2]));
        assert!(i.validated().is_err());
        i.params = Some(json!({ "x": "a".repeat(MAX_JSON_BYTES) }));
        assert!(i.validated().is_err());
    }

    #[test]
    fn spasi_dipangkas() {
        let mut i = input();
        i.nama = "  Batas provinsi  ".into();
        assert_eq!(i.validated().unwrap().nama, "Batas provinsi");
        i.nama = "   ".into();
        assert!(i.validated().is_err());
    }

    #[test]
    fn tanggal_mengikuti_kalender() {
        assert!(valid_iso_date("2026-11-08"));
        assert!(valid_iso_date("2028-02-29"));
        for buruk in ["2026-02-29", "2026-13-01", "2026-00-10", "2026-04-31", "26-11-08", "2026/11/08", "2026-1-08", "abcd-ef-gh", ""] {
            assert!(!valid_iso_date(buruk), "{buruk:?} harus ditolak");
        }
    }

    #[test]
    fn patch_kosong_ditolak_dan_bidang_diperiksa() {
        assert!(CatalogPatch::default().validated().is_err());
        let p = CatalogPatch { aktif: Some(false), ..Default::default() };
        assert_eq!(p.validated().unwrap().aktif, Some(false));
        let q = CatalogPatch { min_role: Some("root".into()), ..Default::default() };
        assert!(q.validated().is_err());
        let r = CatalogPatch { url: Some("http://x".into()), ..Default::default() };
        assert!(r.validated().is_err());
    }

    #[test]
    fn patch_tidak_menerima_slug_atau_kind() {
        let r: Result<CatalogPatch, _> = serde_json::from_value(json!({ "slug": "baru" }));
        assert!(r.is_err());
        let r: Result<CatalogPatch, _> = serde_json::from_value(json!({ "kind": "wms" }));
        assert!(r.is_err());
    }
}
