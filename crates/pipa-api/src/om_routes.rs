//! Endpoint operasi dan pemeliharaan (Tahap C): gangguan dan foto, perbaikan, jadwal perawatan,
//! jatuh tempo, dan rehabilitasi.
//!
//! Peran: pengunjung tanpa login tidak punya akses. Viewer (termasuk kepala desa) hanya membaca, dan
//! nama pelapor disembunyikan dari mereka. Editor dan admin mencatat dan memperbarui.

use std::time::{SystemTime, UNIX_EPOCH};

use argon2::password_hash::rand_core::{OsRng, RngCore};
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, FromRequestParts, Path, Query, State},
    http::{header, request::Parts, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use pipa_core::gangguan::{ekstensi_foto, periksa_foto, StatusGangguan, MAKS_FOTO_BYTES};
use pipa_core::om::OmError;
use pipa_core::perawatan::{periksa_jendela, JENDELA_DEFAULT_HARI};
use pipa_core::{can_read_types, FeatureType};
use pipa_core::rehab::StatusRehab;
use pipa_core::tanggal::Tanggal;
use pipa_core::Role;
use pipa_store::{FilterGangguan, GangguanBaru, JadwalBaru, PerbaikanBaru};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::{session_cookie, verify_token, CurrentUser};
use crate::{ApiError, AppState, Editor};

/// Zona waktu tanggal operasional: WIB (UTC+7), dalam menit.
const OFFSET_WIB_MENIT: i64 = 7 * 60;
/// Tubuh permintaan foto: berkas maksimal ditambah sedikit ruang untuk pemeriksaan di handler.
const BATAS_TUBUH_FOTO: usize = MAKS_FOTO_BYTES + 1024;

impl From<OmError> for ApiError {
    fn from(e: OmError) -> Self {
        match &e {
            OmError::Masukan(_) => ApiError(StatusCode::BAD_REQUEST, e.to_string()),
            OmError::Aturan(_) => ApiError(StatusCode::UNPROCESSABLE_ENTITY, e.to_string()),
            OmError::TidakDitemukan(_) => ApiError(StatusCode::NOT_FOUND, e.to_string()),
        }
    }
}

/// Pengguna yang sudah masuk, peran apa pun. Pengunjung tanpa login ditolak.
pub(crate) struct Viewer(pub(crate) CurrentUser);

impl FromRequestParts<AppState> for Viewer {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let user = session_cookie(parts)
            .and_then(|t| verify_token(t, &state.jwt_secret))
            .ok_or_else(|| ApiError(StatusCode::UNAUTHORIZED, "silakan masuk terlebih dahulu".into()))?;
        Ok(Viewer(user))
    }
}

fn parse<T: serde::de::DeserializeOwned>(body: &[u8]) -> Result<T, ApiError> {
    serde_json::from_slice(body)
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, format!("isian tidak valid: {e}")))
}

fn hari_ini() -> Tanggal {
    let detik = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    Tanggal::dari_unix(detik, OFFSET_WIB_MENIT)
}

/// Nama pelapor hanya untuk editor dan admin. Viewer tidak menerima kolom ini sama sekali.
fn sembunyikan_pelapor(v: &mut Value, role: Role) {
    if role >= Role::Editor {
        return;
    }
    if let Some(obj) = v.as_object_mut() {
        obj.remove("pelapor");
    }
}

/// Berkas foto hanya nama acak buatan server dengan ekstensi jpg atau png. Dipakai sebelum membuka berkas dari disk.
fn berkas_aman(nama: &str) -> bool {
    let Some((stem, ext)) = nama.rsplit_once('.') else {
        return false;
    };
    stem.len() == 32 && stem.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)) && matches!(ext, "jpg" | "png")
}

fn nama_berkas_acak(ext: &str) -> String {
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!("{hex}.{ext}")
}

#[derive(Deserialize)]
struct DaftarGangguanQuery {
    network_id: Option<i64>,
    feature_id: Option<i64>,
    status: Option<String>,
    batas: Option<i64>,
}

