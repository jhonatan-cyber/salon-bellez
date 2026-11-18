#![recursion_limit = "512"]

mod auth;
mod config;
mod db;
mod error;
mod handlers;
mod models;
mod routes;
mod state;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use tower_http::compression::CompressionLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "salon_api=info,tower_http=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cfg = Arc::new(config::Config::from_env()?);
    tracing::info!(entorno = %cfg.app_env, zona = %cfg.tz, "iniciando salon-api");

    let pool = db::connect(&cfg.database_url).await?;
    db::migrate(&pool).await?;
    db::ensure_admin(&pool, &cfg).await?;

    let state = state::AppState::new(pool, Arc::clone(&cfg));
    let mut app = routes::router(state)
        .layer(CompressionLayer::new())
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(30),
        ))
        .layer(TraceLayer::new_for_http());

    // En producción Caddy sirve el frontend; esto permite también un solo contenedor.
    let dist = Path::new("../frontend/dist");
    if dist.is_dir() {
        let index = dist.join("index.html");
        app = app.fallback_service(
            ServeDir::new(dist).not_found_service(ServeFile::new(index)),
        );
        tracing::info!("sirviendo frontend estático desde ../frontend/dist");
    }

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], cfg.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("API lista en http://localhost:{}", cfg.port);
    axum::serve(listener, app).await?;
    Ok(())
}
