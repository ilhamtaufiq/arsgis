//! Server HTTP WebGIS perpipaan: API read-only fase 1 dan penyajian frontend dari satu container.

use std::{env, net::SocketAddr, path::PathBuf};

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use pipa_core::{parse_types, Bbox, CoreError};
use pipa_store::{Store, StoreError};
use serde::Deserialize;
use serde_json::{json, Value};
use tower_http::{compression::CompressionLayer, services::ServeDir, trace::TraceLayer};

#[derive(Clone)]
struct AppState {
    store: Store,
}

#[derive(Debug)]
struct ApiError(StatusCode, String);

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
            StoreError::TooManyFeatures => {
                ApiError(StatusCode::UNPROCESSABLE_ENTITY, e.to_string())
            }
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

async fn healthz(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    state.store.ping().await?;
    Ok(Json(json!({ "status": "ok" })))
}

async fn list_networks(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let networks = state.store.list_networks().await?;
    Ok(Json(json!({ "data": networks })))
}

async fn network_geojson(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(params): Query<GeoParams>,
) -> Result<Response, ApiError> {
    let bbox = Bbox::parse(&params.bbox)?;
    let types = parse_types(params.types.as_deref())?;
    let rows = state
        .store
        .features_in_bbox(id, bbox, types.as_deref())
        .await?;

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
                },
            })
        })
        .collect();

    let body = json!({ "type": "FeatureCollection", "features": features });
    let mut response = Json(body).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/geo+json"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    Ok(response)
}

fn router(state: AppState, web_dist: PathBuf) -> Router {
    let index = web_dist.join("index.html");
    let spa = ServeDir::new(&web_dist).fallback(tower_http::services::ServeFile::new(index));

    Router::new()
        .route("/healthz", get(healthz))
        .route("/api/v1/networks", get(list_networks))
        .route("/api/v1/networks/{id}/geojson", get(network_geojson))
        .fallback_service(spa)
        .layer(CompressionLayer::new().gzip(true).br(true))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pipa_api=info,tower_http=info".into()),
        )
        .init();

    let database_url = env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL wajib di-set (lihat .env.example)")?;
    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let web_dist = PathBuf::from(env::var("WEB_DIST").unwrap_or_else(|_| "web/dist".into()));

    let store = Store::connect(&database_url).await?;
    store.migrate().await?;
    tracing::info!("migrasi selesai");

    let app = router(AppState { store }, web_dist);
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
