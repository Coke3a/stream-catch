use anyhow::Result;
use crates::domain::{
    repositories::subscriptions::SubscriptionRepository,
    value_objects::subscriptions::{InsertSubscriptionModel, PlanModel, SubscriptionModel},
};
use std::sync::Arc;
use uuid::Uuid;

pub struct SubscriptionUseCase<T>
where
    T: SubscriptionRepository,
{
    repository: Arc<T>,
}

impl<T> SubscriptionUseCase<T>
where
    T: SubscriptionRepository,
{
    pub fn new(repository: Arc<T>) -> Self {
        Self { repository }
    }
}

impl<T> SubscriptionUseCase<T>
where
    T: SubscriptionRepository,
{
    pub async fn list_plans(&self) -> Result<Vec<PlanModel>> {
        unimplemented!()
    }

    pub async fn check_current_user_subscription(
        &self,
        user_id: Uuid,
    ) -> Result<SubscriptionModel> {
        unimplemented!()
    }

    pub async fn subscribe(
        &self,
        user_id: Uuid,
        insert_subscription_model: InsertSubscriptionModel,
    ) -> Result<()> {
        unimplemented!()
    }

    pub async fn cancel_subscription(&self) -> Result<()> {
        unimplemented!()
    }
}
