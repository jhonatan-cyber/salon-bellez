use axum::extract::{Request, State};
use axum::http::{Method, StatusCode, header::ORIGIN};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, patch, post, put};
use axum::{Json, Router};
use serde_json::json;

use crate::handlers;
use crate::state::AppState;

/// Rechaza mutaciones que vengan de un origen distinto al configurado.
async fn origin_guard(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let es_seguro = matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS);
    if !es_seguro
        && let Some(origin) = req.headers().get(ORIGIN).and_then(|v| v.to_str().ok())
    {
        let permitido = origin == state.cfg.app_origin
            || origin.starts_with("http://localhost")
            || origin.starts_with("http://127.0.0.1");
        if !permitido {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "origen no permitido" })),
            )
                .into_response();
        }
    }
    next.run(req).await
}

pub fn router(state: AppState) -> Router {
    let publicas = Router::new()
        .route("/health", get(handlers::salud::health))
        .route("/openapi.json", get(handlers::docs::openapi))
        .route("/docs", get(handlers::docs::docs))
        .route("/auth/login", post(handlers::auth::login))
        .route("/auth/logout", post(handlers::auth::logout))
        .route("/public/servicios", get(handlers::publico::servicios))
        .route("/public/estilistas", get(handlers::publico::estilistas))
        .route(
            "/public/disponibilidad",
            get(handlers::publico::disponibilidad),
        )
        .route("/public/reservas", post(handlers::publico::reservar));

    let privadas = Router::new()
        .route("/auth/me", get(handlers::auth::me))
        .route("/auth/password", post(handlers::auth::cambiar_password))
        // usuarios
        .route(
            "/usuarios",
            get(handlers::staff::listar_usuarios).post(handlers::staff::crear_usuario),
        )
        .route(
            "/usuarios/{id}/estado",
            patch(handlers::staff::cambiar_estado_usuario),
        )
        // servicios
        .route(
            "/servicios",
            get(handlers::catalog::listar_servicios).post(handlers::catalog::crear_servicio),
        )
        .route(
            "/servicios/{id}",
            put(handlers::catalog::actualizar_servicio).delete(handlers::catalog::borrar_servicio),
        )
        .route("/categorias", post(handlers::catalog::crear_categoria))
        // clientes
        .route(
            "/clientes",
            get(handlers::catalog::listar_clientes).post(handlers::catalog::crear_cliente),
        )
        .route(
            "/clientes/{id}",
            get(handlers::catalog::obtener_cliente).put(handlers::catalog::actualizar_cliente),
        )
        // estilistas
        .route(
            "/estilistas",
            get(handlers::staff::listar_estilistas).post(handlers::staff::crear_estilista),
        )
        .route(
            "/estilistas/{id}",
            put(handlers::staff::actualizar_estilista),
        )
        .route(
            "/estilistas/{id}/horarios",
            get(handlers::staff::listar_horarios).post(handlers::staff::crear_horario),
        )
        .route("/horarios/{id}", delete(handlers::staff::borrar_horario))
        .route(
            "/estilistas/{id}/bloqueos",
            get(handlers::staff::listar_bloqueos).post(handlers::staff::crear_bloqueo),
        )
        .route("/bloqueos/{id}", delete(handlers::staff::borrar_bloqueo))
        // festivos
        .route(
            "/festivos",
            get(handlers::staff::listar_festivos).post(handlers::staff::crear_festivo),
        )
        .route("/festivos/{id}", delete(handlers::staff::borrar_festivo))
        // agenda
        .route("/disponibilidad", get(handlers::disponibilidad::disponibilidad))
        .route(
            "/citas",
            get(handlers::citas::listar).post(handlers::citas::crear),
        )
        .route(
            "/citas/{id}",
            get(handlers::citas::obtener).patch(handlers::citas::reprogramar),
        )
        .route(
            "/citas/{id}/estado",
            post(handlers::citas::cambiar_estado),
        )
        // ventas y caja
        .route(
            "/ventas",
            get(handlers::ventas::listar).post(handlers::ventas::crear),
        )
        .route("/ventas/{id}", get(handlers::ventas::obtener))
        .route("/caja/cierre", post(handlers::ventas::cierre_caja))
        .route("/caja/cierres", get(handlers::ventas::listar_cierres))
        .route("/caja/resumen", get(handlers::ventas::resumen_dia))
        // inventario
        .route(
            "/productos",
            get(handlers::catalog::listar_productos).post(handlers::catalog::crear_producto),
        )
        .route(
            "/productos/{id}",
            put(handlers::catalog::actualizar_producto),
        )
        .route(
            "/productos/{id}/ajuste",
            post(handlers::catalog::ajustar_stock),
        )
        .route(
            "/productos/{id}/movimientos",
            get(handlers::catalog::movimientos_producto),
        )
        // reportes
        .route("/reportes/resumen", get(handlers::reportes::resumen))
        .route(
            "/reportes/ingresos-por-dia",
            get(handlers::reportes::ingresos_por_dia),
        )
        .route(
            "/reportes/por-servicio",
            get(handlers::reportes::por_servicio),
        )
        .route(
            "/reportes/por-estilista",
            get(handlers::reportes::por_estilista),
        )
        .route(
            "/reportes/top-clientes",
            get(handlers::reportes::top_clientes),
        );

    Router::new()
        .nest("/api", publicas.merge(privadas))
        .layer(middleware::from_fn_with_state(state.clone(), origin_guard))
        .with_state(state)
}
