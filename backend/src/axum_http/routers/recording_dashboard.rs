use std::sync::Arc;

use crate::auth::AuthUser;
use axum::{Router, extract::State, response::IntoResponse, routing::get};
use tracing_subscriber::field::RecordFields;
use uuid::Uuid;

use application::usercases::recording_dashboard::RecordingDashboardUseCase;
use domain::repositories::recording_dashboard::RecordingDashboardRepository;
use infra::postgres::{
    postgres_connection::PgPoolSquad,
    repositories::recording_dashboard::RecordingDashboardPostgres,
};

pub fn routes(db_pool: Arc<PgPoolSquad>) -> Router {
    let recording_dashboard_repository = RecordingDashboardPostgres::new(Arc::clone(&db_pool));
    let recording_dashboard_usecase =
        RecordingDashboardUseCase::new(Arc::new(recording_dashboard_repository));

    Router::new()
        .route("/", get(list_recording))
        .with_state(Arc::new(recording_dashboard_usecase))
}

pub async fn list_recording<T>(
    State(recording_dashboard_usecase): State<Arc<RecordingDashboardUseCase<T>>>,
    auth: AuthUser,
) -> impl IntoResponse
where
    T: RecordingDashboardRepository + Send + Sync,
{
}
