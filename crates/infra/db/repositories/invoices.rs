use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use diesel::{OptionalExtension, RunQueryDsl, insert_into, prelude::*, update};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    domain,
    infra::db::postgres::{postgres_connection::PgPoolSquad, schema::invoices},
};
use domain::{
    entities::invoices::{InsertInvoiceEntity, InvoiceEntity},
    repositories::invoices::InvoiceRepository,
};

pub struct InvoicePostgres {
    db_pool: Arc<PgPoolSquad>,
}

impl InvoicePostgres {
    pub fn new(db_pool: Arc<PgPoolSquad>) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl InvoiceRepository for InvoicePostgres {
    async fn create_invoice(&self, invoice: InsertInvoiceEntity) -> Result<Uuid> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let invoice_id = insert_into(invoices::table)
            .values(&invoice)
            .returning(invoices::id)
            .get_result::<Uuid>(&mut conn)?;

        Ok(invoice_id)
    }

    async fn mark_invoice_paid(&self, invoice_id: Uuid) -> Result<()> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        update(invoices::table.filter(invoices::id.eq(invoice_id)))
            .set((
                invoices::status.eq("paid"),
                invoices::paid_at.eq(Some(Utc::now())),
            ))
            .execute(&mut conn)?;

        Ok(())
    }

    async fn update_status_by_id(&self, invoice_id: Uuid, status: &str) -> Result<()> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        update(invoices::table.filter(invoices::id.eq(invoice_id)))
            .set((
                invoices::status.eq(status),
                invoices::paid_at.eq::<Option<DateTime<Utc>>>(None),
            ))
            .execute(&mut conn)?;

        Ok(())
    }

    async fn find_by_subscription_and_period_start(
        &self,
        subscription_id: Uuid,
        period_start: DateTime<Utc>,
    ) -> Result<Option<InvoiceEntity>> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        let invoice = invoices::table
            .filter(invoices::subscription_id.eq(subscription_id))
            .filter(invoices::period_start.eq(period_start))
            .first::<InvoiceEntity>(&mut conn)
            .optional()?;

        Ok(invoice)
    }
}
