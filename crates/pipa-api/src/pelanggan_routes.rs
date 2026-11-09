//! Endpoint pelanggan, dusun, cakupan layanan, dan riwayat koreksi Pokmas.
//! Setiap handler memakai `Diizinkan<...>` untuk izinnya. Matriks izinnya ada di pipa-core.

use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, put},
    Json, Router,
};
use pipa_core::{
    cakupan_json, hitung_cakupan, validasi_dusun, validasi_pelanggan, DusunMasukan, PelangganMasukan, StatusPelanggan,
};
use pipa_store::{CakupanDesa, DusunCakupan, DusunTulis, FilterPelanggan, PelangganTulis, SetDusunHasil};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::otorisasi::{CakupanBaca, DataKelola, Diizinkan, NamaPokmasBaca, PelangganBaca, PelangganKelola};
use crate::{bad_request, desa_tidak_ditemukan, parse_body, ApiError, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/pelanggan", get(daftar_pelanggan).post(tambah_pelanggan))
        .route("/api/v1/pelanggan/{id}", get(detail_pelanggan).put(ubah_pelanggan))
        .route("/api/v1/pelanggan/{id}/riwayat", get(riwayat_pelanggan))
        .route("/api/v1/sr", get(daftar_sr))
        .route("/api/v1/desa/{id}/dusun", get(daftar_dusun).post(tambah_dusun))
        .route("/api/v1/dusun/{id}", put(ubah_dusun))
        .route("/api/v1/desa/{id}/cakupan", get(cakupan_desa))
        .route("/api/v1/features/{id}/dusun", put(set_dusun_sr))
        .route("/api/v1/desa/{id}/pokmas/riwayat", get(riwayat_pokmas))
}

#[derive(Deserialize)]
struct DaftarParams {
    q: Option<String>,
    desa_id: Option<i64>,
    dusun_id: Option<i64>,
    status: Option<String>,
    limit: Option<i64>,
}

/// Daftar pelanggan dengan pencarian nama atau nomor meter, dan filter dusun, status, dan desa.
async fn daftar_pelanggan(
    State(state): State<AppState>,
    _: Diizinkan<PelangganBaca>,
    Query(p): Query<DaftarParams>,
) -> Result<Json<Value>, ApiError> {
    let status = match p.status.as_deref().filter(|s| !s.is_empty()) {
        None => None,
        Some(s) => Some(StatusPelanggan::parse(s).map_err(|m| bad_request(&m))?),
    };
    let filter =
        FilterPelanggan { q: p.q, desa_id: p.desa_id, dusun_id: p.dusun_id, status, limit: p.limit.unwrap_or(200) };
    let rows = state.store.list_pelanggan(&filter).await?;
    Ok(Json(json!({ "data": rows })))
}

#[derive(Deserialize)]
struct SrParams {
    desa_id: Option<i64>,
    dusun_id: Option<i64>,
    q: Option<String>,
}

/// Titik SR untuk dipilih saat menambah pelanggan.
async fn daftar_sr(
    State(state): State<AppState>,
    _: Diizinkan<PelangganBaca>,
    Query(p): Query<SrParams>,
) -> Result<Json<Value>, ApiError> {
    let rows = state.store.list_sr(p.desa_id, p.dusun_id, p.q.as_deref()).await?;
    Ok(Json(json!({ "data": rows })))
}

async fn detail_pelanggan(
    State(state): State<AppState>,
    _: Diizinkan<PelangganBaca>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let row = state.store.get_pelanggan(id).await?.ok_or_else(pelanggan_tidak_ditemukan)?;
    Ok(Json(json!({ "data": row })))
}

fn pelanggan_tidak_ditemukan() -> ApiError {
    ApiError(StatusCode::NOT_FOUND, "pelanggan tidak ditemukan".into())
}