async fn daftar_gangguan(
    State(state): State<AppState>,
    Viewer(user): Viewer,
    Query(q): Query<DaftarGangguanQuery>,
) -> Result<Json<Value>, ApiError> {
    if q.network_id.is_none() && q.feature_id.is_none() {
        return Err(ApiError(StatusCode::BAD_REQUEST, "isi network_id atau feature_id".into()));
    }
    let status = q.status.as_deref().map(StatusGangguan::parse).transpose()?;
    let batas = q.batas.unwrap_or(100).clamp(1, 200);
    let filter = FilterGangguan { network_id: q.network_id, feature_id: q.feature_id, status };
    let daftar = state.store.daftar_gangguan(filter, batas).await?;
    let mut data = serde_json::to_value(&daftar).map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    for item in data.as_array_mut().into_iter().flatten() {
        sembunyikan_pelapor(item, user.role);
    }
    Ok(Json(json!({ "data": data })))
}

#[derive(Deserialize)]
struct BuatGangguanBody {
    network_id: i64,
    feature_id: Option<i64>,
    lon: Option<f64>,
    lat: Option<f64>,
    jenis: String,
    deskripsi: String,
    /// Nama pelapor. Kosong berarti operator yang mencatat.
    pelapor: Option<String>,
}

async fn buat_gangguan(State(state): State<AppState>, Editor(user): Editor, body: Bytes) -> Result<Response, ApiError> {
    let b: BuatGangguanBody = parse(&body)?;
    let titik = match (b.lon, b.lat) {
        (Some(lon), Some(lat)) => Some((lon, lat)),
        (None, None) => None,
        _ => return Err(ApiError(StatusCode::BAD_REQUEST, "lon dan lat harus diisi berpasangan".into())),
    };
    let pelapor = b.pelapor.filter(|p| !p.trim().is_empty()).unwrap_or_else(|| user.username.clone());
    let baru = GangguanBaru {
        network_id: b.network_id,
        feature_id: b.feature_id,
        titik,
        jenis: b.jenis,
        deskripsi: b.deskripsi,
        pelapor,
    };
    let g = state.store.buat_gangguan(baru, &user.username).await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": g }))).into_response())
}

async fn ambil_gangguan(State(state): State<AppState>, Viewer(user): Viewer, Path(id): Path<i64>) -> Result<Json<Value>, ApiError> {
    let g = state
        .store
        .ambil_gangguan(id)
        .await?
        .ok_or(OmError::TidakDitemukan("laporan"))?;
    let mut data = serde_json::to_value(&g).map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    sembunyikan_pelapor(&mut data, user.role);
    Ok(Json(json!({ "data": data })))
}

#[derive(Deserialize)]
struct UbahStatusBody {
    status: String,
}

async fn ubah_gangguan_status(
    State(state): State<AppState>,
    Editor(user): Editor,
    Path(id): Path<i64>,
    body: Bytes,
) -> Result<Json<Value>, ApiError> {
    let b: UbahStatusBody = parse(&body)?;
    let g = state.store.ubah_status_gangguan(id, &b.status, &user.username).await?;
    Ok(Json(json!({ "data": g })))
}

async fn unggah_foto(
    State(state): State<AppState>,
    Editor(user): Editor,
    Path(id): Path<i64>,
    body: Bytes,
) -> Result<Response, ApiError> {
    // Pemeriksaan awal dengan jumlah foto saat ini. Batas dicek ulang di store di bawah kunci baris.
    let laporan = state.store.ambil_gangguan(id).await?.ok_or(OmError::TidakDitemukan("laporan"))?;
    let mime = periksa_foto(&body, laporan.foto.len() as i64)?;
    let ext = ekstensi_foto(mime).ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "tipe foto tidak didukung".into()))?;

    tokio::fs::create_dir_all(&state.foto_dir)
        .await
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("folder foto tidak bisa dibuat: {e}")))?;
    let berkas = nama_berkas_acak(ext);
    let lokasi = state.foto_dir.join(&berkas);
    tokio::fs::write(&lokasi, &body)
        .await
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("foto tidak bisa disimpan: {e}")))?;

    let ukuran = body.len() as i32;
    match state.store.tambah_foto(id, &berkas, mime, ukuran, &user.username).await {
        Ok(foto) => Ok((StatusCode::CREATED, Json(json!({ "data": foto }))).into_response()),
        Err(e) => {
            // Metadata gagal disimpan, jadi berkasnya dibuang agar tidak ada berkas yatim.
            let _ = tokio::fs::remove_file(&lokasi).await;
            Err(e.into())
        }
    }
}

