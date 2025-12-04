use anyhow::Result;
use async_trait::async_trait;
use diesel::{RunQueryDsl, prelude::*};
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
    entities::recordings::RecordingEntity,
    repositories::recording_dashboard::RecordingDashboardRepository,
    value_objects::{
        enums::{follow_statuses::FollowStatus, sort_order::SortOrder},
        recordings::ListRecordingsFilter,
    },
};

pub struct RecordingDashboardPostgres {
    db_pool: Arc<PgPoolSquad>,
}

impl RecordingDashboardPostgres {
    pub fn new(db_pool: Arc<PgPoolSquad>) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl RecordingDashboardRepository for RecordingDashboardPostgres {
    async fn list_recording(
        &self,
        user_id: Uuid,
        filter: &ListRecordingsFilter,
    ) -> Result<Vec<RecordingEntity>> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let mut query = recordings::table
            .inner_join(live_accounts::table.on(recordings::live_account_id.eq(live_accounts::id)))
            .inner_join(follows::table.on(follows::live_account_id.eq(recordings::live_account_id)))
            .select(RecordingEntity::as_select())
            .filter(follows::user_id.eq(user_id))
            .filter(follows::status.ne(FollowStatus::Inactive.to_string()))
            .into_boxed();

        if let Some(live_account_id) = filter.live_account_id {
            query = query.filter(recordings::live_account_id.eq(live_account_id));
        }

        if let Some(platform) = &filter.platform {
            query = query.filter(live_accounts::platform.eq(platform.to_string()));
        }

        if let Some(status) = &filter.status {
            query = query.filter(recordings::status.eq(status.to_string()));
        }

        query = match filter.sort_order {
            SortOrder::Asc => query.order(recordings::created_at.asc()),
            SortOrder::Desc => query.order(recordings::created_at.desc()),
        };

        if let Some(limit) = filter.limit {
            query = query.limit(limit);
        }

        let results = query.load::<RecordingEntity>(&mut conn)?;

        Ok(results)
    }
}