/// Hasil tulis pelanggan ke respons. Penolakan dipetakan ke status yang sesuai.
fn respons_pelanggan(h: PelangganTulis, sukses: StatusCode) -> Result<Response, ApiError> {
    match h {
        PelangganTulis::Berhasil(row) => Ok((sukses, Json(json!({ "data": row }))).into_response()),
        PelangganTulis::TidakAda => Err(pelanggan_tidak_ditemukan()),
        PelangganTulis::SrTidakValid => {
            Err(ApiError(StatusCode::UNPROCESSABLE_ENTITY, "titik SR tidak ditemukan atau sudah dihapus".into()))
        }
        PelangganTulis::NomorMeterDipakai => {
            Err(ApiError(StatusCode::CONFLICT, "nomor meter sudah dipakai pelanggan lain".into()))
        }
    }
}

fn isian_pelanggan(body: &[u8]) -> Result<pipa_core::PelangganIsian, ApiError> {
    let m: PelangganMasukan = parse_body(body)?;
    validasi_pelanggan(&m).map_err(|m| bad_request(&m))
}

async fn tambah_pelanggan(
    State(state): State<AppState>,
    Diizinkan { user, .. }: Diizinkan<PelangganKelola>,
    body: Bytes,
) -> Result<Response, ApiError> {
    let isian = isian_pelanggan(&body)?;
    let hasil = state.store.tambah_pelanggan(&isian, &user.username).await?;
    if let PelangganTulis::Berhasil(row) = &hasil {
        tracing::info!(user = %user.username, pelanggan = row.id, "pelanggan ditambah");
    }
    respons_pelanggan(hasil, StatusCode::CREATED)
}

async fn ubah_pelanggan(
    State(state): State<AppState>,
    Diizinkan { user, .. }: Diizinkan<PelangganKelola>,
    Path(id): Path<i64>,
    body: Bytes,
) -> Result<Response, ApiError> {
    let isian = isian_pelanggan(&body)?;
    let hasil = state.store.ubah_pelanggan(id, &isian, &user.username).await?;
    if let PelangganTulis::Berhasil(_) = &hasil {
        tracing::info!(user = %user.username, pelanggan = id, "pelanggan diubah");
    }
    respons_pelanggan(hasil, StatusCode::OK)
}

async fn riwayat_pelanggan(
    State(state): State<AppState>,
    _: Diizinkan<PelangganBaca>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let riwayat = state.store.riwayat_pelanggan(id).await?.ok_or_else(pelanggan_tidak_ditemukan)?;
    Ok(Json(json!({ "data": riwayat })))
}

