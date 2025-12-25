use crate::{
    axum_http::auth::AuthUser,
    config::config_model::DotEnvyConfig,
    usecases::{
        plan_resolver::PlanResolver,
        recordings::{HomeRecordingsCursor, RecordingsUseCase},
    },
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
use uuid::Uuid;

use chrono::{DateTime, Utc};
use serde::Deserialize;

const DEFAULT_HOME_LIMIT: i64 = 28;
const MAX_HOME_LIMIT: i64 = 56;

#[derive(Debug, Deserialize)]
pub struct HomeRecordingsQuery {
    limit: Option<i64>,
    cursor_started_at: Option<String>,
    cursor_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FollowsRecordingsQuery {
    live_account_id: Option<String>,
}

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
        .route("/follows/counts", get(list_follows_recording_counts))
        .route(
            "/follows/currently-recording",
            get(follows_currently_recording),
        )
        .with_state(Arc::new(usecase))
}

pub async fn list_home_recordings<R, P, S>(
    State(usecase): State<Arc<RecordingsUseCase<R, P, S>>>,
    AuthUser { user_id, .. }: AuthUser,
    Query(query): Query<HomeRecordingsQuery>,
) -> impl IntoResponse
where
    R: RecordingViewRepository + Send + Sync + 'static,
    P: PlanRepository + Send + Sync + 'static,
    S: SubscriptionRepository + Send + Sync + 'static,
{
    info!(%user_id, "recordings: home list request received");
    let limit = query.limit.unwrap_or(DEFAULT_HOME_LIMIT);
    if limit <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            "limit must be a positive number".to_string(),
        )
            .into_response();
    }
    if limit > MAX_HOME_LIMIT {
        return (
            StatusCode::BAD_REQUEST,
            format!("limit must be <= {}", MAX_HOME_LIMIT),
        )
            .into_response();
    }

    let cursor = match (query.cursor_started_at, query.cursor_id) {
        (None, None) => None,
        (Some(_), None) | (None, Some(_)) => {
            return (
                StatusCode::BAD_REQUEST,
                "cursor_started_at and cursor_id must be provided together".to_string(),
            )
                .into_response();
        }
        (Some(raw_started_at), Some(raw_id)) => {
            let started_at = match DateTime::parse_from_rfc3339(&raw_started_at) {
                Ok(parsed) => parsed.with_timezone(&Utc),
                Err(_) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        "cursor_started_at must be RFC3339 timestamp".to_string(),
                    )
                        .into_response();
                }
            };
            let id = match Uuid::parse_str(&raw_id) {
                Ok(parsed) => parsed,
                Err(_) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        "cursor_id must be a valid UUID".to_string(),
                    )
                        .into_response();
                }
            };
            Some(HomeRecordingsCursor { started_at, id })
        }
    };

    match usecase.list_home_recordings(user_id, limit, cursor).await {
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
    Query(query): Query<FollowsRecordingsQuery>,
) -> impl IntoResponse
where
    R: RecordingViewRepository + Send + Sync + 'static,
    P: PlanRepository + Send + Sync + 'static,
    S: SubscriptionRepository + Send + Sync + 'static,
{
    info!(%user_id, "recordings: follows list request received");
    let live_account_id = match query.live_account_id {
        Some(raw_id) => match Uuid::parse_str(&raw_id) {
            Ok(parsed) => Some(parsed),
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    "live_account_id must be a valid UUID".to_string(),
                )
                    .into_response();
            }
        },
        None => None,
    };

    match usecase
        .list_follows_recordings(user_id, live_account_id)
        .await
    {
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

pub async fn list_follows_recording_counts<R, P, S>(
    State(usecase): State<Arc<RecordingsUseCase<R, P, S>>>,
    AuthUser { user_id, .. }: AuthUser,
) -> impl IntoResponse
where
    R: RecordingViewRepository + Send + Sync + 'static,
    P: PlanRepository + Send + Sync + 'static,
    S: SubscriptionRepository + Send + Sync + 'static,
{
    info!(%user_id, "recordings: follows counts request received");
    match usecase.list_follows_recording_counts(user_id).await {
        Ok(counts) => Json(counts).into_response(),
        Err(err) => {
            error!(
                %user_id,
                error = ?err,
                "recordings: failed to list follows recording counts"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load recording counts".to_string(),
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
