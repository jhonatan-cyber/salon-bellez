use argon2::Argon2;
use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use axum::extract::FromRequestParts;
use axum::http::header::{AUTHORIZATION, COOKIE};
use axum::http::request::Parts;
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub const COOKIE_NAME: &str = "salon_session";
const SESSION_HOURS: i64 = 12;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub rol: String,
    pub nombre: String,
    pub exp: i64,
}

pub fn hash_password(password: &str) -> ApiResult<String> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(|e| ApiError::internal(format!("argon2: {e}")))
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

pub fn issue_token(state: &AppState, id: Uuid, rol: &str, nombre: &str) -> ApiResult<String> {
    let claims = Claims {
        sub: id,
        rol: rol.to_string(),
        nombre: nombre.to_string(),
        exp: (Utc::now() + Duration::hours(SESSION_HOURS)).timestamp(),
    };
    encode(
        &jsonwebtoken::Header::new(Algorithm::HS256),
        &claims,
        &state.encoding,
    )
    .map_err(|e| ApiError::internal(format!("jwt: {e}")))
}

pub fn session_cookie(token: String, secure: bool) -> axum_extra::extract::cookie::Cookie<'static> {
    axum_extra::extract::cookie::Cookie::build((COOKIE_NAME, token))
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .secure(secure)
        .path("/")
        .max_age(time::Duration::hours(SESSION_HOURS))
        .build()
}

pub fn clear_cookie() -> axum_extra::extract::cookie::Cookie<'static> {
    axum_extra::extract::cookie::Cookie::build((COOKIE_NAME, ""))
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .path("/")
        .max_age(time::Duration::ZERO)
        .build()
}

fn token_from_parts(parts: &Parts) -> Option<String> {
    if let Some(value) = parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        return Some(value.trim().to_string());
    }
    let cookies = parts.headers.get(COOKIE)?.to_str().ok()?;
    cookies.split(';').find_map(|pair| {
        let (name, value) = pair.trim().split_once('=')?;
        (name == COOKIE_NAME).then(|| value.trim().to_string())
    })
}

#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub id: Uuid,
    pub rol: String,
}

impl CurrentUser {
    pub fn require(&self, roles: &[&str]) -> ApiResult<()> {
        if roles.contains(&self.rol.as_str()) {
            Ok(())
        } else {
            Err(ApiError::Forbidden)
        }
    }
}

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = token_from_parts(parts).ok_or(ApiError::Unauthorized)?;
        let data = decode::<Claims>(
            &token,
            &state.decoding,
            &Validation::new(Algorithm::HS256),
        )?;
        Ok(CurrentUser {
            id: data.claims.sub,
            rol: data.claims.rol,

        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_y_verificacion_funcionan() {
        let hash = hash_password("secreto12345").unwrap();
        assert!(verify_password("secreto12345", &hash));
        assert!(!verify_password("otra-clave", &hash));
    }
}
