use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub code: u16,
    pub message: String,
}

// Your app-level error type
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Unauthorized")]
    Unauthorized,

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Internal(_) => {
                // Don't leak internal error detail to client
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
        };

        let body = Json(ErrorResponse {
            code: status.as_u16(),
            message,
        });

        (status, body).into_response()
    }
}
