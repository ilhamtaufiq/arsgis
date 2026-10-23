//! Model domain WebGIS perpipaan: jenis fitur, bbox, dan validasi input.
//! Tidak bergantung pada HTTP atau database.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum CoreError {
    #[error("jenis fitur tidak dikenal: {0}")]
    UnknownFeatureType(String),
    #[error("bbox tidak valid: {0}")]
    InvalidBbox(String),
    #[error("peran tidak dikenal: {0}")]
    UnknownRole(String),
}

/// Jenis fitur jaringan. Nilainya disimpan sebagai teks di kolom `features.feature_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FeatureType {
    Pipa,
    Valve,
    Reservoir,
    Sr,
    Bronkaptering,
}

impl FeatureType {
    pub const ALL: [FeatureType; 5] = [
        FeatureType::Pipa,
        FeatureType::Valve,
        FeatureType::Reservoir,
        FeatureType::Sr,
        FeatureType::Bronkaptering,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            FeatureType::Pipa => "pipa",
            FeatureType::Valve => "valve",
            FeatureType::Reservoir => "reservoir",
            FeatureType::Sr => "sr",
            FeatureType::Bronkaptering => "bronkaptering",
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

/// Peran pengguna. Urutan dari yang paling terbatas ke yang paling luas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Viewer,
    Editor,
    Admin,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Viewer => "viewer",
            Role::Editor => "editor",
            Role::Admin => "admin",
        }
    }

    pub fn parse(s: &str) -> Result<Self, CoreError> {
        match s {
            "viewer" => Ok(Role::Viewer),
            "editor" => Ok(Role::Editor),
            "admin" => Ok(Role::Admin),
            other => Err(CoreError::UnknownRole(other.to_string())),
        }
    }

    /// Data pelanggan (sr) hanya untuk editor dan admin.
    pub fn can_read(self, t: FeatureType) -> bool {
        !t.is_customer_data() || self >= Role::Editor
    }
}

/// Peran `None` berarti pengunjung tanpa login.
pub fn can_read_types(role: Option<Role>, types: &[FeatureType]) -> bool {
    types.iter().all(|t| match role {
        Some(r) => r.can_read(*t),
        None => !t.is_customer_data(),
    })
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
    fn only_sr_is_customer_data() {
        assert!(FeatureType::Sr.is_customer_data());
        assert!(!FeatureType::Pipa.is_customer_data());
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
