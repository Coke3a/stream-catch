use crate::usecases::plan_resolver::PlanResolver;
use anyhow::{Context, Result, bail};
use chrono::{Duration, Utc};
use crates::domain::{
    entities::recordings::RecordingEntity,
    repositories::{
        live_following::LiveFollowingRepository, plans::PlanRepository,
        recording_upload::RecordingUploadRepository, subscriptions::SubscriptionRepository,
    },
    value_objects::{enums::follow_statuses::FollowStatus, plans::PlanFeatures},
};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::config::config_model::WatchUrl;

#[derive(Debug, Serialize, Deserialize)]
struct WatchUrlClaims {
    sub: String,
    uid: String,
    exp: usize,
    iat: usize,
    iss: String,
}

/// Generates signed watch URLs for recordings a user is allowed to view.
pub struct WatchUrlUseCase<R, F, P, S>
where
    R: RecordingUploadRepository + Send + Sync + 'static,
    F: LiveFollowingRepository + Send + Sync + 'static,
    P: PlanRepository + Send + Sync + 'static,
    S: SubscriptionRepository + Send + Sync + 'static,
{
    recording_repository: Arc<R>,
    live_following_repository: Arc<F>,
    plan_resolver: Arc<PlanResolver<P, S>>,
    config: WatchUrl,
}

impl<R, F, P, S> WatchUrlUseCase<R, F, P, S>
where
    R: RecordingUploadRepository + Send + Sync + 'static,
    F: LiveFollowingRepository + Send + Sync + 'static,
    P: PlanRepository + Send + Sync + 'static,
    S: SubscriptionRepository + Send + Sync + 'static,
{
    pub fn new(
        recording_repository: Arc<R>,
        live_following_repository: Arc<F>,
        plan_resolver: Arc<PlanResolver<P, S>>,
        config: WatchUrl,
    ) -> Self {
        Self {
            recording_repository,
            live_following_repository,
            plan_resolver,
            config,
        }
    }

    // Generates a signed Cloudflare Worker URL for the given recording and user.
    pub async fn generate_watch_url(&self, user_id: Uuid, recording_id: Uuid) -> Result<String> {
        info!(%user_id, %recording_id, "watch_url: generating url");

        let recording = self
            .recording_repository
            .find_recording_by_id(recording_id)
            .await
            .map_err(|err| {
                error!(
                    %user_id,
                    %recording_id,
                    db_error = ?err,
                    "watch_url: failed to fetch recording by id"
                );
                err
            })?
            .ok_or_else(|| {
                warn!(
                    %user_id,
                    %recording_id,
                    "watch_url: recording not found"
                );
                anyhow::anyhow!("Recording not found")
            })?;

        let features = self.effective_plan_features(user_id).await?;

        self.ensure_user_can_watch(user_id, &recording, &features)
            .await?;

        let recording_id_str = recording.id.to_string();

        let token = self.sign_token(user_id, &recording_id_str)?;
        let base_url = self.config.base_url.trim_end_matches('/');

        if base_url.is_empty() {
            error!(
                %user_id,
                %recording_id,
                "watch_url: base URL is not configured"
            );
            bail!("Watch URL base URL is not configured");
        }

        let url = format!(
            "{}/recording-{}_origin.mp4?token={}",
            base_url, recording_id_str, token
        );

        debug!(%user_id, %recording_id, "watch_url: url generated");
        Ok(url)
    }

    async fn ensure_user_can_watch(
        &self,
        user_id: Uuid,
        recording: &RecordingEntity,
        features: &PlanFeatures,
    ) -> Result<()> {
        let follow = self
            .live_following_repository
            .find_follow(user_id, recording.live_account_id)
            .await
            .map_err(|err| {
                error!(
                    %user_id,
                    live_account_id = %recording.live_account_id,
                    db_error = ?err,
                    "watch_url: failed to load follow status"
                );
                anyhow::anyhow!("Follow is not active")
            })?;

        if follow.status != FollowStatus::Active.to_string() {
            warn!(
                %user_id,
                live_account_id = %recording.live_account_id,
                status = %follow.status,
                "watch_url: follow is not active"
            );
            bail!("Follow is not active");
        }

        let retention_days = features.retention_days.unwrap_or(0);
        if retention_days <= 0 {
            warn!(
                %user_id,
                recording_id = %recording.id,
                retention_days,
                "watch_url: retention not configured for plan"
            );
            bail!("Recording exceeds retention window");
        }

        let view_start_at = if follow.created_at > recording.started_at {
            follow.created_at
        } else {
            recording.started_at
        };
        let view_end_at = view_start_at
            .checked_add_signed(Duration::days(i64::from(retention_days)))
            .context("failed to compute retention window")?;

        let now = Utc::now();
        if now < view_start_at || now >= view_end_at {
            warn!(
                %user_id,
                recording_id = %recording.id,
                started_at = %recording.started_at,
                follow_created_at = %follow.created_at,
                view_start_at = %view_start_at,
                view_end_at = %view_end_at,
                now = %now,
                retention_days,
                "watch_url: recording outside retention window"
            );
            bail!("Recording exceeds retention window");
        }

        Ok(())
    }

    async fn effective_plan_features(&self, user_id: Uuid) -> Result<PlanFeatures> {
        let plan = self
            .plan_resolver
            .resolve_effective_plan_for_user(user_id)
            .await
            .map_err(|err| {
                error!(
                    %user_id,
                    db_error = ?err,
                    "watch_url: failed to resolve effective plan"
                );
                err
            })?;

        Ok(plan.features)
    }

    fn sign_token(&self, user_id: Uuid, recording_id: &str) -> Result<String> {
        let ttl =
            i64::try_from(self.config.ttl_seconds).context("watch_url ttl_seconds is too large")?;

        let now = Utc::now();
        let exp = now
            .checked_add_signed(Duration::seconds(ttl))
            .ok_or_else(|| anyhow::anyhow!("Failed to compute token expiration"))?;

        let claims = WatchUrlClaims {
            sub: recording_id.to_string(),
            uid: user_id.to_string(),
            exp: exp.timestamp() as usize,
            iat: now.timestamp() as usize,
            iss: "stream-rokuo-backend".to_string(),
        };

        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_bytes()),
        )
        .map_err(|err| {
            error!(
                %user_id,
                recording_id,
                error = ?err,
                "watch_url: failed to sign token"
            );
            err
        })
        .context("failed to sign watch url token")
    }
}
