//! Penandaan aset untuk rehabilitasi dan urutan daftar prioritas.
//! Aturan utama: status rehabilitasi yang aktif hanya boleh untuk aset yang kondisinya rusak.

use std::cmp::Ordering;

use crate::om::OmError;
use crate::Kondisi;

pub const PRIORITAS_MIN: i64 = 1;
/// Angka prioritas 1 paling mendesak, 10 paling rendah.
pub const PRIORITAS_MAKS: i64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusRehab {
    TidakAda,
    Perlu,
    Prioritas,
    SedangDikerjakan,
}

impl StatusRehab {
    pub fn as_str(self) -> &'static str {
        match self {
            StatusRehab::TidakAda => "tidak_ada",
            StatusRehab::Perlu => "perlu",
            StatusRehab::Prioritas => "prioritas",
            StatusRehab::SedangDikerjakan => "sedang_dikerjakan",
        }
    }

    pub fn parse(s: &str) -> Result<Self, OmError> {
        match s {
            "tidak_ada" => Ok(StatusRehab::TidakAda),
            "perlu" => Ok(StatusRehab::Perlu),
            "prioritas" => Ok(StatusRehab::Prioritas),
            "sedang_dikerjakan" => Ok(StatusRehab::SedangDikerjakan),
            other => Err(OmError::Masukan(format!("status rehabilitasi tidak dikenal: {other}"))),
        }
    }

    /// Status yang masuk daftar prioritas rehabilitasi.
    pub fn aktif(self) -> bool {
        self != StatusRehab::TidakAda
    }
}

/// Kondisi yang membuat aset layak ditandai untuk rehabilitasi.
pub fn kondisi_rusak(kondisi: Option<&str>) -> bool {
    matches!(
        kondisi.and_then(|k| Kondisi::parse(k).ok()),
        Some(Kondisi::RusakRingan | Kondisi::RusakBerat | Kondisi::TidakBerfungsi)
    )
}

/// Aturan penandaan rehabilitasi:
/// - Status selain `tidak_ada` hanya untuk aset dengan kondisi rusak. Kondisi kosong, baik, atau teks lama ditolak.
/// - `tidak_ada` tidak boleh membawa angka prioritas.
/// - `prioritas` dan `sedang_dikerjakan` wajib punya angka prioritas 1 sampai 10. `perlu` boleh tanpa angka.
pub fn periksa_rehab(status: StatusRehab, prioritas: Option<i64>, kondisi: Option<&str>) -> Result<(), OmError> {
    if let Some(p) = prioritas {
        if !(PRIORITAS_MIN..=PRIORITAS_MAKS).contains(&p) {
            return Err(OmError::Masukan(format!("prioritas harus {PRIORITAS_MIN}–{PRIORITAS_MAKS}")));
        }
    }
    let mut aturan = Vec::new();
    match status {
        StatusRehab::TidakAda => {
            if prioritas.is_some() {
                aturan.push("aset tanpa kebutuhan rehabilitasi tidak boleh punya angka prioritas".to_string());
            }
        }
        StatusRehab::Perlu | StatusRehab::Prioritas | StatusRehab::SedangDikerjakan => {
            if !kondisi_rusak(kondisi) {
                let teks = kondisi.unwrap_or("belum dicatat");
                aturan.push(format!(
                    "aset dengan kondisi {teks} tidak bisa ditandai rehabilitasi; ubah kondisi dulu atau pilih tidak_ada"
                ));
            }
            if status != StatusRehab::Perlu && prioritas.is_none() {
                aturan.push(format!("status {} wajib disertai angka prioritas", status.as_str()));
            }
        }
    }
    if aturan.is_empty() {
        Ok(())
    } else {
        Err(OmError::Aturan(aturan))
    }
}

/// Bobot keparahan kondisi untuk urutan. Makin besar makin mendesak.
pub fn bobot_kondisi(kondisi: Option<&str>) -> u8 {
    match kondisi.and_then(|k| Kondisi::parse(k).ok()) {
        Some(Kondisi::TidakBerfungsi) => 3,
        Some(Kondisi::RusakBerat) => 2,
        Some(Kondisi::RusakRingan) => 1,
        _ => 0,
    }
}

/// Satu baris daftar prioritas. `status` None berarti aset rusak yang belum pernah ditandai.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KandidatRehab {
    pub status: Option<StatusRehab>,
    pub prioritas: Option<i64>,
    pub kondisi: Option<String>,
}

/// Urutan daftar prioritas: angka prioritas kecil dulu (kosong di akhir), lalu kondisi paling parah.
pub fn bandingkan_prioritas(a: &KandidatRehab, b: &KandidatRehab) -> Ordering {
    let urut_angka = match (a.prioritas, b.prioritas) {
        (Some(x), Some(y)) => x.cmp(&y),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    };
    urut_angka.then_with(|| bobot_kondisi(b.kondisi.as_deref()).cmp(&bobot_kondisi(a.kondisi.as_deref())))
}

/// Mengurutkan daftar prioritas di tempat.
pub fn urutkan_prioritas(daftar: &mut [KandidatRehab]) {
    daftar.sort_by(bandingkan_prioritas);
}

