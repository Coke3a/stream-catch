use crate::axum_http::auth::AuthUser;
use crate::usecases::live_following::LiveFollowingUseCase;
use axum::{
    Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::post,
};
use crates::{
    domain::repositories::live_following::LiveFollowingRepository,
    infra::db::{
        postgres::postgres_connection::PgPoolSquad,
        repositories::live_following::LiveFollowingPostgres,
    },
};
use std::sync::Arc;

pub fn routes(db_pool: Arc<PgPoolSquad>) -> Router {
    let live_following_repository = LiveFollowingPostgres::new(Arc::clone(&db_pool));
    let live_following_usecase = LiveFollowingUseCase::new(Arc::new(live_following_repository));

    Router::new()
        .route("/:value", post(follow))
        .with_state(Arc::new(live_following_usecase))
}

pub async fn follow<T>(
    State(live_following_usecase): State<Arc<LiveFollowingUseCase<T>>>,
    auth: AuthUser,
    Path(url): Path<String>,
) -> impl IntoResponse
where
    T: LiveFollowingRepository + Send + Sync,
{
    use base64::{Engine as _, engine::general_purpose};

    // Decode base64url
    let decoded_url_bytes = match general_purpose::URL_SAFE_NO_PAD.decode(&url) {
        Ok(bytes) => bytes,
        Err(_) => {
            return (axum::http::StatusCode::BAD_REQUEST, "Invalid base64url").into_response();
        }
    };

    let decoded_url = match String::from_utf8(decoded_url_bytes) {
        Ok(s) => s,
        Err(_) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                "Invalid UTF-8 sequence",
            )
                .into_response();
        }
    };

    match live_following_usecase
        .follow(auth.user_id, decoded_url)
        .await
    {
        Ok(_) => (axum::http::StatusCode::OK, "Followed successfully").into_response(),
        Err(e) => {
            let error_message = e.to_string();
            if error_message.contains("Follow already exists") {
                (axum::http::StatusCode::CONFLICT, "Follow already exists").into_response()
            } else if error_message.contains("Invalid URL")
                || error_message.contains("Unsupported platform")
            {
                (axum::http::StatusCode::BAD_REQUEST, error_message).into_response()
            } else {
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error_message).into_response()
            }
        }
    }
}
