//! Server HTTP WebGIS perpipaan: API, autentikasi, dan penyajian frontend dari satu container.

mod auth;
mod otorisasi;
mod pelanggan_routes;
mod sync;

use std::{
    convert::Infallible,
    env,
    io::{self, BufRead},
    net::SocketAddr,
    path::PathBuf,
};

use std::collections::BTreeMap;

use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, FromRequestParts, Path, Query, State},
    http::{header, request::Parts, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use pipa_core::{can_read_types, parse_types, AsetError, AtributAset, AtributPatch, Bbox, CoreError, FeatureType, Izin, Role};
use pipa_store::{
    fitur_geojson, AsetBaru, AsetUbah, CatalogInput, CatalogPatch, CommitOutcome, PokmasDesa, PokmasInput, Store,
    StoreError, StagingRow, TulisHasil,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tower_http::{compression::CompressionLayer, services::ServeDir, trace::TraceLayer};

use crate::auth::{clear_cookie_value, session_cookie, set_cookie_value, verify_token, CurrentUser};

/// Batas ukuran berkas impor. KML dengan gambar tertanam bisa besar.
const MAX_IMPORT_BYTES: usize = 20 * 1024 * 1024;

#[derive(Clone)]
struct AppState {
    store: Store,
    syncer: sync::Syncer,
    jwt_secret: Vec<u8>,
    cookie_secure: bool,
}

/// Pengguna dari cookie sesi, atau `None` untuk pengunjung tanpa login.
struct MaybeUser(Option<CurrentUser>);

impl FromRequestParts<AppState> for MaybeUser {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let user = session_cookie(parts).and_then(|t| verify_token(t, &state.jwt_secret));
        Ok(MaybeUser(user))
    }
}

#[derive(Debug)]
struct ApiError(StatusCode, String);

impl ApiError {
    fn unauthorized() -> Self {
        ApiError(StatusCode::UNAUTHORIZED, "username atau password salah".into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

impl From<CoreError> for ApiError {
    fn from(e: CoreError) -> Self {
        ApiError(StatusCode::BAD_REQUEST, e.to_string())
    }
}

impl From<StoreError> for ApiError {
    fn from(e: StoreError) -> Self {
        match e {
            StoreError::TooManyFeatures => ApiError(StatusCode::UNPROCESSABLE_ENTITY, e.to_string()),
            StoreError::Duplicate => ApiError(StatusCode::CONFLICT, "slug sudah dipakai".into()),
            StoreError::Constraint => ApiError(
                StatusCode::BAD_REQUEST,
                "kombinasi isian tidak valid: layer chip butuh chip_urut, layer selain internal butuh url".into(),
            ),
            other => {
                tracing::error!(error = %other, "kesalahan database");
                ApiError(StatusCode::INTERNAL_SERVER_ERROR, "kesalahan internal".into())
            }
        }
    }
}

#[derive(Deserialize)]
struct GeoParams {
    bbox: String,
    types: Option<String>,
}

#[derive(Deserialize)]
struct LoginBody {
    username: String,
    password: String,
}

async fn healthz(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    state.store.ping().await?;
    Ok(Json(json!({ "status": "ok" })))
}

async fn list_networks(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let networks = state.store.list_networks().await?;
    Ok(Json(json!({ "data": networks })))
}

/// Katalog layer menurut peran pemanggil. Isinya berbeda per peran, jadi hanya boleh di-cache oleh
/// peramban pengguna itu sendiri (`private`), bukan oleh proxy bersama.
async fn list_catalog(State(state): State<AppState>, MaybeUser(user): MaybeUser) -> Result<Response, ApiError> {
    let layers = state.store.list_catalog(user.map(|u| u.role)).await?;
    let mut response = Json(json!({ "data": layers })).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("private, max-age=60"));
    response.headers_mut().insert(header::VARY, HeaderValue::from_static("Cookie"));
    Ok(response)
}

async fn login(State(state): State<AppState>, Json(body): Json<LoginBody>) -> Result<Response, ApiError> {
    let user = state
        .store
        .find_user(body.username.trim())
        .await?
        .filter(|u| u.active)
        .ok_or_else(ApiError::unauthorized)?;

    if !auth::verify_password(&body.password, &user.password_hash) {
        return Err(ApiError::unauthorized());
    }
    let role = Role::parse(&user.role).map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let token = auth::issue_token(&user.username, role, &state.jwt_secret)
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut response = Json(json!({ "username": user.username, "role": role.as_str() })).into_response();
    let cookie = HeaderValue::from_str(&set_cookie_value(&token, state.cookie_secure))
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    response.headers_mut().insert(header::SET_COOKIE, cookie);
    Ok(response)
}

async fn logout(State(state): State<AppState>) -> Response {
    let mut response = StatusCode::NO_CONTENT.into_response();
    if let Ok(cookie) = HeaderValue::from_str(&clear_cookie_value(state.cookie_secure)) {
        response.headers_mut().insert(header::SET_COOKIE, cookie);
    }
    response
}

async fn me(MaybeUser(user): MaybeUser) -> Result<Json<Value>, ApiError> {
    let user = user.ok_or_else(ApiError::unauthorized)?;
    Ok(Json(json!({ "username": user.username, "role": user.role.as_str() })))
}

async fn network_geojson(
    State(state): State<AppState>,
    MaybeUser(user): MaybeUser,
    Path(id): Path<i64>,
    Query(params): Query<GeoParams>,
) -> Result<Response, ApiError> {
    features_geojson(&state, user, Some(id), params).await
}

/// Fitur dari semua unit SPAM dalam bbox. Pilihan unit dilakukan di klien.
async fn all_geojson(
    State(state): State<AppState>,
    MaybeUser(user): MaybeUser,
    Query(params): Query<GeoParams>,
) -> Result<Response, ApiError> {
    features_geojson(&state, user, None, params).await
}

async fn features_geojson(
    state: &AppState,
    user: Option<CurrentUser>,
    network_id: Option<i64>,
    params: GeoParams,
) -> Result<Response, ApiError> {
    let bbox = Bbox::parse(&params.bbox)?;
    // Tanpa `types`, hanya jenis publik yang dikembalikan. Data pelanggan harus diminta eksplisit.
    let types: Vec<FeatureType> = match parse_types(params.types.as_deref())? {
        Some(list) => list,
        None => FeatureType::ALL.into_iter().filter(|t| !t.is_customer_data()).collect(),
    };
    let role = user.as_ref().map(|u| u.role);
    if !can_read_types(role, &types) {
        let denied: Vec<&str> = types
            .iter()
            .filter(|t| !role.map_or(!t.is_customer_data(), |r| r.can_read(**t)))
            .map(|t| t.as_str())
            .collect();
        return Err(ApiError(
            StatusCode::FORBIDDEN,
            format!("tidak punya izin untuk jenis: {}", denied.join(", ")),
        ));
    }

    let rows = state.store.features_in_bbox(network_id, bbox, Some(&types)).await?;
    let features: Vec<Value> = rows
        .into_iter()
        .map(|f| {
            json!({
                "type": "Feature",
                "id": f.id,
                "geometry": f.geometry,
                "properties": {
                    "network_id": f.network_id,
                    "feature_type": f.feature_type,
                    "nama": f.nama,
                    "diameter_mm": f.diameter_mm,
                    "material": f.material,
                    "tahun_pasang": f.tahun_pasang,
                    "kondisi": f.kondisi,
                    "status": f.status,
                    "fungsi": f.fungsi,
                    "kapasitas_m3": f.kapasitas_m3,
                    "debit_lps": f.debit_lps,
                    "elevasi_m": f.elevasi_m,
                    "tahun_bangun": f.tahun_bangun,
                    "sumber_dana": f.sumber_dana,
                    "kode_aset": f.kode_aset,
                    "dusun_id": f.dusun_id,
                    "terlayani": f.terlayani,
                },
            })
        })
        .collect();

    let mut response = Json(json!({ "type": "FeatureCollection", "features": features })).into_response();
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("application/geo+json"));
    // Respons bergantung pada peran, jadi cache bersama dilarang.
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("private, no-cache"));
    Ok(response)
}

/// Pengguna yang boleh mengimpor data: editor dan admin.
struct Editor(CurrentUser);

impl FromRequestParts<AppState> for Editor {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let user = session_cookie(parts)
            .and_then(|t| verify_token(t, &state.jwt_secret))
            .ok_or_else(ApiError::unauthorized)?;
        if !user.role.punya(Izin::KelolaData) {
            return Err(ApiError(StatusCode::FORBIDDEN, "aksi ini butuh peran editor atau admin".into()));
        }
        Ok(Editor(user))
    }
}

