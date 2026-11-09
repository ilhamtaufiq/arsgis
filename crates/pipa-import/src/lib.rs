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
    #[error("berkas bukan teks UTF-8")]
    NotUtf8,
    #[error("KMZ tidak valid: {0}")]
    Zip(String),
    #[error("KMZ tidak berisi berkas .kml")]
    NoKml,
    #[error("isi KML melebihi {0} MB setelah dibuka")]
    TooLarge(usize),
}

/// Batas ukuran KML setelah dibuka dari KMZ. Mencegah zip kecil yang mengembang jadi sangat besar.
const MAX_KML_BYTES: usize = 50 * 1024 * 1024;

/// Teks KML dari berkas unggahan. KMZ (zip) dikenali dari awalan `PK\x03\x04` dan dibuka: dipakai `doc.kml`,
/// atau berkas `.kml` pertama bila namanya lain. Selain itu berkas dianggap KML teks biasa.
/// sha256 isi berkas dalam hex huruf kecil. Dipakai untuk mencegah impor ganda; dihitung dari bytes asli
/// (KML atau KMZ sebelum dibuka), bukan dari XML hasil ekstraksi.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

pub fn kml_text(bytes: &[u8]) -> Result<String, ImportError> {
    kml_text_limited(bytes, MAX_KML_BYTES)
}

