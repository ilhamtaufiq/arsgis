//! Parser KML untuk impor jaringan. Menghasilkan fitur siap staging dan daftar item yang dilewati beserta alasannya.
//! Tidak bergantung pada HTTP atau database.

use pipa_core::FeatureType;
use quick_xml::events::Event;
use quick_xml::Reader;
use serde_json::{json, Value};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("KML tidak valid: {0}")]
    Xml(String),
}

/// Satu fitur hasil parse, siap ditulis ke staging.
#[derive(Debug, Clone)]
pub struct StagedFeature {
    pub feature_type: FeatureType,
    pub nama: Option<String>,
    pub diameter_mm: Option<i32>,
    pub geometry: Value,
}

/// Placemark yang tidak diimpor, dengan alasannya.
#[derive(Debug, Clone)]
pub struct Skipped {
    pub nama: Option<String>,
    pub alasan: String,
}

#[derive(Debug, Default)]
pub struct Preview {
    pub features: Vec<StagedFeature>,
    pub skipped: Vec<Skipped>,
    /// Jumlah elemen GroundOverlay (gambar, bukan fitur jaringan). Tidak diimpor.
    pub bukan_fitur: usize,
}

#[derive(Default)]
struct Placemark {
    name: Option<String>,
    kind: Option<String>,
    coords: String,
}

enum Field {
    Name,
    Coords,
}

