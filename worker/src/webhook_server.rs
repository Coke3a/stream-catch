use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use application::usercases::recording_engine_webhook::RecordingEngineWebhookUseCase;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use domain::value_objects::recording_engine_webhook::{
    RecordingEngineFileFinishWebhook, RecordingEngineLiveStartWebhook,
    RecordingEngineTransmuxFinishWebhook,
};
use tokio::net::TcpListener;
use tracing::{error, info};
use uuid::Uuid;

pub fn router(usecase: Arc<RecordingEngineWebhookUseCase>) -> Router {
    Router::new()
        .route("/live-start", post(live_start))
        .route("/video-file-finish", post(video_file_finish))
        .route("/video-transmux-finish", post(video_transmux_finish))
        .route("/video-uploading", post(video_uploading))
        .with_state(usecase)
}

pub async fn start_webhook_server(
    usecase: Arc<RecordingEngineWebhookUseCase>,
    port: u16,
) -> Result<()> {
    let app = Router::new().nest("/internal/recording-engine", router(usecase));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await?;
    info!("Recording engine webhook server running on {}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}

async fn live_start(
    State(usecase): State<Arc<RecordingEngineWebhookUseCase>>,
    Json(payload): Json<RecordingEngineLiveStartWebhook>,
) -> Response {
    match usecase.handle_live_start(payload).await {
        Ok(recording_id) => success_response(recording_id),
        Err(err) => map_error("live_start", err),
    }
}

async fn video_file_finish(
    State(usecase): State<Arc<RecordingEngineWebhookUseCase>>,
    Json(payload): Json<RecordingEngineFileFinishWebhook>,
) -> Response {
    match usecase.handle_file_finish(payload).await {
        Ok(recording_id) => success_response(recording_id),
        Err(err) => map_error("video_file_finish", err),
    }
}

async fn video_transmux_finish(
    State(usecase): State<Arc<RecordingEngineWebhookUseCase>>,
    Json(payload): Json<RecordingEngineTransmuxFinishWebhook>,
) -> Response {
    match usecase.handle_transmux_finish(payload).await {
        Ok(recording_id) => success_response(recording_id),
        Err(err) => map_error("video_transmux_finish", err),
    }
}

async fn video_uploading(
    State(usecase): State<Arc<RecordingEngineWebhookUseCase>>,
    Json(payload): Json<RecordingEngineFileFinishWebhook>,
) -> Response {
    match usecase
        .handle_uploading_status(payload.data.platform.clone(), payload.data.channel.clone())
        .await
    {
        Ok(recording_id) => success_response(recording_id),
        Err(err) => map_error("video_uploading", err),
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

    error!("{label} webhook failed: {message}");
    (status, message).into_response()
}
