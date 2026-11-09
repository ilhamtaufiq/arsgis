//! Pemeriksaan izin per endpoint. Handler yang butuh izin menulis `Diizinkan<Syarat>` sebagai parameter,
//! sehingga peran diperiksa sebelum handler berjalan dan tidak bisa terlewat. Matriks izin ada di
//! `pipa_core::Role::punya`; file ini hanya menghubungkannya ke permintaan HTTP.

use std::marker::PhantomData;

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use pipa_core::Izin;

use crate::{
    auth::{session_cookie, verify_token, CurrentUser},
    ApiError, AppState,
};

/// Izin yang dibutuhkan satu endpoint. Diwujudkan sebagai tipe kosong supaya bisa dipakai di signature handler.
pub trait Syarat {
    const IZIN: Izin;
}

/// Pengguna yang sudah login dan punya izin `S`. Tanpa login: 401. Login tapi tanpa izin: 403.
pub struct Diizinkan<S: Syarat> {
    pub user: CurrentUser,
    _syarat: PhantomData<S>,
}

#[derive(Debug, PartialEq)]
pub enum Penolakan {
    TanpaSesi,
    TanpaIzin,
}

/// Inti pemeriksaan, dipisah dari extractor supaya bisa diuji tanpa membangun `AppState`.
pub fn periksa_izin(token: Option<&str>, secret: &[u8], izin: Izin) -> Result<CurrentUser, Penolakan> {
    let user = token.and_then(|t| verify_token(t, secret)).ok_or(Penolakan::TanpaSesi)?;
    if user.role.punya(izin) {
        Ok(user)
    } else {
        Err(Penolakan::TanpaIzin)
    }
}

impl<S: Syarat> FromRequestParts<AppState> for Diizinkan<S> {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        match periksa_izin(session_cookie(parts), &state.jwt_secret, S::IZIN) {
            Ok(user) => Ok(Self { user, _syarat: PhantomData }),
            Err(Penolakan::TanpaSesi) => Err(ApiError::unauthorized()),
            Err(Penolakan::TanpaIzin) => {
                Err(ApiError(StatusCode::FORBIDDEN, "peran ini tidak punya izin untuk tindakan ini".into()))
            }
        }
    }
}