#[derive(Deserialize)]
struct UploadParams {
    /// Jaringan (= unit SPAM) tujuan impor. Wajib.
    network_id: Option<i64>,
    nama_file: Option<String>,
    /// Jenis untuk Point yang namanya tidak menyebut jenis, misalnya `sr`.
    titik: Option<String>,
}

#[derive(Deserialize)]
struct CommitParams {
    network_id: i64,
}

async fn upload_import(
    State(state): State<AppState>,
    Editor(user): Editor,
    Query(params): Query<UploadParams>,
    body: Bytes,
) -> Result<Response, ApiError> {
    if body.is_empty() {
        return Err(ApiError(StatusCode::BAD_REQUEST, "berkas kosong".into()));
    }
    // KML teks biasa atau KMZ (zip berisi doc.kml); dikenali dari isi berkas, bukan dari nama.
    let xml = pipa_import::kml_text(&body).map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    let xml = xml.as_str();
    let network_id = params
        .network_id
        .ok_or_else(|| bad_request("network_id wajib: pilih unit SPAM tujuan impor"))?;
    if state.store.network_unit(network_id).await?.is_none() {
        return Err(bad_request("jaringan ini bukan unit SPAM dengan desa, impor tidak bisa dilakukan"));
    }
    let titik = params.titik.as_deref().map(FeatureType::parse).transpose()?;
    let preview = pipa_import::parse_kml(xml, titik).map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;

    let mut per_jenis: BTreeMap<&str, usize> = BTreeMap::new();
    for f in &preview.features {
        *per_jenis.entry(f.feature_type.as_str()).or_default() += 1;
    }
    let dilewati: Vec<Value> = preview
        .skipped
        .iter()
        .map(|s| json!({ "nama": s.nama, "alasan": s.alasan }))
        .collect();
    let laporan = json!({
        "jumlah_fitur": preview.features.len(),
        "per_jenis": per_jenis,
        "dilewati": dilewati,
        "bukan_fitur": preview.bukan_fitur,
    });
    if preview.features.is_empty() {
        return Ok((StatusCode::UNPROCESSABLE_ENTITY, Json(json!({ "error": "tidak ada fitur yang bisa diimpor", "laporan": laporan }))).into_response());
    }

    let rows: Vec<StagingRow> = preview
        .features
        .iter()
        .map(|f| StagingRow {
            feature_type: f.feature_type.as_str().to_string(),
            nama: f.nama.clone(),
            diameter_mm: f.diameter_mm,
            geometry: f.geometry.to_string(),
        })
        .collect();
    let nama_file = params.nama_file.unwrap_or_else(|| "impor.kml".into());
    let id = state
        .store
        .create_batch(&nama_file, &laporan, &user.username, network_id, &rows)
        .await?;
    let batch = state
        .store
        .get_batch(id)
        .await?
        .ok_or_else(|| internal("batch hilang sesaat setelah dibuat"))?;

    Ok((StatusCode::CREATED, Json(json!({ "id": id, "status": "staged", "nama_file": nama_file, "network_id": network_id, "laporan": batch.laporan }))).into_response())
}

