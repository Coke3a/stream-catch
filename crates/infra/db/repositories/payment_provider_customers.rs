use anyhow::Result;
use async_trait::async_trait;
use diesel::{OptionalExtension, RunQueryDsl, insert_into, prelude::*, update};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    domain,
    infra::db::postgres::{postgres_connection::PgPoolSquad, schema::payment_provider_customers},
    payments::stripe_client::StripeClient,
};
use domain::{
    entities::payment_provider_customers::InsertPaymentProviderCustomerEntity,
    repositories::payment_provider_customers::PaymentProviderCustomerRepository,
};

pub struct PaymentProviderCustomerPostgres {
    db_pool: Arc<PgPoolSquad>,
    stripe_client: Arc<StripeClient>,
}

impl PaymentProviderCustomerPostgres {
    pub fn new(db_pool: Arc<PgPoolSquad>, stripe_client: Arc<StripeClient>) -> Self {
        Self {
            db_pool,
            stripe_client,
        }
    }
}

#[async_trait]
impl PaymentProviderCustomerRepository for PaymentProviderCustomerPostgres {
    async fn find_or_create_stripe_customer_id(
        &self,
        user_id: Uuid,
        email: &str,
    ) -> Result<String> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        if let Some(existing) = payment_provider_customers::table
            .filter(payment_provider_customers::user_id.eq(user_id))
            .filter(payment_provider_customers::provider.eq("stripe"))
            .select(payment_provider_customers::customer_ref)
            .first::<String>(&mut conn)
            .optional()?
        {
            return Ok(existing);
        }

        let customer_ref = self.stripe_client.create_customer(email, user_id).await?;

        let insert_entity = InsertPaymentProviderCustomerEntity {
            user_id,
            provider: "stripe".to_string(),
            customer_ref: customer_ref.clone(),
            metadata: json!({ "email": email }),
        };

        insert_into(payment_provider_customers::table)
            .values(&insert_entity)
            .execute(&mut conn)?;

        Ok(customer_ref)
    }

    async fn upsert_customer_ref(
        &self,
        user_id: Uuid,
        provider: &str,
        customer_ref: &str,
    ) -> Result<()> {
        let mut conn = Arc::clone(&self.db_pool).get()?;

        if let Some(existing_id) = payment_provider_customers::table
            .filter(payment_provider_customers::user_id.eq(user_id))
            .filter(payment_provider_customers::provider.eq(provider))
            .select(payment_provider_customers::id)
            .first::<Uuid>(&mut conn)
            .optional()?
        {
            update(
                payment_provider_customers::table
                    .filter(payment_provider_customers::id.eq(existing_id)),
            )
            .set((
                payment_provider_customers::customer_ref.eq(customer_ref),
                payment_provider_customers::metadata.eq(json!({})),
            ))
            .execute(&mut conn)?;
            return Ok(());
        }

        let insert_entity = InsertPaymentProviderCustomerEntity {
            user_id,
            provider: provider.to_string(),
            customer_ref: customer_ref.to_string(),
            metadata: json!({}),
        };

        insert_into(payment_provider_customers::table)
            .values(&insert_entity)
            .execute(&mut conn)?;

        Ok(())
    }
}
