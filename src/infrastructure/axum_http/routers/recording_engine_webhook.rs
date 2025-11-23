use std::sync::Arc;

use axum::{Json, Router, extract::State, response::IntoResponse};

use crate::{
    application::usercases::recording_engine_webhook::RecordingEngineWebhookUseCase,
    domain::value_objects::recording_engine_webhook::RecordingEngineWebhook,
    infrastructure::postgres::postgres_connection::PgPoolSquad,
};

pub fn routes(_db_pool: Arc<PgPoolSquad>) -> Router {
    let _recording_engine_webhook_usecase = RecordingEngineWebhookUseCase::new();

    Router::new()
}

pub async fn handle_recording_engine_webhook(
    State(recording_engine_webhook_usecase): State<Arc<RecordingEngineWebhookUseCase>>,
    Json(payload): Json<RecordingEngineWebhook>,
) -> impl IntoResponse {
    let _usecase = recording_engine_webhook_usecase;
    let _payload = payload;
}