async fn get_import(State(state): State<AppState>, _: Editor, Path(id): Path<i64>) -> Result<Json<Value>, ApiError> {
    let batch = state
        .store
        .get_batch(id)
        .await?
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "batch impor tidak ditemukan".into()))?;
    Ok(Json(json!({
        "id": batch.id,
        "nama_file": batch.nama_file,
        "status": batch.status,
        "network_id": batch.network_id,
        "dibuat_oleh": batch.dibuat_oleh,
        "laporan": batch.laporan,
    })))
}

async fn commit_import(
    State(state): State<AppState>,
    _: Editor,
    Path(id): Path<i64>,
    Query(params): Query<CommitParams>,
) -> Result<Json<Value>, ApiError> {
    let networks = state.store.list_networks().await?;
    if !networks.iter().any(|n| n.id == params.network_id) {
        return Err(ApiError(StatusCode::BAD_REQUEST, "jaringan tidak ditemukan".into()));
    }
    match state.store.commit_batch(id, params.network_id).await? {
        CommitOutcome::Committed { ditulis, dilewati } => Ok(Json(json!({
            "id": id,
            "status": "committed",
            "fitur_ditulis": ditulis,
            "fitur_dilewati": dilewati,
        }))),
        CommitOutcome::NetworkMismatch => Err(bad_request("jaringan tidak sama dengan tujuan batch impor")),
        CommitOutcome::NotFound => Err(ApiError(StatusCode::NOT_FOUND, "batch impor tidak ditemukan".into())),
        CommitOutcome::AlreadyCommitted => Err(ApiError(StatusCode::CONFLICT, "batch sudah di-commit".into())),
    }
}

/// Ringkasan jaringan. Sambungan rumah dan riwayat impor hanya untuk editor dan admin.
async fn dashboard(
    State(state): State<AppState>,
    MaybeUser(user): MaybeUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let customer = user.as_ref().is_some_and(|u| u.role.punya(Izin::BacaDataPelanggan));
    let networks = state.store.list_networks().await?;
    let nama = networks
        .iter()
        .find(|n| n.id == id)
        .map(|n| n.nama.clone())
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "jaringan tidak ditemukan".into()))?;

    let fitur = state.store.feature_counts(id, customer).await?;
    let pipa = state.store.pipa_lengths(id).await?;
    let impor = if customer { state.store.recent_imports(10).await? } else { Vec::new() };
    Ok(Json(json!({
        "jaringan": nama,
        "bisa_melihat_sr": customer,
        "fitur": fitur,
        "pipa": pipa,
        "impor_terakhir": impor,
    })))
}

/// Pengguna dengan peran admin. Dipakai untuk semua endpoint /api/v1/admin.
struct Admin(CurrentUser);

