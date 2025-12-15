use anyhow::Result;
use chrono::{DateTime, Utc};
use crates::domain::{
    entities::{live_accounts::LiveAccountEntity, recordings::RecordingEntity},
    repositories::{
        plans::PlanRepository, recording_view::RecordingViewRepository,
        subscriptions::SubscriptionRepository,
    },
};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::usecases::plan_resolver::PlanResolver;

#[derive(Debug, Serialize)]
pub struct RecordingDto {
    pub id: Uuid,
    pub live_account_id: Uuid,
    pub recording_key: Option<String>,
    pub title: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_sec: Option<i32>,
    pub size_bytes: Option<i64>,
    pub storage_path: Option<String>,
    pub storage_temp_path: Option<String>,
    pub status: String,
    pub poster_storage_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<RecordingEntity> for RecordingDto {
    fn from(value: RecordingEntity) -> Self {
        Self {
            id: value.id,
            live_account_id: value.live_account_id,
            recording_key: value.recording_key,
            title: value.title,
            started_at: value.started_at,
            ended_at: value.ended_at,
            duration_sec: value.duration_sec,
            size_bytes: value.size_bytes,
            storage_path: value.storage_path,
            storage_temp_path: value.storage_temp_path,
            status: value.status,
            poster_storage_path: value.poster_storage_path,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LiveAccountSnippetDto {
    pub platform: String,
    pub account_id: String,
    pub canonical_url: String,
}

impl From<LiveAccountEntity> for LiveAccountSnippetDto {
    fn from(value: LiveAccountEntity) -> Self {
        Self {
            platform: value.platform,
            account_id: value.account_id,
            canonical_url: value.canonical_url,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RecordingHomeDto {
    #[serde(flatten)]
    pub recording: RecordingDto,
    pub live_accounts: LiveAccountSnippetDto,
}

impl RecordingHomeDto {
    fn from_entities(recording: RecordingEntity, live_account: LiveAccountEntity) -> Self {
        Self {
            recording: RecordingDto::from(recording),
            live_accounts: LiveAccountSnippetDto::from(live_account),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct HomeRecordingStatsDto {
    pub total_recordings: i64,
    pub currently_recording: i64,
}

#[derive(Debug, Serialize)]
pub struct CurrentlyRecordingLiveAccountsDto {
    pub live_account_ids: Vec<Uuid>,
}

pub struct RecordingsUseCase<R, P, S>
where
    R: RecordingViewRepository + Send + Sync + 'static,
    P: PlanRepository + Send + Sync + 'static,
    S: SubscriptionRepository + Send + Sync + 'static,
{
    recording_view_repo: Arc<R>,
    plan_resolver: Arc<PlanResolver<P, S>>,
}

impl<R, P, S> RecordingsUseCase<R, P, S>
where
    R: RecordingViewRepository + Send + Sync + 'static,
    P: PlanRepository + Send + Sync + 'static,
    S: SubscriptionRepository + Send + Sync + 'static,
{
    pub fn new(recording_view_repo: Arc<R>, plan_resolver: Arc<PlanResolver<P, S>>) -> Self {
        Self {
            recording_view_repo,
            plan_resolver,
        }
    }

    pub async fn list_home_recordings(&self, user_id: Uuid) -> Result<Vec<RecordingHomeDto>> {
        let retention_days = self.effective_retention_days(user_id).await?;
        let recordings = self
            .recording_view_repo
            .list_home_entitled_recordings(user_id, retention_days)
            .await?;

        Ok(recordings
            .into_iter()
            .map(|(recording, live_account)| {
                RecordingHomeDto::from_entities(recording, live_account)
            })
            .collect())
    }

    pub async fn list_follows_recordings(&self, user_id: Uuid) -> Result<Vec<RecordingDto>> {
        let retention_days = self.effective_retention_days(user_id).await?;
        let recordings = self
            .recording_view_repo
            .list_follows_entitled_recordings(user_id, retention_days)
            .await?;

        Ok(recordings.into_iter().map(RecordingDto::from).collect())
    }

    pub async fn home_stats(&self, user_id: Uuid) -> Result<HomeRecordingStatsDto> {
        let retention_days = self.effective_retention_days(user_id).await?;
        let total_recordings = self
            .recording_view_repo
            .count_home_entitled_recordings(user_id, retention_days)
            .await?;
        let currently_recording = self
            .recording_view_repo
            .count_currently_recording(user_id)
            .await?;

        Ok(HomeRecordingStatsDto {
            total_recordings,
            currently_recording,
        })
    }

    pub async fn follows_currently_recording(
        &self,
        user_id: Uuid,
    ) -> Result<CurrentlyRecordingLiveAccountsDto> {
        let live_account_ids = self
            .recording_view_repo
            .list_currently_recording_live_account_ids(user_id)
            .await?;

        Ok(CurrentlyRecordingLiveAccountsDto { live_account_ids })
    }

    async fn effective_retention_days(&self, user_id: Uuid) -> Result<i64> {
        let plan = self
            .plan_resolver
            .resolve_effective_plan_for_user(user_id)
            .await?;
        Ok(i64::from(plan.features.retention_days_or_default().max(0)))
    }
}
