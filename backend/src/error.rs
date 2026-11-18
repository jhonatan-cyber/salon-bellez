use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    BadRequest(String),
    #[error("no autenticado")]
    Unauthorized,
    #[error("sin permisos para esta acción")]
    Forbidden,
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Internal(String),
}

impl ApiError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::BadRequest(msg.into())
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::Conflict(msg.into())
    }

    pub fn internal(msg: impl std::fmt::Display) -> Self {
        Self::Internal(msg.to_string())
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        let message = match &self {
            Self::Internal(detail) => {
                tracing::error!(error = %detail, "error interno");
                "error interno del servidor".to_string()
            }
            other => other.to_string(),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        match &err {
            sqlx::Error::RowNotFound => Self::NotFound("registro no encontrado".into()),
            sqlx::Error::Database(db_err) => match db_err.code().as_deref() {
                // exclusion_violation: cita solapada
                Some("23P01") => Self::Conflict("ya existe una cita en ese horario".into()),
                Some("23505") => Self::Conflict("el registro ya existe".into()),
                Some("23503") => Self::BadRequest("referencia a un registro inexistente".into()),
                Some("23502") | Some("23514") => {
                    Self::BadRequest("datos inválidos para el registro".into())
                }
                _ => Self::Internal(format!("{err:?}")),
            },
            _ => Self::Internal(format!("{err:?}")),
        }
    }
}

impl From<jsonwebtoken::errors::Error> for ApiError {
    fn from(_: jsonwebtoken::errors::Error) -> Self {
        Self::Unauthorized
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