fn kml_text_limited(bytes: &[u8], max: usize) -> Result<String, ImportError> {
    use std::io::Read;

    if !bytes.starts_with(b"PK\x03\x04") {
        return std::str::from_utf8(bytes).map(str::to_string).map_err(|_| ImportError::NotUtf8);
    }
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| ImportError::Zip(e.to_string()))?;
    let names: Vec<String> = archive.file_names().map(str::to_string).collect();
    let is_kml = |n: &str| !n.ends_with('/') && n.to_ascii_lowercase().ends_with(".kml");
    let chosen = names
        .iter()
        .find(|n| n.eq_ignore_ascii_case("doc.kml"))
        .or_else(|| names.iter().find(|n| is_kml(n)))
        .ok_or(ImportError::NoKml)?;
    let entry = archive.by_name(chosen).map_err(|e| ImportError::Zip(e.to_string()))?;
    let mut buf = Vec::new();
    // Baca satu byte lebih dari batas supaya kelebihan terdeteksi tanpa memuat seluruh isi.
    entry.take(max as u64 + 1).read_to_end(&mut buf).map_err(|e| ImportError::Zip(e.to_string()))?;
    if buf.len() > max {
        return Err(ImportError::TooLarge(max / (1024 * 1024)));
    }
    String::from_utf8(buf).map_err(|_| ImportError::NotUtf8)
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
///
/// Pencocokan per kata (dipisah tanda baca dan spasi), supaya singkatan lapangan ikut terbaca tetapi
/// nama desa yang kebetulan mengandung huruf yang sama tidak: `BRONCAP`, `BRONCAPTERING`, `BRONKAP...`
/// untuk bronkaptering, dan kata `RV` berdiri sendiri untuk reservoir (`MKJCKL RV EXISTING`).
fn jenis_titik(nama: Option<&str>) -> Option<FeatureType> {
    let upper = nama?.to_uppercase();
    let words: Vec<&str> = upper.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();
    let has_prefix = |prefix: &str| words.iter().any(|w| w.starts_with(prefix));
    if has_prefix("BRONCAP") || has_prefix("BRONKAP") {
        Some(FeatureType::Bronkaptering)
    } else if upper.contains("VALVE") {
        Some(FeatureType::Valve)
    } else if upper.contains("RESERVOIR") || words.contains(&"RV") {
        Some(FeatureType::Reservoir)
    } else if has_prefix("TAPPING") {
        // Titik tapping pipa adalah sambungan pengambilan air dari pipa, bukan SR pelanggan.
        Some(FeatureType::Tapping)
    } else if words.contains(&"SR") {
        // "SR 1" adalah titik pengambilan air ke pelanggan.
        Some(FeatureType::Sr)
    } else if words.contains(&"SUMUR") {
        // "Sumur Bor" adalah sumber air, dicatat sebagai intake.
        Some(FeatureType::Intake)
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
    #[test]
    fn sha256_hex_sesuai_vektor_uji() {
        // Vektor uji standar FIPS 180-2 untuk "abc".
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(sha256_hex(b"").len(), 64);
    }

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

    #[test]
    fn field_abbreviations_are_recognised_as_point_types() {
        // Nama titik nyata dari KMZ lapangan.
        assert_eq!(jenis_titik(Some("MKJCKL RV EXISTING")), Some(FeatureType::Reservoir));
        assert_eq!(jenis_titik(Some("KARANGWANGI RV Kap. 25 M3")), Some(FeatureType::Reservoir));
        assert_eq!(jenis_titik(Some("MKJCKL BRONCAP CISALADA")), Some(FeatureType::Bronkaptering));
        assert_eq!(jenis_titik(Some("MKJKL BRONCAP RAWA RAMPE")), Some(FeatureType::Bronkaptering));
        assert_eq!(jenis_titik(Some("BJKASIH BRONCAPTERING")), Some(FeatureType::Bronkaptering));
        assert_eq!(jenis_titik(Some("Broncap-1")), Some(FeatureType::Bronkaptering));
        assert_eq!(jenis_titik(Some("Reservoir utama")), Some(FeatureType::Reservoir));
        assert_eq!(jenis_titik(Some("valve induk")), Some(FeatureType::Valve));
    }

    #[test]
    fn kmz_asbuilt_names_map_to_sr_tapping_and_intake() {
        // Nama dari berkas as-built Babakan Sari: SR bernomor, tapping pipa, dan sumur bor.
        assert_eq!(jenis_titik(Some("SR 1")), Some(FeatureType::Sr));
        assert_eq!(jenis_titik(Some("SR 40")), Some(FeatureType::Sr));
        assert_eq!(jenis_titik(Some("Tapping poin pipa 1.5 in")), Some(FeatureType::Tapping));
        assert_eq!(jenis_titik(Some("Tapping pipa 2 in babakansari")), Some(FeatureType::Tapping));
        assert_eq!(jenis_titik(Some("tapping pipa 1 in")), Some(FeatureType::Tapping));
        // Kata SR di tengah nama tapping tidak mengubah jenisnya: awalan TAPPING yang menang.
        assert_eq!(jenis_titik(Some("Tapping SR 3")), Some(FeatureType::Tapping));
        assert_eq!(jenis_titik(Some("Sumur Bor Eksisting")), Some(FeatureType::Intake));
        // Nama yang hanya mirip tidak ikut terklasifikasi.
        assert_eq!(jenis_titik(Some("SRIGALA 3")), None);
    }

    #[test]
    fn numbered_points_and_lookalikes_stay_unclassified() {
        // Titik bernomor adalah sambungan rumah (jatuh ke jenis default). "RV" harus kata sendiri.
        for nama in ["BJKASIH 10", "KARANGWANGI 4", "KARANGWANGI  1", "SERVIS 1", "ARVIN 2", "RVX 3", ""] {
            assert_eq!(jenis_titik(Some(nama)), None, "{nama:?}");
        }
        assert_eq!(jenis_titik(None), None);
    }

    fn make_zip(files: &[(&str, &[u8])], method: zip::CompressionMethod) -> Vec<u8> {
        use std::io::Write;
        let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let opts = zip::write::SimpleFileOptions::default().compression_method(method);
        for (name, data) in files {
            w.start_file(*name, opts).unwrap();
            w.write_all(data).unwrap();
        }
        w.finish().unwrap().into_inner()
    }

    #[test]
    fn plain_kml_passes_through() {
        assert_eq!(kml_text(SAMPLE.as_bytes()).unwrap(), SAMPLE);
    }

    #[test]
    fn kmz_doc_kml_is_extracted_and_parses_like_kml() {
        for method in [zip::CompressionMethod::Stored, zip::CompressionMethod::Deflated] {
            let kmz = make_zip(&[("doc.kml", SAMPLE.as_bytes())], method);
            let text = kml_text(&kmz).unwrap();
            assert_eq!(text, SAMPLE);
            let p = parse_kml(&text, Some(FeatureType::Sr)).unwrap();
            assert_eq!(p.features.iter().filter(|f| f.feature_type == FeatureType::Pipa).count(), 2);
        }
    }

    #[test]
    fn kmz_prefers_doc_kml_then_any_kml() {
        let kmz = make_zip(
            &[("files/icon.png", b"x"), ("lain.kml", b"<kml>lain</kml>"), ("doc.kml", b"<kml>doc</kml>")],
            zip::CompressionMethod::Stored,
        );
        assert_eq!(kml_text(&kmz).unwrap(), "<kml>doc</kml>");
        let tanpa_doc = make_zip(&[("files/icon.png", b"x"), ("peta.KML", b"<kml>peta</kml>")], zip::CompressionMethod::Stored);
        assert_eq!(kml_text(&tanpa_doc).unwrap(), "<kml>peta</kml>");
    }

    #[test]
    fn kmz_without_kml_or_broken_is_rejected() {
        let kosong = make_zip(&[("files/icon.png", b"x")], zip::CompressionMethod::Stored);
        assert!(matches!(kml_text(&kosong), Err(ImportError::NoKml)));
        // Awalan zip tetapi isinya rusak.
        assert!(matches!(kml_text(b"PK\x03\x04bukan-zip"), Err(ImportError::Zip(_))));
    }

    #[test]
    fn non_utf8_is_rejected_for_plain_and_kmz() {
        assert!(matches!(kml_text(&[0xff, 0xfe, 0x00]), Err(ImportError::NotUtf8)));
        let kmz = make_zip(&[("doc.kml", &[0xff, 0xfe, 0x00][..])], zip::CompressionMethod::Stored);
        assert!(matches!(kml_text(&kmz), Err(ImportError::NotUtf8)));
    }

    #[test]
    fn oversized_kml_inside_kmz_is_rejected() {
        let besar = vec![b'a'; 2048];
        let kmz = make_zip(&[("doc.kml", &besar[..])], zip::CompressionMethod::Deflated);
        assert!(kml_text_limited(&kmz, 4096).is_ok());
        assert!(matches!(kml_text_limited(&kmz, 1024), Err(ImportError::TooLarge(_))));
    }
}
