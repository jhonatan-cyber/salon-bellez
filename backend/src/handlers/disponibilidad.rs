use axum::Json;
use axum::extract::{Query, State};
use chrono::{DateTime, Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::error::{ApiError, ApiResult};
use crate::handlers::STAFF;
use crate::models::{Servicio, Slot};
use crate::state::AppState;

pub const PASO_MIN: i64 = 15;

/// Convierte una hora local del salón a un instante UTC.
pub fn local_a_utc(tz: Tz, fecha: NaiveDate, hora: NaiveTime) -> Option<DateTime<Utc>> {
    tz.from_local_datetime(&fecha.and_time(hora))
        .earliest()
        .map(|dt| dt.with_timezone(&Utc))
}

pub fn inicio_del_dia(tz: Tz, fecha: NaiveDate) -> DateTime<Utc> {
    local_a_utc(tz, fecha, NaiveTime::MIN).unwrap_or_else(|| fecha.and_time(NaiveTime::MIN).and_utc())
}

pub fn fin_del_dia(tz: Tz, fecha: NaiveDate) -> DateTime<Utc> {
    inicio_del_dia(tz, fecha + Duration::days(1))
}

/// Nombre del festivo que cae en esa fecha, si el salón cierra ese día.
pub async fn nombre_festivo(state: &AppState, fecha: NaiveDate) -> ApiResult<Option<String>> {
    let festivo: Option<(String,)> =
        sqlx::query_as("SELECT nombre FROM festivos WHERE fecha = $1")
            .bind(fecha)
            .fetch_optional(&state.pool)
            .await?;
    Ok(festivo.map(|(nombre,)| nombre))
}

/// Genera huecos libres dentro de las franjas, evitando los intervalos ocupados.
pub fn generar_slots(
    franjas: &[(DateTime<Utc>, DateTime<Utc>)],
    duracion_min: i64,
    ocupado: &[(DateTime<Utc>, DateTime<Utc>)],
) -> Vec<Slot> {
    let duracion = Duration::minutes(duracion_min);
    let paso = Duration::minutes(PASO_MIN);
    let mut slots = Vec::new();

    for (inicio_franja, fin_franja) in franjas {
        let mut cursor = *inicio_franja;
        while cursor + duracion <= *fin_franja {
            let fin = cursor + duracion;
            let choca = ocupado
                .iter()
                .any(|(ocupado_ini, ocupado_fin)| cursor < *ocupado_fin && fin > *ocupado_ini);
            if !choca {
                slots.push(Slot {
                    inicio: cursor,
                    fin,
                });
            }
            cursor += paso;
        }
    }
    slots
}

#[derive(Debug, Deserialize)]
pub struct FiltroDisponibilidad {
    pub servicio_id: Uuid,
    pub estilista_id: Uuid,
    pub fecha: NaiveDate,
    /// Duración total cuando se reservan varios servicios en una sola cita.
    pub duracion_min: Option<i32>,
}

pub async fn calcular_disponibilidad(
    state: &AppState,
    servicio_id: Uuid,
    estilista_id: Uuid,
    fecha: NaiveDate,
    duracion_min: Option<i32>,
) -> ApiResult<(Servicio, Vec<Slot>)> {
    let servicio: Servicio = sqlx::query_as(
        "SELECT id, categoria_id, nombre, descripcion, duracion_min, buffer_min,
                precio_centavos, activo
         FROM servicios WHERE id = $1 AND activo",
    )
    .bind(servicio_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("servicio no encontrado"))?;

    if nombre_festivo(state, fecha).await?.is_some() {
        return Ok((servicio, Vec::new()));
    }

    let tz = state.cfg.tz;
    let dia_inicio = inicio_del_dia(tz, fecha);
    let dia_fin = fin_del_dia(tz, fecha);
    let dia_semana = chrono::Datelike::weekday(&fecha).num_days_from_sunday() as i16;

    let horarios: Vec<(NaiveTime, NaiveTime)> = sqlx::query_as(
        "SELECT hora_inicio, hora_fin FROM horarios_estilista
         WHERE estilista_id = $1 AND dia_semana = $2 ORDER BY hora_inicio",
    )
    .bind(estilista_id)
    .bind(dia_semana)
    .fetch_all(&state.pool)
    .await?;

    if horarios.is_empty() {
        return Ok((servicio, Vec::new()));
    }

    let mut franjas = Vec::with_capacity(horarios.len());
    for (inicio, fin) in horarios {
        if let (Some(i), Some(f)) = (local_a_utc(tz, fecha, inicio), local_a_utc(tz, fecha, fin)) {
            franjas.push((i, f));
        }
    }

    let citas: Vec<(DateTime<Utc>, DateTime<Utc>)> = sqlx::query_as(
        "SELECT inicio, fin FROM citas
         WHERE estilista_id = $1 AND estado <> 'cancelada'
           AND inicio < $3 AND fin > $2",
    )
    .bind(estilista_id)
    .bind(dia_inicio)
    .bind(dia_fin)
    .fetch_all(&state.pool)
    .await?;

    let bloqueos: Vec<(DateTime<Utc>, DateTime<Utc>)> = sqlx::query_as(
        "SELECT inicio, fin FROM bloqueos
         WHERE estilista_id = $1 AND inicio < $3 AND fin > $2",
    )
    .bind(estilista_id)
    .bind(dia_inicio)
    .bind(dia_fin)
    .fetch_all(&state.pool)
    .await?;

    let mut ocupado = citas;
    ocupado.extend(bloqueos);

    let duracion = duracion_min.unwrap_or(servicio.duracion_min).clamp(PASO_MIN as i32, 480);
    let slots = generar_slots(&franjas, duracion as i64, &ocupado);
    Ok((servicio, slots))
}

pub async fn disponibilidad(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Query(filtro): Query<FiltroDisponibilidad>,
) -> ApiResult<Json<Value>> {
    usuario.require(STAFF)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn utc(h: u32, m: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 17, h, m, 0).unwrap()
    }

    #[test]
    fn genera_huecos_sin_ocupacion() {
        let franjas = vec![(utc(9, 0), utc(11, 0))];
        let slots = generar_slots(&franjas, 60, &[]);
        // 09:00, 09:15, 09:30, 09:45 y 10:00 (10:00+60 = 11:00)
        assert_eq!(slots.len(), 5);
        assert_eq!(slots[0].inicio, utc(9, 0));
        assert_eq!(slots[0].fin, utc(10, 0));
    }

    #[test]
    fn evita_solapamiento_con_citas() {
        // La franja cierra a las 11:30 para que quepa una cita de 60 min a las 10:30.
        let franjas = vec![(utc(9, 0), utc(11, 30))];
        let ocupado = vec![(utc(9, 30), utc(10, 30))];
        let slots = generar_slots(&franjas, 60, &ocupado);
        assert!(slots.iter().all(|s| s.inicio >= utc(10, 30) || s.fin <= utc(9, 30)));
        assert!(slots.iter().any(|s| s.inicio == utc(10, 30)));
    }

    #[test]
    fn no_genera_slots_si_no_cabe() {
        let franjas = vec![(utc(9, 0), utc(9, 30))];
        let slots = generar_slots(&franjas, 45, &[]);
        assert!(slots.is_empty());
    }

    #[test]
    fn respeta_el_ultimo_tramo() {
        let franjas = vec![(utc(9, 0), utc(10, 0))];
        let slots = generar_slots(&franjas, 60, &[]);
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].fin, utc(10, 0));
    }
}
