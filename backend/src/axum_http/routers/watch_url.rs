use crate::{
    axum_http::auth::AuthUser, config::config_model::WatchUrl as WatchUrlConfig,
    usecases::watch_url::WatchUrlUseCase,
};
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use crates::{
    domain::repositories::{
        live_following::LiveFollowingRepository, recording_upload::RecordingUploadRepository,
    },
    infra::db::{
        postgres::postgres_connection::PgPoolSquad,
        repositories::{
            live_following::LiveFollowingPostgres, recording_upload::RecordingUploadPostgres,
        },
    },
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::error;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct WatchUrlQuery {
    recording_id: String,
}

#[derive(Debug, Serialize)]
pub struct WatchUrlResponse {
    pub url: String,
}

pub fn routes(db_pool: Arc<PgPoolSquad>, config: WatchUrlConfig) -> Router {
    let recording_repository = RecordingUploadPostgres::new(Arc::clone(&db_pool));
    let live_following_repository = LiveFollowingPostgres::new(Arc::clone(&db_pool));

    let usecase = WatchUrlUseCase::new(
        Arc::new(recording_repository),
        Arc::new(live_following_repository),
        config,
    );

    Router::new()
        .route(
            "/",
            get(generate_watch_url::<RecordingUploadPostgres, LiveFollowingPostgres>),
        )
        .with_state(Arc::new(usecase))
}

pub async fn generate_watch_url<R, F>(
    State(usecase): State<Arc<WatchUrlUseCase<R, F>>>,
    AuthUser { user_id, .. }: AuthUser,
    Query(query): Query<WatchUrlQuery>,
) -> impl IntoResponse
where
    R: RecordingUploadRepository + Send + Sync,
    F: LiveFollowingRepository + Send + Sync,
{
    let recording_id = match Uuid::parse_str(&query.recording_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                "Invalid recording_id format".to_string(),
            )
                .into_response();
        }
    };

    match usecase.generate_watch_url(user_id, recording_id).await {
        Ok(url) => (StatusCode::OK, Json(WatchUrlResponse { url })).into_response(),
        Err(err) => {
            let message = err.to_string();
            let status = if message.contains("Recording not found") {
                StatusCode::NOT_FOUND
            } else if message.contains("Follow is not active") {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };

            if status.is_server_error() {
                error!(
                    error = %message,
                    %user_id,
                    %recording_id,
                    "watch_url: failed to generate url"
                );
            }

            (status, message).into_response()
        }
    }
}
