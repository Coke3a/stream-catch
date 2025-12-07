use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use diesel::{OptionalExtension, RunQueryDsl, insert_into, prelude::*, update};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    domain,
    infra::db::postgres::{postgres_connection::PgPoolSquad, schema::subscriptions},
};
use domain::{
    entities::subscriptions::{InsertSubscriptionEntity, SubscriptionEntity},
    repositories::subscriptions::SubscriptionRepository,
    value_objects::enums::{billing_modes::BillingMode, subscription_statuses::SubscriptionStatus},
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
    async fn find_current_active_subscription(
        &self,
        user_id: Uuid,
    ) -> Result<Option<SubscriptionEntity>> {
        self.find_current_active_subscription_filtered(user_id, None)
            .await
    }

    async fn find_current_active_non_free_subscription(
        &self,
        user_id: Uuid,
        free_plan_id: Uuid,
    ) -> Result<Option<SubscriptionEntity>> {
        self.find_current_active_subscription_filtered(user_id, Some(free_plan_id))
            .await
    }

    async fn create_or_update_subscription_after_checkout(
        &self,
        user_id: Uuid,
        plan_id: Uuid,
        billing_mode: BillingMode,
        starts_at: DateTime<Utc>,
        ends_at: DateTime<Utc>,
        status: SubscriptionStatus,
        provider_subscription_id: Option<String>,
    ) -> Result<Uuid> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        if let Some(existing) = subscriptions::table
            .filter(subscriptions::user_id.eq(user_id))
            .order(subscriptions::starts_at.desc())
            .first::<SubscriptionEntity>(&mut conn)
            .optional()?
        {
            let updated_id = update(subscriptions::table.filter(subscriptions::id.eq(existing.id)))
                .set((
                    subscriptions::plan_id.eq(plan_id),
                    subscriptions::starts_at.eq(starts_at),
                    subscriptions::ends_at.eq(ends_at),
                    subscriptions::billing_mode.eq(billing_mode.to_string()),
                    subscriptions::cancel_at_period_end.eq(false),
                    subscriptions::canceled_at.eq::<Option<DateTime<Utc>>>(None),
                    subscriptions::status.eq(status.to_string()),
                    subscriptions::provider_subscription_id.eq(provider_subscription_id),
                ))
                .returning(subscriptions::id)
                .get_result::<Uuid>(&mut conn)?;

            return Ok(updated_id);
        }

        let insert_subscription_entity = InsertSubscriptionEntity {
            user_id,
            plan_id,
            starts_at,
            ends_at,
            billing_mode: billing_mode.to_string(),
            default_payment_method_id: None,
            cancel_at_period_end: false,
            canceled_at: None,
            provider_subscription_id,
            status: status.to_string(),
        };

        let subscription_id = insert_into(subscriptions::table)
            .values(&insert_subscription_entity)
            .returning(subscriptions::id)
            .get_result::<Uuid>(&mut conn)?;

        Ok(subscription_id)
    }

    async fn cancel_recurring_subscription(&self, user_id: Uuid) -> Result<()> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        if let Some(current) = subscriptions::table
            .filter(subscriptions::user_id.eq(user_id))
            .filter(subscriptions::billing_mode.eq(BillingMode::Recurring.to_string()))
            .filter(subscriptions::status.eq(SubscriptionStatus::Active.to_string()))
            .order(subscriptions::starts_at.desc())
            .first::<SubscriptionEntity>(&mut conn)
            .optional()?
        {
            update(subscriptions::table.filter(subscriptions::id.eq(current.id)))
                .set((
                    subscriptions::status.eq(SubscriptionStatus::Canceled.to_string()),
                    subscriptions::cancel_at_period_end.eq(true),
                    subscriptions::canceled_at.eq(Some(Utc::now())),
                ))
                .execute(&mut conn)?;
        }

        Ok(())
    }

    async fn list_active_subscriptions(&self) -> Result<Vec<SubscriptionEntity>> {
        let mut conn = Arc::clone(&self.db_pool).get()?;
        let now = Utc::now();

        let subscriptions = subscriptions::table
            .filter(subscriptions::status.eq(SubscriptionStatus::Active.to_string()))
            .filter(subscriptions::starts_at.le(now))
            .filter(subscriptions::ends_at.gt(now))
            .load::<SubscriptionEntity>(&mut conn)?;

        Ok(subscriptions)
    }
}

impl SubscriptionPostgres {
    async fn find_current_active_subscription_filtered(
        &self,
        user_id: Uuid,
        exclude_plan_id: Option<Uuid>,
    ) -> Result<Option<SubscriptionEntity>> {
        let mut conn = Arc::clone(&self.db_pool).get()?;
        let now = Utc::now();

        let mut query = subscriptions::table
            .filter(subscriptions::user_id.eq(user_id))
            .filter(subscriptions::status.eq(SubscriptionStatus::Active.to_string()))
            .filter(subscriptions::starts_at.le(now))
            .filter(subscriptions::ends_at.gt(now))
            .into_boxed();

        if let Some(exclude_plan_id) = exclude_plan_id {
            query = query.filter(subscriptions::plan_id.ne(exclude_plan_id));
        }

        let current = query
            .order(subscriptions::starts_at.desc())
            .first::<SubscriptionEntity>(&mut conn)
            .optional()?;

        Ok(current)
    }
}
