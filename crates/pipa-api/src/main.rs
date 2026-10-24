//! Server HTTP WebGIS perpipaan: API, autentikasi, dan penyajian frontend dari satu container.

mod auth;

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
use pipa_core::{can_read_types, parse_types, Bbox, CoreError, FeatureType, Role};
use pipa_store::{CommitOutcome, Store, StoreError, StagingRow};
use serde::Deserialize;
use serde_json::{json, Value};
use tower_http::{compression::CompressionLayer, services::ServeDir, trace::TraceLayer};

use crate::auth::{clear_cookie_value, session_cookie, set_cookie_value, verify_token, CurrentUser};

/// Batas ukuran berkas impor. KML dengan gambar tertanam bisa besar.
const MAX_IMPORT_BYTES: usize = 20 * 1024 * 1024;

#[derive(Clone)]
struct AppState {
    store: Store,
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

    let rows = state.store.features_in_bbox(id, bbox, Some(&types)).await?;
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
        if user.role < Role::Editor {
            return Err(ApiError(StatusCode::FORBIDDEN, "impor butuh peran editor atau admin".into()));
        }
        Ok(Editor(user))
    }
}

#[derive(Deserialize)]
struct UploadParams {
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
    let xml = std::str::from_utf8(&body)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "berkas bukan teks UTF-8".into()))?;
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
    let id = state.store.create_batch(&nama_file, &laporan, &user.username, &rows).await?;

    Ok((StatusCode::CREATED, Json(json!({ "id": id, "status": "staged", "nama_file": nama_file, "laporan": laporan }))).into_response())
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
        CommitOutcome::Committed(n) => Ok(Json(json!({ "id": id, "status": "committed", "fitur_ditulis": n }))),
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
    let customer = user.as_ref().is_some_and(|u| u.role >= Role::Editor);
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

async fn admin_list_imports(State(state): State<AppState>, _: Admin) -> Result<Json<Value>, ApiError> {
    let imports = state.store.list_imports(100).await?;
    Ok(Json(json!({ "data": imports })))
}

fn router(state: AppState, web_dist: PathBuf, basemap_dir: PathBuf) -> Router {
    let index = web_dist.join("index.html");
    let spa = ServeDir::new(&web_dist).fallback(tower_http::services::ServeFile::new(index));

    Router::new()
        .route("/healthz", get(healthz))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/me", get(me))
        .route("/api/v1/networks", get(list_networks))
        .route("/api/v1/networks/{id}/geojson", get(network_geojson))
        .route(
            "/api/v1/imports",
            post(upload_import).layer(DefaultBodyLimit::max(MAX_IMPORT_BYTES)),
        )
        .route("/api/v1/networks/{id}/dashboard", get(dashboard))
        .route("/api/v1/admin/users", get(admin_list_users).post(admin_create_user))
        .route("/api/v1/admin/users/{id}", axum::routing::patch(admin_update_user))
        .route("/api/v1/admin/networks", get(admin_list_networks).post(admin_create_network))
        .route("/api/v1/admin/imports", get(admin_list_imports))
        .route("/api/v1/imports/{id}", get(get_import))
        .route("/api/v1/imports/{id}/commit", post(commit_import))
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
    if args.first().map(String::as_str) == Some("create-user") {
        return create_user(&args[1..]).await;
    }

    let database_url = env::var("DATABASE_URL").map_err(|_| "DATABASE_URL wajib di-set (lihat .env.example)")?;
    let port: u16 = env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080);
    let web_dist = PathBuf::from(env::var("WEB_DIST").unwrap_or_else(|_| "web/dist".into()));
    let cookie_secure = env::var("COOKIE_SECURE").map(|v| v == "true").unwrap_or(false);

    let store = Store::connect(&database_url).await?;
    store.migrate().await?;
    tracing::info!("migrasi selesai");

    let state = AppState { store, jwt_secret: jwt_secret()?, cookie_secure };
    let basemap_dir = PathBuf::from(env::var("BASEMAP_DIR").unwrap_or_else(|_| "data".into()));
    let app = router(state, web_dist, basemap_dir);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("pipa-api mendengarkan di {addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
