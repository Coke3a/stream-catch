use anyhow::Result;
use async_trait::async_trait;
use mockall::automock;
use uuid::Uuid;

use crate::entities::recordings::RecordingEntity;
use crate::value_objects::recordings::ListRecordingsFilter;

#[async_trait]
#[automock]
pub trait RecordingDashboardRepository {
    async fn list_recording(
        &self,
        user_id: Uuid,
        filter: &ListRecordingsFilter,
    ) -> Result<Vec<RecordingEntity>>;
}
