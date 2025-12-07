use anyhow::Result;
use async_trait::async_trait;
use diesel::{RunQueryDsl, insert_into};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    domain,
    infra::db::postgres::{postgres_connection::PgPoolSquad, schema::payments},
};
use domain::{entities::payments::NewPaymentEntity, repositories::payments::PaymentRepository};

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
}
