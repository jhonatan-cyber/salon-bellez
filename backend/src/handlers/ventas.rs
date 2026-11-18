use axum::Json;
use axum::extract::{Path, Query, State};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::error::{ApiError, ApiResult};
use crate::handlers::ADMIN_RECEP;
use crate::handlers::disponibilidad::{fin_del_dia, inicio_del_dia};
use crate::models::{
    CierreCaja, CierreInput, Pago, Venta, VentaDetalle, VentaInput, VentaLinea,
};
use crate::state::AppState;

type LineaNueva = (String, Option<Uuid>, Option<Uuid>, String, i32, i32);

#[derive(Debug, Deserialize)]
pub struct VentaFiltro {
    pub desde: Option<NaiveDate>,
    pub hasta: Option<NaiveDate>,
}

pub async fn listar(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Query(filtro): Query<VentaFiltro>,
) -> ApiResult<Json<Vec<Venta>>> {
    usuario.require(ADMIN_RECEP)?;
    let hoy = Utc::now().with_timezone(&state.cfg.tz).date_naive();
    let desde = inicio_del_dia(state.cfg.tz, filtro.desde.unwrap_or(hoy));
    let hasta = fin_del_dia(state.cfg.tz, filtro.hasta.unwrap_or(hoy));
    let ventas: Vec<Venta> = sqlx::query_as(
        "SELECT id, cita_id, cliente_id, subtotal_centavos, descuento_centavos,
                propina_centavos, total_centavos, estado, creada_en, pagada_en
         FROM ventas WHERE creada_en >= $1 AND creada_en < $2
         ORDER BY creada_en DESC",
    )
    .bind(desde)
    .bind(hasta)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(ventas))
}

