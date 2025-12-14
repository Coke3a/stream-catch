use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
    response::{IntoResponse, Response},
    routing::post,
};
use serde::{Deserialize, Serialize};
use tracing::error;
use uuid::Uuid;

use crate::{
    config::config_model::DotEnvyConfig,
    usecases::cleanup_expired_recordings::{
        CleanupExpiredRecordingsParams, CleanupExpiredRecordingsUseCase,
    },
};

// Run example
//   curl -X POST "http://localhost:$SERVER_PORT_WORKER/internal/v1/cleanup/recordings" \
//     -H "Authorization: Bearer $INTERNAL_CLEANUP_TOKEN" \
//     -H "Content-Type: application/json" \
//     -d '{"older_than_days":60,"limit":100,"dry_run":true}'

#[derive(Clone)]
pub struct CleanupRouteState {
    config: Arc<DotEnvyConfig>,
    usecase: Arc<CleanupExpiredRecordingsUseCase>,
}

pub fn routes(config: Arc<DotEnvyConfig>, usecase: Arc<CleanupExpiredRecordingsUseCase>) -> Router {
    Router::new()
        .route("/recordings", post(cleanup_recordings))
        .with_state(CleanupRouteState { config, usecase })
}

#[derive(Debug, Deserialize)]
pub struct CleanupRecordingsRequest {
    pub older_than_days: Option<i64>,
    pub limit: Option<i64>,
    pub dry_run: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CleanupRecordingsResponse {
    pub scanned: usize,
    pub deleted: usize,
    pub skipped_video_delete_failed: usize,
    pub cover_delete_failed: usize,
    pub updated_db: usize,
    pub dry_run: bool,
    pub candidate_ids: Vec<Uuid>,
    pub deleted_ids: Vec<Uuid>,
    pub skipped_ids: Vec<Uuid>,
    pub cover_failed_ids: Vec<Uuid>,
}

pub async fn cleanup_recordings(
    State(state): State<CleanupRouteState>,
    headers: HeaderMap,
    Json(payload): Json<CleanupRecordingsRequest>,
) -> Response {
    let expected_token = match state.config.cleanup.internal_token.as_deref() {
        Some(token) => token,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "cleanup token is not configured",
            )
                .into_response();
        }
    };

    if let Err(status) = authorize_bearer(&headers, expected_token) {
        return (status, "unauthorized").into_response();
    }

    let older_than_days = payload
        .older_than_days
        .unwrap_or(state.config.cleanup.default_retention_days);
    let params = CleanupExpiredRecordingsParams {
        older_than_days,
        limit: payload.limit,
        dry_run: payload.dry_run.unwrap_or(false),
    };

    match state.usecase.run(params.clone()).await {
        Ok(result) => Json(CleanupRecordingsResponse {
            scanned: result.scanned,
            deleted: result.deleted,
            skipped_video_delete_failed: result.skipped_video_delete_failed,
            cover_delete_failed: result.cover_delete_failed,
            updated_db: result.updated_db,
            dry_run: params.dry_run,
            candidate_ids: result.candidate_ids,
            deleted_ids: result.deleted_ids,
            skipped_ids: result.skipped_ids,
            cover_failed_ids: result.cover_failed_ids,
        })
        .into_response(),
        Err(err) => {
            error!(error = ?err, "cleanup_recordings: usecase failed");
            (StatusCode::INTERNAL_SERVER_ERROR, "cleanup failed").into_response()
        }
    }
}

fn authorize_bearer(headers: &HeaderMap, expected_token: &str) -> Result<(), StatusCode> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token = auth
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if token == expected_token {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}
