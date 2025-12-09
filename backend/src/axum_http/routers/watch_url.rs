use crate::{
    axum_http::auth::AuthUser,
    config::config_model::DotEnvyConfig,
    usecases::{plan_resolver::PlanResolver, watch_url::WatchUrlUseCase},
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
        live_following::LiveFollowingRepository, plans::PlanRepository,
        recording_upload::RecordingUploadRepository, subscriptions::SubscriptionRepository,
    },
    infra::db::{
        postgres::postgres_connection::PgPoolSquad,
        repositories::{
            live_following::LiveFollowingPostgres, plans::PlanPostgres,
            recording_upload::RecordingUploadPostgres, subscriptions::SubscriptionPostgres,
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

pub fn routes(db_pool: Arc<PgPoolSquad>, config: Arc<DotEnvyConfig>) -> Router {
    let recording_repository = RecordingUploadPostgres::new(Arc::clone(&db_pool));
    let live_following_repository = LiveFollowingPostgres::new(Arc::clone(&db_pool));
    let plan_repository = PlanPostgres::new(Arc::clone(&db_pool));
    let subscription_repository = SubscriptionPostgres::new(Arc::clone(&db_pool));

    let plan_resolver = PlanResolver::new(
        Arc::new(plan_repository),
        Arc::new(subscription_repository),
        config.free_plan_id,
    );

    let usecase = WatchUrlUseCase::new(
        Arc::new(recording_repository),
        Arc::new(live_following_repository),
        Arc::new(plan_resolver),
        config.watch_url.clone(),
    );

    Router::new()
        .route("/", get(generate_watch_url))
        .with_state(Arc::new(usecase))
}

pub async fn generate_watch_url<R, F, P, S>(
    State(usecase): State<Arc<WatchUrlUseCase<R, F, P, S>>>,
    AuthUser { user_id, .. }: AuthUser,
    Query(query): Query<WatchUrlQuery>,
) -> impl IntoResponse
where
    R: RecordingUploadRepository + Send + Sync + 'static,
    F: LiveFollowingRepository + Send + Sync + 'static,
    P: PlanRepository + Send + Sync + 'static,
    S: SubscriptionRepository + Send + Sync + 'static,
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
            } else if message.contains("Follow is not active")
                || message.contains("Recording exceeds retention window")
            {
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
