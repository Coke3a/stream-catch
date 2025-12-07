use anyhow::{Result, anyhow};
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
        platforms::Platform,
    },
};
use tracing::{debug, info, warn};

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
        info!(%user_id, url = %insert_url, "live_following: follow requested");

        let url = url::Url::parse(&insert_url)?;
        let (platform, account_id) = Self::parse_platform_and_account_id(&url)?;
        debug!(platform = %platform, account_id, "live_following: parsed platform/account");

        let find_live_account_model = domain::value_objects::live_following::FindLiveAccountModel {
            platform: platform.clone(),
            account_id: account_id.clone(),
        };

        let active_status = FollowStatus::Active.to_string();
        let inactive_status = FollowStatus::Inactive.to_string();
        let now = chrono::Utc::now();

        self.ensure_follow_quota(user_id).await?;

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
                            follow_status = existing_follow.status,
                            "live_following: follow already active"
                        );
                        return Err(anyhow::anyhow!("Follow already exists"));
                    }

                    // Inactive → reactivate and return
                    Ok(existing_follow) if existing_follow.status == inactive_status => {
                        info!("live_following: reactivating existing follow");
                        self.live_following_repository
                            .to_active(user_id, live_account.id)
                            .await?;
                        return Ok(());
                    }

                    // Not found or other error → continue to create new follow
                    Ok(_) | Err(_) => {
                        debug!("live_following: no reusable follow found, creating new follow");
                    }
                }

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
                    .await?;

                info!("live_following: follow created for existing live account");
            }

            // ─────────────────────────────────────────────
            // Case 2: live account does NOT exist → create both
            // ─────────────────────────────────────────────
            Err(_) => {
                info!(
                    platform = %platform,
                    account_id,
                    "live_following: creating new live account and follow"
                );

                let insert_live_account_entity =
                    domain::entities::live_accounts::InsertLiveAccountEntity {
                        platform: platform.to_string(),
                        account_id,
                        canonical_url: insert_url,
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
                    .await?;

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
            .await?;
        let features = plan.features;

        let current = self
            .live_following_repository
            .count_active_follows(user_id)
            .await?;

        let max_follows = features.max_follows.unwrap_or(0);

        if max_follows <= 0 || current >= max_follows {
            return Err(anyhow!(
                "follow limit reached: current={} max={}",
                current,
                max_follows
            ));
        }

        Ok(())
    }

    fn parse_platform_and_account_id(url: &url::Url) -> Result<(Platform, String)> {
        let host = url
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid URL"))?;

        if host.contains("tiktok.com") {
            let path_segments: Vec<&str> = url
                .path_segments()
                .ok_or_else(|| anyhow::anyhow!("Invalid TikTok URL"))?
                .collect();
            // Expected format: /@username/live
            if path_segments.len() >= 2
                && path_segments[0].starts_with('@')
                && path_segments[1] == "live"
            {
                Ok((
                    Platform::TikTok,
                    path_segments[0].trim_start_matches('@').to_string(),
                ))
            } else {
                Err(anyhow::anyhow!("Invalid TikTok URL format"))
            }
        } else if host.contains("bigo.tv") {
            let path_segments: Vec<&str> = url
                .path_segments()
                .ok_or_else(|| anyhow::anyhow!("Invalid Bigo URL"))?
                .collect();
            // Expected format: /username
            if let Some(username) = path_segments.first() {
                Ok((Platform::Bigo, username.to_string()))
            } else {
                Err(anyhow::anyhow!("Invalid Bigo URL format"))
            }
        } else if host.contains("twitch.tv") {
            let path_segments: Vec<&str> = url
                .path_segments()
                .ok_or_else(|| anyhow::anyhow!("Invalid Twitch URL"))?
                .collect();
            // Expected format: /username
            if let Some(username) = path_segments.first() {
                Ok((Platform::Twitch, username.to_string()))
            } else {
                Err(anyhow::anyhow!("Invalid Twitch URL format"))
            }
        } else {
            Err(anyhow::anyhow!("Unsupported platform"))
        }
    }
}
