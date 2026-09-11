use axum::{Json, http::StatusCode, response::{IntoResponse, Response}};
use etyloom_core::ApiError;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Sign in to continue")]
    Unauthorized,
    #[error("This request is not authorized")]
    Forbidden,
    #[error("The requested resource was not found")]
    NotFound,
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Conflict(String),
    #[error("The job queue is full. Try again after an active job finishes")]
    Busy,
    #[error(transparent)]
    Engine(#[from] etyloom_core::Error),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, AppError>;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED, Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND, Self::Invalid(_) | Self::Engine(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Conflict(_) => StatusCode::CONFLICT, Self::Busy => StatusCode::TOO_MANY_REQUESTS,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        static SEQUENCE: AtomicU64 = AtomicU64::new(1);
        let request_id = format!("{}-{}", std::process::id(), SEQUENCE.fetch_add(1, Ordering::Relaxed));
        let message = if status.is_server_error() {
            tracing::error!(%request_id, error = %self, "request failed");
            "The operation failed. Saved revisions are unchanged; consult the server log with this reference.".into()
        } else { self.to_string() };
        (status, Json(ApiError { error: message, request_id: Some(request_id) })).into_response()
    }
}
