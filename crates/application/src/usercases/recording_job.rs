use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;

use domain::{
    repositories::recording_engine_webhook::RecordingJobRepository,
    value_objects::{
        jobs::{InsertJobModel, UpdateJobModel},
        recordings::InsertRecordingModel,
    },
};

pub struct RecordingJobUseCase<T>
where
    T: RecordingJobRepository,
{
    repository: Arc<T>,
}

impl<T> RecordingJobUseCase<T>
where
    T: RecordingJobRepository,
{
    pub fn new(repository: Arc<T>) -> Self {
        Self { repository }
    }
}

impl<T> RecordingJobUseCase<T>
where
    T: RecordingJobRepository,
{
    pub async fn upload_recording_job_start(&self, insert_job_model: InsertJobModel) -> Result<()> {
        unimplemented!()
    }

    pub async fn upload_recording_job_end(
        &self,
        update_recording_model: UpdateJobModel,
    ) -> Result<()> {
        unimplemented!()
    }
}
