use axum::Json;
use axum::extract::State;
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::auth::{CurrentUser, clear_cookie, hash_password, issue_token, session_cookie, verify_password};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct LoginInput {
    pub email: String,
    pub password: String,
}

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(input): Json<LoginInput>,
) -> ApiResult<(CookieJar, Json<Value>)> {
    if !state
        .limiter
        .check(&format!("login:{}", input.email.to_lowercase()))
    {
        return Err(ApiError::bad_request(
            "demasiados intentos de acceso, espera un minuto",
        ));
    }

    let row: Option<(Uuid, String, String, String, bool)> = sqlx::query_as(
        "SELECT id, nombre, rol, password_hash, activo
         FROM usuarios WHERE lower(email) = lower($1)",
    )
    .bind(&input.email)
    .fetch_optional(&state.pool)
    .await?;

    let Some((id, nombre, rol, password_hash, activo)) = row else {
        return Err(ApiError::Unauthorized);
    };
    if !activo || !verify_password(&input.password, &password_hash) {
        return Err(ApiError::Unauthorized);
    }

    let token = issue_token(&state, id, &rol, &nombre)?;
    let jar = jar.add(session_cookie(token, state.cfg.is_production()));
    Ok((
        jar,
        Json(json!({ "usuario": { "id": id, "nombre": nombre, "rol": rol } })),
    ))
}

pub async fn logout(jar: CookieJar) -> (CookieJar, Json<Value>) {
    (jar.add(clear_cookie()), Json(json!({ "ok": true })))
}

pub async fn me(
    State(state): State<AppState>,
    usuario: CurrentUser,
) -> ApiResult<Json<Value>> {
    let row: Option<(Uuid, String, String, String, bool)> = sqlx::query_as(
        "SELECT id, email, nombre, rol, activo FROM usuarios WHERE id = $1",
    )
    .bind(usuario.id)
    .fetch_optional(&state.pool)
    .await?;
    let Some((id, email, nombre, rol, activo)) = row else {
        return Err(ApiError::Unauthorized);
    };
    Ok(Json(json!({
        "usuario": { "id": id, "email": email, "nombre": nombre, "rol": rol, "activo": activo }
    })))
}

#[derive(Debug, Deserialize)]
pub struct CambioPassword {
    pub actual: String,
    pub nueva: String,
}

pub async fn cambiar_password(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Json(input): Json<CambioPassword>,
) -> ApiResult<Json<Value>> {
    if input.nueva.len() < 8 {
        return Err(ApiError::bad_request(
            "la nueva contraseña debe tener al menos 8 caracteres",
        ));
    }
    let (hash,): (String,) = sqlx::query_as("SELECT password_hash FROM usuarios WHERE id = $1")
        .bind(usuario.id)
        .fetch_one(&state.pool)
        .await?;
    if !verify_password(&input.actual, &hash) {
        return Err(ApiError::bad_request("la contraseña actual no coincide"));
    }
    let new_hash = hash_password(&input.nueva)?;
    sqlx::query("UPDATE usuarios SET password_hash = $2, actualizado_en = now() WHERE id = $1")
        .bind(usuario.id)
        .bind(new_hash)
        .execute(&state.pool)
        .await?;
    Ok(Json(json!({ "ok": true })))
}
