use axum::Json;
use axum::response::Html;
use serde_json::{Value, json};

const SWAGGER_HTML: &str = r#"<!doctype html>
<html lang="es">
  <head>
    <meta charset="utf-8" />
    <title>API del salón — documentación</title>
    <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
  </head>
  <body>
    <div id="swagger"></div>
    <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
    <script>
      window.ui = SwaggerUIBundle({ url: '/api/openapi.json', dom_id: '#swagger' });
    </script>
  </body>
</html>
"#;

fn op(method: &str, summary: &str, tag: &str, roles: &str) -> Value {
    json!({
        "tags": [tag],
        "summary": summary,
        "description": format!("Método {method}. Roles: {roles}"),
        "responses": { "200": { "description": "OK" } }
    })
}

pub async fn openapi() -> Json<Value> {
    Json(json!({
        "openapi": "3.1.0",
        "info": {
            "title": "API del salón de belleza",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Sesión por cookie httpOnly. Roles: admin, recepcionista, estilista."
        },
        "servers": [{ "url": "/api" }],
        "tags": [
            {"name": "salud"}, {"name": "auth"}, {"name": "catalogo"}, {"name": "agenda"},
            {"name": "ventas"}, {"name": "inventario"}, {"name": "reportes"}, {"name": "publico"}
        ],
        "paths": {
            "/health": {"get": op("get", "Estado del servicio", "salud", "público")},
            "/auth/login": {"post": op("post", "Inicia sesión (email y contraseña)", "auth", "público")},
            "/auth/logout": {"post": op("post", "Cierra la sesión", "auth", "público")},
            "/auth/me": {"get": op("get", "Usuario de la sesión", "auth", "staff")},
            "/auth/password": {"post": op("post", "Cambia la contraseña", "auth", "staff")},
            "/servicios": {
                "get": op("get", "Lista servicios y categorías", "catalogo", "staff"),
                "post": op("post", "Crea un servicio", "catalogo", "admin, recepcionista")
            },
            "/servicios/{id}": {
                "put": op("put", "Actualiza un servicio", "catalogo", "admin, recepcionista"),
                "delete": op("delete", "Borra o desactiva un servicio", "catalogo", "admin")
            },
            "/categorias": {"post": op("post", "Crea una categoría", "catalogo", "admin")},
            "/clientes": {
                "get": op("get", "Busca clientes (?q=)", "catalogo", "staff"),
                "post": op("post", "Crea un cliente", "catalogo", "staff")
            },
            "/clientes/{id}": {
                "get": op("get", "Cliente con su historial", "catalogo", "staff"),
                "put": op("put", "Actualiza un cliente", "catalogo", "staff")
            },
            "/estilistas": {
                "get": op("get", "Lista estilistas", "catalogo", "staff"),
                "post": op("post", "Crea una estilista", "catalogo", "admin, recepcionista")
            },
            "/estilistas/{id}/horarios": {
                "get": op("get", "Horarios de la estilista", "agenda", "staff"),
                "post": op("post", "Agrega una franja de horario", "agenda", "admin, recepcionista")
            },
            "/estilistas/{id}/bloqueos": {
                "get": op("get", "Bloqueos futuros", "agenda", "staff"),
                "post": op("post", "Crea un bloqueo", "agenda", "admin, recepcionista")
            },
            "/festivos": {
                "get": op("get", "Días festivos", "agenda", "staff"),
                "post": op("post", "Marca un festivo", "agenda", "admin, recepcionista")
            },
            "/disponibilidad": {"get": op("get", "Huecos libres (?servicio_id&estilista_id&fecha)", "agenda", "staff")},
            "/citas": {
                "get": op("get", "Citas por rango (?desde&hasta&estilista_id)", "agenda", "staff"),
                "post": op("post", "Agenda una cita", "agenda", "admin, recepcionista")
            },
            "/citas/{id}": {
                "get": op("get", "Detalle de la cita", "agenda", "staff"),
                "patch": op("patch", "Reprograma la cita", "agenda", "admin, recepcionista")
            },
            "/citas/{id}/estado": {"post": op("post", "Cambia el estado de la cita", "agenda", "staff")},
            "/ventas": {
                "get": op("get", "Ventas del rango", "ventas", "admin, recepcionista"),
                "post": op("post", "Registra una venta y sus pagos", "ventas", "admin, recepcionista")
            },
            "/ventas/{id}": {"get": op("get", "Ticket con líneas y pagos", "ventas", "admin, recepcionista")},
            "/caja/cierre": {"post": op("post", "Cierra la caja del día", "ventas", "admin, recepcionista")},
            "/caja/cierres": {"get": op("get", "Últimos cierres", "ventas", "admin, recepcionista")},
            "/caja/resumen": {"get": op("get", "Resumen de caja del día", "ventas", "admin, recepcionista")},
            "/productos": {
                "get": op("get", "Lista productos", "inventario", "staff"),
                "post": op("post", "Crea un producto", "inventario", "admin, recepcionista")
            },
            "/productos/{id}": {"put": op("put", "Actualiza un producto", "inventario", "admin, recepcionista")},
            "/productos/{id}/ajuste": {"post": op("post", "Ajusta el stock", "inventario", "admin, recepcionista")},
            "/productos/{id}/movimientos": {"get": op("get", "Movimientos de inventario", "inventario", "admin, recepcionista")},
            "/reportes/resumen": {"get": op("get", "KPIs del rango", "reportes", "admin, recepcionista")},
            "/reportes/ingresos-por-dia": {"get": op("get", "Ingresos por día", "reportes", "admin, recepcionista")},
            "/reportes/por-servicio": {"get": op("get", "Servicios más vendidos", "reportes", "admin, recepcionista")},
            "/reportes/por-estilista": {"get": op("get", "Rendimiento por estilista", "reportes", "admin, recepcionista")},
            "/reportes/top-clientes": {"get": op("get", "Mejores clientes", "reportes", "admin, recepcionista")},
            "/public/servicios": {"get": op("get", "Catálogo público", "publico", "público")},
            "/public/estilistas": {"get": op("get", "Estilistas activas", "publico", "público")},
            "/public/disponibilidad": {"get": op("get", "Huecos para reservar en línea", "publico", "público")},
            "/public/reservas": {"post": op("post", "Reserva en línea (sin cuenta)", "publico", "público")},
            "/usuarios": {
                "get": op("get", "Lista usuarios", "auth", "admin"),
                "post": op("post", "Crea usuario del staff", "auth", "admin")
            }
        }
    }))
}

pub async fn docs() -> Html<&'static str> {
    Html(SWAGGER_HTML)
}
