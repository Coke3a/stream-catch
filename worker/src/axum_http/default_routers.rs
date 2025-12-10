use axum::{http::StatusCode, response::IntoResponse};
use tracing::info;

pub async fn not_found() -> impl IntoResponse {
    info!("worker router: not_found handler invoked");
    (StatusCode::NOT_FOUND, "NOT_FOUND").into_response()
}

pub async fn health_check() -> impl IntoResponse {
    info!("worker router: health_check handler invoked");
    (StatusCode::OK, "OK").into_response()
}
