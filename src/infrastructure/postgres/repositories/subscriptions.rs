use anyhow::Result;
use axum::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    domain::{
        entities::{plans::PlanEntity, subscriptions::InsertSubscriptionEntity},
        repositories::subscriptions::SubscriptionRepository,
    },
    infrastructure::postgres::postgres_connection::PgPoolSquad,
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
        unimplemented!()
    }
    async fn subscribe(
        &self,
        insert_subscription_entity: InsertSubscriptionEntity,
    ) -> Result<Uuid> {
        unimplemented!()
    }
    async fn cancel_subscription(&self, subscription_id: Uuid) -> Result<()> {
        unimplemented!()
    }
    async fn check_current_user_subscription(&self, user_id: Uuid) -> Result<bool> {
        unimplemented!()
    }
}
