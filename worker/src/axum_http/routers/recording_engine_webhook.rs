use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use crates::domain;
use domain::value_objects::recording_engine_webhook::{
    RecordingEngineErrorWebhook, RecordingEngineFileFinishWebhook,
    RecordingEngineLiveStartWebhook,
    RecordingEngineTransmuxFinishWebhook,
};
use tracing::{error, info, warn};
use url::Url;
use uuid::Uuid;

use crate::usecases::recording_engine_webhook::RecordingEngineWebhookUseCase;

pub fn routes(usecase: Arc<RecordingEngineWebhookUseCase>) -> Router {
    Router::new()
        .route("/live-start", post(live_start))
        .route("/video-transmux-finish", post(video_transmux_finish))
        .route("/video-uploading", post(video_uploading))
        .route("/error", post(error_webhook))
        .with_state(usecase)
}

pub async fn live_start(
    State(usecase): State<Arc<RecordingEngineWebhookUseCase>>,
    Json(payload): Json<RecordingEngineLiveStartWebhook>,
) -> Response {
    info!(
        payload = ?payload,
        "recording_engine_webhook: live_start received"
    );
    match usecase.handle_live_start(payload).await {
        Ok(recording_id) => success_response(recording_id),
        Err(err) => map_error("live_start", err),
    }
}

pub async fn video_transmux_finish(
    State(usecase): State<Arc<RecordingEngineWebhookUseCase>>,
    Json(payload): Json<RecordingEngineTransmuxFinishWebhook>,
) -> Response {
    info!(
        payload = ?payload,
        "recording_engine_webhook: video_transmux_finish received"
    );
    match usecase.handle_transmux_finish(payload).await {
        Ok(recording_id) => success_response(recording_id),
        Err(err) => map_error("video_transmux_finish", err),
    }
}

pub async fn video_uploading(
    State(usecase): State<Arc<RecordingEngineWebhookUseCase>>,
    Json(payload): Json<RecordingEngineFileFinishWebhook>,
) -> Response {
    info!(
        payload = ?payload,
        "recording_engine_webhook: video_uploading received"
    );
    match usecase
        .handle_uploading_status(payload.data.platform.clone(), payload.data.channel.clone())
        .await
    {
        Ok(recording_id) => success_response(recording_id),
        Err(err) => map_error("video_uploading", err),
    }
}

pub async fn error_webhook(
    State(usecase): State<Arc<RecordingEngineWebhookUseCase>>,
    Json(payload): Json<RecordingEngineErrorWebhook>,
) -> Response {
    info!(
        payload = ?payload,
        "recording_engine_webhook: error received"
    );

    if let Ok(raw_url) = std::env::var("DISCORD_RECORDING_ENGINE_ERROR_WEBHOOK_URL") {
        let trimmed = raw_url.trim();
        if !trimmed.is_empty() {
            match Url::parse(trimmed) {
                Ok(webhook_url) => {
                    let content = recording_engine_alert_content(&payload);
                    if let Err(err) =
                        crates::observability::send_discord_webhook(webhook_url, content).await
                    {
                        warn!(
                            error = %err,
                            "recording_engine_webhook: failed to send recording engine alert"
                        );
                    }
                }
                Err(err) => {
                    warn!(
                        error = %err,
                        "recording_engine_webhook: DISCORD_RECORDING_ENGINE_ERROR_WEBHOOK_URL is invalid"
                    );
                }
            }
        }
    }

    match usecase.handle_error(payload).await {
        Ok(recording_id) => success_response(recording_id),
        Err(err) => map_error("error", err),
    }
}

fn success_response(recording_id: Uuid) -> Response {
    (StatusCode::OK, recording_id.to_string()).into_response()
}

fn map_error(label: &str, err: anyhow::Error) -> Response {
    let message = err.to_string();
    let status = if message.contains("required") || message.contains("Unsupported platform") {
        StatusCode::BAD_REQUEST
    } else if message.contains("not found") {
        StatusCode::INTERNAL_SERVER_ERROR
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };

    error!(
        status = status.as_u16(),
        error = %message,
        "recording_engine_webhook: {} webhook failed",
        label
    );
    (status, message).into_response()
}

fn recording_engine_alert_content(payload: &RecordingEngineErrorWebhook) -> String {
    let data = &payload.data;
    let platform = data.platform.as_deref().unwrap_or("unknown");
    let channel = data.channel.as_deref().unwrap_or("unknown");
    let error_message = data.error.as_deref().unwrap_or("missing error");

    format!(
        "**Recording engine error**\nid: `{}`\nts: `{}`\ntype: `{}`\nplatform: `{}`\nchannel: `{}`\nerror: {}",
        payload.id,
        payload.ts.to_rfc3339(),
        payload.type_,
        platform,
        channel,
        error_message
    )
}
