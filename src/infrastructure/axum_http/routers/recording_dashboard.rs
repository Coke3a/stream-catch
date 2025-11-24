use std::sync::Arc;

use axum::{Extension, Router, extract::State, response::IntoResponse};
use tracing_subscriber::field::RecordFields;
use uuid::Uuid;

use crate::{
    application::usercases::recording_dashboard::RecordingDashboardUseCase,
    domain::repositories::recording_dashboard::RecordingDashboardRepository,
    infrastructure::postgres::{
        postgres_connection::PgPoolSquad,
        repositories::recording_dashboard::RecordingDashboardPostgres,
    },
};

pub fn routes(db_pool: Arc<PgPoolSquad>) -> Router {
    let recording_dashboard_repository = RecordingDashboardPostgres::new(Arc::clone(&db_pool));
    let recording_dashboard_usecase =
        RecordingDashboardUseCase::new(Arc::new(recording_dashboard_repository));

    Router::new()
}

pub async fn list_recording<T>(
    State(recording_dashboard_usecase): State<Arc<RecordingDashboardUseCase<T>>>,
    Extension(user_id): Extension<Uuid>,
) -> impl IntoResponse
where
    T: RecordingDashboardRepository + Send + Sync,
{
}
