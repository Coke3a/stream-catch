use std::sync::Arc;
use anyhow::Result;
use uuid::Uuid;

use crate::domain::{repositories::live_following::LiveFollowingRepository, value_objects::live_following::{ListFollowsFilter, LiveAccountModel}};

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
        unimplemented!()
    }
    
    pub async fn unfollow(&self, follow_id: i64) -> Result<()> {
        unimplemented!()
    }
    
    pub async fn list_follows(&self, user_id: Uuid, list_follows_filter: ListFollowsFilter) -> Result<Vec<LiveAccountModel>> {
        unimplemented!()
    }
    
}

