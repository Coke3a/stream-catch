use std::sync::Arc;
use anyhow::Result;
use axum::async_trait;
use uuid::Uuid;

use crate::{domain::{entities::{follows::InsertFollowEntity, live_accounts::{InsertLiveAccountEntity, LiveAccountEntity}}, repositories::live_following::LiveFollowingRepository, value_objects::live_following::{FindLiveAccountModel, ListFollowsFilter}}, infrastructure::postgres::postgres_connection::PgPoolSquad};

pub struct LiveFollowingPostgres {
    db_pool: Arc<PgPoolSquad>
}

impl LiveFollowingPostgres {
    pub fn new(db_pool: Arc<PgPoolSquad>) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl LiveFollowingRepository for LiveFollowingPostgres {
    async fn follow_and_create_live_account(&self, follow_entity: InsertFollowEntity, live_account_entry: InsertLiveAccountEntity) -> Result<i64> {
        unimplemented!()
    }
    
    async fn follow(&self, follow_entity: InsertFollowEntity) -> Result<i64> {
        unimplemented!()
    }

    async fn list_following_live_accounts(&self, user_id: Uuid, filter: &ListFollowsFilter) -> Result<Vec<LiveAccountEntity>> {
        unimplemented!()
    }
    
    async fn unfollow(&self, follow_id: i64) -> Result<()> {
        unimplemented!()
    }
    
    async fn find_live_account(&self, follow_entity: &FindLiveAccountModel) -> Result<LiveAccountEntity> {
        unimplemented!()
    }

}