use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;

use domain::{
    repositories::live_following::LiveFollowingRepository,
    value_objects::live_following::{ListFollowsFilter, LiveAccountModel},
    value_objects::enums::platforms::Platform,
};

pub struct LiveFollowingUseCase<T>
where
    T: LiveFollowingRepository + Send + Sync,
{
    live_following_repository: Arc<T>,
}

impl<T> LiveFollowingUseCase<T>
where
    T: LiveFollowingRepository + Send + Sync,
{
    pub fn new(live_following_repository: Arc<T>) -> Self {
        Self {
            live_following_repository,
        }
    }

    pub async fn follow(&self, user_id: Uuid, insert_url: String) -> Result<()> {
        let url = url::Url::parse(&insert_url)?;
        let (platform, account_id) = Self::parse_platform_and_account_id(&url)?;

        let find_live_account_model = domain::value_objects::live_following::FindLiveAccountModel {
            platform: platform.clone(),
            account_id: account_id.clone(),
        };

        // Check if live account exists
        match self.live_following_repository.find_live_account(&find_live_account_model).await {
            Ok(live_account) => {
                // Check if already following
                let follows = self.live_following_repository.list_following_live_accounts(
                    user_id,
                    &ListFollowsFilter {
                        live_account_id: Some(live_account.id),
                        platform: None,
                        status: Some(domain::value_objects::enums::follow_statuses::FollowStatus::Active),
                        limit: Some(1),
                        sort_order: domain::value_objects::enums::sort_order::SortOrder::Desc,
                    }
                ).await?;

                if !follows.is_empty() {
                    return Err(anyhow::anyhow!("Follow already exists"));
                }

                // Create follow
                let insert_follow_entity = domain::entities::follows::InsertFollowEntity {
                    user_id,
                    live_account_id: Some(live_account.id),
                    status: domain::value_objects::enums::follow_statuses::FollowStatus::Active.to_string(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                };
                self.live_following_repository.follow(insert_follow_entity).await?;
            }
            Err(_) => {
                // Create live account and follow
                let insert_live_account_entity = domain::entities::live_accounts::InsertLiveAccountEntity {
                    platform: platform.to_string(),
                    account_id,
                    canonical_url: insert_url,
                    status: domain::value_objects::enums::live_account_statuses::LiveAccountStatus::Unsynced.to_string(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                };

                let insert_follow_entity = domain::entities::follows::InsertFollowEntity {
                    user_id,
                    live_account_id: None, // Will be set by repository
                    status: domain::value_objects::enums::follow_statuses::FollowStatus::Active.to_string(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                };

                self.live_following_repository.follow_and_create_live_account(insert_follow_entity, insert_live_account_entity).await?;
            }
        }

        Ok(())
    }

    fn parse_platform_and_account_id(url: &url::Url) -> Result<(Platform, String)> {
        let host = url.host_str().ok_or_else(|| anyhow::anyhow!("Invalid URL"))?;

        if host.contains("tiktok.com") {
            let path_segments: Vec<&str> = url.path_segments().ok_or_else(|| anyhow::anyhow!("Invalid TikTok URL"))?.collect();
            // Expected format: /@username/live
            if path_segments.len() >= 2 && path_segments[0].starts_with('@') && path_segments[1] == "live" {
                Ok((Platform::TikTok, path_segments[0].trim_start_matches('@').to_string()))
            } else {
                Err(anyhow::anyhow!("Invalid TikTok URL format"))
            }
        } else if host.contains("bigo.tv") {
            let path_segments: Vec<&str> = url.path_segments().ok_or_else(|| anyhow::anyhow!("Invalid Bigo URL"))?.collect();
            // Expected format: /username
            if let Some(username) = path_segments.first() {
                Ok((Platform::Bigo, username.to_string()))
            } else {
                Err(anyhow::anyhow!("Invalid Bigo URL format"))
            }
        } else if host.contains("twitch.tv") {
            let path_segments: Vec<&str> = url.path_segments().ok_or_else(|| anyhow::anyhow!("Invalid Twitch URL"))?.collect();
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

    pub async fn unfollow(&self, follow_id: Uuid) -> Result<()> {
        unimplemented!()
    }

    pub async fn list_follows(
        &self,
        user_id: Uuid,
        list_follows_filter: ListFollowsFilter,
    ) -> Result<Vec<LiveAccountModel>> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_platform_and_account_id_tiktok() {
        let url = url::Url::parse("https://www.tiktok.com/@yolp.ptkm/live").unwrap();
        let (platform, account_id) = LiveFollowingUseCase::<domain::repositories::live_following::MockLiveFollowingRepository>::parse_platform_and_account_id(&url).unwrap();
        assert_eq!(platform, Platform::TikTok);
        assert_eq!(account_id, "yolp.ptkm");
    }

    #[test]
    fn test_parse_platform_and_account_id_bigo() {
        let url = url::Url::parse("https://www.bigo.tv/GGee_p45.").unwrap();
        let (platform, account_id) = LiveFollowingUseCase::<domain::repositories::live_following::MockLiveFollowingRepository>::parse_platform_and_account_id(&url).unwrap();
        assert_eq!(platform, Platform::Bigo);
        assert_eq!(account_id, "GGee_p45.");
    }

    #[test]
    fn test_parse_platform_and_account_id_twitch() {
        let url = url::Url::parse("https://www.twitch.tv/arii").unwrap();
        let (platform, account_id) = LiveFollowingUseCase::<domain::repositories::live_following::MockLiveFollowingRepository>::parse_platform_and_account_id(&url).unwrap();
        assert_eq!(platform, Platform::Twitch);
        assert_eq!(account_id, "arii");
    }

    #[test]
    fn test_parse_platform_and_account_id_invalid() {
        let url = url::Url::parse("https://www.google.com").unwrap();
        let result = LiveFollowingUseCase::<domain::repositories::live_following::MockLiveFollowingRepository>::parse_platform_and_account_id(&url);
        assert!(result.is_err());
    }
}

