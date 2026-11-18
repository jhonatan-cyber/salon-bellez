use axum::Json;
use axum::extract::{Path, Query, State};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::error::{ApiError, ApiResult};
use crate::handlers::{ADMIN_RECEP, STAFF};
use crate::models::{
    AjusteStock, Categoria, Cliente, ClienteInput, Movimiento, Producto, ProductoInput, Servicio,
    ServicioInput,
};
use crate::state::AppState;

// ---------- Servicios ----------

pub async fn listar_servicios(
    State(state): State<AppState>,
    usuario: CurrentUser,
) -> ApiResult<Json<Value>> {
    usuario.require(STAFF)?;
    let servicios: Vec<Servicio> = sqlx::query_as(
        "SELECT id, categoria_id, nombre, descripcion, duracion_min, buffer_min,
                precio_centavos, activo
         FROM servicios ORDER BY nombre",
    )
    .fetch_all(&state.pool)
    .await?;
    let categorias: Vec<Categoria> =
        sqlx::query_as("SELECT id, nombre, orden FROM categorias_servicio ORDER BY orden, nombre")
            .fetch_all(&state.pool)
            .await?;
    Ok(Json(json!({ "servicios": servicios, "categorias": categorias })))
}

pub async fn crear_servicio(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Json(input): Json<ServicioInput>,
) -> ApiResult<Json<Servicio>> {
    usuario.require(ADMIN_RECEP)?;
    if input.nombre.trim().is_empty() || input.duracion_min <= 0 {
        return Err(ApiError::bad_request("nombre y duración son obligatorios"));
    }
    let servicio: Servicio = sqlx::query_as(
        "INSERT INTO servicios (categoria_id, nombre, descripcion, duracion_min, buffer_min,
                                precio_centavos, activo)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, categoria_id, nombre, descripcion, duracion_min, buffer_min,
                   precio_centavos, activo",
    )
    .bind(input.categoria_id)
    .bind(input.nombre.trim())
    .bind(input.descripcion)
    .bind(input.duracion_min)
    .bind(input.buffer_min)
    .bind(input.precio_centavos)
    .bind(input.activo)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(servicio))
}

pub async fn actualizar_servicio(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Path(id): Path<Uuid>,
    Json(input): Json<ServicioInput>,
) -> ApiResult<Json<Servicio>> {
    usuario.require(ADMIN_RECEP)?;
    let servicio: Servicio = sqlx::query_as(
        "UPDATE servicios SET categoria_id = $2, nombre = $3, descripcion = $4,
                duracion_min = $5, buffer_min = $6, precio_centavos = $7, activo = $8
         WHERE id = $1
         RETURNING id, categoria_id, nombre, descripcion, duracion_min, buffer_min,
                   precio_centavos, activo",
    )
    .bind(id)
    .bind(input.categoria_id)
    .bind(input.nombre.trim())
    .bind(input.descripcion)
    .bind(input.duracion_min)
    .bind(input.buffer_min)
    .bind(input.precio_centavos)
    .bind(input.activo)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(servicio))
}

pub async fn borrar_servicio(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    usuario.require(&["admin"])?;
    let usado: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM cita_servicios WHERE servicio_id = $1 LIMIT 1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?;
    if usado.is_some() {
        sqlx::query("UPDATE servicios SET activo = false WHERE id = $1")
            .bind(id)
            .execute(&state.pool)
            .await?;
        return Ok(Json(json!({ "ok": true, "desactivado": true })));
    }
    sqlx::query("DELETE FROM servicios WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;
    Ok(Json(json!({ "ok": true })))
}

// ---------- Categorías ----------

pub async fn crear_categoria(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Json(input): Json<CategoriaInput>,
) -> ApiResult<Json<Categoria>> {
    usuario.require(&["admin"])?;
    let categoria: Categoria = sqlx::query_as(
        "INSERT INTO categorias_servicio (nombre, orden) VALUES ($1, $2)
         RETURNING id, nombre, orden",
    )
    .bind(input.nombre.trim())
    .bind(input.orden)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(categoria))
}

#[derive(Debug, Deserialize)]
pub struct CategoriaInput {
    pub nombre: String,
    #[serde(default)]
    pub orden: i32,
}

// ---------- Clientes ----------

#[derive(Debug, Deserialize)]
pub struct ClienteFiltro {
    pub q: Option<String>,
}

pub async fn listar_clientes(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Query(filtro): Query<ClienteFiltro>,
) -> ApiResult<Json<Vec<Cliente>>> {
    usuario.require(STAFF)?;
    let busqueda = filtro
        .q
        .as_deref()
        .map(|q| format!("%{}%", q.trim().to_lowercase()));
    let clientes: Vec<Cliente> = sqlx::query_as(
        "SELECT id, nombre, telefono, email, notas, acepta_marketing, creado_en
         FROM clientes
         WHERE $1::text IS NULL
            OR lower(nombre) LIKE $1
            OR coalesce(telefono, '') LIKE $1
            OR lower(coalesce(email, '')) LIKE $1
         ORDER BY nombre
         LIMIT 200",
    )
    .bind(busqueda)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(clientes))
}