impl FromRequestParts<AppState> for Admin {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let user = session_cookie(parts)
            .and_then(|t| verify_token(t, &state.jwt_secret))
            .ok_or_else(ApiError::unauthorized)?;
        if user.role != Role::Admin {
            return Err(ApiError(StatusCode::FORBIDDEN, "khusus admin".into()));
        }
        Ok(Admin(user))
    }
}

#[derive(Deserialize)]
struct NewUserBody {
    username: String,
    password: String,
    role: String,
}

#[derive(Deserialize)]
struct UserPatchBody {
    role: Option<String>,
    active: Option<bool>,
}

#[derive(Deserialize)]
struct NewNetworkBody {
    nama: String,
    deskripsi: Option<String>,
}

fn internal(e: impl std::fmt::Display) -> ApiError {
    tracing::error!(error = %e, "kesalahan internal");
    ApiError(StatusCode::INTERNAL_SERVER_ERROR, "kesalahan internal".into())
}

fn bad_request(msg: &str) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, msg.into())
}

async fn admin_list_users(State(state): State<AppState>, _: Admin) -> Result<Json<Value>, ApiError> {
    let users = state.store.list_users().await?;
    Ok(Json(json!({ "data": users })))
}

async fn admin_create_user(
    State(state): State<AppState>,
    _: Admin,
    Json(body): Json<NewUserBody>,
) -> Result<Response, ApiError> {
    let username = body.username.trim();
    if username.is_empty() || username.len() > 50 || !username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.') {
        return Err(bad_request("username 1–50 karakter: huruf, angka, titik, atau garis bawah"));
    }
    if body.password.len() < 10 {
        return Err(bad_request("password minimal 10 karakter"));
    }
    let role = Role::parse(&body.role).map_err(|e| bad_request(&e.to_string()))?;
    if state.store.list_users().await?.iter().any(|u| u.username == username) {
        return Err(ApiError(StatusCode::CONFLICT, "username sudah dipakai".into()));
    }
    let hash = auth::hash_password(&body.password).map_err(internal)?;
    let id = state.store.create_user(username, &hash, role.as_str()).await?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id, "username": username, "role": role.as_str() }))).into_response())
}

async fn admin_update_user(
    State(state): State<AppState>,
    Admin(me): Admin,
    Path(id): Path<i64>,
    Json(body): Json<UserPatchBody>,
) -> Result<Json<Value>, ApiError> {
    let role = body.role.as_deref().map(Role::parse).transpose().map_err(|e| bad_request(&e.to_string()))?;
    let users = state.store.list_users().await?;
    let target = users
        .iter()
        .find(|u| u.id == id)
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "pengguna tidak ditemukan".into()))?;

    // Admin tidak boleh mengunci dirinya sendiri dari halaman ini.
    if target.username == me.username {
        let demoted = role.is_some_and(|r| r != Role::Admin);
        let deactivated = body.active == Some(false);
        if demoted || deactivated {
            return Err(bad_request("tidak bisa menurunkan atau menonaktifkan akun sendiri"));
        }
    }

    state.store.update_user(id, role.map(Role::as_str), body.active).await?;
    Ok(Json(json!({ "id": id, "ok": true })))
}

async fn admin_list_networks(State(state): State<AppState>, _: Admin) -> Result<Json<Value>, ApiError> {
    let networks = state.store.list_networks().await?;
    Ok(Json(json!({ "data": networks })))
}

async fn admin_create_network(
    State(state): State<AppState>,
    _: Admin,
    Json(body): Json<NewNetworkBody>,
) -> Result<Response, ApiError> {
    let nama = body.nama.trim();
    if nama.is_empty() || nama.len() > 120 {
        return Err(bad_request("nama jaringan 1–120 karakter"));
    }
    let deskripsi = body.deskripsi.as_deref().map(str::trim).filter(|d| !d.is_empty());
    let id = state.store.create_network(nama, deskripsi).await?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id, "nama": nama }))).into_response())
}

/// Semua layer katalog, termasuk yang nonaktif dan untuk semua peran.
async fn admin_list_catalog(State(state): State<AppState>, _: Admin) -> Result<Json<Value>, ApiError> {
    let layers = state.store.list_catalog_admin().await?;
    Ok(Json(json!({ "data": layers })))
}

async fn admin_create_catalog(
    State(state): State<AppState>,
    Admin(me): Admin,
    Json(body): Json<CatalogInput>,
) -> Result<Response, ApiError> {
    let input = body.validated().map_err(|m| bad_request(&m))?;
    let id = state.store.create_catalog(&input).await?;
    tracing::info!(admin = %me.username, id, slug = %input.slug, min_role = %input.min_role, "layer katalog ditambah");
    Ok((StatusCode::CREATED, Json(json!({ "id": id, "slug": input.slug }))).into_response())
}