async fn ambil_foto(State(state): State<AppState>, Viewer(_): Viewer, Path(id): Path<i64>) -> Result<Response, ApiError> {
    let foto = state.store.ambil_berkas_foto(id).await?.ok_or(OmError::TidakDitemukan("foto"))?;
    if !berkas_aman(&foto.berkas) {
        return Err(ApiError(StatusCode::INTERNAL_SERVER_ERROR, "nama berkas foto tidak valid".into()));
    }
    let bytes = tokio::fs::read(state.foto_dir.join(&foto.berkas))
        .await
        .map_err(|_| OmError::TidakDitemukan("berkas foto"))?;
    let mut response = bytes.into_response();
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_str(&foto.mime).unwrap_or(HeaderValue::from_static("application/octet-stream")));
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("private, max-age=3600"));
    headers.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    Ok(response)
}

#[derive(Deserialize)]
struct JatuhTempoQuery {
    network_id: i64,
    hari: Option<i64>,
}

async fn jatuh_tempo(State(state): State<AppState>, Viewer(_): Viewer, Query(q): Query<JatuhTempoQuery>) -> Result<Json<Value>, ApiError> {
    let jendela = periksa_jendela(q.hari.unwrap_or(JENDELA_DEFAULT_HARI))?;
    let daftar = state.store.jatuh_tempo(q.network_id, hari_ini(), jendela).await?;
    Ok(Json(json!({ "data": daftar, "jendela_hari": jendela })))
}

#[derive(Deserialize)]
struct RehabPrioritasQuery {
    network_id: i64,
}

async fn rehab_prioritas(State(state): State<AppState>, Viewer(_): Viewer, Query(q): Query<RehabPrioritasQuery>) -> Result<Json<Value>, ApiError> {
    let daftar = state.store.daftar_prioritas(q.network_id).await?;
    Ok(Json(json!({ "data": daftar })))
}

async fn om_aset(State(state): State<AppState>, Viewer(user): Viewer, Path(feature_id): Path<i64>) -> Result<Json<Value>, ApiError> {
    // Aset sr adalah data pelanggan. Panel O&M-nya mengikuti batas baca yang sama dengan peta.
    let tipe = state.store.tipe_aset(feature_id).await?.ok_or(OmError::TidakDitemukan("aset"))?;
    if !can_read_types(Some(user.role), &[FeatureType::parse(&tipe)?]) {
        return Err(ApiError(StatusCode::FORBIDDEN, "aset ini butuh peran editor atau admin".into()));
    }
    let om = state.store.om_aset(feature_id).await?.ok_or(OmError::TidakDitemukan("aset"))?;
    let mut data = serde_json::to_value(&om).map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if let Some(daftar) = data.get_mut("gangguan").and_then(Value::as_array_mut) {
        for g in daftar {
            sembunyikan_pelapor(g, user.role);
        }
    }
    Ok(Json(json!({ "data": data })))
}

#[derive(Deserialize)]
struct PerbaikanBody {
    tanggal: String,
    tindakan: String,
    teknisi: String,
    biaya: Option<f64>,
    catatan: Option<String>,
    gangguan_id: Option<i64>,
}

async fn tambah_perbaikan(
    State(state): State<AppState>,
    Editor(user): Editor,
    Path(feature_id): Path<i64>,
    body: Bytes,
) -> Result<Response, ApiError> {
    let b: PerbaikanBody = parse(&body)?;
    let baru = PerbaikanBaru {
        tanggal: b.tanggal,
        tindakan: b.tindakan,
        teknisi: b.teknisi,
        biaya: b.biaya,
        catatan: b.catatan,
        gangguan_id: b.gangguan_id,
    };
    let hasil = state.store.tambah_perbaikan(feature_id, baru, hari_ini(), &user.username).await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": hasil }))).into_response())
}

#[derive(Deserialize)]
struct JadwalBody {
    jenis: String,
    interval_hari: i64,
    tanggal_berikutnya: Option<String>,
}

async fn tambah_jadwal(
    State(state): State<AppState>,
    Editor(user): Editor,
    Path(feature_id): Path<i64>,
    body: Bytes,
) -> Result<Response, ApiError> {
    let b: JadwalBody = parse(&body)?;
    let baru = JadwalBaru { jenis: b.jenis, interval_hari: b.interval_hari, tanggal_berikutnya: b.tanggal_berikutnya };
    let hasil = state.store.tambah_jadwal(feature_id, baru, hari_ini(), &user.username).await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": hasil }))).into_response())
}

#[derive(Deserialize, Default)]
struct CatatBody {
    tanggal: Option<String>,
}

