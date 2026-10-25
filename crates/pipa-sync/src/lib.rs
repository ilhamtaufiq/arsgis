//! Sinkron data referensi SPM air minum dari Arumanis (APIAMIS), dan pembaca batas desa GeoJSON.
//! Hanya membaca dari Arumanis. Tidak bergantung pada database arsgis.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("permintaan ke Arumanis gagal: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Arumanis menjawab {status} untuk {url}")]
    Status { status: u16, url: String },
    #[error("data dari Arumanis tidak sesuai format: {0}")]
    Format(String),
    #[error("GeoJSON batas desa tidak valid: {0}")]
    Geojson(String),
}

/// Angka dari Arumanis bisa berupa angka, string angka, atau null. Null dan teks tidak valid menjadi 0.
fn lenient_i64<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    Ok(match Value::deserialize(d)? {
        Value::Number(n) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)).unwrap_or(0),
        Value::String(s) => s.trim().parse::<f64>().map(|f| f as i64).unwrap_or(0),
        Value::Bool(b) => i64::from(b),
        _ => 0,
    })
}

fn lenient_bool<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    Ok(match Value::deserialize(d)? {
        Value::Bool(b) => b,
        Value::Number(n) => n.as_i64().unwrap_or(0) != 0,
        Value::String(s) => matches!(s.trim(), "1" | "true"),
        _ => false,
    })
}

