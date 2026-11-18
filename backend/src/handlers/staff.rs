use axum::Json;
use axum::extract::{Path, State};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::auth::{CurrentUser, hash_password};
use crate::error::{ApiError, ApiResult};
use crate::handlers::{ADMIN_RECEP, STAFF};
use crate::models::{
    Bloqueo, BloqueoInput, Estilista, EstilistaInput, Festivo, FestivoInput, Horario, HorarioInput,
    Usuario,
};
use crate::state::AppState;

// ---------- Usuarios (solo admin) ----------

pub async fn listar_usuarios(
    State(state): State<AppState>,
    usuario: CurrentUser,
) -> ApiResult<Json<Vec<Usuario>>> {
    usuario.require(&["admin"])?;
    let usuarios: Vec<Usuario> = sqlx::query_as(
        "SELECT id, email, nombre, rol, activo, creado_en FROM usuarios ORDER BY nombre",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(usuarios))
}

#[derive(Debug, Deserialize)]
pub struct UsuarioInput {
    pub email: String,
    pub nombre: String,
    pub rol: String,
    pub password: String,
}

pub async fn crear_usuario(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Json(input): Json<UsuarioInput>,
) -> ApiResult<Json<Usuario>> {
    usuario.require(&["admin"])?;
    if input.password.len() < 8 {
        return Err(ApiError::bad_request(
            "la contraseña debe tener al menos 8 caracteres",
        ));
    }
    if !["admin", "recepcionista", "estilista"].contains(&input.rol.as_str()) {
        return Err(ApiError::bad_request("rol inválido"));
    }
    let hash = hash_password(&input.password)?;
    let creado: Usuario = sqlx::query_as(
        "INSERT INTO usuarios (email, nombre, rol, password_hash)
         VALUES ($1, $2, $3, $4)
         RETURNING id, email, nombre, rol, activo, creado_en",
    )
    .bind(input.email.trim())
    .bind(input.nombre.trim())
    .bind(&input.rol)
    .bind(hash)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(creado))
}

#[derive(Debug, Deserialize)]
pub struct UsuarioEstado {
    pub activo: bool,
}

pub async fn cambiar_estado_usuario(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UsuarioEstado>,
) -> ApiResult<Json<Value>> {
    usuario.require(&["admin"])?;
    if id == usuario.id && !input.activo {
        return Err(ApiError::bad_request("no puedes desactivarte a ti misma"));
    }
    sqlx::query("UPDATE usuarios SET activo = $2, actualizado_en = now() WHERE id = $1")
        .bind(id)
        .bind(input.activo)
        .execute(&state.pool)
        .await?;
    Ok(Json(json!({ "ok": true })))
}

// ---------- Estilistas ----------

pub async fn listar_estilistas(
    State(state): State<AppState>,
    usuario: CurrentUser,
) -> ApiResult<Json<Vec<Estilista>>> {
    usuario.require(STAFF)?;
    let estilistas: Vec<Estilista> = sqlx::query_as(
        "SELECT id, usuario_id, nombre, telefono, color, activo
         FROM estilistas ORDER BY activo DESC, nombre",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(estilistas))
}

pub async fn crear_estilista(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Json(input): Json<EstilistaInput>,
) -> ApiResult<Json<Estilista>> {
    usuario.require(ADMIN_RECEP)?;
    let estilista: Estilista = sqlx::query_as(
        "INSERT INTO estilistas (usuario_id, nombre, telefono, color, activo)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, usuario_id, nombre, telefono, color, activo",
    )
    .bind(input.usuario_id)
    .bind(input.nombre.trim())
    .bind(input.telefono)
    .bind(input.color)
    .bind(input.activo)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(estilista))
}

pub async fn actualizar_estilista(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Path(id): Path<Uuid>,
    Json(input): Json<EstilistaInput>,
) -> ApiResult<Json<Estilista>> {
    usuario.require(ADMIN_RECEP)?;
    let estilista: Estilista = sqlx::query_as(
        "UPDATE estilistas SET usuario_id = $2, nombre = $3, telefono = $4, color = $5, activo = $6
         WHERE id = $1
         RETURNING id, usuario_id, nombre, telefono, color, activo",
    )
    .bind(id)
    .bind(input.usuario_id)
    .bind(input.nombre.trim())
    .bind(input.telefono)
    .bind(input.color)
    .bind(input.activo)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(estilista))
}

