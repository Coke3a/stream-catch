use anyhow::Result;
use axum::async_trait;
use mockall::automock;
use uuid::Uuid;

use crate::domain::entities::{plans::PlanEntity, subscriptions::InsertSubscriptionEntity};

#[async_trait]
#[automock]
pub trait SubscriptionRepository {
    async fn list_plans(&self) -> Result<Vec<PlanEntity>>;
    async fn subscribe(&self, insert_subscription_entity: InsertSubscriptionEntity)
    -> Result<Uuid>;
    async fn cancel_subscription(&self, subscription_id: Uuid) -> Result<()>;
    async fn check_current_user_subscription(&self, user_id: Uuid) -> Result<bool>;
}
