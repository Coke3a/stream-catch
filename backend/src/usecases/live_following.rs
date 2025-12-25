use anyhow::{Result, anyhow};
use chrono::{DateTime, Duration, Utc};
use crates::domain;
use std::sync::Arc;
use uuid::Uuid;

use crate::usecases::plan_resolver::PlanResolver;
use domain::{
    repositories::{
        live_following::LiveFollowingRepository, plans::PlanRepository,
        subscriptions::SubscriptionRepository,
    },
    value_objects::enums::{
        follow_statuses::FollowStatus, live_account_statuses::LiveAccountStatus,
    },
};
use tracing::{debug, error, info, warn};

const FOLLOW_REACTIVATION_COOLDOWN_HOURS: i64 = 72;

#[derive(Debug, Clone)]
pub struct FollowCooldownError {
    cooldown_until: DateTime<Utc>,
    remaining: Duration,
}

impl FollowCooldownError {
    pub fn new(unfollowed_at: DateTime<Utc>, now: DateTime<Utc>) -> Option<Self> {
        let cooldown_until = unfollowed_at + Duration::hours(FOLLOW_REACTIVATION_COOLDOWN_HOURS);
        if now < cooldown_until {
            Some(Self {
                cooldown_until,
                remaining: cooldown_until - now,
            })
        } else {
            None
        }
    }

    pub fn cooldown_until(&self) -> DateTime<Utc> {
        self.cooldown_until
    }

    pub fn remaining_seconds(&self) -> i64 {
        self.remaining.num_seconds().max(0)
    }

    pub fn remaining_hours(&self) -> i64 {
        (self.remaining_seconds() + 3599) / 3600
    }

    pub fn message(&self) -> String {
        let remaining_hours = self.remaining_hours();
        let days = remaining_hours / 24;
        let hours = remaining_hours % 24;
        let time_label = if days > 0 && hours > 0 {
            format!(
                "{} {}",
                Self::pluralize(days, "day"),
                Self::pluralize(hours, "hour")
            )
        } else if days > 0 {
            Self::pluralize(days, "day")
        } else {
            Self::pluralize(remaining_hours.max(1), "hour")
        };

        format!(
            "You recently unfollowed this streamer. You can follow again in {}.",
            time_label
        )
    }

    fn pluralize(value: i64, unit: &str) -> String {
        if value == 1 {
            format!("1 {}", unit)
        } else {
            format!("{} {}s", value, unit)
        }
    }
}

impl std::fmt::Display for FollowCooldownError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for FollowCooldownError {}

pub struct LiveFollowingUseCase<L, P, S>
where
    L: LiveFollowingRepository + Send + Sync + 'static,
    P: PlanRepository + Send + Sync + 'static,
    S: SubscriptionRepository + Send + Sync + 'static,
{
    live_following_repository: Arc<L>,
    plan_resolver: Arc<PlanResolver<P, S>>,
}

