use std::sync::Arc;

use axum::{Json, Router, extract::State, response::IntoResponse};

use crate::{
    application::usercases::oneliverec_webhook::OneLiveRecWebhookUseCase,
    domain::value_objects::oneliverec_webhook::OneLiveRecWebhook,
    infrastructure::postgres::postgres_connection::PgPoolSquad,
};

pub fn routes(db_pool: Arc<PgPoolSquad>) -> Router {
    let oneliverec_webhook_usecase = OneLiveRecWebhookUseCase::new();

    Router::new()
}

pub async fn handle_oneliverec_webhook(
    State(oneliverec_webhook_usecase): State<Arc<OneLiveRecWebhookUseCase>>,
    Json(payload): Json<OneLiveRecWebhook>,
) -> impl IntoResponse {
}
