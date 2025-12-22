use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use mockall::automock;
use uuid::Uuid;

use crate::domain::entities::subscriptions::SubscriptionEntity;
use crate::domain::value_objects::enums::{
    billing_modes::BillingMode, subscription_statuses::SubscriptionStatus,
};

#[async_trait]
#[automock]
pub trait SubscriptionRepository {
    async fn find_current_active_subscription(
        &self,
        user_id: Uuid,
    ) -> Result<Option<SubscriptionEntity>>;

    async fn find_current_active_non_free_subscription(
        &self,
        user_id: Uuid,
        free_plan_id: Uuid,
    ) -> Result<Option<SubscriptionEntity>>;

    async fn update_status_by_provider_subscription_id(
        &self,
        provider_subscription_id: &str,
        status: SubscriptionStatus,
    ) -> Result<()>;

    async fn find_by_provider_subscription_id(
        &self,
        provider_subscription_id: &str,
    ) -> Result<Option<SubscriptionEntity>>;

    async fn update_status_and_period_by_provider_subscription_id(
        &self,
        provider_subscription_id: &str,
        status: SubscriptionStatus,
        starts_at: DateTime<Utc>,
        ends_at: DateTime<Utc>,
    ) -> Result<()>;

    async fn create_or_update_subscription_after_checkout(
        &self,
        user_id: Uuid,
        plan_id: Uuid,
        billing_mode: BillingMode,
        starts_at: DateTime<Utc>,
        ends_at: DateTime<Utc>,
        status: SubscriptionStatus,
        provider_subscription_id: Option<String>,
    ) -> Result<Uuid>;

    async fn cancel_recurring_subscription(&self, user_id: Uuid) -> Result<()>;

    async fn list_active_subscriptions(&self) -> Result<Vec<SubscriptionEntity>>;
}
