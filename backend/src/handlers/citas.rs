use std::collections::HashMap;

use axum::Json;
use axum::extract::{Path, Query, State};
use chrono::Duration;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::error::{ApiError, ApiResult};
use crate::handlers::disponibilidad::nombre_festivo;
use crate::handlers::{ADMIN_RECEP, STAFF};
use crate::models::{CitaDetalle, CitaFiltro, CitaInput, CitaRow, CitaServicioRow, EstadoInput};
use crate::state::AppState;

const ESTADOS: &[&str] = &["pendiente", "confirmada", "atendida", "no_show", "cancelada"];

#[derive(Debug, sqlx::FromRow)]
struct ServicioCita {
    id: Uuid,
    nombre: String,
    duracion_min: i32,
    precio_centavos: i32,
}

/// Cita con los datos de cliente y estilista. Parámetro: $1 = id de la cita.
const SQL_CITA: &str = "\
SELECT c.id, c.cliente_id, cl.nombre AS cliente_nombre, cl.telefono AS cliente_telefono,
       c.estilista_id, e.nombre AS estilista_nombre, c.inicio, c.fin, c.estado,
       c.notas, c.canal
FROM citas c
JOIN clientes cl ON cl.id = c.cliente_id
JOIN estilistas e ON e.id = c.estilista_id
WHERE c.id = $1";

/// Citas de un rango; $3 opcional filtra por estilista.
const SQL_CITAS_RANGO: &str = "\
SELECT c.id, c.cliente_id, cl.nombre AS cliente_nombre, cl.telefono AS cliente_telefono,
       c.estilista_id, e.nombre AS estilista_nombre, c.inicio, c.fin, c.estado,
       c.notas, c.canal
FROM citas c
JOIN clientes cl ON cl.id = c.cliente_id
JOIN estilistas e ON e.id = c.estilista_id
WHERE c.inicio >= $1 AND c.inicio < $2
  AND ($3::uuid IS NULL OR c.estilista_id = $3)
ORDER BY c.inicio";

fn agrupar(citas: Vec<CitaRow>, servicios: Vec<CitaServicioRow>) -> Vec<CitaDetalle> {
    let mut por_cita: HashMap<Uuid, Vec<CitaServicioRow>> = HashMap::new();
    for servicio in servicios {
        por_cita.entry(servicio.cita_id).or_default().push(servicio);
    }
    citas
        .into_iter()
        .map(|cita| {
            let servicios = por_cita.remove(&cita.id).unwrap_or_default();
            CitaDetalle { cita, servicios }
        })
        .collect()
}

pub async fn listar(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Query(filtro): Query<CitaFiltro>,
) -> ApiResult<Json<Vec<CitaDetalle>>> {
    usuario.require(STAFF)?;
    let desde = filtro.desde.unwrap_or_else(|| chrono::Utc::now() - Duration::days(1));
    let hasta = filtro.hasta.unwrap_or_else(|| desde + Duration::days(8));

    let citas: Vec<CitaRow> = sqlx::query_as(SQL_CITAS_RANGO)
        .bind(desde)
        .bind(hasta)
        .bind(filtro.estilista_id)
        .fetch_all(&state.pool)
        .await?;

    let ids: Vec<Uuid> = citas.iter().map(|c| c.id).collect();
    let servicios: Vec<CitaServicioRow> = if ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as(
            "SELECT cita_id, servicio_id, descripcion, duracion_min, precio_centavos
             FROM cita_servicios WHERE cita_id = ANY($1)",
        )
        .bind(&ids)
        .fetch_all(&state.pool)
        .await?
    };

    Ok(Json(agrupar(citas, servicios)))
}

