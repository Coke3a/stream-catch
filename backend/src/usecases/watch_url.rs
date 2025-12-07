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
use tracing::{debug, info};
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
            .await?
            .ok_or_else(|| anyhow::anyhow!("Recording not found"))?;

        let features = self.effective_plan_features(user_id).await?;

        self.ensure_user_can_watch(user_id, &recording, &features)
            .await?;

        let recording_id_str = recording.id.to_string();

        let token = self.sign_token(user_id, &recording_id_str)?;
        let base_url = self.config.base_url.trim_end_matches('/');

        if base_url.is_empty() {
            bail!("Watch URL base URL is not configured");
        }

        let url = format!(
            "{}/recording-{}.mp4?token={}",
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
            .map_err(|_| anyhow::anyhow!("Follow is not active"))?;

        if follow.status != FollowStatus::Active.to_string() {
            bail!("Follow is not active");
        }

        let retention_days = features.retention_days.unwrap_or(0);
        if retention_days <= 0 {
            bail!("Recording exceeds retention window");
        }

        let cutoff = Utc::now() - Duration::days(i64::from(retention_days));
        if recording.started_at < cutoff {
            bail!("Recording exceeds retention window");
        }

        Ok(())
    }

    async fn effective_plan_features(&self, user_id: Uuid) -> Result<PlanFeatures> {
        let plan = self
            .plan_resolver
            .resolve_effective_plan_for_user(user_id)
            .await?;

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
            iss: "streamcatch-backend".to_string(),
        };

        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_bytes()),
        )
        .context("failed to sign watch url token")
    }
}
