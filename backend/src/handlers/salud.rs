use axum::Json;
use axum::extract::State;
use serde_json::{Value, json};

use crate::error::ApiResult;
use crate::state::AppState;

pub async fn health(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let (ok,): (i32,) = sqlx::query_as("SELECT 1").fetch_one(&state.pool).await?;
    Ok(Json(json!({
        "status": "ok",
        "database": ok == 1,
        "version": env!("CARGO_PKG_VERSION"),
    })))
}