async fn admin_patch_catalog(
    State(state): State<AppState>,
    Admin(me): Admin,
    Path(id): Path<i64>,
    Json(body): Json<CatalogPatch>,
) -> Result<Json<Value>, ApiError> {
    let patch = body.validated().map_err(|m| bad_request(&m))?;
    if !state.store.update_catalog(id, &patch).await? {
        return Err(ApiError(StatusCode::NOT_FOUND, "layer tidak ditemukan".into()));
    }
    tracing::info!(admin = %me.username, id, "layer katalog diubah");
    Ok(Json(json!({ "id": id, "ok": true })))
}

async fn admin_delete_catalog(
    State(state): State<AppState>,
    Admin(me): Admin,
    Path(id): Path<i64>,
) -> Result<Response, ApiError> {
    if !state.store.delete_catalog(id).await? {
        return Err(ApiError(StatusCode::NOT_FOUND, "layer tidak ditemukan".into()));
    }
    tracing::info!(admin = %me.username, id, "layer katalog dihapus");
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn admin_list_imports(State(state): State<AppState>, _: Admin) -> Result<Json<Value>, ApiError> {
    let imports = state.store.list_imports(100).await?;
    Ok(Json(json!({ "data": imports })))
}

/// Daftar desa dengan capaian SPM (publik, tanpa nama pengurus Pokmas).
async fn list_desa(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let desa = state.store.list_desa().await?;
    Ok(Json(json!({ "data": desa })))
}

/// Detail satu desa. Publik untuk data umum; nama pengurus Pokmas hanya untuk editor dan admin.
async fn desa_detail(
    State(state): State<AppState>,
    MaybeUser(user): MaybeUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let lihat_pengurus = user.as_ref().is_some_and(|u| u.role.punya(Izin::BacaNamaPokmas));
    let mut detail = state.store.desa_detail(id).await?.ok_or_else(desa_tidak_ditemukan)?;
    if !lihat_pengurus {
        sembunyikan_pengurus(&mut detail.pokmas);
    }
    Ok(Json(json!({ "data": detail })))
}

fn desa_tidak_ditemukan() -> ApiError {
    ApiError(StatusCode::NOT_FOUND, "desa tidak ditemukan".into())
}

fn sembunyikan_pengurus(pokmas: &mut Option<PokmasDesa>) {
    if let Some(p) = pokmas.as_mut() {
        p.kepala = None;
        p.bendahara = None;
        p.sekretaris = None;
    }
}

/// Pokmas efektif satu desa. Publik untuk nama dan perdes; nama pengurus dan catatan koreksi hanya untuk editor dan admin.
async fn desa_pokmas(
    State(state): State<AppState>,
    MaybeUser(user): MaybeUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let editor = user.as_ref().is_some_and(|u| u.role.punya(Izin::BacaNamaPokmas));
    let detail = state.store.pokmas_detail(id).await?.ok_or_else(desa_tidak_ditemukan)?;
    let mut pokmas = detail.efektif.pokmas;
    let koreksi = if editor {
        detail.koreksi.map(|k| {
            json!({ "catatan": k.catatan, "diperbarui_oleh": k.diperbarui_oleh, "diperbarui_pada": k.diperbarui_pada })
        })
    } else {
        sembunyikan_pengurus(&mut pokmas);
        None
    };
    Ok(Json(json!({
        "data": { "sumber": detail.efektif.sumber, "pokmas": pokmas, "koreksi": koreksi }
    })))
}

/// Ganti Pokmas satu desa dengan isian dari arsgis. Data Arumanis tidak diubah.
async fn simpan_desa_pokmas(
    State(state): State<AppState>,
    Editor(me): Editor,
    Path(id): Path<i64>,
    Json(body): Json<PokmasInput>,
) -> Result<Json<Value>, ApiError> {
    let isian = body.validated().map_err(|m| bad_request(&m))?;
    if !state.store.simpan_pokmas_koreksi(id, &isian, &me.username).await? {
        return Err(desa_tidak_ditemukan());
    }
    tracing::info!(user = %me.username, desa = id, "pokmas desa diubah");
    Ok(Json(json!({ "id": id, "ok": true })))
}

/// Kembalikan Pokmas satu desa ke data Arumanis dengan menghapus koreksinya.
async fn hapus_desa_pokmas(
    State(state): State<AppState>,
    Editor(me): Editor,
    Path(id): Path<i64>,
) -> Result<Response, ApiError> {
    if !state.store.desa_aktif(id).await? {
        return Err(desa_tidak_ditemukan());
    }
    state.store.hapus_pokmas_koreksi(id, &me.username).await?;
    tracing::info!(user = %me.username, desa = id, "pokmas desa dikembalikan ke data arumanis");
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// Poligon desa dengan capaian SPM, untuk peta cakupan. Publik; nama pengurus tidak ikut.
async fn desa_geojson(State(state): State<AppState>) -> Result<Response, ApiError> {
    let rows = state.store.desa_geo().await?;
    let features: Vec<Value> = rows
        .into_iter()
        .filter_map(|d| {
            let geometry: Value = serde_json::from_str(&d.geometry).ok()?;
            Some(json!({
                "type": "Feature",
                "id": d.summary.id,
                "geometry": geometry,
                "properties": d.summary,
            }))
        })
        .collect();
    let mut response = Json(json!({ "type": "FeatureCollection", "features": features })).into_response();
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("application/geo+json"));
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=300"));
    Ok(response)
}

/// Kelengkapan data per desa: SR tercatat vs SR di peta, pipa, batas, dan Pokmas. Hanya editor dan admin.
async fn kelengkapan(State(state): State<AppState>, _: Editor) -> Result<Json<Value>, ApiError> {
    let rows = state.store.kelengkapan_desa().await?;
    let mut por_status: BTreeMap<&str, i64> = BTreeMap::new();
    for r in &rows {
        *por_status.entry(r.status).or_default() += 1;
    }
    let ringkasan = json!({
        "jumlah_desa": rows.len(),
        "per_status": por_status,
        "sr_tercatat": rows.iter().map(|r| r.sr_tercatat).sum::<i64>(),
        "sr_peta": rows.iter().map(|r| r.sr_peta).sum::<i64>(),
        "pipa_km": rows.iter().map(|r| r.pipa_km).sum::<f64>(),
        "tanpa_pokmas": rows.iter().filter(|r| !r.punya_pokmas).count(),
        "pokmas_konflik": rows.iter().filter(|r| r.pokmas_konflik).count(),
    });
    Ok(Json(json!({ "ringkasan": ringkasan, "data": rows })))
}

async fn admin_sync(State(state): State<AppState>, Admin(me): Admin) -> Result<Json<Value>, ApiError> {
    match state.syncer.run(&format!("admin:{}", me.username)).await {
        Ok(report) => Ok(Json(json!(report))),
        Err(sync::SyncBusy) => Err(ApiError(StatusCode::CONFLICT, "sinkron lain sedang berjalan".into())),
    }
}

async fn admin_sync_runs(State(state): State<AppState>, _: Admin) -> Result<Json<Value>, ApiError> {
    let runs = state.store.list_sync_runs(20).await?;
    Ok(Json(json!({ "data": runs, "interval_menit": state.syncer.interval_menit() })))
}

/// Isian POST /api/v1/features. Atribut ikut di tingkat atas body, sama dengan nama kolomnya.
#[derive(Deserialize)]
struct AsetBaruBody {
    network_id: i64,
    feature_type: String,
    geometry: Value,
    #[serde(flatten)]
    atribut: AtributAset,
}

fn parse_body<T: serde::de::DeserializeOwned>(body: &[u8]) -> Result<T, ApiError> {
    serde_json::from_slice(body).map_err(|e| bad_request(&format!("isian tidak valid: {e}")))
}

/// 400 untuk isian yang tidak well-formed, 422 untuk aturan bisnis.
fn aset_error(e: AsetError) -> ApiError {
    match e {
        AsetError::Masukan(m) => ApiError(StatusCode::BAD_REQUEST, m),
        AsetError::Aturan(daftar) => ApiError(StatusCode::UNPROCESSABLE_ENTITY, daftar.join("; ")),
    }
}

fn hasil_tulis<T>(h: TulisHasil<T>) -> Result<T, ApiError> {
    match h {
        TulisHasil::Berhasil(v) => Ok(v),
        TulisHasil::TidakAda => Err(ApiError(StatusCode::NOT_FOUND, "data tidak ditemukan".into())),
        TulisHasil::Ditolak(e) => Err(aset_error(e)),
    }
}

async fn create_feature(State(state): State<AppState>, Editor(user): Editor, body: Bytes) -> Result<Response, ApiError> {
    let b: AsetBaruBody = parse_body(&body)?;
    let feature_type = FeatureType::parse(&b.feature_type)?;
    let baru = AsetBaru { network_id: b.network_id, feature_type, geometry: b.geometry, atribut: b.atribut };
    let fitur = hasil_tulis(state.store.tambah_aset(baru, &user.username).await?)?;
    Ok((StatusCode::CREATED, Json(json!({ "data": fitur_geojson(&fitur) }))).into_response())
}

async fn patch_feature(
    State(state): State<AppState>,
    Editor(user): Editor,
    Path(id): Path<i64>,
    body: Bytes,
) -> Result<Json<Value>, ApiError> {
    let mut v: Value = parse_body(&body)?;
    let obj = v.as_object_mut().ok_or_else(|| bad_request("isian harus berupa objek JSON"))?;
    let geometry = obj.remove("geometry");
    if matches!(geometry, Some(Value::Null)) {
        return Err(bad_request("geometry tidak boleh null"));
    }
    let atribut: AtributPatch = serde_json::from_value(v).map_err(|e| bad_request(&format!("isian tidak valid: {e}")))?;
    let fitur = hasil_tulis(state.store.ubah_aset(id, AsetUbah { geometry, atribut }, &user.username).await?)?;
    Ok(Json(json!({ "data": fitur_geojson(&fitur) })))
}

async fn delete_feature(State(state): State<AppState>, Editor(user): Editor, Path(id): Path<i64>) -> Result<Response, ApiError> {
    hasil_tulis(state.store.hapus_aset(id, &user.username).await?)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn feature_riwayat(State(state): State<AppState>, _: Editor, Path(id): Path<i64>) -> Result<Json<Value>, ApiError> {
    let riwayat = state
        .store
        .riwayat_aset(id)
        .await?
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "data tidak ditemukan".into()))?;
    Ok(Json(json!({ "data": riwayat })))
}

