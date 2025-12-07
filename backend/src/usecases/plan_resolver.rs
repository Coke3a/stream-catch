use anyhow::Result;
use crates::domain::{
    entities::plans::PlanEntity,
    repositories::{plans::PlanRepository, subscriptions::SubscriptionRepository},
};
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

/// Resolves the effective plan for a user: active paid subscription or free plan fallback.
pub struct PlanResolver<P, S>
where
    P: PlanRepository + Send + Sync + 'static,
    S: SubscriptionRepository + Send + Sync + 'static,
{
    plan_repo: Arc<P>,
    subscription_repo: Arc<S>,
    free_plan_id: Uuid,
}

impl<P, S> PlanResolver<P, S>
where
    P: PlanRepository + Send + Sync + 'static,
    S: SubscriptionRepository + Send + Sync + 'static,
{
    pub fn new(plan_repo: Arc<P>, subscription_repo: Arc<S>, free_plan_id: Uuid) -> Self {
        Self {
            plan_repo,
            subscription_repo,
            free_plan_id,
        }
    }

    pub async fn resolve_effective_plan_for_user(&self, user_id: Uuid) -> Result<PlanEntity> {
        if let Some(subscription) = self
            .subscription_repo
            .find_current_active_non_free_subscription(user_id, self.free_plan_id)
            .await?
        {
            debug!(
                %user_id,
                plan_id = %subscription.plan_id,
                "plan_resolver: using active subscription plan"
            );
            return self.plan_repo.find_by_id(subscription.plan_id).await;
        }

        debug!(%user_id, "plan_resolver: falling back to free plan");
        self.plan_repo.find_by_id(self.free_plan_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use crates::domain::{
        entities::{plans::PlanEntity, subscriptions::SubscriptionEntity},
        repositories::{plans::MockPlanRepository, subscriptions::MockSubscriptionRepository},
        value_objects::{
            enums::subscription_statuses::SubscriptionStatus,
            plans::{FREE_PLAN_ID, PlanFeatures},
        },
    };
    use mockall::predicate::eq;

    fn sample_plan(id: Uuid) -> PlanEntity {
        PlanEntity {
            id,
            name: Some("Plan".to_string()),
            price_minor: 1000,
            duration_days: 30,
            features: PlanFeatures::default(),
            is_active: true,
            stripe_price_recurring: None,
            stripe_price_one_time_card: None,
            stripe_price_one_time_promptpay: None,
        }
    }

    fn sample_subscription(user_id: Uuid, plan_id: Uuid) -> SubscriptionEntity {
        let now = Utc::now();
        SubscriptionEntity {
            id: Uuid::new_v4(),
            user_id,
            plan_id,
            starts_at: now - Duration::days(1),
            ends_at: now + Duration::days(1),
            billing_mode: "recurring".to_string(),
            default_payment_method_id: None,
            cancel_at_period_end: false,
            canceled_at: None,
            provider_subscription_id: None,
            status: SubscriptionStatus::Active.to_string(),
            created_at: now,
        }
    }

    #[tokio::test]
    async fn returns_paid_plan_when_subscription_exists() {
        let user_id = Uuid::new_v4();
        let paid_plan_id = Uuid::new_v4();

        let mut plan_repo = MockPlanRepository::new();
        let mut subscription_repo = MockSubscriptionRepository::new();

        let paid_plan = sample_plan(paid_plan_id);
        let subscription = sample_subscription(user_id, paid_plan_id);

        subscription_repo
            .expect_find_current_active_non_free_subscription()
            .with(eq(user_id), eq(FREE_PLAN_ID))
            .returning(move |_, _| {
                let subscription = subscription.clone();
                Box::pin(async move { Ok(Some(subscription)) })
            });

        plan_repo
            .expect_find_by_id()
            .with(eq(paid_plan_id))
            .returning(move |_| {
                let plan = paid_plan.clone();
                Box::pin(async move { Ok(plan) })
            });

        let resolver = PlanResolver::new(
            Arc::new(plan_repo),
            Arc::new(subscription_repo),
            FREE_PLAN_ID,
        );

        let plan = resolver
            .resolve_effective_plan_for_user(user_id)
            .await
            .unwrap();

        assert_eq!(plan.id, paid_plan_id);
    }

    #[tokio::test]
    async fn falls_back_to_free_plan_when_no_active_subscription() {
        let user_id = Uuid::new_v4();

        let mut plan_repo = MockPlanRepository::new();
        let mut subscription_repo = MockSubscriptionRepository::new();

        let free_plan = sample_plan(FREE_PLAN_ID);

        subscription_repo
            .expect_find_current_active_non_free_subscription()
            .with(eq(user_id), eq(FREE_PLAN_ID))
            .returning(|_, _| Box::pin(async { Ok(None) }));

        plan_repo
            .expect_find_by_id()
            .with(eq(FREE_PLAN_ID))
            .returning(move |_| {
                let plan = free_plan.clone();
                Box::pin(async move { Ok(plan) })
            });

        let resolver = PlanResolver::new(
            Arc::new(plan_repo),
            Arc::new(subscription_repo),
            FREE_PLAN_ID,
        );

        let plan = resolver
            .resolve_effective_plan_for_user(user_id)
            .await
            .unwrap();

        assert_eq!(plan.id, FREE_PLAN_ID);
    }
}
