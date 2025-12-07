use anyhow::Result;
use async_trait::async_trait;
use diesel::{RunQueryDsl, prelude::*};
use std::sync::Arc;
use uuid::Uuid;

use crate::domain;
use crate::infra::db::postgres::{postgres_connection::PgPoolSquad, schema::plans};
use domain::{
    entities::plans::{PlanEntity, PlanRow},
    repositories::plans::PlanRepository,
};

pub struct PlanPostgres {
    db_pool: Arc<PgPoolSquad>,
}

impl PlanPostgres {
    pub fn new(db_pool: Arc<PgPoolSquad>) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl PlanRepository for PlanPostgres {
    async fn find_by_id(&self, plan_id: Uuid) -> Result<PlanEntity> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let row = plans::table
            .filter(plans::id.eq(plan_id))
            .filter(plans::is_active.eq(true))
            .first::<PlanRow>(&mut conn)?;

        Ok(row.into())
    }

    async fn find_active_plan_by_id(&self, plan_id: Uuid) -> Result<PlanEntity> {
        self.find_by_id(plan_id).await
    }

    async fn list_active_plans(&self) -> Result<Vec<PlanEntity>> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let rows = plans::table
            .filter(plans::is_active.eq(true))
            .select(PlanRow::as_select())
            .load::<PlanRow>(&mut conn)?;

        Ok(rows.into_iter().map(PlanEntity::from).collect())
    }
}