// ---------- Horarios ----------

pub async fn listar_horarios(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Path(estilista_id): Path<Uuid>,
) -> ApiResult<Json<Vec<Horario>>> {
    usuario.require(STAFF)?;
    let horarios: Vec<Horario> = sqlx::query_as(
        "SELECT id, estilista_id, dia_semana, hora_inicio, hora_fin
         FROM horarios_estilista WHERE estilista_id = $1
         ORDER BY dia_semana, hora_inicio",
    )
    .bind(estilista_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(horarios))
}

pub async fn crear_horario(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Path(estilista_id): Path<Uuid>,
    Json(input): Json<HorarioInput>,
) -> ApiResult<Json<Horario>> {
    usuario.require(ADMIN_RECEP)?;
    if !(0..=6).contains(&input.dia_semana) || input.hora_fin <= input.hora_inicio {
        return Err(ApiError::bad_request("horario inválido"));
    }
    let horario: Horario = sqlx::query_as(
        "INSERT INTO horarios_estilista (estilista_id, dia_semana, hora_inicio, hora_fin)
         VALUES ($1, $2, $3, $4)
         RETURNING id, estilista_id, dia_semana, hora_inicio, hora_fin",
    )
    .bind(estilista_id)
    .bind(input.dia_semana)
    .bind(input.hora_inicio)
    .bind(input.hora_fin)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(horario))
}

pub async fn borrar_horario(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    usuario.require(ADMIN_RECEP)?;
    sqlx::query("DELETE FROM horarios_estilista WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;
    Ok(Json(json!({ "ok": true })))
}

// ---------- Bloqueos ----------

pub async fn listar_bloqueos(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Path(estilista_id): Path<Uuid>,
) -> ApiResult<Json<Vec<Bloqueo>>> {
    usuario.require(STAFF)?;
    let bloqueos: Vec<Bloqueo> = sqlx::query_as(
        "SELECT id, estilista_id, inicio, fin, motivo FROM bloqueos
         WHERE estilista_id = $1 AND fin >= now() ORDER BY inicio",
    )
    .bind(estilista_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(bloqueos))
}

pub async fn crear_bloqueo(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Path(estilista_id): Path<Uuid>,
    Json(input): Json<BloqueoInput>,
) -> ApiResult<Json<Bloqueo>> {
    usuario.require(ADMIN_RECEP)?;
    if input.fin <= input.inicio {
        return Err(ApiError::bad_request("el fin debe ser posterior al inicio"));
    }
    let bloqueo: Bloqueo = sqlx::query_as(
        "INSERT INTO bloqueos (estilista_id, inicio, fin, motivo) VALUES ($1, $2, $3, $4)
         RETURNING id, estilista_id, inicio, fin, motivo",
    )
    .bind(estilista_id)
    .bind(input.inicio)
    .bind(input.fin)
    .bind(input.motivo)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(bloqueo))
}

pub async fn borrar_bloqueo(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    usuario.require(ADMIN_RECEP)?;
    sqlx::query("DELETE FROM bloqueos WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;
    Ok(Json(json!({ "ok": true })))
}

// ---------- Festivos ----------

pub async fn listar_festivos(
    State(state): State<AppState>,
    usuario: CurrentUser,
) -> ApiResult<Json<Vec<Festivo>>> {
    usuario.require(STAFF)?;
    let festivos: Vec<Festivo> =
        sqlx::query_as("SELECT id, fecha, nombre FROM festivos ORDER BY fecha")
            .fetch_all(&state.pool)
            .await?;
    Ok(Json(festivos))
}

pub async fn crear_festivo(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Json(input): Json<FestivoInput>,
) -> ApiResult<Json<Festivo>> {
    usuario.require(ADMIN_RECEP)?;
    let festivo: Festivo = sqlx::query_as(
        "INSERT INTO festivos (fecha, nombre) VALUES ($1, $2) RETURNING id, fecha, nombre",
    )
    .bind(input.fecha)
    .bind(input.nombre.trim())
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(festivo))
}

pub async fn borrar_festivo(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    usuario.require(ADMIN_RECEP)?;
    sqlx::query("DELETE FROM festivos WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;
    Ok(Json(json!({ "ok": true })))
}
