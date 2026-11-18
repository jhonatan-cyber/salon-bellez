use axum::Json;
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use chrono::{Duration, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::disponibilidad::{FiltroDisponibilidad, calcular_disponibilidad, nombre_festivo};
use crate::models::{Categoria, Estilista, ReservaInput, Servicio};
use crate::state::AppState;

pub async fn servicios(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let servicios: Vec<Servicio> = sqlx::query_as(
        "SELECT id, categoria_id, nombre, descripcion, duracion_min, buffer_min,
                precio_centavos, activo
         FROM servicios WHERE activo ORDER BY nombre",
    )
    .fetch_all(&state.pool)
    .await?;
    let categorias: Vec<Categoria> =
        sqlx::query_as("SELECT id, nombre, orden FROM categorias_servicio ORDER BY orden, nombre")
            .fetch_all(&state.pool)
            .await?;
    Ok(Json(json!({ "servicios": servicios, "categorias": categorias })))
}

pub async fn estilistas(State(state): State<AppState>) -> ApiResult<Json<Vec<Estilista>>> {
    let estilistas: Vec<Estilista> = sqlx::query_as(
        "SELECT id, usuario_id, nombre, telefono, color, activo
         FROM estilistas WHERE activo ORDER BY nombre",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(estilistas))
}

pub async fn disponibilidad(
    State(state): State<AppState>,
    Query(filtro): Query<FiltroDisponibilidad>,
) -> ApiResult<Json<Value>> {
    let (servicio, slots) = calcular_disponibilidad(
        &state,
        filtro.servicio_id,
        filtro.estilista_id,
        filtro.fecha,
        filtro.duracion_min,
    )
    .await?;
    Ok(Json(json!({ "servicio": servicio, "slots": slots })))
}

fn client_key(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|v| v.trim().to_string())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|v| v.to_string())
        })
        .unwrap_or_else(|| "local".to_string())
}

pub async fn reservar(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<ReservaInput>,
) -> ApiResult<Json<Value>> {
    if !input.website.trim().is_empty() {
        // Bot: respondemos como si todo hubiera ido bien.
        return Ok(Json(json!({ "ok": true })));
    }
    if !state.limiter.check(&format!("reserva:{}", client_key(&headers))) {
        return Err(ApiError::bad_request(
            "demasiadas reservas desde esta conexión, intenta en un minuto",
        ));
    }
    if input.nombre.trim().len() < 2 {
        return Err(ApiError::bad_request("escribe tu nombre"));
    }
    if input.telefono.as_deref().unwrap_or("").trim().is_empty()
        && input.email.as_deref().unwrap_or("").trim().is_empty()
    {
        return Err(ApiError::bad_request("deja un teléfono o un correo"));
    }
    if input.inicio < Utc::now() {
        return Err(ApiError::bad_request("elige un horario futuro"));
    }

    let servicio: Option<(Uuid, String, i32, i32)> = sqlx::query_as(
        "SELECT id, nombre, duracion_min, precio_centavos
         FROM servicios WHERE id = $1 AND activo",
    )
    .bind(input.servicio_id)
    .fetch_optional(&state.pool)
    .await?;
    let Some((servicio_id, descripcion, duracion_min, precio_centavos)) = servicio else {
        return Err(ApiError::not_found("servicio no disponible"));
    };

    let fecha_local = input.inicio.with_timezone(&state.cfg.tz).date_naive();
    if let Some(nombre) = nombre_festivo(&state, fecha_local).await? {
        return Err(ApiError::conflict(format!("ese día no abrimos ({nombre})")));
    }

    let fin = input.inicio + Duration::minutes(duracion_min as i64);

    // Cliente: se busca por teléfono y si no, por correo.
    let cliente: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM clientes
         WHERE ($1::text IS NOT NULL AND telefono = $1)
            OR ($2::text IS NOT NULL AND lower(coalesce(email, '')) = lower($2))
         LIMIT 1",
    )
    .bind(input.telefono.as_deref().map(str::trim))
    .bind(input.email.as_deref().map(str::trim))
    .fetch_optional(&state.pool)
    .await?;

    let cliente_id = match cliente {
        Some((id,)) => id,
        None => {
            let (id,): (Uuid,) = sqlx::query_as(
                "INSERT INTO clientes (nombre, telefono, email, acepta_marketing)
                 VALUES ($1, $2, $3, $4) RETURNING id",
            )
            .bind(input.nombre.trim())
            .bind(input.telefono.as_deref().map(str::trim))
            .bind(input.email.as_deref().map(str::trim))
            .bind(input.acepta_marketing)
            .fetch_one(&state.pool)
            .await?;
            id
        }
    };

    let mut tx = state.pool.begin().await?;
    let (cita_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO citas (cliente_id, estilista_id, inicio, fin, estado, canal)
         VALUES ($1, $2, $3, $4, 'pendiente', 'online')
         RETURNING id",
    )
    .bind(cliente_id)
    .bind(input.estilista_id)
    .bind(input.inicio)
    .bind(fin)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO cita_servicios (cita_id, servicio_id, descripcion, duracion_min, precio_centavos)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(cita_id)
    .bind(servicio_id)
    .bind(&descripcion)
    .bind(duracion_min)
    .bind(precio_centavos)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(json!({
        "ok": true,
        "cita_id": cita_id,
        "inicio": input.inicio,
        "fin": fin,
        "servicio": descripcion,
        "estado": "pendiente",
    })))
}
