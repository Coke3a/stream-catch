use anyhow::Result;
use axum::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    domain::{
        entities::recordings::RecordingEntity,
        repositories::recording_dashboard::RecordingDashboardRepository,
        value_objects::recordings::ListRecordingsFilter,
    },
    infrastructure::postgres::postgres_connection::PgPoolSquad,
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
        unimplemented!()
    }
}