impl<L, P, S> LiveFollowingUseCase<L, P, S>
where
    L: LiveFollowingRepository + Send + Sync + 'static,
    P: PlanRepository + Send + Sync + 'static,
    S: SubscriptionRepository + Send + Sync + 'static,
{
    pub fn new(live_following_repository: Arc<L>, plan_resolver: Arc<PlanResolver<P, S>>) -> Self {
        Self {
            live_following_repository,
            plan_resolver,
        }
    }

    pub async fn follow(&self, user_id: Uuid, insert_url: String) -> Result<()> {
        info!(
            %user_id,
            url_len = insert_url.len(),
            "live_following: follow requested"
        );

        let normalized =
            domain::value_objects::live_account_url::normalize_live_account_url(&insert_url)
                .map_err(|err| {
                    warn!(
                        %user_id,
                        error = %err,
                        status = axum::http::StatusCode::BAD_REQUEST.as_u16(),
                        "live_following: invalid follow URL"
                    );
                    err
                })?;

        let platform = normalized.platform;
        let account_id = normalized.account_id;
        let canonical_url = normalized.canonical_url;

        debug!(
            platform = %platform,
            account_id,
            canonical_url,
            "live_following: normalized follow URL"
        );

        let find_live_account_model = domain::value_objects::live_following::FindLiveAccountModel {
            platform,
            account_id: account_id.clone(),
        };

        let active_status = FollowStatus::Active.to_string();
        let inactive_status = FollowStatus::Inactive.to_string();
        let now = Utc::now();

        // Try to find existing live account first
        let live_account_result = self
            .live_following_repository
            .find_live_account(&find_live_account_model)
            .await;

        match live_account_result {
            // ─────────────────────────────────────────────
            // Case 1: live account already exists
            // ─────────────────────────────────────────────
            Ok(live_account) => {
                info!(
                    live_account_id = %live_account.id,
                    "live_following: found existing live account"
                );

                // Check existing follow status (if any)
                match self
                    .live_following_repository
                    .find_follow(user_id, live_account.id)
                    .await
                {
                    // Already active → return error
                    Ok(existing_follow) if existing_follow.status == active_status => {
                        warn!(
                            %user_id,
                            follow_status = existing_follow.status,
                            status = axum::http::StatusCode::CONFLICT.as_u16(),
                            "live_following: follow already active"
                        );
                        return Err(anyhow::anyhow!("Follow already exists"));
                    }

                    // Inactive → reactivate and return
                    Ok(existing_follow) if existing_follow.status == inactive_status => {
                        if let Some(cooldown) = FollowCooldownError::new(existing_follow.updated_at, now) {
                            warn!(
                                %user_id,
                                live_account_id = %live_account.id,
                                remaining_hours = cooldown.remaining_hours(),
                                "live_following: follow cooldown active"
                            );
                            return Err(anyhow!(cooldown));
                        }

                        self.ensure_follow_quota(user_id).await?;

                        info!("live_following: reactivating existing follow");
                        self.live_following_repository
                            .to_active(user_id, live_account.id)
                            .await
                            .map_err(|err| {
                                error!(
                                    %user_id,
                                    live_account_id = %live_account.id,
                                    db_error = ?err,
                                    "live_following: failed to reactivate follow"
                                );
                                err
                            })?;
                        return Ok(());
                    }

                    // Not found or other error → continue to create new follow
                    Ok(_) => {
                        debug!("live_following: no reusable follow found, creating new follow");
                    }
                    Err(err) => {
                        if err.downcast_ref::<diesel::result::Error>()
                            == Some(&diesel::result::Error::NotFound)
                        {
                            info!(
                                %user_id,
                                live_account_id = %live_account.id,
                                "live_following: follow not found, creating new follow"
                            );
                        } else {
                            error!(
                                %user_id,
                                live_account_id = %live_account.id,
                                db_error = ?err,
                                "live_following: failed to load follow state, creating new follow"
                            );
                        }
                    }
                }

                self.ensure_follow_quota(user_id).await?;

                // Create a new follow for existing live account
                let insert_follow_entity = domain::entities::follows::InsertFollowEntity {
                    user_id,
                    live_account_id: Some(live_account.id),
                    status: active_status,
                    created_at: now,
                    updated_at: now,
                };

                self.live_following_repository
                    .follow(insert_follow_entity)
                    .await
                    .map_err(|err| {
                        error!(
                            %user_id,
                            live_account_id = %live_account.id,
                            db_error = ?err,
                            "live_following: failed to create follow for existing live account"
                        );
                        err
                    })?;

                info!("live_following: follow created for existing live account");
            }

            // ─────────────────────────────────────────────
            // Case 2: live account does NOT exist → create both
            // ─────────────────────────────────────────────
            Err(err) => {
                info!(
                    platform = %platform,
                    account_id,
                    db_error = ?err,
                    "live_following: creating new live account and follow"
                );

                self.ensure_follow_quota(user_id).await?;

                let insert_live_account_entity =
                    domain::entities::live_accounts::InsertLiveAccountEntity {
                        platform: platform.to_string(),
                        account_id: account_id.clone(),
                        canonical_url,
                        status: LiveAccountStatus::Unsynced.to_string(),
                        created_at: now,
                        updated_at: now,
                    };

                let insert_follow_entity = domain::entities::follows::InsertFollowEntity {
                    user_id,
                    live_account_id: None, // Will be set by repository
                    status: active_status,
                    created_at: now,
                    updated_at: now,
                };

                self.live_following_repository
                    .follow_and_create_live_account(
                        insert_follow_entity,
                        insert_live_account_entity,
                    )
                    .await
                    .map_err(|err| {
                        error!(
                            %user_id,
                            platform = %platform,
                            account_id,
                            db_error = ?err,
                            "live_following: failed to create live account/follow"
                        );
                        err
                    })?;

                info!("live_following: new live account and follow created");
            }
        }

        Ok(())
    }

    /// Ensures the user has remaining follow slots based on the active plan.
    async fn ensure_follow_quota(&self, user_id: Uuid) -> Result<()> {
        let plan = self
            .plan_resolver
            .resolve_effective_plan_for_user(user_id)
            .await
            .map_err(|err| {
                error!(
                    %user_id,
                    db_error = ?err,
                    "live_following: failed to resolve plan while checking quota"
                );
                err
            })?;
        let features = plan.features;

        let current = self
            .live_following_repository
            .count_active_follows(user_id)
            .await
            .map_err(|err| {
                error!(
                    %user_id,
                    db_error = ?err,
                    "live_following: failed to count active follows"
                );
                err
            })?;

        let max_follows = features.max_follows.unwrap_or(0);

        if max_follows <= 0 || current >= max_follows {
            warn!(
                %user_id,
                max_follows,
                current_active = current,
                status = axum::http::StatusCode::FORBIDDEN.as_u16(),
                "live_following: follow limit reached"
            );
            return Err(anyhow!(
                "follow limit reached: current={} max={}",
                current,
                max_follows
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usecases::plan_resolver::PlanResolver;
    use crates::domain::{
        entities::{
            follows::FollowEntity, live_accounts::LiveAccountEntity, plans::PlanEntity,
        },
        repositories::{
            live_following::MockLiveFollowingRepository, plans::MockPlanRepository,
            subscriptions::MockSubscriptionRepository,
        },
        value_objects::{
            enums::{
                follow_statuses::FollowStatus, live_account_statuses::LiveAccountStatus,
                platforms::Platform,
            },
            live_following::FindLiveAccountModel,
            plans::{PlanFeatures, FREE_PLAN_ID},
        },
    };
    use mockall::predicate::eq;
    use std::sync::Arc;
    use uuid::Uuid;

    fn sample_plan(plan_id: Uuid, max_follows: i64) -> PlanEntity {
        PlanEntity {
            id: plan_id,
            name: Some("Test Plan".to_string()),
            price_minor: 0,
            duration_days: 30,
            features: PlanFeatures {
                max_follows: Some(max_follows),
                ..PlanFeatures::default()
            },
            is_active: true,
            stripe_price_recurring: None,
            stripe_price_one_time_card: None,
            stripe_price_one_time_promptpay: None,
        }
    }

    fn sample_live_account(id: Uuid, now: DateTime<Utc>) -> LiveAccountEntity {
        LiveAccountEntity {
            id,
            platform: Platform::TikTok.to_string(),
            account_id: "alice".to_string(),
            canonical_url: "https://www.tiktok.com/@alice/live".to_string(),
            status: LiveAccountStatus::Unsynced.to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    fn sample_follow(
        user_id: Uuid,
        live_account_id: Uuid,
        status: FollowStatus,
        timestamp: DateTime<Utc>,
    ) -> FollowEntity {
        FollowEntity {
            user_id,
            live_account_id,
            status: status.to_string(),
            created_at: timestamp,
            updated_at: timestamp,
        }
    }

    fn plan_resolver_with_max_follows(
        user_id: Uuid,
        max_follows: i64,
    ) -> PlanResolver<MockPlanRepository, MockSubscriptionRepository> {
        let mut plan_repo = MockPlanRepository::new();
        let mut subscription_repo = MockSubscriptionRepository::new();

        subscription_repo
            .expect_find_current_active_non_free_subscription()
            .with(eq(user_id), eq(FREE_PLAN_ID))
            .returning(|_, _| Box::pin(async { Ok(None) }));

        let plan = sample_plan(FREE_PLAN_ID, max_follows);
        plan_repo
            .expect_find_by_id()
            .with(eq(FREE_PLAN_ID))
            .returning(move |_| {
                let plan = plan.clone();
                Box::pin(async move { Ok(plan) })
            });

        PlanResolver::new(
            Arc::new(plan_repo),
            Arc::new(subscription_repo),
            FREE_PLAN_ID,
        )
    }

    #[tokio::test]
    async fn blocks_reactivation_when_unfollowed_within_cooldown() {
        let user_id = Uuid::new_v4();
        let live_account_id = Uuid::new_v4();
        let now = Utc::now();

        let live_account = sample_live_account(live_account_id, now);
        let follow = sample_follow(
            user_id,
            live_account_id,
            FollowStatus::Inactive,
            now - Duration::hours(1),
        );

        let expected_model = FindLiveAccountModel {
            platform: Platform::TikTok,
            account_id: "alice".to_string(),
        };

        let mut live_following_repo = MockLiveFollowingRepository::new();
        live_following_repo
            .expect_find_live_account()
            .with(eq(expected_model))
            .returning(move |_| {
                let live_account = live_account.clone();
                Box::pin(async move { Ok(live_account) })
            });
        live_following_repo
            .expect_find_follow()
            .with(eq(user_id), eq(live_account_id))
            .returning(move |_, _| {
                let follow = follow.clone();
                Box::pin(async move { Ok(follow) })
            });

        let plan_resolver = PlanResolver::new(
            Arc::new(MockPlanRepository::new()),
            Arc::new(MockSubscriptionRepository::new()),
            FREE_PLAN_ID,
        );

        let usecase =
            LiveFollowingUseCase::new(Arc::new(live_following_repo), Arc::new(plan_resolver));

        let err = usecase
            .follow(user_id, "https://www.tiktok.com/@alice/live".to_string())
            .await
            .unwrap_err();

        assert!(err.downcast_ref::<FollowCooldownError>().is_some());
    }

    #[tokio::test]
    async fn reactivates_when_cooldown_elapsed() {
        let user_id = Uuid::new_v4();
        let live_account_id = Uuid::new_v4();
        let now = Utc::now();

        let live_account = sample_live_account(live_account_id, now);
        let follow = sample_follow(
            user_id,
            live_account_id,
            FollowStatus::Inactive,
            now - Duration::hours(73),
        );

        let expected_model = FindLiveAccountModel {
            platform: Platform::TikTok,
            account_id: "alice".to_string(),
        };

        let mut live_following_repo = MockLiveFollowingRepository::new();
        live_following_repo
            .expect_find_live_account()
            .with(eq(expected_model))
            .returning(move |_| {
                let live_account = live_account.clone();
                Box::pin(async move { Ok(live_account) })
            });
        live_following_repo
            .expect_find_follow()
            .with(eq(user_id), eq(live_account_id))
            .returning(move |_, _| {
                let follow = follow.clone();
                Box::pin(async move { Ok(follow) })
            });
        live_following_repo
            .expect_count_active_follows()
            .with(eq(user_id))
            .returning(|_| Box::pin(async { Ok(0) }));
        live_following_repo
            .expect_to_active()
            .with(eq(user_id), eq(live_account_id))
            .returning(|_, _| Box::pin(async { Ok(()) }));

        let plan_resolver = plan_resolver_with_max_follows(user_id, 5);
        let usecase =
            LiveFollowingUseCase::new(Arc::new(live_following_repo), Arc::new(plan_resolver));

        let result = usecase
            .follow(user_id, "https://www.tiktok.com/@alice/live".to_string())
            .await;

        assert!(result.is_ok());
    }
}
