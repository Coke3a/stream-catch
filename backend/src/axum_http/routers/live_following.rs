use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    response::IntoResponse, routing::{delete, get, post},
};
use uuid::Uuid;

use crate::{auth::AuthUser, config::stage::Stage};
use application::usercases::live_following::LiveFollowingUseCase;
use domain::{
    repositories::live_following::LiveFollowingRepository,
    value_objects::live_following::{InsertFollowLiveAccountModel, ListFollowsFilter},
};
use infra::postgres::{
    postgres_connection::PgPoolSquad,
    repositories::live_following::LiveFollowingPostgres,
};

pub fn routes(db_pool: Arc<PgPoolSquad>) -> Router {
    let live_following_repository = LiveFollowingPostgres::new(Arc::clone(&db_pool));
    let live_following_usecase = LiveFollowingUseCase::new(Arc::new(live_following_repository));

    Router::new()
        .route("/", post(follow))
        .route("/", delete(unfollow))
        .route("/", get(list))
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
        Err(_) => return (axum::http::StatusCode::BAD_REQUEST, "Invalid base64url").into_response(),
    };

    let decoded_url = match String::from_utf8(decoded_url_bytes) {
        Ok(s) => s,
        Err(_) => return (axum::http::StatusCode::BAD_REQUEST, "Invalid UTF-8 sequence").into_response(),
    };

    match live_following_usecase.follow(auth.user_id, decoded_url).await {
        Ok(_) => (axum::http::StatusCode::OK, "Followed successfully").into_response(),
        Err(e) => {
            let error_message = e.to_string();
            if error_message.contains("Follow already exists") {
                (axum::http::StatusCode::CONFLICT, "Follow already exists").into_response()
            } else if error_message.contains("Invalid URL") || error_message.contains("Unsupported platform") {
                (axum::http::StatusCode::BAD_REQUEST, error_message).into_response()
            } else {
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error_message).into_response()
            }
        }
    }
}

pub async fn unfollow<T>(
    State(live_following_usecase): State<Arc<LiveFollowingUseCase<T>>>,
    auth: AuthUser,
    Path(follow_id): Path<Uuid>,
) -> impl IntoResponse
where
    T: LiveFollowingRepository + Send + Sync,
{
    // find follow by user_id and follow_id
    // If the update time is less than 1 day
    // -	return a failed response
    // else set follow status to Inactive
}

pub async fn list<T>(
    State(live_following_usecase): State<Arc<LiveFollowingUseCase<T>>>,
    auth: AuthUser,
) -> impl IntoResponse
where
    T: LiveFollowingRepository + Send + Sync,
{
    // get (join recording with follow) by user_id
    // response
}
