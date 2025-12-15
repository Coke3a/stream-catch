use crate::{
    axum_http::auth::AuthUser,
    config::config_model::DotEnvyConfig,
    usecases::{plan_resolver::PlanResolver, recordings::RecordingsUseCase},
};
use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use crates::{
    domain::repositories::{
        plans::PlanRepository, recording_view::RecordingViewRepository,
        subscriptions::SubscriptionRepository,
    },
    infra::db::{
        postgres::postgres_connection::PgPoolSquad,
        repositories::{
            plans::PlanPostgres, recording_view::RecordingViewPostgres,
            subscriptions::SubscriptionPostgres,
        },
    },
};
use std::sync::Arc;
use tracing::{error, info};

pub fn routes(db_pool: Arc<PgPoolSquad>, config: Arc<DotEnvyConfig>) -> Router {
    let recording_view_repository = RecordingViewPostgres::new(Arc::clone(&db_pool));
    let plan_repository = PlanPostgres::new(Arc::clone(&db_pool));
    let subscription_repository = SubscriptionPostgres::new(Arc::clone(&db_pool));

    let plan_resolver = PlanResolver::new(
        Arc::new(plan_repository),
        Arc::new(subscription_repository),
        config.free_plan_id,
    );

    let usecase =
        RecordingsUseCase::new(Arc::new(recording_view_repository), Arc::new(plan_resolver));

    Router::new()
        .route("/home", get(list_home_recordings))
        .route("/home/stats", get(home_stats))
        .route("/follows", get(list_follows_recordings))
        .route(
            "/follows/currently-recording",
            get(follows_currently_recording),
        )
        .with_state(Arc::new(usecase))
}

pub async fn list_home_recordings<R, P, S>(
    State(usecase): State<Arc<RecordingsUseCase<R, P, S>>>,
    AuthUser { user_id, .. }: AuthUser,
) -> impl IntoResponse
where
    R: RecordingViewRepository + Send + Sync + 'static,
    P: PlanRepository + Send + Sync + 'static,
    S: SubscriptionRepository + Send + Sync + 'static,
{
    info!(%user_id, "recordings: home list request received");
    match usecase.list_home_recordings(user_id).await {
        Ok(recordings) => Json(recordings).into_response(),
        Err(err) => {
            error!(%user_id, error = ?err, "recordings: failed to list home recordings");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load recordings".to_string(),
            )
                .into_response()
        }
    }
}

pub async fn list_follows_recordings<R, P, S>(
    State(usecase): State<Arc<RecordingsUseCase<R, P, S>>>,
    AuthUser { user_id, .. }: AuthUser,
) -> impl IntoResponse
where
    R: RecordingViewRepository + Send + Sync + 'static,
    P: PlanRepository + Send + Sync + 'static,
    S: SubscriptionRepository + Send + Sync + 'static,
{
    info!(%user_id, "recordings: follows list request received");
    match usecase.list_follows_recordings(user_id).await {
        Ok(recordings) => Json(recordings).into_response(),
        Err(err) => {
            error!(
                %user_id,
                error = ?err,
                "recordings: failed to list follows recordings"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load recordings".to_string(),
            )
                .into_response()
        }
    }
}

pub async fn home_stats<R, P, S>(
    State(usecase): State<Arc<RecordingsUseCase<R, P, S>>>,
    AuthUser { user_id, .. }: AuthUser,
) -> impl IntoResponse
where
    R: RecordingViewRepository + Send + Sync + 'static,
    P: PlanRepository + Send + Sync + 'static,
    S: SubscriptionRepository + Send + Sync + 'static,
{
    info!(%user_id, "recordings: home stats request received");
    match usecase.home_stats(user_id).await {
        Ok(stats) => Json(stats).into_response(),
        Err(err) => {
            error!(%user_id, error = ?err, "recordings: failed to load home stats");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load stats".to_string(),
            )
                .into_response()
        }
    }
}

pub async fn follows_currently_recording<R, P, S>(
    State(usecase): State<Arc<RecordingsUseCase<R, P, S>>>,
    AuthUser { user_id, .. }: AuthUser,
) -> impl IntoResponse
where
    R: RecordingViewRepository + Send + Sync + 'static,
    P: PlanRepository + Send + Sync + 'static,
    S: SubscriptionRepository + Send + Sync + 'static,
{
    info!(%user_id, "recordings: follows currently-recording request received");
    match usecase.follows_currently_recording(user_id).await {
        Ok(response) => Json(response).into_response(),
        Err(err) => {
            error!(
                %user_id,
                error = ?err,
                "recordings: failed to load follows currently-recording"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load recording status".to_string(),
            )
                .into_response()
        }
    }
}
