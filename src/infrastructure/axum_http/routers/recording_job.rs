use std::sync::Arc;

use axum::{Extension, Router, extract::State, response::IntoResponse};
use tracing_subscriber::field::RecordFields;
use uuid::Uuid;

use crate::{application::usercases::recording_job::RecordingJobUseCase, domain::repositories::recording_job::RecordingJobRepository, infrastructure::postgres::{postgres_connection::PgPoolSquad, repositories::recording_job::RecordingJobPostgres}};

pub fn routes(db_pool: Arc<PgPoolSquad>) -> Router {
    let recording_job_repository = RecordingJobPostgres::new(Arc::clone(&db_pool));
    let recording_job_usecase = RecordingJobUseCase::new(Arc::new(recording_job_repository));
    
    Router::new()
}


// TODO: Fix later

pub async fn webhook_recording_start<T>(
    State(recording_job_usecase): State<Arc<RecordingJobUseCase<T>>>,
) -> impl IntoResponse
where
    T: RecordingJobRepository + Send + Sync,
{
    
}

pub async fn webhook_recording_end<T>(
    State(recording_job_usecase): State<Arc<RecordingJobUseCase<T>>>,
) -> impl IntoResponse
where
    T: RecordingJobRepository + Send + Sync,
{
    
}

pub async fn upload_recording_job_start<T>(
    State(recording_job_usecase): State<Arc<RecordingJobUseCase<T>>>,
) -> impl IntoResponse
where
    T: RecordingJobRepository + Send + Sync,
{
    
}

pub async fn upload_recording_job_end<T>(
    State(recording_job_usecase): State<Arc<RecordingJobUseCase<T>>>,
) -> impl IntoResponse
where
    T: RecordingJobRepository + Send + Sync,
{
    
}