use std::sync::Arc;
use anyhow::Result;
use axum::async_trait;
use uuid::Uuid;

use crate::{domain::{entities::{jobs::InsertJobEntity, recordings::{InsertRecordingEntity, UpdateRecordingEntity}}, repositories::recording_job::RecordingJobRepository}, infrastructure::postgres::postgres_connection::PgPoolSquad};

pub struct RecordingJobPostgres {
    db_pool: Arc<PgPoolSquad>
}

impl RecordingJobPostgres {
    pub fn new(db_pool: Arc<PgPoolSquad>) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl RecordingJobRepository for RecordingJobPostgres {
    async fn webhook_recording_start(&self, insert_recording_entity: InsertRecordingEntity) -> Result<i64> {
        unimplemented!()
    }
    async fn webhook_recording_end(&self, update_recording_entity: UpdateRecordingEntity) -> Result<i64> {
        unimplemented!()
    }
    async fn upload_recording_job_start(&self, insert_job_entity: InsertJobEntity) -> Result<i64> {
        unimplemented!()
    }
    async fn upload_recording_job_end(&self, update_recording_entity: UpdateRecordingEntity) -> Result<i64> {
        unimplemented!()
    }
}