fn router(state: AppState, web_dist: PathBuf, basemap_dir: PathBuf) -> Router {
    let index = web_dist.join("index.html");
    let spa = ServeDir::new(&web_dist).fallback(tower_http::services::ServeFile::new(index));

    Router::new()
        .route("/healthz", get(healthz))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/me", get(me))
        .route("/api/v1/catalog", get(list_catalog))
        .route("/api/v1/networks", get(list_networks))
        .route("/api/v1/networks/{id}/geojson", get(network_geojson))
        .route("/api/v1/features/geojson", get(all_geojson))
        .route("/api/v1/features", post(create_feature))
        .route(
            "/api/v1/features/{id}",
            axum::routing::patch(patch_feature).delete(delete_feature),
        )
        .route("/api/v1/features/{id}/riwayat", get(feature_riwayat))
        .route(
            "/api/v1/imports",
            post(upload_import).layer(DefaultBodyLimit::max(MAX_IMPORT_BYTES)),
        )
        .route("/api/v1/networks/{id}/dashboard", get(dashboard))
        .route("/api/v1/admin/users", get(admin_list_users).post(admin_create_user))
        .route("/api/v1/admin/users/{id}", axum::routing::patch(admin_update_user))
        .route("/api/v1/admin/networks", get(admin_list_networks).post(admin_create_network))
        .route("/api/v1/admin/catalog", get(admin_list_catalog).post(admin_create_catalog))
        .route(
            "/api/v1/admin/catalog/{id}",
            axum::routing::patch(admin_patch_catalog).delete(admin_delete_catalog),
        )
        .route("/api/v1/admin/imports", get(admin_list_imports))
        .route("/api/v1/admin/sync", post(admin_sync))
        .route("/api/v1/admin/sync-runs", get(admin_sync_runs))
        .route("/api/v1/desa", get(list_desa))
        .route("/api/v1/desa/geojson", get(desa_geojson))
        .route("/api/v1/desa/{id}", get(desa_detail))
        .route(
            "/api/v1/desa/{id}/pokmas",
            get(desa_pokmas).put(simpan_desa_pokmas).delete(hapus_desa_pokmas),
        )
        .route("/api/v1/dashboard/kelengkapan", get(kelengkapan))
        .route("/api/v1/imports/{id}", get(get_import))
        .route("/api/v1/imports/{id}/commit", post(commit_import))
        .merge(pelanggan_routes::router())
        // Berkas PMTiles disajikan statis; ServeDir mendukung range request yang dibutuhkan PMTiles.
        .nest_service("/basemap", ServeDir::new(basemap_dir))
        .fallback_service(spa)
        .layer(CompressionLayer::new().gzip(true).br(true))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

fn jwt_secret() -> Result<Vec<u8>, String> {
    let secret = env::var("JWT_SECRET").map_err(|_| "JWT_SECRET wajib di-set (minimal 32 karakter)".to_string())?;
    if secret.len() < 32 {
        return Err("JWT_SECRET terlalu pendek (minimal 32 karakter)".into());
    }
    Ok(secret.into_bytes())
}

/// `pipa-api create-user <username> <role>`: password dari env PIPA_PASSWORD atau baris pertama stdin.
async fn create_user(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let [username, role] = args else {
        return Err("pemakaian: pipa-api create-user <username> <viewer|editor|admin>".into());
    };
    let role = Role::parse(role)?;
    let password = match env::var("PIPA_PASSWORD") {
        Ok(p) => p,
        Err(_) => {
            let mut line = String::new();
            io::stdin().lock().read_line(&mut line)?;
            line.trim_end_matches(['\r', '\n']).to_string()
        }
    };
    if password.len() < 10 {
        return Err("password minimal 10 karakter".into());
    }

    let database_url = env::var("DATABASE_URL").map_err(|_| "DATABASE_URL wajib di-set")?;
    let store = Store::connect(&database_url).await?;
    store.migrate().await?;
    let hash = auth::hash_password(&password)?;
    let id = store.create_user(username, &hash, role.as_str()).await?;
    println!("pengguna '{username}' dibuat (id {id}, peran {})", role.as_str());
    Ok(())
}

async fn connect_and_migrate() -> Result<Store, Box<dyn std::error::Error>> {
    let database_url = env::var("DATABASE_URL").map_err(|_| "DATABASE_URL wajib di-set")?;
    let store = Store::connect(&database_url).await?;
    store.migrate().await?;
    Ok(store)
}

/// Membuat admin awal saat startup bila belum ada. Kredensial dari env, tidak tertulis di kode:
/// `ADMIN_USERNAME` (default `admin`) dan `ADMIN_PASSWORD` (minimal 10 karakter).
/// Bila pengguna itu sudah ada, tidak ada yang diubah, termasuk kata sandinya.
async fn ensure_admin(store: &Store) -> Result<(), Box<dyn std::error::Error>> {
    let username = env::var("ADMIN_USERNAME").ok().map(|u| u.trim().to_string()).filter(|u| !u.is_empty()).unwrap_or_else(|| "admin".into());
    let Some(password) = env::var("ADMIN_PASSWORD").ok().filter(|p| !p.is_empty()) else {
        tracing::info!("ADMIN_PASSWORD kosong: admin awal tidak dibuat otomatis");
        return Ok(());
    };
    if password.chars().count() < 10 {
        return Err("ADMIN_PASSWORD minimal 10 karakter".into());
    }
    if store.find_user(&username).await?.is_some() {
        return Ok(());
    }
    let hash = auth::hash_password(&password)?;
    store.create_user(&username, &hash, "admin").await?;
    tracing::info!(username = %username, "admin awal dibuat dari ADMIN_USERNAME/ADMIN_PASSWORD");
    Ok(())
}

/// `pipa-api load-batas-desa <file.geojson>`: ganti seluruh batas desa dari GeoJSON BPS.
async fn load_batas_desa(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let [path] = args else {
        return Err("pemakaian: pipa-api load-batas-desa <file.geojson>".into());
    };
    let text = std::fs::read_to_string(path)?;
    let batas = pipa_sync::parse_batas_desa(&text)?;
    let rows: Vec<pipa_store::BatasInput> = batas
        .into_iter()
        .map(|b| pipa_store::BatasInput {
            kode_bps: b.kode_bps,
            kecamatan: b.kecamatan,
            desa: b.desa,
            geometry: b.geometry.to_string(),
        })
        .collect();
    let sumber = PathBuf::from(path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let store = connect_and_migrate().await?;
    let (jumlah, tanpa) = store.replace_batas_desa(&rows, &sumber).await?;
    println!("{jumlah} poligon desa dimuat dari {sumber}; desa aktif tanpa batas: {tanpa}");
    Ok(())
}

/// `pipa-api sync`: sinkron sekali dari Arumanis, lalu keluar.
async fn sync_once() -> Result<(), Box<dyn std::error::Error>> {
    let store = connect_and_migrate().await?;
    let syncer = sync::Syncer::new(store, sync::SyncConfig::from_env());
    let report = syncer.run("cli").await.map_err(|_| "sinkron lain sedang berjalan")?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if report.status == "gagal" {
        return Err("sinkron gagal".into());
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Memuat `.env` dari folder kerja. Variabel yang sudah di-set di shell tidak ditimpa.
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pipa_api=info,tower_http=info".into()),
        )
        .init();

    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("create-user") => return create_user(&args[1..]).await,
        Some("load-batas-desa") => return load_batas_desa(&args[1..]).await,
        Some("sync") => return sync_once().await,
        _ => {}
    }

    let database_url = env::var("DATABASE_URL").map_err(|_| "DATABASE_URL wajib di-set (lihat .env.example)")?;
    let port: u16 = env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080);
    let web_dist = PathBuf::from(env::var("WEB_DIST").unwrap_or_else(|_| "web/dist".into()));
    let cookie_secure = env::var("COOKIE_SECURE").map(|v| v == "true").unwrap_or(false);

    let store = Store::connect(&database_url).await?;
    store.migrate().await?;
    tracing::info!("migrasi selesai");
    ensure_admin(&store).await?;

    let closed = store.close_stale_sync_runs().await?;
    if closed > 0 {
        tracing::warn!(closed, "catatan sinkron yang terhenti ditandai gagal");
    }
    let syncer = sync::Syncer::new(store.clone(), sync::SyncConfig::from_env());
    syncer.spawn_schedule();

    let state = AppState { store, syncer, jwt_secret: jwt_secret()?, cookie_secure };
    let basemap_dir = PathBuf::from(env::var("BASEMAP_DIR").unwrap_or_else(|_| "data".into()));
    let app = router(state, web_dist, basemap_dir);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("pipa-api mendengarkan di {addr}");

    axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await?;
    Ok(())
}

/// Berhenti rapi saat Ctrl+C (SIGINT) atau SIGTERM. Docker dan Coolify mengirim SIGTERM saat redeploy;
/// tanpa ini proses baru dimatikan paksa setelah batas waktu berhenti habis.
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
