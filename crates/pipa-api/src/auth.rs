//! Autentikasi: hash password argon2, token JWT dalam cookie HttpOnly, dan peran pengguna.

use std::time::{SystemTime, UNIX_EPOCH};

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::http::{header, request::Parts};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use pipa_core::Role;
use serde::{Deserialize, Serialize};

pub const COOKIE_NAME: &str = "arsgis_session";
/// Masa berlaku sesi: 8 jam.
pub const SESSION_TTL_SECS: u64 = 8 * 60 * 60;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub role: String,
    pub exp: u64,
}

/// Pengguna yang sudah terautentikasi dari cookie sesi.
#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub username: String,
    pub role: Role,
}

pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok(),
        Err(_) => false,
    }
}

pub fn issue_token(username: &str, role: Role, secret: &[u8]) -> Result<String, jsonwebtoken::errors::Error> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let claims = Claims {
        sub: username.to_string(),
        role: role.as_str().to_string(),
        exp: now + SESSION_TTL_SECS,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret))
}

pub fn verify_token(token: &str, secret: &[u8]) -> Option<CurrentUser> {
    let data = decode::<Claims>(token, &DecodingKey::from_secret(secret), &Validation::default()).ok()?;
    let role = Role::parse(&data.claims.role).ok()?;
    Some(CurrentUser { username: data.claims.sub, role })
}

/// Ambil nilai cookie sesi dari header `Cookie`.
pub fn session_cookie(parts: &Parts) -> Option<&str> {
    parts
        .headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .map(str::trim)
        .find_map(|pair| pair.strip_prefix(COOKIE_NAME).and_then(|rest| rest.strip_prefix('=')))
}

/// Header `Set-Cookie` untuk login. `secure` true di produksi (HTTPS).
pub fn set_cookie_value(token: &str, secure: bool) -> String {
    let flag = if secure { "; Secure" } else { "" };
    format!("{COOKIE_NAME}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={SESSION_TTL_SECS}{flag}")
}

/// Header `Set-Cookie` untuk logout: kosongkan dan kedaluwarsakan.
pub fn clear_cookie_value(secure: bool) -> String {
    let flag = if secure { "; Secure" } else { "" };
    format!("{COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{flag}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &[u8] = b"uji-rahasia-yang-cukup-panjang-untuk-test";

    #[test]
    fn password_roundtrip() {
        let h = hash_password("rahasia123").unwrap();
        assert!(verify_password("rahasia123", &h));
        assert!(!verify_password("salah", &h));
        assert!(!verify_password("rahasia123", "bukan-hash"));
    }

    #[test]
    fn token_roundtrip_and_tamper() {
        let t = issue_token("budi", Role::Editor, SECRET).unwrap();
        let u = verify_token(&t, SECRET).unwrap();
        assert_eq!(u.username, "budi");
        assert_eq!(u.role, Role::Editor);
        assert!(verify_token(&t, b"secret-lain-yang-juga-panjang-sekali-ya").is_none());
        assert!(verify_token("abc.def.ghi", SECRET).is_none());
    }

    #[test]
    fn cookie_flags() {
        let c = set_cookie_value("t", true);
        assert!(c.contains("HttpOnly") && c.contains("SameSite=Lax") && c.contains("Secure"));
        assert!(!set_cookie_value("t", false).contains("Secure"));
        assert!(clear_cookie_value(false).contains("Max-Age=0"));
    }
}
