use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use jsonwebtoken::{DecodingKey, EncodingKey};
use sqlx::PgPool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub cfg: Arc<Config>,
    pub encoding: EncodingKey,
    pub decoding: DecodingKey,
    pub limiter: Arc<RateLimiter>,
}

impl AppState {
    pub fn new(pool: PgPool, cfg: Arc<Config>) -> Self {
        let encoding = EncodingKey::from_secret(cfg.jwt_secret.as_bytes())
            .expect("JWT_SECRET inválido: usa al menos 32 caracteres");
        let decoding = DecodingKey::from_secret(cfg.jwt_secret.as_bytes())
            .expect("JWT_SECRET inválido: usa al menos 32 caracteres");
        Self {
            pool,
            cfg,
            encoding,
            decoding,
            limiter: Arc::new(RateLimiter::new(10, Duration::from_secs(60))),
        }
    }
}

/// Limitador sencillo en memoria: N peticiones por ventana y clave.
pub struct RateLimiter {
    inner: Mutex<HashMap<String, (u32, Instant)>>,
    limit: u32,
    window: Duration,
}

impl RateLimiter {
    pub fn new(limit: u32, window: Duration) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            limit,
            window,
        }
    }

    /// Devuelve false si la clave superó el límite de la ventana actual.
    pub fn check(&self, key: &str) -> bool {
        let mut map = match self.inner.lock() {
            Ok(map) => map,
            Err(poisoned) => poisoned.into_inner(),
        };
        let now = Instant::now();
        map.retain(|_, (_, started)| now.duration_since(*started) < self.window);
        let entry = map.entry(key.to_string()).or_insert((0, now));
        entry.0 += 1;
        entry.0 <= self.limit
    }
}
