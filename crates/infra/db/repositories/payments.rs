use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use diesel::{OptionalExtension, RunQueryDsl, insert_into, prelude::*, update};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    domain,
    infra::db::postgres::{postgres_connection::PgPoolSquad, schema::payments},
};
use domain::{
    entities::payments::NewPaymentEntity,
    repositories::payments::PaymentRepository,
    value_objects::enums::payment_statuses::PaymentStatus,
};

pub struct PaymentPostgres {
    db_pool: Arc<PgPoolSquad>,
}

impl PaymentPostgres {
    pub fn new(db_pool: Arc<PgPoolSquad>) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl PaymentRepository for PaymentPostgres {
    async fn record_payment(&self, payment: NewPaymentEntity) -> Result<Uuid> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let payment_id = insert_into(payments::table)
            .values(&payment)
            .returning(payments::id)
            .get_result::<Uuid>(&mut conn)?;

        Ok(payment_id)
    }

    async fn update_status_by_provider_payment_id(
        &self,
        provider_payment_id: &str,
        status: PaymentStatus,
    ) -> Result<()> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        update(payments::table.filter(payments::provider_payment_id.eq(provider_payment_id)))
            .set((
                payments::status.eq(status.to_string()),
                payments::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)?;

        Ok(())
    }

    async fn exists_by_provider_payment_id(&self, provider_payment_id: &str) -> Result<bool> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let exists = payments::table
            .filter(payments::provider_payment_id.eq(provider_payment_id))
            .select(payments::id)
            .first::<Uuid>(&mut conn)
            .optional()?
            .is_some();

        Ok(exists)
    }
}
