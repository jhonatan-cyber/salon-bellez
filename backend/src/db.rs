use std::time::Duration;

use anyhow::{Context, Result};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::auth;
use crate::config::Config;

pub async fn connect(database_url: &str) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(8))
        .connect(database_url)
        .await
        .context("no se pudo conectar a Postgres (revisa DATABASE_URL)")
}

pub async fn migrate(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .context("fallaron las migraciones")?;
    Ok(())
}

/// Crea el admin inicial si ADMIN_EMAIL y ADMIN_PASSWORD están definidos y no existe.
pub async fn ensure_admin(pool: &PgPool, cfg: &Config) -> Result<()> {
    let (Some(email), Some(password)) = (&cfg.admin_email, &cfg.admin_password) else {
        return Ok(());
    };
    if password.len() < 8 {
        anyhow::bail!("ADMIN_PASSWORD debe tener al menos 8 caracteres");
    }

    let exists: Option<(uuid::Uuid,)> =
        sqlx::query_as("SELECT id FROM usuarios WHERE lower(email) = lower($1)")
            .bind(email)
            .fetch_optional(pool)
            .await?;
    if exists.is_some() {
        return Ok(());
    }

    let hash = auth::hash_password(password)
        .map_err(|e| anyhow::anyhow!("no se pudo hashear la contraseña: {e:?}"))?;
    sqlx::query(
        "INSERT INTO usuarios (email, nombre, rol, password_hash)
         VALUES ($1, $2, 'admin', $3)",
    )
    .bind(email)
    .bind("Administradora")
    .bind(hash)
    .execute(pool)
    .await?;
    tracing::info!(%email, "usuario admin creado");
    Ok(())
}
