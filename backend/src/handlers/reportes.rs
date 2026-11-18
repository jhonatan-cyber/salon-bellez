use axum::Json;
use axum::extract::{Query, State};
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::auth::CurrentUser;
use crate::error::ApiResult;
use crate::handlers::disponibilidad::{fin_del_dia, inicio_del_dia};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct Rango {
    pub desde: Option<NaiveDate>,
    pub hasta: Option<NaiveDate>,
}

fn rango(state: &AppState, filtro: &Rango) -> (chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>) {
    let hoy = chrono::Utc::now().with_timezone(&state.cfg.tz).date_naive();
    let desde = filtro
        .desde
        .unwrap_or_else(|| hoy - chrono::Duration::days(29));
    let hasta = filtro.hasta.unwrap_or(hoy);
    (
        inicio_del_dia(state.cfg.tz, desde),
        fin_del_dia(state.cfg.tz, hasta),
    )
}

pub async fn resumen(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Query(filtro): Query<Rango>,
) -> ApiResult<Json<Value>> {
    usuario.require(&["admin", "recepcionista"])?;
    let (desde, hasta) = rango(&state, &filtro);
    let (ingresos, tickets, propinas): (i64, i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(total_centavos), 0)::bigint,
                COUNT(*)::bigint,
                COALESCE(SUM(propina_centavos), 0)::bigint
         FROM ventas WHERE estado = 'pagada' AND pagada_en >= $1 AND pagada_en < $2",
    )
    .bind(desde)
    .bind(hasta)
    .fetch_one(&state.pool)
    .await?;

    let (citas, atendidas, no_show): (i64, i64, i64) = sqlx::query_as(
        "SELECT COUNT(*)::bigint,
                COUNT(*) FILTER (WHERE estado = 'atendida')::bigint,
                COUNT(*) FILTER (WHERE estado = 'no_show')::bigint
         FROM citas WHERE inicio >= $1 AND inicio < $2",
    )
    .bind(desde)
    .bind(hasta)
    .fetch_one(&state.pool)
    .await?;

    let ticket_promedio = if tickets > 0 { ingresos / tickets } else { 0 };
    let tasa_no_show = if citas > 0 {
        (no_show as f64 * 100.0 / citas as f64 * 10.0).round() / 10.0
    } else {
        0.0
    };

    Ok(Json(json!({
        "desde": desde,
        "hasta": hasta,
        "ingresos_centavos": ingresos,
        "propinas_centavos": propinas,
        "tickets": tickets,
        "ticket_promedio_centavos": ticket_promedio,
        "citas": citas,
        "citas_atendidas": atendidas,
        "citas_no_show": no_show,
        "tasa_no_show": tasa_no_show,
    })))
}

pub async fn ingresos_por_dia(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Query(filtro): Query<Rango>,
) -> ApiResult<Json<Vec<Value>>> {
    usuario.require(&["admin", "recepcionista"])?;
    let (desde, hasta) = rango(&state, &filtro);
    let filas: Vec<(NaiveDate, i64, i64)> = sqlx::query_as(
        "SELECT (pagada_en AT TIME ZONE $3)::date AS dia,
                COALESCE(SUM(total_centavos), 0)::bigint,
                COUNT(*)::bigint
         FROM ventas
         WHERE estado = 'pagada' AND pagada_en >= $1 AND pagada_en < $2
         GROUP BY dia ORDER BY dia",
    )
    .bind(desde)
    .bind(hasta)
    .bind(state.cfg.tz.to_string())
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        filas
            .into_iter()
            .map(|(dia, total, tickets)| {
                json!({ "dia": dia, "total_centavos": total, "tickets": tickets })
            })
            .collect(),
    ))
}

pub async fn por_servicio(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Query(filtro): Query<Rango>,
) -> ApiResult<Json<Vec<Value>>> {
    usuario.require(&["admin", "recepcionista"])?;
    let (desde, hasta) = rango(&state, &filtro);
    let filas: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT cs.descripcion,
                COUNT(*)::bigint,
                COALESCE(SUM(cs.precio_centavos), 0)::bigint
         FROM cita_servicios cs
         JOIN citas c ON c.id = cs.cita_id
         WHERE c.estado = 'atendida' AND c.inicio >= $1 AND c.inicio < $2
         GROUP BY cs.descripcion ORDER BY 3 DESC",
    )
    .bind(desde)
    .bind(hasta)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        filas
            .into_iter()
            .map(|(servicio, cantidad, total)| {
                json!({ "servicio": servicio, "cantidad": cantidad, "total_centavos": total })
            })
            .collect(),
    ))
}

pub async fn por_estilista(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Query(filtro): Query<Rango>,
) -> ApiResult<Json<Vec<Value>>> {
    usuario.require(&["admin", "recepcionista"])?;
    let (desde, hasta) = rango(&state, &filtro);
    let filas: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT e.nombre,
                COUNT(c.id)::bigint,
                COALESCE(SUM(v.total_centavos), 0)::bigint
         FROM estilistas e
         LEFT JOIN citas c ON c.estilista_id = e.id
              AND c.inicio >= $1 AND c.inicio < $2 AND c.estado = 'atendida'
         LEFT JOIN ventas v ON v.cita_id = c.id AND v.estado = 'pagada'
         GROUP BY e.nombre ORDER BY 3 DESC",
    )
    .bind(desde)
    .bind(hasta)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        filas
            .into_iter()
            .map(|(estilista, citas, total)| {
                json!({ "estilista": estilista, "citas": citas, "total_centavos": total })
            })
            .collect(),
    ))
}

pub async fn top_clientes(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Query(filtro): Query<Rango>,
) -> ApiResult<Json<Vec<Value>>> {
    usuario.require(&["admin", "recepcionista"])?;
    let (desde, hasta) = rango(&state, &filtro);
    let filas: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT cl.nombre, COUNT(v.id)::bigint, COALESCE(SUM(v.total_centavos), 0)::bigint
         FROM ventas v JOIN clientes cl ON cl.id = v.cliente_id
         WHERE v.estado = 'pagada' AND v.pagada_en >= $1 AND v.pagada_en < $2
         GROUP BY cl.nombre ORDER BY 3 DESC LIMIT 10",
    )
    .bind(desde)
    .bind(hasta)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        filas
            .into_iter()
            .map(|(cliente, visitas, total)| {
                json!({ "cliente": cliente, "visitas": visitas, "total_centavos": total })
            })
            .collect(),
    ))
}
