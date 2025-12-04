use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use diesel::{RunQueryDsl, insert_into, prelude::*, update};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    domain,
    infra::db::postgres::{
        postgres_connection::PgPoolSquad,
        schema::{plans, subscriptions},
    },
};
use domain::{
    entities::{plans::PlanEntity, subscriptions::InsertSubscriptionEntity},
    repositories::subscriptions::SubscriptionRepository,
    value_objects::enums::subscription_statuses::SubscriptionStatus,
};

pub struct SubscriptionPostgres {
    db_pool: Arc<PgPoolSquad>,
}

impl SubscriptionPostgres {
    pub fn new(db_pool: Arc<PgPoolSquad>) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl SubscriptionRepository for SubscriptionPostgres {
    async fn list_plans(&self) -> Result<Vec<PlanEntity>> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let results = plans::table
            .filter(plans::is_active.eq(true))
            .select(PlanEntity::as_select())
            .load::<PlanEntity>(&mut conn)?;

        Ok(results)
    }
    async fn subscribe(
        &self,
        insert_subscription_entity: InsertSubscriptionEntity,
    ) -> Result<Uuid> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let result = insert_into(subscriptions::table)
            .values(&insert_subscription_entity)
            .returning(subscriptions::id)
            .get_result::<Uuid>(&mut conn)?;

        Ok(result)
    }
    async fn cancel_subscription(&self, subscription_id: Uuid) -> Result<()> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        update(subscriptions::table)
            .filter(subscriptions::id.eq(subscription_id))
            .set((
                subscriptions::status.eq(SubscriptionStatus::Canceled.to_string()),
                subscriptions::canceled_at.eq(Some(Utc::now())),
            ))
            .execute(&mut conn)?;

        Ok(())
    }
    async fn check_current_user_subscription(&self, user_id: Uuid) -> Result<bool> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let current_subscription = subscriptions::table
            .filter(subscriptions::user_id.eq(user_id))
            .filter(subscriptions::status.eq(SubscriptionStatus::Active.to_string()))
            .filter(subscriptions::ends_at.gt(Utc::now()))
            .select(subscriptions::id)
            .first::<Uuid>(&mut conn)
            .optional()?;

        Ok(current_subscription.is_some())
    }
}
