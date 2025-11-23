use std::sync::Arc;
use anyhow::Result;
use axum::async_trait;
use uuid::Uuid;

use crate::{domain::{entities::{plans::PlanEntity, subscriptions::InsertSubscriptionEntity}, repositories::subscriptions::SubscriptionRepository}, infrastructure::postgres::postgres_connection::PgPoolSquad};

pub struct SubscriptionPostgres {
    db_pool: Arc<PgPoolSquad>
}

impl SubscriptionPostgres {
    pub fn new(db_pool: Arc<PgPoolSquad>) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl SubscriptionRepository for SubscriptionPostgres {
    async fn list_plans(&self) -> Result<Vec<PlanEntity>> {
        unimplemented!()
    }
    async fn subscribe(&self, insert_subscription_entity: InsertSubscriptionEntity) -> Result<i64> {
        unimplemented!()
    }
    async fn cancel_subscription(&self, subscription_id: i64) -> Result<()> {
        unimplemented!()
    }
    async fn check_current_user_subscription(&self, user_id: Uuid) -> Result<bool> {
        unimplemented!()
    }
}