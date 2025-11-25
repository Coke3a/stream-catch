use std::sync::Arc;

use anyhow::Result;
use uuid::Uuid;

use crate::domain::{
    repositories::recording_dashboard::RecordingDashboardRepository,
    value_objects::recordings::RecordingModel,
};

pub struct RecordingDashboardUseCase<T>
where
    T: RecordingDashboardRepository,
{
    repository: Arc<T>,
}

impl<T> RecordingDashboardUseCase<T>
where
    T: RecordingDashboardRepository,
{
    pub fn new(repository: Arc<T>) -> Self {
        Self { repository }
    }
}

impl<T> RecordingDashboardUseCase<T>
where
    T: RecordingDashboardRepository,
{
    pub async fn list_recording(&self, user_id: Uuid) -> Result<Vec<RecordingModel>> {
        unimplemented!()
    }
}