/// Baris `GET /api/public/spam-units/map-stats`: ringkasan capaian per desa.
#[derive(Debug, Clone, Deserialize)]
pub struct DesaStat {
    pub desa_id: i64,
    pub desa: Option<String>,
    pub kecamatan: Option<String>,
    #[serde(default, deserialize_with = "lenient_i64")]
    pub target: i64,
    #[serde(default, deserialize_with = "lenient_i64")]
    pub unit_count: i64,
    #[serde(default, deserialize_with = "lenient_i64")]
    pub sr: i64,
    #[serde(default, deserialize_with = "lenient_i64")]
    pub kk: i64,
    #[serde(default, deserialize_with = "lenient_i64")]
    pub jiwa: i64,
    #[serde(default, deserialize_with = "lenient_i64")]
    pub bjp_master: i64,
    #[serde(default, deserialize_with = "lenient_i64")]
    pub bjp_unit: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Pengelola {
    pub pokmas: Option<String>,
    pub perdes: Option<String>,
    pub kepala: Option<String>,
    pub bendahara: Option<String>,
    pub sekretaris: Option<String>,
}

impl Pengelola {
    fn is_empty(&self) -> bool {
        [&self.pokmas, &self.perdes, &self.kepala, &self.bendahara, &self.sekretaris]
            .iter()
            .all(|v| clean(v.as_deref()).is_none())
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Achievement {
    #[serde(default, deserialize_with = "lenient_i64")]
    pub jumlah_sr: i64,
    #[serde(default, deserialize_with = "lenient_i64")]
    pub jumlah_kk: i64,
    #[serde(default, deserialize_with = "lenient_i64")]
    pub jumlah_jiwa: i64,
    #[serde(default, deserialize_with = "lenient_i64")]
    pub jumlah_bjp_kk: i64,
}

/// Baris `GET /api/spam-units`, hanya kolom yang dipakai arsgis.
#[derive(Debug, Clone, Deserialize)]
pub struct Unit {
    pub id: i64,
    #[serde(deserialize_with = "lenient_i64")]
    pub desa_id: i64,
    pub name: Option<String>,
    pub sistem_layanan: Option<String>,
    pub sumber_dana: Option<String>,
    #[serde(default, deserialize_with = "lenient_bool")]
    pub is_simspam: bool,
    pub pengelola: Option<Pengelola>,
    #[serde(default)]
    pub achievements: Vec<Achievement>,
}

/// Capaian unit: jumlah seluruh rekam tahunan (acuan s/d 2025 + integrasi 2026+), sama dengan Arumanis.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Capaian {
    pub sr: i64,
    pub kk: i64,
    pub jiwa: i64,
    pub bjp_kk: i64,
}

impl Unit {
    pub fn capaian(&self) -> Capaian {
        self.achievements.iter().fold(Capaian::default(), |acc, a| Capaian {
            sr: acc.sr + a.jumlah_sr,
            kk: acc.kk + a.jumlah_kk,
            jiwa: acc.jiwa + a.jumlah_jiwa,
            bjp_kk: acc.bjp_kk + a.jumlah_bjp_kk,
        })
    }
}

/// Pokmas tingkat desa. Di Arumanis pengelola disimpan per unit, jadi arsgis menurunkannya per desa.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PokmasDesa {
    pub desa_id: i64,
    pub nama: Option<String>,
    pub perdes: Option<String>,
    pub kepala: Option<String>,
    pub bendahara: Option<String>,
    pub sekretaris: Option<String>,
    /// True bila unit-unit di desa ini mencatat nama Pokmas yang berbeda.
    pub konflik: bool,
    /// Unit yang punya data pengelola, urut id.
    pub sumber_unit_ids: Vec<i64>,
}

fn clean(v: Option<&str>) -> Option<String> {
    v.map(str::trim).filter(|s| !s.is_empty() && *s != "-").map(str::to_string)
}

/// Kunci pembanding nama wilayah: huruf kecil, hanya huruf dan angka.
/// SQL padanannya: `regexp_replace(lower(x), '[^a-z0-9]', '', 'g')`.
pub fn normalize(s: &str) -> String {
    s.to_lowercase().chars().filter(|c| c.is_ascii_alphanumeric()).collect()
}

/// Satu Pokmas per desa: diambil dari unit dengan id terkecil yang punya data pengelola.
/// Desa yang unitnya tidak punya data pengelola tetap muncul dengan field kosong.
pub fn derive_pokmas(units: &[Unit]) -> Vec<PokmasDesa> {
    let mut by_desa: BTreeMap<i64, Vec<&Unit>> = BTreeMap::new();
    for u in units {
        by_desa.entry(u.desa_id).or_default().push(u);
    }

    by_desa
        .into_iter()
        .map(|(desa_id, mut list)| {
            list.sort_by_key(|u| u.id);
            let with_data: Vec<&Unit> = list
                .iter()
                .copied()
                .filter(|u| u.pengelola.as_ref().is_some_and(|p| !p.is_empty()))
                .collect();
            let names: BTreeSet<String> = with_data
                .iter()
                .filter_map(|u| clean(u.pengelola.as_ref()?.pokmas.as_deref()))
                .map(|n| normalize(&n))
                .collect();

            let primary = with_data.first().and_then(|u| u.pengelola.clone()).unwrap_or_default();
            PokmasDesa {
                desa_id,
                nama: clean(primary.pokmas.as_deref()),
                perdes: clean(primary.perdes.as_deref()),
                kepala: clean(primary.kepala.as_deref()),
                bendahara: clean(primary.bendahara.as_deref()),
                sekretaris: clean(primary.sekretaris.as_deref()),
                konflik: names.len() > 1,
                sumber_unit_ids: with_data.iter().map(|u| u.id).collect(),
            }
        })
        .collect()
}

/// Satu poligon desa dari GeoJSON BPS (properti `village_code`, `district`, `village`).
#[derive(Debug, Clone)]
pub struct BatasDesa {
    pub kode_bps: String,
    pub kecamatan: String,
    pub desa: String,
    pub geometry: Value,
}

pub fn parse_batas_desa(json: &str) -> Result<Vec<BatasDesa>, SyncError> {
    let root: Value = serde_json::from_str(json).map_err(|e| SyncError::Geojson(e.to_string()))?;
    let features = root
        .get("features")
        .and_then(Value::as_array)
        .ok_or_else(|| SyncError::Geojson("tidak ada FeatureCollection.features".into()))?;

    let mut out = Vec::with_capacity(features.len());
    for (i, f) in features.iter().enumerate() {
        let props = f.get("properties").cloned().unwrap_or(Value::Null);
        let text = |k: &str| props.get(k).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
        let (Some(kode_bps), Some(kecamatan), Some(desa)) = (text("village_code"), text("district"), text("village")) else {
            return Err(SyncError::Geojson(format!("fitur ke-{i} tanpa village_code, district, atau village")));
        };
        let geometry = f.get("geometry").cloned().unwrap_or(Value::Null);
        let kind = geometry.get("type").and_then(Value::as_str).unwrap_or("");
        if kind != "Polygon" && kind != "MultiPolygon" {
            return Err(SyncError::Geojson(format!("fitur {kode_bps} bukan Polygon/MultiPolygon")));
        }
        out.push(BatasDesa { kode_bps, kecamatan, desa, geometry });
    }
    Ok(out)
}

/// Klien baca Arumanis. `token` dipakai untuk endpoint yang butuh login (Bearer).
pub struct ArumanisClient {
    http: reqwest::Client,
    base: String,
    token: Option<String>,
}

#[derive(Deserialize)]
struct Envelope<T> {
    data: T,
    #[serde(default)]
    meta: Option<Meta>,
}

#[derive(Deserialize)]
struct Meta {
    #[serde(default, deserialize_with = "lenient_i64")]
    last_page: i64,
}

/// Token Sanctum berbentuk `id|rahasia`. Nilai yang disalin dari cookie Arumanis di-URL-encode
/// (`id%7Crahasia`) dan akan ditolak sebagai Bearer, jadi dikembalikan ke bentuk aslinya.
pub fn normalize_token(token: &str) -> String {
    token.trim().replace("%7C", "|").replace("%7c", "|")
}

impl ArumanisClient {
    pub fn new(base: &str, token: Option<String>) -> Result<Self, SyncError> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .user_agent("arsgis-sync")
            .build()?;
        let token = token.map(|t| normalize_token(&t)).filter(|t| !t.is_empty());
        Ok(Self { http, base: base.trim_end_matches('/').to_string(), token })
    }

