use anyhow::Result;
use async_trait::async_trait;
use diesel::{RunQueryDsl, dsl::count_star, prelude::*};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    domain,
    infra::db::postgres::{
        postgres_connection::PgPoolSquad,
        schema::{follows, live_accounts, recordings},
    },
};
use domain::{
    entities::{live_accounts::LiveAccountEntity, recordings::RecordingEntity},
    repositories::recording_view::RecordingViewRepository,
    value_objects::enums::{follow_statuses::FollowStatus, recording_statuses::RecordingStatus},
};

pub struct RecordingViewPostgres {
    db_pool: Arc<PgPoolSquad>,
}

impl RecordingViewPostgres {
    pub fn new(db_pool: Arc<PgPoolSquad>) -> Self {
        Self { db_pool }
    }

    fn view_window_filter_sql(retention_days: i64) -> String {
        let retention_days = retention_days.max(0);
        format!(
            "now() >= GREATEST(follows.created_at, recordings.started_at) \
             AND now() < (GREATEST(follows.created_at, recordings.started_at) + (INTERVAL '1 day' * {}))",
            retention_days
        )
    }
}

#[async_trait]
impl RecordingViewRepository for RecordingViewPostgres {
    async fn list_home_entitled_recordings(
        &self,
        user_id: Uuid,
        retention_days: i64,
    ) -> Result<Vec<(RecordingEntity, LiveAccountEntity)>> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let view_filter_sql = Self::view_window_filter_sql(retention_days);

        let results = recordings::table
            .inner_join(live_accounts::table.on(recordings::live_account_id.eq(live_accounts::id)))
            .inner_join(follows::table.on(follows::live_account_id.eq(recordings::live_account_id)))
            .select((RecordingEntity::as_select(), LiveAccountEntity::as_select()))
            .filter(follows::user_id.eq(user_id))
            .filter(follows::status.eq(FollowStatus::Active.to_string()))
            .filter(recordings::status.eq(RecordingStatus::Ready.to_string()))
            .filter(diesel::dsl::sql::<diesel::sql_types::Bool>(
                &view_filter_sql,
            ))
            .order(recordings::started_at.desc())
            .load::<(RecordingEntity, LiveAccountEntity)>(&mut conn)?;

        Ok(results)
    }

    async fn list_follows_entitled_recordings(
        &self,
        user_id: Uuid,
        retention_days: i64,
    ) -> Result<Vec<RecordingEntity>> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let view_filter_sql = Self::view_window_filter_sql(retention_days);

        let results = recordings::table
            .inner_join(follows::table.on(follows::live_account_id.eq(recordings::live_account_id)))
            .select(RecordingEntity::as_select())
            .filter(follows::user_id.eq(user_id))
            .filter(follows::status.eq(FollowStatus::Active.to_string()))
            .filter(recordings::status.eq(RecordingStatus::Ready.to_string()))
            .filter(diesel::dsl::sql::<diesel::sql_types::Bool>(
                &view_filter_sql,
            ))
            .order(recordings::started_at.desc())
            .load::<RecordingEntity>(&mut conn)?;

        Ok(results)
    }

    async fn list_currently_recording_live_account_ids(&self, user_id: Uuid) -> Result<Vec<Uuid>> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let statuses = vec![
            RecordingStatus::LiveRecording.to_string(),
            RecordingStatus::LiveEnd.to_string(),
            RecordingStatus::WaitingUpload.to_string(),
            RecordingStatus::Uploading.to_string(),
        ];

        let live_account_ids = recordings::table
            .inner_join(follows::table.on(follows::live_account_id.eq(recordings::live_account_id)))
            .filter(follows::user_id.eq(user_id))
            .filter(follows::status.eq(FollowStatus::Active.to_string()))
            .filter(recordings::status.eq_any(statuses))
            .select(recordings::live_account_id)
            .distinct()
            .load::<Uuid>(&mut conn)?;

        Ok(live_account_ids)
    }

    async fn count_home_entitled_recordings(
        &self,
        user_id: Uuid,
        retention_days: i64,
    ) -> Result<i64> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let view_filter_sql = Self::view_window_filter_sql(retention_days);

        let total = recordings::table
            .inner_join(follows::table.on(follows::live_account_id.eq(recordings::live_account_id)))
            .filter(follows::user_id.eq(user_id))
            .filter(follows::status.eq(FollowStatus::Active.to_string()))
            .filter(recordings::status.eq(RecordingStatus::Ready.to_string()))
            .filter(diesel::dsl::sql::<diesel::sql_types::Bool>(
                &view_filter_sql,
            ))
            .select(count_star())
            .first::<i64>(&mut conn)?;

        Ok(total)
    }

    async fn count_currently_recording(&self, user_id: Uuid) -> Result<i64> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let statuses = vec![
            RecordingStatus::LiveRecording.to_string(),
            RecordingStatus::LiveEnd.to_string(),
            RecordingStatus::WaitingUpload.to_string(),
            RecordingStatus::Uploading.to_string(),
        ];

        let total = recordings::table
            .inner_join(follows::table.on(follows::live_account_id.eq(recordings::live_account_id)))
            .filter(follows::user_id.eq(user_id))
            .filter(follows::status.eq(FollowStatus::Active.to_string()))
            .filter(recordings::status.eq_any(statuses))
            .select(count_star())
            .first::<i64>(&mut conn)?;

        Ok(total)
    }
}