/// Parse KML. `titik_default` dipakai untuk Point yang namanya tidak menyebut jenisnya.
pub fn parse_kml(xml: &str, titik_default: Option<FeatureType>) -> Result<Preview, ImportError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut preview = Preview::default();
    let mut path: Vec<String> = Vec::new();
    let mut current: Option<Placemark> = None;
    let mut field: Option<Field> = None;

    loop {
        let event = reader.read_event().map_err(|e| ImportError::Xml(e.to_string()))?;
        match event {
            Event::Start(e) => {
                let tag = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                match tag.as_str() {
                    "Placemark" => current = Some(Placemark::default()),
                    "GroundOverlay" => preview.bukan_fitur += 1,
                    "name" if path.last().map(String::as_str) == Some("Placemark") => field = Some(Field::Name),
                    "coordinates" if current.is_some() => field = Some(Field::Coords),
                    "Point" | "LineString" | "Polygon" => {
                        if let Some(p) = current.as_mut() {
                            p.kind.get_or_insert(tag.clone());
                        }
                    }
                    _ => {}
                }
                path.push(tag);
            }
            Event::Text(t) => {
                let text = t.unescape().map_err(|e| ImportError::Xml(e.to_string()))?.into_owned();
                if let (Some(f), Some(p)) = (&field, current.as_mut()) {
                    match f {
                        Field::Name => p.name = Some(text),
                        Field::Coords => p.coords.push_str(&text),
                    }
                }
            }
            Event::End(e) => {
                let tag = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                path.pop();
                match tag.as_str() {
                    "name" | "coordinates" => field = None,
                    "Placemark" => {
                        if let Some(p) = current.take() {
                            finish(p, titik_default, &mut preview);
                        }
                    }
                    _ => {}
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(preview)
}

fn finish(p: Placemark, titik_default: Option<FeatureType>, out: &mut Preview) {
    let nama = p.name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
    let coords = parse_coords(&p.coords);

    match p.kind.as_deref() {
        Some("LineString") => {
            if coords.len() < 2 {
                return skip(out, nama, "garis kurang dari 2 titik valid");
            }
            let diameter_mm = nama.as_deref().and_then(diameter_mm);
            out.features.push(StagedFeature {
                feature_type: FeatureType::Pipa,
                nama,
                diameter_mm,
                geometry: json!({ "type": "LineString", "coordinates": coords }),
            });
        }
        Some("Point") => {
            if coords.len() != 1 {
                return skip(out, nama, "titik tanpa koordinat valid");
            }
            let Some(feature_type) = jenis_titik(nama.as_deref()).or(titik_default) else {
                return skip(out, nama, "jenis titik tidak dikenal dan tidak ada jenis default");
            };
            out.features.push(StagedFeature {
                feature_type,
                nama,
                diameter_mm: None,
                geometry: json!({ "type": "Point", "coordinates": coords[0] }),
            });
        }
        Some(other) => skip(out, nama, &format!("geometri {other} belum didukung")),
        None => skip(out, nama, "tanpa geometri"),
    }
}

fn skip(out: &mut Preview, nama: Option<String>, alasan: &str) {
    out.skipped.push(Skipped { nama, alasan: alasan.to_string() });
}

/// Jenis titik dari kata kunci di nama. Nama yang tidak punya kata kunci mengembalikan None.
fn jenis_titik(nama: Option<&str>) -> Option<FeatureType> {
    let upper = nama?.to_uppercase();
    if upper.contains("BRONCAPTERING") {
        Some(FeatureType::Bronkaptering)
    } else if upper.contains("VALVE") {
        Some(FeatureType::Valve)
    } else if upper.contains("RESERVOIR") {
        Some(FeatureType::Reservoir)
    } else {
        None
    }
}

/// Diameter dari pola `<angka> IN` di nama, dalam inci, dikonversi ke milimeter.
/// Contoh: `BJKASIH 1,5 IN 600 MTR` menghasilkan 38.
fn diameter_mm(nama: &str) -> Option<i32> {
    let tokens: Vec<&str> = nama.split_whitespace().collect();
    (1..tokens.len()).find_map(|i| {
        if !tokens[i].eq_ignore_ascii_case("IN") {
            return None;
        }
        let inch: f64 = tokens[i - 1].replace(',', ".").parse().ok()?;
        let mm = (inch * 25.4).round() as i32;
        (mm > 0).then_some(mm)
    })
}

/// Koordinat KML berbentuk `lon,lat[,alt]` dipisah spasi. Hanya pasangan dalam rentang WGS84 yang diambil.
fn parse_coords(raw: &str) -> Vec<[f64; 2]> {
    raw.split_whitespace()
        .filter_map(|tuple| {
            let mut parts = tuple.split(',');
            let lon: f64 = parts.next()?.parse().ok()?;
            let lat: f64 = parts.next()?.parse().ok()?;
            let valid = (-180.0..=180.0).contains(&lon) && (-90.0..=90.0).contains(&lat);
            valid.then_some([lon, lat])
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<kml xmlns="http://www.opengis.net/kml/2.2"><Document>
  <Folder>
    <name>BJKASIH</name>
    <Placemark><name>BJKASIH 1 IN 28 MTR </name>
      <LineString><tessellate>1</tessellate>
        <coordinates>107.0336,-7.2206,0 107.0335,-7.2207,0</coordinates>
      </LineString></Placemark>
    <Placemark><name>BJKASIH 1,5 IN 600 MTR</name>
      <LineString><coordinates>107.0331,-7.2198,0 107.0332,-7.2201,0 107.0333,-7.2203,0</coordinates></LineString></Placemark>
    <Placemark><name>BJKASIH 10</name>
      <Point><coordinates>107.0357922,-7.2210189,0</coordinates></Point></Placemark>
    <Placemark><name>BJKASIH BRONCAPTERING</name>
      <Point><coordinates>107.0345450,-7.2174354,0</coordinates></Point></Placemark>
    <Placemark><name>Area</name>
      <Polygon><outerBoundaryIs><LinearRing><coordinates>0,0,0 1,0,0 1,1,0 0,0,0</coordinates></LinearRing></outerBoundaryIs></Polygon></Placemark>
  </Folder>
  <GroundOverlay><name>peta.kmz</name></GroundOverlay>
</Document></kml>"#;

    #[test]
    fn diameter_from_inch_name() {
        assert_eq!(diameter_mm("BJKASIH 1 IN 28 MTR "), Some(25));
        assert_eq!(diameter_mm("BJKASIH 1,5 IN 600 MTR"), Some(38));
        assert_eq!(diameter_mm("BJKASIH  3 IN 12 MTR"), Some(76));
        assert_eq!(diameter_mm("BJKASIH 10"), None);
    }

    #[test]
    fn point_type_from_keyword() {
        assert_eq!(jenis_titik(Some("BJKASIH BRONCAPTERING")), Some(FeatureType::Bronkaptering));
        assert_eq!(jenis_titik(Some("Valve Utama")), Some(FeatureType::Valve));
        assert_eq!(jenis_titik(Some("BJKASIH 10")), None);
    }

    #[test]
    fn parses_sample() {
        let p = parse_kml(SAMPLE, Some(FeatureType::Sr)).unwrap();
        assert_eq!(p.bukan_fitur, 1);
        assert_eq!(p.features.len(), 4);
        let pipa: Vec<_> = p.features.iter().filter(|f| f.feature_type == FeatureType::Pipa).collect();
        assert_eq!(pipa.len(), 2);
        assert_eq!(pipa[0].diameter_mm, Some(25));
        assert_eq!(pipa[1].diameter_mm, Some(38));
        let sr = p.features.iter().find(|f| f.nama.as_deref() == Some("BJKASIH 10")).unwrap();
        assert_eq!(sr.feature_type, FeatureType::Sr);
        assert_eq!(sr.geometry["type"], "Point");
        assert_eq!(p.skipped.len(), 1);
        assert_eq!(p.skipped[0].alasan, "geometri Polygon belum didukung");
    }

    #[test]
    fn point_without_default_is_skipped() {
        let p = parse_kml(SAMPLE, None).unwrap();
        assert_eq!(p.features.len(), 3);
        assert!(p.skipped.iter().any(|s| s.nama.as_deref() == Some("BJKASIH 10")));
    }
}
