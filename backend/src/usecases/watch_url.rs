use anyhow::{Context, Result, bail};
use chrono::{Duration, Utc};
use crates::domain::{
    repositories::{
        live_following::LiveFollowingRepository, recording_upload::RecordingUploadRepository,
    },
    value_objects::{
        enums::{follow_statuses::FollowStatus, sort_order::SortOrder},
        live_following::ListFollowsFilter,
    },
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
pub struct WatchUrlUseCase<R, F>
where
    R: RecordingUploadRepository + Send + Sync,
    F: LiveFollowingRepository + Send + Sync,
{
    recording_repository: Arc<R>,
    live_following_repository: Arc<F>,
    config: WatchUrl,
}

impl<R, F> WatchUrlUseCase<R, F>
where
    R: RecordingUploadRepository + Send + Sync,
    F: LiveFollowingRepository + Send + Sync,
{
    pub fn new(
        recording_repository: Arc<R>,
        live_following_repository: Arc<F>,
        config: WatchUrl,
    ) -> Self {
        Self {
            recording_repository,
            live_following_repository,
            config,
        }
    }

    /// Generates a signed Cloudflare Worker URL for the given recording and user.
    pub async fn generate_watch_url(&self, user_id: Uuid, recording_id: Uuid) -> Result<String> {
        info!(%user_id, %recording_id, "watch_url: generating url");

        let recording = self
            .recording_repository
            .find_recording_by_id(recording_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Recording not found"))?;

        self.ensure_user_can_watch(user_id, recording.live_account_id)
            .await?;

        let recording_key = recording
            .recording_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Recording key is missing"))?;

        let token = self.sign_token(user_id, recording_key)?;
        let base_url = self.config.base_url.trim_end_matches('/');

        if base_url.is_empty() {
            bail!("Watch URL base URL is not configured");
        }

        let url = format!(
            "{}/recording-{}.mp4?token={}",
            base_url, recording_key, token
        );

        debug!(%user_id, %recording_id, "watch_url: url generated");
        Ok(url)
    }

    async fn ensure_user_can_watch(&self, user_id: Uuid, live_account_id: Uuid) -> Result<()> {
        let filter = ListFollowsFilter {
            live_account_id: Some(live_account_id),
            platform: None,
            status: Some(FollowStatus::Active),
            limit: Some(1),
            sort_order: SortOrder::Desc,
        };

        let follows = self
            .live_following_repository
            .list_following_live_accounts(user_id, &filter)
            .await?;

        if follows.is_empty() {
            bail!("Follow is not active");
        }

        Ok(())
    }

    fn sign_token(&self, user_id: Uuid, recording_key: &str) -> Result<String> {
        let ttl =
            i64::try_from(self.config.ttl_seconds).context("watch_url ttl_seconds is too large")?;

        let now = Utc::now();
        let exp = now
            .checked_add_signed(Duration::seconds(ttl))
            .ok_or_else(|| anyhow::anyhow!("Failed to compute token expiration"))?;

        let claims = WatchUrlClaims {
            sub: recording_key.to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crates::domain::{
        entities::{live_accounts::LiveAccountEntity, recordings::RecordingEntity},
        repositories::{
            live_following::MockLiveFollowingRepository,
            recording_upload::MockRecordingUploadRepository,
        },
        value_objects::enums::{platforms::Platform, recording_statuses::RecordingStatus},
    };
    use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
    use mockall::predicate::eq;

    fn sample_recording(
        recording_id: Uuid,
        live_account_id: Uuid,
        recording_key: &str,
    ) -> RecordingEntity {
        let now = Utc::now();
        RecordingEntity {
            id: recording_id,
            live_account_id,
            recording_key: Some(recording_key.to_string()),
            title: None,
            started_at: now,
            ended_at: None,
            duration_sec: Some(120),
            size_bytes: Some(1024),
            storage_path: Some("path.mp4".to_string()),
            storage_temp_path: None,
            status: RecordingStatus::Ready.to_string(),
            poster_storage_path: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn sample_live_account(id: Uuid) -> LiveAccountEntity {
        let now = Utc::now();
        LiveAccountEntity {
            id,
            platform: Platform::Twitch.to_string(),
            account_id: "tester".to_string(),
            canonical_url: "https://twitch.tv/tester".to_string(),
            status: "synced".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn generates_url_for_active_follow() {
        let user_id = Uuid::new_v4();
        let recording_id = Uuid::new_v4();
        let live_account_id = Uuid::new_v4();
        let recording_key = "rec-key-123";

        let mut recording_repo = MockRecordingUploadRepository::new();
        let mut follow_repo = MockLiveFollowingRepository::new();

        let recording = sample_recording(recording_id, live_account_id, recording_key);
        recording_repo
            .expect_find_recording_by_id()
            .with(eq(recording_id))
            .returning(move |_| {
                let recording = recording.clone();
                Box::pin(async move { Ok(Some(recording)) })
            });

        follow_repo
            .expect_list_following_live_accounts()
            .withf(move |uid, filter| {
                *uid == user_id
                    && filter.live_account_id == Some(live_account_id)
                    && filter.status == Some(FollowStatus::Active)
            })
            .returning(move |_, _| {
                let live_account = sample_live_account(live_account_id);
                Box::pin(async move { Ok(vec![live_account]) })
            });

        let config = WatchUrl {
            jwt_secret: "super-secret".to_string(),
            base_url: "https://gateway.example.com".to_string(),
            ttl_seconds: 600,
        };

        let usecase = WatchUrlUseCase::new(
            Arc::new(recording_repo),
            Arc::new(follow_repo),
            config.clone(),
        );

        let url = usecase
            .generate_watch_url(user_id, recording_id)
            .await
            .expect("url should be generated");

        let expected_prefix = format!("{}/recording-{}.mp4?token=", config.base_url, recording_key);
        assert!(
            url.starts_with(&expected_prefix),
            "url did not start with expected prefix: {url}"
        );

        let token = url
            .split("token=")
            .nth(1)
            .expect("token query param should exist");
        let decoded = decode::<WatchUrlClaims>(
            token,
            &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
            &Validation::new(Algorithm::HS256),
        )
        .expect("token should decode");

        assert_eq!(decoded.claims.sub, recording_key);
        assert_eq!(decoded.claims.uid, user_id.to_string());
    }

    #[tokio::test]
    async fn rejects_when_follow_inactive() {
        let user_id = Uuid::new_v4();
        let recording_id = Uuid::new_v4();
        let live_account_id = Uuid::new_v4();
        let recording_key = "rec-key-123";

        let mut recording_repo = MockRecordingUploadRepository::new();
        let mut follow_repo = MockLiveFollowingRepository::new();

        let recording = sample_recording(recording_id, live_account_id, recording_key);
        recording_repo
            .expect_find_recording_by_id()
            .with(eq(recording_id))
            .returning(move |_| {
                let recording = recording.clone();
                Box::pin(async move { Ok(Some(recording)) })
            });

        follow_repo
            .expect_list_following_live_accounts()
            .returning(|_, _| Box::pin(async { Ok(Vec::new()) }));

        let config = WatchUrl {
            jwt_secret: "super-secret".to_string(),
            base_url: "https://gateway.example.com".to_string(),
            ttl_seconds: 600,
        };

        let usecase = WatchUrlUseCase::new(Arc::new(recording_repo), Arc::new(follow_repo), config);

        let err = usecase
            .generate_watch_url(user_id, recording_id)
            .await
            .expect_err("missing follow should be rejected");

        assert!(err.to_string().contains("Follow is not active"));
    }
}
