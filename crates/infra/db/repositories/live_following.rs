use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use diesel::{Connection, RunQueryDsl, dsl::count_star, insert_into};
use diesel::{prelude::*, update};
use std::sync::Arc;
use uuid::Uuid;

use crate::domain;
use crate::infra::db::postgres::{
    postgres_connection::PgPoolSquad,
    schema::{follows, live_accounts},
};
use domain::{
    entities::{
        follows::{FollowEntity, InsertFollowEntity},
        live_accounts::{InsertLiveAccountEntity, LiveAccountEntity},
    },
    repositories::live_following::LiveFollowingRepository,
    value_objects::{
        enums::{follow_statuses::FollowStatus, sort_order::SortOrder},
        live_following::{FindLiveAccountModel, ListFollowsFilter},
    },
};

pub struct LiveFollowingPostgres {
    db_pool: Arc<PgPoolSquad>,
}

impl LiveFollowingPostgres {
    pub fn new(db_pool: Arc<PgPoolSquad>) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl LiveFollowingRepository for LiveFollowingPostgres {
    async fn follow_and_create_live_account(
        &self,
        mut follow_entity: InsertFollowEntity,
        live_account_entity: InsertLiveAccountEntity,
    ) -> Result<Uuid> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let result = conn.transaction::<Uuid, diesel::result::Error, _>(|tx| {
            let live_account_id: Uuid = insert_into(live_accounts::table)
                .values(&live_account_entity)
                .returning(live_accounts::id)
                .get_result::<Uuid>(tx)?;

            follow_entity.live_account_id = Some(live_account_id);
            insert_into(follows::table)
                .values(&follow_entity)
                .execute(tx)?;

            Ok(live_account_id)
        })?;

        Ok(result)
    }

    async fn follow(&self, follow_entity: InsertFollowEntity) -> Result<Uuid> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let result = insert_into(follows::table)
            .values(&follow_entity)
            .returning(follows::live_account_id)
            .get_result::<Uuid>(&mut conn)?;

        Ok(result)
    }

    async fn list_following_live_accounts(
        &self,
        user_id: Uuid,
        filter: &ListFollowsFilter,
    ) -> Result<Vec<LiveAccountEntity>> {
        let mut conn = Arc::clone(&self.db_pool).get()?;
        let mut query = follows::table
            .inner_join(live_accounts::table.on(follows::live_account_id.eq(live_accounts::id)))
            .select(LiveAccountEntity::as_select())
            .filter(follows::user_id.eq(user_id))
            .into_boxed();

        if let Some(live_account_id) = filter.live_account_id {
            query = query.filter(follows::live_account_id.eq(live_account_id));
        }

        if let Some(status) = filter.status {
            query = query.filter(follows::status.eq(status.to_string()));
        } else {
            query = query.filter(follows::status.ne(FollowStatus::Inactive.to_string()));
        }

        query = match filter.sort_order {
            SortOrder::Asc => query.order(follows::created_at.asc()),
            SortOrder::Desc => query.order(follows::created_at.desc()),
        };

        if let Some(limit) = filter.limit {
            query = query.limit(limit);
        }

        let results = query.load::<LiveAccountEntity>(&mut conn)?;

        Ok(results)
    }

    async fn to_active(&self, user_id: Uuid, live_account_id: Uuid) -> Result<()> {
        let mut conn = Arc::clone(&self.db_pool).get()?;
        let now = Utc::now();

        update(follows::table)
            .filter(follows::user_id.eq(user_id))
            .filter(follows::live_account_id.eq(live_account_id))
            .set((
                follows::status.eq(FollowStatus::Active.to_string()),
                follows::updated_at.eq(now),
            ))
            .execute(&mut conn)?;
        Ok(())
    }

    async fn find_live_account(
        &self,
        find_live_account_model: &FindLiveAccountModel,
    ) -> Result<LiveAccountEntity> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let result = live_accounts::table
            .filter(live_accounts::account_id.eq(&find_live_account_model.account_id))
            .filter(live_accounts::platform.eq(find_live_account_model.platform.to_string()))
            .first::<LiveAccountEntity>(&mut conn)?;

        Ok(result)
    }

    async fn find_follow(&self, user_id: Uuid, live_account_id: Uuid) -> Result<FollowEntity> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let result = follows::table
            .filter(follows::user_id.eq(user_id))
            .filter(follows::live_account_id.eq(live_account_id))
            .first::<FollowEntity>(&mut conn)?;

        Ok(result)
    }

    async fn count_active_follows(&self, user_id: Uuid) -> Result<i64> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let statuses = vec![
            FollowStatus::Active.to_string(),
            FollowStatus::TemporaryInactive.to_string(),
        ];

        let total = follows::table
            .filter(follows::user_id.eq(user_id))
            .filter(follows::status.eq_any(statuses))
            .select(count_star())
            .first::<i64>(&mut conn)?;

        Ok(total)
    }
}