macro_rules! syarat {
    ($($(#[$doc:meta])* $nama:ident => $izin:ident),* $(,)?) => {
        $(
            $(#[$doc])*
            pub struct $nama;
            impl Syarat for $nama {
                const IZIN: Izin = Izin::$izin;
            }
        )*
    };
}

syarat! {
    /// Membaca data pelanggan dan riwayatnya.
    PelangganBaca => BacaDataPelanggan,
    /// Mengubah pelanggan dan menghubungkan SR ke dusun.
    PelangganKelola => KelolaPelanggan,
    /// Mengubah aset, impor, koreksi Pokmas, dan dusun.
    DataKelola => KelolaData,
    /// Membaca nama pengurus Pokmas dan riwayat koreksinya.
    NamaPokmasBaca => BacaNamaPokmas,
    /// Membaca ringkasan cakupan per dusun dan desa.
    CakupanBaca => BacaCakupan,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::issue_token;
    use pipa_core::Role;

    const SECRET: &[u8] = b"uji-rahasia-yang-cukup-panjang-untuk-test";

    fn token(role: Role) -> String {
        issue_token("uji", role, SECRET).unwrap()
    }

    #[test]
    fn tanpa_sesi_ditolak_sebagai_tanpa_sesi() {
        assert_eq!(periksa_izin(None, SECRET, Izin::BacaCakupan), Err(Penolakan::TanpaSesi));
        assert_eq!(periksa_izin(Some("bukan-token"), SECRET, Izin::BacaCakupan), Err(Penolakan::TanpaSesi));
    }

    #[test]
    fn kepala_desa_dan_viewer_hanya_lolos_cakupan() {
        for role in [Role::KepalaDesa, Role::Viewer] {
            let t = token(role);
            assert!(periksa_izin(Some(&t), SECRET, Izin::BacaCakupan).is_ok(), "{role:?}");
            assert_eq!(periksa_izin(Some(&t), SECRET, Izin::BacaDataPelanggan), Err(Penolakan::TanpaIzin));
            assert_eq!(periksa_izin(Some(&t), SECRET, Izin::KelolaPelanggan), Err(Penolakan::TanpaIzin));
            assert_eq!(periksa_izin(Some(&t), SECRET, Izin::KelolaData), Err(Penolakan::TanpaIzin));
        }
    }

    #[test]
    fn bendahara_membaca_pelanggan_tapi_tidak_mengubah() {
        let t = token(Role::Bendahara);
        assert!(periksa_izin(Some(&t), SECRET, Izin::BacaDataPelanggan).is_ok());
        assert_eq!(periksa_izin(Some(&t), SECRET, Izin::KelolaPelanggan), Err(Penolakan::TanpaIzin));
        assert_eq!(periksa_izin(Some(&t), SECRET, Izin::KelolaData), Err(Penolakan::TanpaIzin));
        assert_eq!(periksa_izin(Some(&t), SECRET, Izin::BacaNamaPokmas), Err(Penolakan::TanpaIzin));
    }

    #[test]
    fn operator_dan_admin_sesuai_matriks() {
        let op = token(Role::Editor);
        assert!(periksa_izin(Some(&op), SECRET, Izin::KelolaPelanggan).is_ok());
        assert!(periksa_izin(Some(&op), SECRET, Izin::KelolaData).is_ok());
        assert_eq!(periksa_izin(Some(&op), SECRET, Izin::KelolaPengguna), Err(Penolakan::TanpaIzin));
        let admin = token(Role::Admin);
        assert!(periksa_izin(Some(&admin), SECRET, Izin::KelolaPengguna).is_ok());
    }

    /// Lewat extractor sungguhan, dengan `AppState` yang pool-nya malas. Penolakan terjadi sebelum handler
    /// (dan sebelum database) dipanggil, sehingga tes ini tidak butuh PostgreSQL.
    #[tokio::test]
    async fn extractor_menolak_sebelum_handler_dan_database() {
        use axum::extract::FromRequestParts;
        use axum::http::{header::COOKIE, Request, StatusCode};

        use crate::auth::COOKIE_NAME;
        use crate::sync::{SyncConfig, Syncer};

        let store = pipa_store::Store::connect_lazy("postgres://uji@127.0.0.1:1/uji").unwrap();
        let syncer = Syncer::new(
            store.clone(),
            SyncConfig { base_url: "http://127.0.0.1:1".into(), token: None, api_key: None, interval_menit: 0 },
        );
        let state = AppState { store, syncer, jwt_secret: SECRET.to_vec(), cookie_secure: false, foto_dir: std::env::temp_dir() };

        async fn kode<S: Syarat>(state: &AppState, token: Option<String>) -> Option<StatusCode>
        where
            Diizinkan<S>: FromRequestParts<AppState, Rejection = ApiError>,
        {
            let mut req = Request::builder();
            if let Some(t) = token {
                req = req.header(COOKIE, format!("{COOKIE_NAME}={t}"));
            }
            let (mut parts, _) = req.body(()).unwrap().into_parts();
            match Diizinkan::<S>::from_request_parts(&mut parts, state).await {
                Ok(_) => None,
                Err(ApiError(code, _)) => Some(code),
            }
        }

        // Tanpa sesi: 401 untuk semua izin.
        assert_eq!(kode::<PelangganBaca>(&state, None).await, Some(StatusCode::UNAUTHORIZED));
        // Kepala desa tidak boleh membaca pelanggan maupun mengubahnya.
        let kades = Some(token(Role::KepalaDesa));
        assert_eq!(kode::<PelangganBaca>(&state, kades.clone()).await, Some(StatusCode::FORBIDDEN));
        assert_eq!(kode::<CakupanBaca>(&state, kades).await, None);
        // Bendahara membaca pelanggan, tidak mengubah.
        let bendahara = Some(token(Role::Bendahara));
        assert_eq!(kode::<PelangganBaca>(&state, bendahara.clone()).await, None);
        assert_eq!(kode::<PelangganKelola>(&state, bendahara).await, Some(StatusCode::FORBIDDEN));
        // Operator boleh mengubah pelanggan dan dusun, tidak boleh mengelola pengguna (tidak ada extractor untuk itu).
        assert_eq!(kode::<PelangganKelola>(&state, Some(token(Role::Editor))).await, None);
        assert_eq!(kode::<DataKelola>(&state, Some(token(Role::Editor))).await, None);
    }

    #[test]
    fn token_dengan_rahasia_lain_ditolak() {
        let t = token(Role::Admin);
        assert_eq!(
            periksa_izin(Some(&t), b"rahasia-lain-yang-juga-cukup-panjang-sekali", Izin::BacaCakupan),
            Err(Penolakan::TanpaSesi)
        );
    }
}
