use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use diesel::{RunQueryDsl, insert_into, prelude::*, update};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    domain,
    infra::db::postgres::{postgres_connection::PgPoolSquad, schema::invoices},
};
use domain::{entities::invoices::InsertInvoiceEntity, repositories::invoices::InvoiceRepository};

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
}