async fn catat_jadwal_selesai(
    State(state): State<AppState>,
    Editor(user): Editor,
    Path(jadwal_id): Path<i64>,
    body: Bytes,
) -> Result<Json<Value>, ApiError> {
    // Tubuh boleh kosong: tanggal pelaksanaan dianggap hari ini.
    let b: CatatBody = if body.is_empty() { CatatBody::default() } else { parse(&body)? };
    let hasil = state
        .store
        .catat_perawatan_selesai(jadwal_id, b.tanggal.as_deref(), hari_ini(), &user.username)
        .await?;
    Ok(Json(json!({ "data": hasil })))
}

#[derive(Deserialize)]
struct RehabBody {
    status_rehab: String,
    prioritas: Option<i64>,
    catatan: Option<String>,
}

async fn simpan_rehab(
    State(state): State<AppState>,
    Editor(user): Editor,
    Path(feature_id): Path<i64>,
    body: Bytes,
) -> Result<Json<Value>, ApiError> {
    let b: RehabBody = parse(&body)?;
    let status = StatusRehab::parse(&b.status_rehab)?;
    let hasil = state
        .store
        .simpan_rehab(feature_id, status, b.prioritas, b.catatan.as_deref(), &user.username)
        .await?;
    Ok(Json(json!({ "data": hasil })))
}

/// Rute O&M. Digabung ke router utama di main.rs.
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/om/gangguan", get(daftar_gangguan).post(buat_gangguan))
        .route("/api/v1/om/gangguan/{id}", get(ambil_gangguan).patch(ubah_gangguan_status))
        .route(
            "/api/v1/om/gangguan/{id}/foto",
            post(unggah_foto).layer(DefaultBodyLimit::max(BATAS_TUBUH_FOTO)),
        )
        .route("/api/v1/om/foto/{id}", get(ambil_foto))
        .route("/api/v1/om/jatuh-tempo", get(jatuh_tempo))
        .route("/api/v1/om/rehab-prioritas", get(rehab_prioritas))
        .route("/api/v1/om/aset/{feature_id}", get(om_aset))
        .route("/api/v1/om/aset/{feature_id}/perbaikan", post(tambah_perbaikan))
        .route("/api/v1/om/aset/{feature_id}/jadwal", post(tambah_jadwal))
        .route("/api/v1/om/aset/{feature_id}/rehab", axum::routing::put(simpan_rehab))
        .route("/api/v1/om/jadwal/{id}/catat", post(catat_jadwal_selesai))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn berkas_aman_hanya_nama_acak_dengan_ekstensi_foto() {
        let hex = "0123456789abcdef0123456789abcdef";
        assert!(berkas_aman(&format!("{hex}.jpg")));
        assert!(berkas_aman(&format!("{hex}.png")));
        assert!(!berkas_aman(&format!("../{hex}.jpg")));
        assert!(!berkas_aman(&format!("{hex}.gif")));
        assert!(!berkas_aman(&format!("{}.jpg", hex.to_uppercase())));
        assert!(!berkas_aman("rahasia.jpg"));
        assert!(!berkas_aman(hex));
    }

    #[test]
    fn nama_berkas_acak_lolos_pemeriksaan() {
        for ext in ["jpg", "png"] {
            let nama = nama_berkas_acak(ext);
            assert!(berkas_aman(&nama), "{nama}");
        }
        assert_ne!(nama_berkas_acak("jpg"), nama_berkas_acak("jpg"));
    }

    #[test]
    fn pelapor_disembunyikan_dari_viewer_saja() {
        let asal = json!({ "id": 1, "pelapor": "Siti", "status": "dilaporkan" });
        let mut viewer = asal.clone();
        sembunyikan_pelapor(&mut viewer, Role::Viewer);
        assert!(viewer.get("pelapor").is_none());
        assert_eq!(viewer["status"], "dilaporkan");

        let mut editor = asal.clone();
        sembunyikan_pelapor(&mut editor, Role::Editor);
        assert_eq!(editor["pelapor"], "Siti");
    }

    #[test]
    fn galat_om_dipetakan_ke_status_http() {
        assert_eq!(ApiError::from(OmError::Masukan("x".into())).0, StatusCode::BAD_REQUEST);
        assert_eq!(ApiError::from(OmError::Aturan(vec!["x".into()])).0, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(ApiError::from(OmError::TidakDitemukan("aset")).0, StatusCode::NOT_FOUND);
    }
}