    pub fn has_token(&self) -> bool {
        self.token.is_some()
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<Envelope<T>, SyncError> {
        let url = format!("{}{}", self.base, path);
        let mut req = self.http.get(&url).header("Accept", "application/json");
        if let Some(t) = &self.token {
            req = req.bearer_auth(t);
        }
        let res = req.send().await?;
        if !res.status().is_success() {
            return Err(SyncError::Status { status: res.status().as_u16(), url });
        }
        let body = res.text().await?;
        serde_json::from_str(&body).map_err(|e| SyncError::Format(format!("{path}: {e}")))
    }

    /// Ringkasan capaian per desa. Endpoint publik, tidak butuh token.
    pub async fn desa_stats(&self) -> Result<Vec<DesaStat>, SyncError> {
        Ok(self.get::<Vec<DesaStat>>("/public/spam-units/map-stats").await?.data)
    }

    /// Semua unit SPAM beserta pengelola dan capaian. Butuh token.
    pub async fn units(&self) -> Result<Vec<Unit>, SyncError> {
        let mut out = Vec::new();
        let mut page = 1;
        loop {
            let env = self.get::<Vec<Unit>>(&format!("/spam-units?per_page=200&page={page}")).await?;
            let last = env.meta.map(|m| m.last_page).unwrap_or(1).max(1);
            out.extend(env.data);
            if page >= last {
                break;
            }
            page += 1;
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(id: i64, desa_id: i64, pokmas: Option<&str>, kepala: Option<&str>) -> Unit {
        Unit {
            id,
            desa_id,
            name: None,
            sistem_layanan: None,
            sumber_dana: None,
            is_simspam: false,
            pengelola: Some(Pengelola {
                pokmas: pokmas.map(String::from),
                kepala: kepala.map(String::from),
                ..Default::default()
            }),
            achievements: vec![],
        }
    }

    #[test]
    fn normalize_ignores_case_and_punctuation() {
        assert_eq!(normalize("Cikidang Bayang-bang"), "cikidangbayangbang");
        assert_eq!(normalize("CAMPAKA MULYA"), normalize("Campaka Mulya"));
    }

    #[test]
    fn pokmas_single_unit() {
        let p = derive_pokmas(&[unit(5, 1, Some(" Tirta Jaya "), Some("Asep"))]);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].nama.as_deref(), Some("Tirta Jaya"));
        assert!(!p[0].konflik);
        assert_eq!(p[0].sumber_unit_ids, vec![5]);
    }

    #[test]
    fn pokmas_takes_lowest_unit_and_flags_conflict() {
        let p = derive_pokmas(&[unit(9, 1, Some("Mekar Sari"), None), unit(3, 1, Some("Tirta Jaya"), None)]);
        assert_eq!(p[0].nama.as_deref(), Some("Tirta Jaya"));
        assert!(p[0].konflik);
        assert_eq!(p[0].sumber_unit_ids, vec![3, 9]);
    }

    #[test]
    fn pokmas_same_name_different_spelling_is_not_conflict() {
        let p = derive_pokmas(&[unit(1, 1, Some("TIRTA JAYA"), None), unit(2, 1, Some("Tirta-Jaya"), None)]);
        assert!(!p[0].konflik);
    }

    #[test]
    fn pokmas_empty_pengelola_still_listed() {
        let mut u = unit(1, 7, None, None);
        u.pengelola = Some(Pengelola { pokmas: Some(" - ".into()), ..Default::default() });
        let p = derive_pokmas(&[u]);
        assert_eq!(p[0].desa_id, 7);
        assert_eq!(p[0].nama, None);
        assert!(p[0].sumber_unit_ids.is_empty());
    }

    #[test]
    fn unit_parses_lenient_numbers_and_sums_capaian() {
        let u: Unit = serde_json::from_str(
            r#"{"id":1,"desa_id":"206","is_simspam":1,"pengelola":null,
                "achievements":[{"jumlah_sr":"10","jumlah_kk":8,"jumlah_jiwa":null,"jumlah_bjp_kk":2},
                                {"jumlah_sr":5,"jumlah_kk":5,"jumlah_jiwa":25,"jumlah_bjp_kk":0}]}"#,
        )
        .unwrap();
        assert_eq!(u.desa_id, 206);
        assert!(u.is_simspam);
        assert_eq!(u.capaian(), Capaian { sr: 15, kk: 13, jiwa: 25, bjp_kk: 2 });
    }

    #[test]
    fn batas_desa_parses_bps_properties() {
        let json = r#"{"type":"FeatureCollection","features":[
            {"type":"Feature","properties":{"village_code":"id3203170009","district":"Ciranjang","village":"Cibiuk"},
             "geometry":{"type":"MultiPolygon","coordinates":[[[[107.1,-6.8],[107.2,-6.8],[107.2,-6.9],[107.1,-6.8]]]]}}]}"#;
        let b = parse_batas_desa(json).unwrap();
        assert_eq!(b[0].kode_bps, "id3203170009");
        assert_eq!(b[0].desa, "Cibiuk");
    }

    #[test]
    fn token_from_cookie_is_decoded() {
        assert_eq!(normalize_token(" 12%7Cabc "), "12|abc");
        assert_eq!(normalize_token("12|abc"), "12|abc");
    }

    #[test]
    fn batas_desa_rejects_points() {
        let json = r#"{"features":[{"properties":{"village_code":"x","district":"a","village":"b"},"geometry":{"type":"Point","coordinates":[1,2]}}]}"#;
        assert!(parse_batas_desa(json).is_err());
    }
}