pub async fn obtener(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<CitaDetalle>> {
    usuario.require(STAFF)?;
    let cita: CitaRow = sqlx::query_as(SQL_CITA)
        .bind(id)
    .fetch_one(&state.pool)
    .await?;
    let servicios: Vec<CitaServicioRow> = sqlx::query_as(
        "SELECT cita_id, servicio_id, descripcion, duracion_min, precio_centavos
         FROM cita_servicios WHERE cita_id = $1",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(CitaDetalle { cita, servicios }))
}

pub async fn crear(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Json(input): Json<CitaInput>,
) -> ApiResult<Json<CitaDetalle>> {
    usuario.require(ADMIN_RECEP)?;
    if input.servicio_ids.is_empty() {
        return Err(ApiError::bad_request("selecciona al menos un servicio"));
    }

    let servicios: Vec<ServicioCita> = sqlx::query_as(
        "SELECT id, nombre, duracion_min, precio_centavos
         FROM servicios WHERE id = ANY($1) AND activo",
    )
    .bind(&input.servicio_ids)
    .fetch_all(&state.pool)
    .await?;
    if servicios.len() != input.servicio_ids.len() {
        return Err(ApiError::bad_request(
            "algún servicio no existe o está inactivo",
        ));
    }

    let duracion: i32 = servicios.iter().map(|s| s.duracion_min).sum();
    let fin = input.inicio + Duration::minutes(duracion as i64);

    let fecha_local = input.inicio.with_timezone(&state.cfg.tz).date_naive();
    if let Some(nombre) = nombre_festivo(&state, fecha_local).await? {
        return Err(ApiError::conflict(format!(
            "el salón está cerrado por {nombre}"
        )));
    }

    let mut tx = state.pool.begin().await?;
    let (cita_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO citas (cliente_id, estilista_id, inicio, fin, estado, notas, canal, creado_por)
         VALUES ($1, $2, $3, $4, 'pendiente', $5, 'staff', $6)
         RETURNING id",
    )
    .bind(input.cliente_id)
    .bind(input.estilista_id)
    .bind(input.inicio)
    .bind(fin)
    .bind(input.notas.as_deref().unwrap_or(""))
    .bind(usuario.id)
    .fetch_one(&mut *tx)
    .await?;

    for servicio in &servicios {
        sqlx::query(
            "INSERT INTO cita_servicios (cita_id, servicio_id, descripcion, duracion_min, precio_centavos)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(cita_id)
        .bind(servicio.id)
        .bind(&servicio.nombre)
        .bind(servicio.duracion_min)
        .bind(servicio.precio_centavos)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query(
        "INSERT INTO auditoria (usuario_id, accion, entidad, entidad_id, datos)
         VALUES ($1, 'crear', 'cita', $2, $3)",
    )
    .bind(usuario.id)
    .bind(cita_id.to_string())
    .bind(json!({ "inicio": input.inicio, "estilista_id": input.estilista_id }))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let cita: CitaRow = sqlx::query_as(SQL_CITA)
        .bind(cita_id)
    .fetch_one(&state.pool)
    .await?;
    let servicios: Vec<CitaServicioRow> = sqlx::query_as(
        "SELECT cita_id, servicio_id, descripcion, duracion_min, precio_centavos
         FROM cita_servicios WHERE cita_id = $1",
    )
    .bind(cita_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(CitaDetalle { cita, servicios }))
}

#[derive(Debug, Deserialize)]
pub struct ReprogramarInput {
    pub inicio: chrono::DateTime<chrono::Utc>,
    pub estilista_id: Option<Uuid>,
    pub notas: Option<String>,
}

pub async fn reprogramar(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Path(id): Path<Uuid>,
    Json(input): Json<ReprogramarInput>,
) -> ApiResult<Json<Value>> {
    usuario.require(ADMIN_RECEP)?;
    let (duracion,): (i32,) = sqlx::query_as(
        "SELECT COALESCE(SUM(duracion_min), 0)::int FROM cita_servicios WHERE cita_id = $1",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await?;
    if duracion <= 0 {
        return Err(ApiError::not_found("la cita no tiene servicios"));
    }
    let fin = input.inicio + Duration::minutes(duracion as i64);
    let (inicio, fin): (chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>) =
        sqlx::query_as(
            "UPDATE citas SET inicio = $2, fin = $3,
                    estilista_id = COALESCE($4, estilista_id),
                    notas = COALESCE($5, notas),
                    actualizado_en = now()
             WHERE id = $1
             RETURNING inicio, fin",
        )
        .bind(id)
        .bind(input.inicio)
        .bind(fin)
        .bind(input.estilista_id)
        .bind(input.notas)
        .fetch_one(&state.pool)
        .await?;
    Ok(Json(json!({ "ok": true, "inicio": inicio, "fin": fin })))
}

pub async fn cambiar_estado(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Path(id): Path<Uuid>,
    Json(input): Json<EstadoInput>,
) -> ApiResult<Json<Value>> {
    usuario.require(STAFF)?;
    if !ESTADOS.contains(&input.estado.as_str()) {
        return Err(ApiError::bad_request("estado inválido"));
    }
    sqlx::query("UPDATE citas SET estado = $2, actualizado_en = now() WHERE id = $1")
        .bind(id)
        .bind(&input.estado)
        .execute(&state.pool)
        .await?;
    sqlx::query(
        "INSERT INTO auditoria (usuario_id, accion, entidad, entidad_id, datos)
         VALUES ($1, 'estado', 'cita', $2, $3)",
    )
    .bind(usuario.id)
    .bind(id.to_string())
    .bind(json!({ "estado": input.estado }))
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "ok": true })))
}