/// Aset masuk daftar bila statusnya aktif, atau bila kondisinya rusak tetapi belum pernah ditandai.
pub fn masuk_daftar(status: Option<StatusRehab>, kondisi: Option<&str>) -> bool {
    match status {
        Some(s) => s.aktif(),
        None => kondisi_rusak(kondisi),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kandidat(prioritas: Option<i64>, kondisi: Option<&str>) -> KandidatRehab {
        KandidatRehab { status: Some(StatusRehab::Perlu), prioritas, kondisi: kondisi.map(String::from) }
    }

    #[test]
    fn status_roundtrip_dan_tolak_nilai_asing() {
        for s in [StatusRehab::TidakAda, StatusRehab::Perlu, StatusRehab::Prioritas, StatusRehab::SedangDikerjakan] {
            assert_eq!(StatusRehab::parse(s.as_str()).unwrap(), s);
        }
        assert!(StatusRehab::parse("selesai").is_err());
        assert!(StatusRehab::parse("Perlu").is_err());
    }

    #[test]
    fn kondisi_baik_dan_kosong_tidak_bisa_ditandai() {
        assert!(periksa_rehab(StatusRehab::Perlu, None, Some("baik")).is_err());
        assert!(periksa_rehab(StatusRehab::Perlu, None, None).is_err());
        // Teks bebas lama juga tidak dianggap rusak.
        assert!(periksa_rehab(StatusRehab::Perlu, None, Some("perlu cek")).is_err());
        assert!(periksa_rehab(StatusRehab::Perlu, None, Some("rusak_ringan")).is_ok());
    }

    #[test]
    fn tidak_ada_boleh_untuk_kondisi_apa_pun_tanpa_prioritas() {
        assert!(periksa_rehab(StatusRehab::TidakAda, None, Some("baik")).is_ok());
        assert!(periksa_rehab(StatusRehab::TidakAda, None, None).is_ok());
        assert!(periksa_rehab(StatusRehab::TidakAda, Some(3), Some("rusak_berat")).is_err());
    }

    #[test]
    fn prioritas_wajib_untuk_prioritas_dan_sedang_dikerjakan() {
        assert!(periksa_rehab(StatusRehab::Prioritas, None, Some("rusak_berat")).is_err());
        assert!(periksa_rehab(StatusRehab::SedangDikerjakan, None, Some("tidak_berfungsi")).is_err());
        assert!(periksa_rehab(StatusRehab::Prioritas, Some(1), Some("rusak_berat")).is_ok());
        // Perlu boleh tanpa angka.
        assert!(periksa_rehab(StatusRehab::Perlu, None, Some("rusak_ringan")).is_ok());
    }

    #[test]
    fn angka_prioritas_dibatasi_1_sampai_10() {
        assert!(periksa_rehab(StatusRehab::Prioritas, Some(0), Some("rusak_berat")).is_err());
        assert!(periksa_rehab(StatusRehab::Prioritas, Some(11), Some("rusak_berat")).is_err());
        assert!(periksa_rehab(StatusRehab::Prioritas, Some(10), Some("rusak_berat")).is_ok());
    }

    #[test]
    fn bobot_kondisi_urut_keparahan() {
        assert!(bobot_kondisi(Some("tidak_berfungsi")) > bobot_kondisi(Some("rusak_berat")));
        assert!(bobot_kondisi(Some("rusak_berat")) > bobot_kondisi(Some("rusak_ringan")));
        assert!(bobot_kondisi(Some("rusak_ringan")) > bobot_kondisi(Some("baik")));
        assert_eq!(bobot_kondisi(None), 0);
    }

    #[test]
    fn urutan_angka_prioritas_lalu_kondisi() {
        let mut daftar = vec![
            kandidat(None, Some("tidak_berfungsi")),
            kandidat(Some(2), Some("rusak_ringan")),
            kandidat(Some(1), Some("rusak_ringan")),
            kandidat(Some(2), Some("tidak_berfungsi")),
        ];
        urutkan_prioritas(&mut daftar);
        let urut: Vec<(Option<i64>, Option<&str>)> =
            daftar.iter().map(|k| (k.prioritas, k.kondisi.as_deref())).collect();
        assert_eq!(
            urut,
            vec![
                (Some(1), Some("rusak_ringan")),
                (Some(2), Some("tidak_berfungsi")),
                (Some(2), Some("rusak_ringan")),
                (None, Some("tidak_berfungsi")),
            ]
        );
    }

    #[test]
    fn masuk_daftar_untuk_status_aktif_atau_rusak_belum_ditandai() {
        assert!(!masuk_daftar(Some(StatusRehab::TidakAda), Some("rusak_berat")));
        assert!(masuk_daftar(Some(StatusRehab::Perlu), Some("baik")));
        assert!(masuk_daftar(Some(StatusRehab::SedangDikerjakan), None));
        assert!(masuk_daftar(None, Some("tidak_berfungsi")));
        assert!(!masuk_daftar(None, Some("baik")));
        assert!(!masuk_daftar(None, None));
    }
}
