//! Server HTTP WebGIS perpipaan: API, autentikasi, dan penyajian frontend dari satu container.

mod auth;

use std::{
    convert::Infallible,
    env,
    io::{self, BufRead},
    net::SocketAddr,
    path::PathBuf,
};

use axum::{
    extract::{FromRequestParts, Path, Query, State},
    http::{header, request::Parts, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use pipa_core::{can_read_types, parse_types, Bbox, CoreError, FeatureType, Role};
use pipa_store::{Store, StoreError};
use serde::Deserialize;
use serde_json::{json, Value};
use tower_http::{compression::CompressionLayer, services::ServeDir, trace::TraceLayer};

use crate::auth::{clear_cookie_value, session_cookie, set_cookie_value, verify_token, CurrentUser};

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
