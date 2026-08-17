use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

/// Application error, mapped to a consistent JSON problem response.
///
/// Internal causes are logged, never exposed to the client.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Validation(String),
    #[error("too many requests")]
    TooManyRequests,
    #[error("internal error")]
    Internal,
}

impl AppError {
    fn code(&self) -> (StatusCode, &'static str) {
        match self {
            AppError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "unauthorized"),
            AppError::Forbidden(_) => (StatusCode::FORBIDDEN, "forbidden"),
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            AppError::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
            AppError::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "validation"),
            AppError::TooManyRequests => (StatusCode::TOO_MANY_REQUESTS, "too_many_requests"),
            AppError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = self.code();
        let message = match &self {
            AppError::Internal => "internal server error".to_string(),
            other => other.to_string(),
        };
        (
            status,
            Json(json!({ "error": { "code": code, "message": message } })),
        )
            .into_response()
    }
}

impl From<sea_orm::DbErr> for AppError {
    fn from(err: sea_orm::DbErr) -> Self {
        // SeaORM wraps sqlx errors in `Exec` for statements (our inserts
        // provide the PK, so they go through `execute`) and in `Query` for
        // fetches — a unique violation can surface through either.
        let sqlx_err = match &err {
            sea_orm::DbErr::Exec(sea_orm::RuntimeErr::SqlxError(e)) => Some(e),
            sea_orm::DbErr::Query(sea_orm::RuntimeErr::SqlxError(e)) => Some(e),
            _ => None,
        };
        if let Some(db_err) = sqlx_err.and_then(|e| e.as_database_error()) {
            if db_err.is_unique_violation() {
                return AppError::Conflict("resource already exists".to_string());
            }
        }
        tracing::error!(error = %err, "database error");
        AppError::Internal
    }
}
