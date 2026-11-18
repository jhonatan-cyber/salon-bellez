use anyhow::{Context, Result};
use chrono_tz::Tz;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub port: u16,
    pub app_env: String,
    pub app_origin: String,
    pub tz: Tz,
    pub admin_email: Option<String>,
    pub admin_password: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://salon:salon@localhost:5432/salon".to_string());
        let jwt_secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "cambia-este-secreto-en-produccion".to_string());
        let port = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000);
        let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
        let app_origin =
            std::env::var("APP_ORIGIN").unwrap_or_else(|_| "http://localhost:4321".to_string());
        let tz_name =
            std::env::var("APP_TZ").unwrap_or_else(|_| "America/Mexico_City".to_string());
        let tz: Tz = tz_name
            .parse()
            .with_context(|| format!("zona horaria inválida: {tz_name}"))?;

        if app_env == "production" && jwt_secret.starts_with("cambia-este") {
            anyhow::bail!("JWT_SECRET debe configurarse en producción");
        }

        Ok(Self {
            database_url,
            jwt_secret,
            port,
            app_env,
            app_origin,
            tz,
            admin_email: std::env::var("ADMIN_EMAIL").ok(),
            admin_password: std::env::var("ADMIN_PASSWORD").ok(),
        })
    }

    pub fn is_production(&self) -> bool {
        self.app_env == "production"
    }
}