pub async fn obtener(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<VentaDetalle>> {
    usuario.require(ADMIN_RECEP)?;
    let venta: Venta = sqlx::query_as(
        "SELECT id, cita_id, cliente_id, subtotal_centavos, descuento_centavos,
                propina_centavos, total_centavos, estado, creada_en, pagada_en
         FROM ventas WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await?;
    let lineas: Vec<VentaLinea> = sqlx::query_as(
        "SELECT id, tipo, descripcion, cantidad, precio_unitario_centavos, total_centavos
         FROM venta_lineas WHERE venta_id = $1 ORDER BY descripcion",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;
    let pagos: Vec<Pago> = sqlx::query_as(
        "SELECT id, metodo, monto_centavos, referencia FROM pagos WHERE venta_id = $1",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(VentaDetalle {
        venta,
        lineas,
        pagos,
    }))
}

pub async fn crear(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Json(input): Json<VentaInput>,
) -> ApiResult<Json<VentaDetalle>> {
    usuario.require(ADMIN_RECEP)?;

    // Líneas: si viene de una cita se toman sus servicios.
    let mut lineas: Vec<LineaNueva> = Vec::new();
    let mut cliente_id: Option<Uuid> = None;

    if let Some(cita_id) = input.cita_id {
        let cita: Option<(Uuid,)> =
            sqlx::query_as("SELECT cliente_id FROM citas WHERE id = $1")
                .bind(cita_id)
                .fetch_optional(&state.pool)
                .await?;
        let Some((cid,)) = cita else {
            return Err(ApiError::not_found("cita no encontrada"));
        };
        cliente_id = Some(cid);
        let servicios: Vec<(Uuid, String, i32, i32)> = sqlx::query_as(
            "SELECT servicio_id, descripcion, 1::int, precio_centavos
             FROM cita_servicios WHERE cita_id = $1",
        )
        .bind(cita_id)
        .fetch_all(&state.pool)
        .await?;
        for (servicio_id, descripcion, cantidad, precio) in servicios {
            lineas.push((
                "servicio".into(),
                Some(servicio_id),
                None,
                descripcion,
                cantidad,
                precio,
            ));
        }
    }

    for linea in &input.lineas {
        if linea.tipo != "servicio" && linea.tipo != "producto" {
            return Err(ApiError::bad_request("tipo de línea inválido"));
        }
        if linea.cantidad <= 0 {
            return Err(ApiError::bad_request("la cantidad debe ser positiva"));
        }
        lineas.push((
            linea.tipo.clone(),
            linea.servicio_id,
            linea.producto_id,
            linea.descripcion.clone(),
            linea.cantidad,
            linea.precio_unitario_centavos,
        ));
    }

    if lineas.is_empty() {
        return Err(ApiError::bad_request("la venta no tiene conceptos"));
    }

    let subtotal: i32 = lineas.iter().map(|(_, _, _, _, c, p)| c * p).sum();
    let total = subtotal - input.descuento_centavos + input.propina_centavos;
    if total < 0 {
        return Err(ApiError::bad_request("el total no puede ser negativo"));
    }
    let pagado: i32 = input.pagos.iter().filter(|p| p.metodo != "otro").map(|p| p.monto_centavos).sum();
    if !input.pagos.is_empty() && pagado != total {
        return Err(ApiError::bad_request(format!(
            "los pagos suman {pagado} y el total es {total} (centavos)"
        )));
    }

    let mut tx = state.pool.begin().await?;
    let (venta_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO ventas (cita_id, cliente_id, subtotal_centavos, descuento_centavos,
                             propina_centavos, total_centavos, estado, abierta_por, pagada_en)
         VALUES ($1, $2, $3, $4, $5, $6, 'pagada', $7, now())
         RETURNING id",
    )
    .bind(input.cita_id)
    .bind(cliente_id)
    .bind(subtotal)
    .bind(input.descuento_centavos)
    .bind(input.propina_centavos)
    .bind(total)
    .bind(usuario.id)
    .fetch_one(&mut *tx)
    .await?;

    for (tipo, servicio_id, producto_id, descripcion, cantidad, precio) in &lineas {
        sqlx::query(
            "INSERT INTO venta_lineas (venta_id, tipo, servicio_id, producto_id, descripcion,
                                       cantidad, precio_unitario_centavos, total_centavos)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(venta_id)
        .bind(tipo)
        .bind(servicio_id)
        .bind(producto_id)
        .bind(descripcion)
        .bind(cantidad)
        .bind(precio)
        .bind(cantidad * precio)
        .execute(&mut *tx)
        .await?;

        // Descontar inventario de productos vendidos directamente.
        if let Some(producto_id) = producto_id {
            sqlx::query(
                "INSERT INTO movimientos_inventario (producto_id, delta, motivo, referencia)
                 VALUES ($1, $2, 'venta', $3)",
            )
            .bind(producto_id)
            .bind(-cantidad)
            .bind(venta_id.to_string())
            .execute(&mut *tx)
            .await?;
            sqlx::query("UPDATE productos SET stock = stock - $2 WHERE id = $1")
                .bind(producto_id)
                .bind(cantidad)
                .execute(&mut *tx)
                .await?;
        }
    }

    for pago in &input.pagos {
        sqlx::query(
            "INSERT INTO pagos (venta_id, metodo, monto_centavos, referencia)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(venta_id)
        .bind(&pago.metodo)
        .bind(pago.monto_centavos)
        .bind(&pago.referencia)
        .execute(&mut *tx)
        .await?;
    }

    if let Some(cita_id) = input.cita_id {
        sqlx::query("UPDATE citas SET estado = 'atendida', actualizado_en = now() WHERE id = $1")
            .bind(cita_id)
            .execute(&mut *tx)
            .await?;
        // Consumo de productos por servicio.
        let consumos: Vec<(Uuid, f64)> = sqlx::query_as(
            "SELECT cs.producto_id, cs.cantidad::float8
             FROM cita_servicios cserv
             JOIN consumo_servicio cs ON cs.servicio_id = cserv.servicio_id
             WHERE cserv.cita_id = $1",
        )
        .bind(cita_id)
        .fetch_all(&mut *tx)
        .await?;
        for (producto_id, cantidad) in consumos {
            let delta = -(cantidad.ceil() as i32);
            sqlx::query(
                "INSERT INTO movimientos_inventario (producto_id, delta, motivo, referencia)
                 VALUES ($1, $2, 'consumo', $3)",
            )
            .bind(producto_id)
            .bind(delta)
            .bind(cita_id.to_string())
            .execute(&mut *tx)
            .await?;
            sqlx::query("UPDATE productos SET stock = stock + $2 WHERE id = $1")
                .bind(producto_id)
                .bind(delta)
                .execute(&mut *tx)
                .await?;
        }
    }

    tx.commit().await?;

    let venta: Venta = sqlx::query_as(
        "SELECT id, cita_id, cliente_id, subtotal_centavos, descuento_centavos,
                propina_centavos, total_centavos, estado, creada_en, pagada_en
         FROM ventas WHERE id = $1",
    )
    .bind(venta_id)
    .fetch_one(&state.pool)
    .await?;
    let lineas_db: Vec<VentaLinea> = sqlx::query_as(
        "SELECT id, tipo, descripcion, cantidad, precio_unitario_centavos, total_centavos
         FROM venta_lineas WHERE venta_id = $1",
    )
    .bind(venta_id)
    .fetch_all(&state.pool)
    .await?;
    let pagos_db: Vec<Pago> = sqlx::query_as(
        "SELECT id, metodo, monto_centavos, referencia FROM pagos WHERE venta_id = $1",
    )
    .bind(venta_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(VentaDetalle {
        venta,
        lineas: lineas_db,
        pagos: pagos_db,
    }))
}

pub async fn cierre_caja(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Json(input): Json<CierreInput>,
) -> ApiResult<Json<CierreCaja>> {
    usuario.require(ADMIN_RECEP)?;
    let desde = inicio_del_dia(state.cfg.tz, input.fecha);
    let hasta = fin_del_dia(state.cfg.tz, input.fecha);

    let (total, propinas): (i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(total_centavos), 0)::bigint, COALESCE(SUM(propina_centavos), 0)::bigint
         FROM ventas WHERE estado = 'pagada' AND pagada_en >= $1 AND pagada_en < $2",
    )
    .bind(desde)
    .bind(hasta)
    .fetch_one(&state.pool)
    .await?;

    let por_metodo: Vec<(String, i64)> = sqlx::query_as(
        "SELECT p.metodo, COALESCE(SUM(p.monto_centavos), 0)::bigint
         FROM pagos p JOIN ventas v ON v.id = p.venta_id
         WHERE v.estado = 'pagada' AND v.pagada_en >= $1 AND v.pagada_en < $2
         GROUP BY p.metodo",
    )
    .bind(desde)
    .bind(hasta)
    .fetch_all(&state.pool)
    .await?;
    let monto = |metodo: &str| -> i32 {
        por_metodo
            .iter()
            .find(|(m, _)| m == metodo)
            .map(|(_, v)| *v as i32)
            .unwrap_or(0)
    };

    let cierre: CierreCaja = sqlx::query_as(
        "INSERT INTO cierres_caja (fecha, total_centavos, efectivo_centavos, tarjeta_centavos,
                                   transferencia_centavos, propinas_centavos, notas, cerrado_por)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING id, fecha, total_centavos, efectivo_centavos, tarjeta_centavos,
                   transferencia_centavos, propinas_centavos, notas",
    )
    .bind(input.fecha)
    .bind(total as i32)
    .bind(monto("efectivo"))
    .bind(monto("tarjeta"))
    .bind(monto("transferencia"))
    .bind(propinas as i32)
    .bind(input.notas)
    .bind(usuario.id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(cierre))
}

pub async fn listar_cierres(
    State(state): State<AppState>,
    usuario: CurrentUser,
) -> ApiResult<Json<Vec<CierreCaja>>> {
    usuario.require(ADMIN_RECEP)?;
    let cierres: Vec<CierreCaja> = sqlx::query_as(
        "SELECT id, fecha, total_centavos, efectivo_centavos, tarjeta_centavos,
                transferencia_centavos, propinas_centavos, notas
         FROM cierres_caja ORDER BY fecha DESC LIMIT 60",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(cierres))
}

#[derive(Debug, Deserialize)]
pub struct CajaDelDia {
    pub fecha: Option<NaiveDate>,
}

pub async fn resumen_dia(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Query(filtro): Query<CajaDelDia>,
) -> ApiResult<Json<Value>> {
    usuario.require(ADMIN_RECEP)?;
    let fecha = filtro
        .fecha
        .unwrap_or_else(|| Utc::now().with_timezone(&state.cfg.tz).date_naive());
    let desde: DateTime<Utc> = inicio_del_dia(state.cfg.tz, fecha);
    let hasta = fin_del_dia(state.cfg.tz, fecha);
    let (total, tickets): (i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(total_centavos), 0)::bigint, COUNT(*)::bigint
         FROM ventas WHERE estado = 'pagada' AND pagada_en >= $1 AND pagada_en < $2",
    )
    .bind(desde)
    .bind(hasta)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(json!({ "fecha": fecha, "total_centavos": total, "tickets": tickets })))
}
