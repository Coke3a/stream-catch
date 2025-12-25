use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use mockall::automock;
use uuid::Uuid;

use crate::domain::entities::{live_accounts::LiveAccountEntity, recordings::RecordingEntity};

#[async_trait]
#[automock]
pub trait RecordingViewRepository {
    async fn list_home_entitled_recordings(
        &self,
        user_id: Uuid,
        retention_days: i64,
        limit: i64,
        cursor_started_at: Option<DateTime<Utc>>,
        cursor_id: Option<Uuid>,
    ) -> Result<Vec<(RecordingEntity, LiveAccountEntity)>>;

    async fn list_follows_entitled_recordings(
        &self,
        user_id: Uuid,
        retention_days: i64,
        live_account_id: Option<Uuid>,
    ) -> Result<Vec<RecordingEntity>>;

    async fn count_follows_entitled_recordings(
        &self,
        user_id: Uuid,
        retention_days: i64,
    ) -> Result<Vec<(Uuid, i64)>>;

    async fn list_currently_recording_live_account_ids(&self, user_id: Uuid) -> Result<Vec<Uuid>>;

    async fn count_home_entitled_recordings(
        &self,
        user_id: Uuid,
        retention_days: i64,
    ) -> Result<i64>;

    async fn count_currently_recording(&self, user_id: Uuid) -> Result<i64>;
}