pub async fn obtener_cliente(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    usuario.require(STAFF)?;
    let cliente: Cliente = sqlx::query_as(
        "SELECT id, nombre, telefono, email, notas, acepta_marketing, creado_en
         FROM clientes WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await?;
    let citas: Vec<crate::models::CitaRow> = sqlx::query_as(
        "SELECT c.id, c.cliente_id, cl.nombre AS cliente_nombre, cl.telefono AS cliente_telefono,
                c.estilista_id, e.nombre AS estilista_nombre, c.inicio, c.fin, c.estado,
                c.notas, c.canal
         FROM citas c
         JOIN clientes cl ON cl.id = c.cliente_id
         JOIN estilistas e ON e.id = c.estilista_id
         WHERE c.cliente_id = $1
         ORDER BY c.inicio DESC
         LIMIT 50",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({ "cliente": cliente, "citas": citas })))
}

pub async fn crear_cliente(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Json(input): Json<ClienteInput>,
) -> ApiResult<Json<Cliente>> {
    usuario.require(STAFF)?;
    if input.nombre.trim().is_empty() {
        return Err(ApiError::bad_request("el nombre es obligatorio"));
    }
    let cliente: Cliente = sqlx::query_as(
        "INSERT INTO clientes (nombre, telefono, email, notas, acepta_marketing)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, nombre, telefono, email, notas, acepta_marketing, creado_en",
    )
    .bind(input.nombre.trim())
    .bind(input.telefono)
    .bind(input.email)
    .bind(input.notas)
    .bind(input.acepta_marketing)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(cliente))
}

pub async fn actualizar_cliente(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Path(id): Path<Uuid>,
    Json(input): Json<ClienteInput>,
) -> ApiResult<Json<Cliente>> {
    usuario.require(STAFF)?;
    let cliente: Cliente = sqlx::query_as(
        "UPDATE clientes SET nombre = $2, telefono = $3, email = $4, notas = $5,
                acepta_marketing = $6
         WHERE id = $1
         RETURNING id, nombre, telefono, email, notas, acepta_marketing, creado_en",
    )
    .bind(id)
    .bind(input.nombre.trim())
    .bind(input.telefono)
    .bind(input.email)
    .bind(input.notas)
    .bind(input.acepta_marketing)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(cliente))
}

// ---------- Productos / inventario ----------

pub async fn listar_productos(
    State(state): State<AppState>,
    usuario: CurrentUser,
) -> ApiResult<Json<Vec<Producto>>> {
    usuario.require(STAFF)?;
    let productos: Vec<Producto> = sqlx::query_as(
        "SELECT id, nombre, sku, precio_centavos, costo_centavos, stock, stock_minimo, activo
         FROM productos ORDER BY nombre",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(productos))
}

pub async fn crear_producto(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Json(input): Json<ProductoInput>,
) -> ApiResult<Json<Producto>> {
    usuario.require(ADMIN_RECEP)?;
    let producto: Producto = sqlx::query_as(
        "INSERT INTO productos (nombre, sku, precio_centavos, costo_centavos, stock, stock_minimo, activo)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, nombre, sku, precio_centavos, costo_centavos, stock, stock_minimo, activo",
    )
    .bind(input.nombre.trim())
    .bind(input.sku)
    .bind(input.precio_centavos)
    .bind(input.costo_centavos)
    .bind(input.stock)
    .bind(input.stock_minimo)
    .bind(input.activo)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(producto))
}

pub async fn actualizar_producto(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Path(id): Path<Uuid>,
    Json(input): Json<ProductoInput>,
) -> ApiResult<Json<Producto>> {
    usuario.require(ADMIN_RECEP)?;
    let producto: Producto = sqlx::query_as(
        "UPDATE productos SET nombre = $2, sku = $3, precio_centavos = $4,
                costo_centavos = $5, stock = $6, stock_minimo = $7, activo = $8
         WHERE id = $1
         RETURNING id, nombre, sku, precio_centavos, costo_centavos, stock, stock_minimo, activo",
    )
    .bind(id)
    .bind(input.nombre.trim())
    .bind(input.sku)
    .bind(input.precio_centavos)
    .bind(input.costo_centavos)
    .bind(input.stock)
    .bind(input.stock_minimo)
    .bind(input.activo)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(producto))
}

pub async fn ajustar_stock(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Path(id): Path<Uuid>,
    Json(input): Json<AjusteStock>,
) -> ApiResult<Json<Value>> {
    usuario.require(ADMIN_RECEP)?;
    if input.delta == 0 {
        return Err(ApiError::bad_request("el ajuste no puede ser cero"));
    }
    let mut tx = state.pool.begin().await?;
    sqlx::query("INSERT INTO movimientos_inventario (producto_id, delta, motivo) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(input.delta)
        .bind(input.motivo.trim())
        .execute(&mut *tx)
        .await?;
    let producto: Producto = sqlx::query_as(
        "UPDATE productos SET stock = stock + $2 WHERE id = $1
         RETURNING id, nombre, sku, precio_centavos, costo_centavos, stock, stock_minimo, activo",
    )
    .bind(id)
    .bind(input.delta)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(json!({ "producto": producto })))
}

pub async fn movimientos_producto(
    State(state): State<AppState>,
    usuario: CurrentUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<Movimiento>>> {
    usuario.require(ADMIN_RECEP)?;
    let movimientos: Vec<Movimiento> = sqlx::query_as(
        "SELECT id, producto_id, delta, motivo, referencia, creado_en
         FROM movimientos_inventario WHERE producto_id = $1
         ORDER BY creado_en DESC LIMIT 100",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(movimientos))
}
