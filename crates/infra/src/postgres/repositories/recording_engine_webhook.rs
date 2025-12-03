use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use diesel::{OptionalExtension, RunQueryDsl, insert_into, prelude::*, update};
use std::sync::Arc;
use uuid::Uuid;

use crate::postgres::{
    postgres_connection::PgPoolSquad,
    schema::{live_accounts, recordings},
};
use domain::{
    entities::{
        live_accounts::LiveAccountEntity,
        recordings::{InsertRecordingEntity, RecordingEntity},
    },
    repositories::recording_engine_webhook::RecordingJobRepository,
    value_objects::enums::{
        live_account_statuses::LiveAccountStatus, recording_statuses::RecordingStatus,
    },
};

pub struct RecordingJobPostgres {
    db_pool: Arc<PgPoolSquad>,
}

impl RecordingJobPostgres {
    pub fn new(db_pool: Arc<PgPoolSquad>) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl RecordingJobRepository for RecordingJobPostgres {
    async fn find_recording_by_live_account_and_status(
        &self,
        platform: String,
        account_id: String,
        status: RecordingStatus,
    ) -> Result<Option<RecordingEntity>> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let result = recordings::table
            .inner_join(live_accounts::table.on(recordings::live_account_id.eq(live_accounts::id)))
            .select(RecordingEntity::as_select())
            .filter(live_accounts::platform.eq(platform))
            .filter(live_accounts::account_id.eq(account_id))
            .filter(recordings::status.eq(status.to_string()))
            .first::<RecordingEntity>(&mut conn)
            .optional()?;

        Ok(result)
    }

    async fn find_recording_by_id(&self, recording_id: Uuid) -> Result<Option<RecordingEntity>> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let result = recordings::table
            .find(recording_id)
            .select(RecordingEntity::as_select())
            .first::<RecordingEntity>(&mut conn)
            .optional()?;

        Ok(result)
    }

    async fn find_live_account_by_platform_and_account_id(
        &self,
        platform: String,
        account_id: String,
    ) -> Result<Option<LiveAccountEntity>> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let result = live_accounts::table
            .filter(live_accounts::platform.eq(platform))
            .filter(live_accounts::account_id.eq(account_id))
            .first::<LiveAccountEntity>(&mut conn)
            .optional()?;

        Ok(result)
    }

    async fn insert(&self, insert_recording_entity: InsertRecordingEntity) -> Result<Uuid> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let result = insert_into(recordings::table)
            .values(&insert_recording_entity)
            .returning(recordings::id)
            .get_result::<Uuid>(&mut conn)?;

        Ok(result)
    }

    async fn update_live_end(&self, recording_id: Uuid, duration: i64) -> Result<Uuid> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let duration_i32 =
            i32::try_from(duration).context("duration value does not fit into i32 column")?;
        let now = Utc::now();

        let result = update(recordings::table.filter(recordings::id.eq(recording_id)))
            .set((
                recordings::duration_sec.eq(Some(duration_i32)),
                recordings::ended_at.eq(Some(now)),
                recordings::status.eq(RecordingStatus::LiveEnd.to_string()),
                recordings::updated_at.eq(now),
            ))
            .returning(recordings::id)
            .get_result::<Uuid>(&mut conn)?;

        Ok(result)
    }

    async fn update_live_transmux_finish(
        &self,
        recording_id: Uuid,
        storage_path: String,
    ) -> Result<Uuid> {
        let mut conn = Arc::clone(&self.db_pool).get()?;
        let now = Utc::now();

        let result = update(recordings::table.filter(recordings::id.eq(recording_id)))
            .set((
                recordings::storage_path.eq(Some(storage_path)),
                recordings::status.eq(RecordingStatus::WaitingUpload.to_string()),
                recordings::updated_at.eq(now),
            ))
            .returning(recordings::id)
            .get_result::<Uuid>(&mut conn)?;

        Ok(result)
    }

    async fn update_file_uploading(&self, recording_id: Uuid) -> Result<Uuid> {
        let mut conn = Arc::clone(&self.db_pool).get()?;
        let now = Utc::now();

        let result = update(recordings::table.filter(recordings::id.eq(recording_id)))
            .set((
                recordings::status.eq(RecordingStatus::Uploading.to_string()),
                recordings::updated_at.eq(now),
            ))
            .returning(recordings::id)
            .get_result::<Uuid>(&mut conn)?;

        Ok(result)
    }

    async fn find_unsynced_live_accounts(&self) -> Result<Vec<LiveAccountEntity>> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let result = live_accounts::table
            .filter(live_accounts::status.eq(LiveAccountStatus::Unsynced.to_string()))
            .select(LiveAccountEntity::as_select())
            .load::<LiveAccountEntity>(&mut conn)?;

        Ok(result)
    }

    async fn update_live_account_status(
        &self,
        live_account_id: Uuid,
        status: LiveAccountStatus,
    ) -> Result<Uuid> {
        let mut conn = Arc::clone(&self.db_pool).get()?;
        let now = Utc::now();

        let result = update(live_accounts::table.filter(live_accounts::id.eq(live_account_id)))
            .set((
                live_accounts::status.eq(status.to_string()),
                live_accounts::updated_at.eq(now),
            ))
            .returning(live_accounts::id)
            .get_result::<Uuid>(&mut conn)?;

        Ok(result)
    }

    async fn mark_recording_ready(
        &self,
        recording_id: Uuid,
        storage_path: String,
        size_bytes: i64,
        duration_sec: i32,
    ) -> Result<Uuid> {
        let mut conn = Arc::clone(&self.db_pool).get()?;
        let now = Utc::now();

        let result = update(recordings::table.filter(recordings::id.eq(recording_id)))
            .set((
                recordings::status.eq(RecordingStatus::Ready.to_string()),
                recordings::storage_path.eq(Some(storage_path)),
                recordings::size_bytes.eq(Some(size_bytes)),
                recordings::duration_sec.eq(Some(duration_sec)),
                recordings::updated_at.eq(now),
            ))
            .returning(recordings::id)
            .get_result::<Uuid>(&mut conn)?;

        Ok(result)
    }
}