/// Dusun satu desa. Kepala desa dan peran lain yang punya `BacaCakupan` ikut melihat daftarnya.
async fn daftar_dusun(
    State(state): State<AppState>,
    _: Diizinkan<CakupanBaca>,
    Path(desa_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    if !state.store.desa_aktif(desa_id).await? {
        return Err(desa_tidak_ditemukan());
    }
    let rows = state.store.list_dusun(desa_id).await?;
    Ok(Json(json!({ "data": rows })))
}

fn respons_dusun(h: DusunTulis, sukses: StatusCode) -> Result<Response, ApiError> {
    match h {
        DusunTulis::Berhasil(row) => Ok((sukses, Json(json!({ "data": row }))).into_response()),
        DusunTulis::TidakAda => Err(ApiError(StatusCode::NOT_FOUND, "dusun tidak ditemukan".into())),
        DusunTulis::DesaTidakAda => Err(desa_tidak_ditemukan()),
        DusunTulis::NamaDipakai => Err(ApiError(StatusCode::CONFLICT, "nama dusun sudah ada di desa ini".into())),
    }
}

fn isian_dusun(body: &[u8]) -> Result<pipa_core::DusunIsian, ApiError> {
    let m: DusunMasukan = parse_body(body)?;
    validasi_dusun(&m).map_err(|m| bad_request(&m))
}

async fn tambah_dusun(
    State(state): State<AppState>,
    Diizinkan { user, .. }: Diizinkan<DataKelola>,
    Path(desa_id): Path<i64>,
    body: Bytes,
) -> Result<Response, ApiError> {
    let isian = isian_dusun(&body)?;
    let hasil = state.store.tambah_dusun(desa_id, &isian, &user.username).await?;
    respons_dusun(hasil, StatusCode::CREATED)
}

async fn ubah_dusun(
    State(state): State<AppState>,
    Diizinkan { user, .. }: Diizinkan<DataKelola>,
    Path(id): Path<i64>,
    body: Bytes,
) -> Result<Response, ApiError> {
    let isian = isian_dusun(&body)?;
    let hasil = state.store.ubah_dusun(id, &isian, &user.username).await?;
    respons_dusun(hasil, StatusCode::OK)
}

#[derive(Deserialize)]
struct SetDusunBody {
    /// None melepas SR dari dusun.
    dusun_id: Option<i64>,
}

async fn set_dusun_sr(
    State(state): State<AppState>,
    _: Diizinkan<PelangganKelola>,
    Path(feature_id): Path<i64>,
    body: Bytes,
) -> Result<Json<Value>, ApiError> {
    let b: SetDusunBody = parse_body(&body)?;
    match state.store.set_dusun_sr(feature_id, b.dusun_id).await? {
        SetDusunHasil::Berhasil => Ok(Json(json!({ "ok": true, "feature_id": feature_id, "dusun_id": b.dusun_id }))),
        SetDusunHasil::SrTidakAda => Err(ApiError(StatusCode::NOT_FOUND, "titik SR tidak ditemukan".into())),
        SetDusunHasil::DusunBedaDesa => Err(ApiError(
            StatusCode::UNPROCESSABLE_ENTITY,
            "dusun tidak ditemukan atau bukan dusun dari desa tempat SR berada".into(),
        )),
    }
}

/// Bentuk satu baris cakupan untuk API: angka mentah plus hasil hitung.
fn baris_cakupan(d: &DusunCakupan) -> Value {
    let c = hitung_cakupan(d.jumlah_sr, d.sr_terlayani, d.jumlah_rumah, d.jumlah_jiwa);
    json!({
        "dusun_id": d.dusun_id,
        "nama": d.nama,
        "jumlah_rumah": d.jumlah_rumah,
        "jumlah_jiwa": d.jumlah_jiwa,
        "cakupan": cakupan_json(&c),
    })
}

/// Ringkasan cakupan satu desa: per dusun, SR tanpa dusun, dan total desa.
/// Hanya angka agregat. Nama dan nomor meter pelanggan tidak ikut.
async fn cakupan_desa(
    State(state): State<AppState>,
    _: Diizinkan<CakupanBaca>,
    Path(desa_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    if !state.store.desa_aktif(desa_id).await? {
        return Err(desa_tidak_ditemukan());
    }
    let CakupanDesa { per_dusun, tanpa_dusun } = state.store.cakupan_desa(desa_id).await?;
    let semua: Vec<&DusunCakupan> = per_dusun.iter().chain(std::iter::once(&tanpa_dusun)).collect();
    let total = DusunCakupan {
        dusun_id: None,
        nama: "Total desa".into(),
        jumlah_rumah: per_dusun.iter().map(|d| d.jumlah_rumah).sum(),
        jumlah_jiwa: per_dusun.iter().map(|d| d.jumlah_jiwa).sum(),
        jumlah_sr: semua.iter().map(|d| d.jumlah_sr).sum(),
        sr_terlayani: semua.iter().map(|d| d.sr_terlayani).sum(),
    };
    Ok(Json(json!({
        "data": {
            "desa_id": desa_id,
            "per_dusun": per_dusun.iter().map(baris_cakupan).collect::<Vec<Value>>(),
            "tanpa_dusun": baris_cakupan(&tanpa_dusun),
            "total": baris_cakupan(&total),
        }
    })))
}

/// Riwayat koreksi Pokmas satu desa. Berisi nama pengurus, jadi hanya untuk operator dan admin.
async fn riwayat_pokmas(
    State(state): State<AppState>,
    _: Diizinkan<NamaPokmasBaca>,
    Path(desa_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let riwayat = state.store.riwayat_pokmas(desa_id).await?.ok_or_else(desa_tidak_ditemukan)?;
    Ok(Json(json!({ "data": riwayat })))
}
