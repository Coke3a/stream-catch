use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result as AnyResult, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Duration, TimeZone, Utc};
use crates::{
    domain::{
        entities::plans::PlanEntity,
        repositories::{
            invoices::InvoiceRepository,
            payment_provider_customers::PaymentProviderCustomerRepository,
            payments::PaymentRepository, plans::PlanRepository,
            subscriptions::SubscriptionRepository,
        },
        value_objects::{
            enums::{
                billing_modes::BillingMode, payment_methods::PaymentMethod,
                payment_statuses::PaymentStatus, subscription_statuses::SubscriptionStatus,
            },
            plans::PlanFeatures,
            subscriptions::{CurrentSubscriptionDto, PlanDto},
        },
    },
    payments::stripe_client::{StripeClient, StripeEvent, StripeSubscription},
};
use serde::Deserialize;
use thiserror::Error;
use tracing::{debug, error, info};
use uuid::Uuid;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait StripeGateway: Send + Sync {
    async fn create_checkout_session(
        &self,
        price_id: &str,
        mode: &str,
        payment_method_types: Vec<String>,
        customer_id: Option<String>,
        metadata: HashMap<String, String>,
    ) -> AnyResult<String>;

    async fn cancel_subscription(&self, provider_subscription_id: &str) -> AnyResult<()>;

    fn verify_webhook_signature(&self, payload: &[u8], signature: &str) -> AnyResult<StripeEvent>;

    async fn retrieve_subscription(&self, subscription_id: &str) -> AnyResult<StripeSubscription>;
}

#[async_trait]
impl StripeGateway for StripeClient {
    async fn create_checkout_session(
        &self,
        price_id: &str,
        mode: &str,
        payment_method_types: Vec<String>,
        customer_id: Option<String>,
        metadata: HashMap<String, String>,
    ) -> AnyResult<String> {
        self.create_checkout_session(price_id, mode, payment_method_types, customer_id, metadata)
            .await
    }

    async fn cancel_subscription(&self, provider_subscription_id: &str) -> AnyResult<()> {
        self.cancel_subscription(provider_subscription_id).await
    }

    fn verify_webhook_signature(&self, payload: &[u8], signature: &str) -> AnyResult<StripeEvent> {
        self.verify_webhook_signature(payload, signature)
    }

    async fn retrieve_subscription(&self, subscription_id: &str) -> AnyResult<StripeSubscription> {
        self.retrieve_subscription(subscription_id).await
    }
}

#[derive(Debug, Error)]
pub enum SubscriptionError {
    #[error("plan not found")]
    PlanNotFound,
    #[error("missing or inactive plan price: {0}")]
    MissingPrice(&'static str),
    #[error("invalid payment combination: {0}")]
    InvalidCombination(String),
    #[error("user email is required for checkout")]
    MissingEmail,
    #[error("invalid webhook payload: {0}")]
    InvalidWebhook(String),
    #[error("no active subscription to cancel")]
    SubscriptionNotFound,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl SubscriptionError {
    pub fn status_code(&self) -> axum::http::StatusCode {
        use axum::http::StatusCode;
        match self {
            SubscriptionError::PlanNotFound => StatusCode::NOT_FOUND,
            SubscriptionError::MissingPrice(_)
            | SubscriptionError::InvalidCombination(_)
            | SubscriptionError::MissingEmail
            | SubscriptionError::InvalidWebhook(_) => StatusCode::BAD_REQUEST,
            SubscriptionError::SubscriptionNotFound => StatusCode::NOT_FOUND,
            SubscriptionError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub type UseCaseResult<T> = std::result::Result<T, SubscriptionError>;

pub struct SubscriptionUseCase<P, S, Pay, Cust, Inv, Stripe>
where
    P: PlanRepository + Send + Sync + 'static,
    S: SubscriptionRepository + Send + Sync + 'static,
    Pay: PaymentRepository + Send + Sync + 'static,
    Cust: PaymentProviderCustomerRepository + Send + Sync + 'static,
    Inv: InvoiceRepository + Send + Sync + 'static,
    Stripe: StripeGateway + Send + Sync + 'static,
{
    plan_repo: Arc<P>,
    subscription_repo: Arc<S>,
    payment_repo: Arc<Pay>,
    customer_repo: Arc<Cust>,
    invoice_repo: Arc<Inv>,
    stripe_client: Arc<Stripe>,
    free_plan_id: Uuid,
}

impl<P, S, Pay, Cust, Inv, Stripe> SubscriptionUseCase<P, S, Pay, Cust, Inv, Stripe>
where
    P: PlanRepository + Send + Sync + 'static,
    S: SubscriptionRepository + Send + Sync + 'static,
    Pay: PaymentRepository + Send + Sync + 'static,
    Cust: PaymentProviderCustomerRepository + Send + Sync + 'static,
    Inv: InvoiceRepository + Send + Sync + 'static,
    Stripe: StripeGateway + Send + Sync + 'static,
{
    pub fn new(
        plan_repo: Arc<P>,
        subscription_repo: Arc<S>,
        payment_repo: Arc<Pay>,
        customer_repo: Arc<Cust>,
        invoice_repo: Arc<Inv>,
        stripe_client: Arc<Stripe>,
        free_plan_id: Uuid,
    ) -> Self {
        Self {
            plan_repo,
            subscription_repo,
            payment_repo,
            customer_repo,
            invoice_repo,
            stripe_client,
            free_plan_id,
        }
    }

    pub async fn list_plans(&self) -> UseCaseResult<Vec<PlanDto>> {
        let plans = self.plan_repo.list_active_plans().await?;
        Ok(plans.into_iter().map(PlanDto::from).collect())
    }

    pub async fn get_current_subscription(
        &self,
        user_id: Uuid,
    ) -> UseCaseResult<Option<CurrentSubscriptionDto>> {
        let subscription = match self
            .subscription_repo
            .find_current_active_non_free_subscription(user_id, self.free_plan_id)
            .await?
        {
            Some(sub) => sub,
            None => return Ok(None),
        };

        let plan = self
            .plan_repo
            .find_active_plan_by_id(subscription.plan_id)
            .await?;

        Ok(Some(CurrentSubscriptionDto {
            plan_id: plan.id,
            plan_name: plan.name,
            billing_mode: BillingMode::from_str(&subscription.billing_mode)
                .unwrap_or(BillingMode::Recurring),
            status: SubscriptionStatus::from_str(&subscription.status),
            starts_at: subscription.starts_at,
            ends_at: subscription.ends_at,
            features: plan.features,
        }))
    }

    pub async fn create_checkout_session(
        &self,
        user_id: Uuid,
        user_email: Option<String>,
        plan_id: Uuid,
        billing_mode: BillingMode,
        payment_method: PaymentMethod,
    ) -> UseCaseResult<String> {
        let email = user_email.ok_or(SubscriptionError::MissingEmail)?;

        if plan_id == self.free_plan_id {
            return Err(SubscriptionError::InvalidCombination(
                "free plan does not require checkout".to_string(),
            ));
        }

        let plan = self.plan_repo.find_active_plan_by_id(plan_id).await?;
        let current_subscription = self
            .subscription_repo
            .find_current_active_non_free_subscription(user_id, self.free_plan_id)
            .await?;

        if let Some(current_subscription) = current_subscription.as_ref() {
            let current_billing_mode = BillingMode::from_str(&current_subscription.billing_mode)
                .unwrap_or(BillingMode::Recurring);

            if current_billing_mode == BillingMode::Recurring
                && (billing_mode == BillingMode::Recurring || billing_mode == BillingMode::Manual)
            {
                let provider_subscription_id = current_subscription
                    .provider_subscription_id
                    .clone()
                    .ok_or_else(|| {
                        SubscriptionError::Internal(anyhow!(
                            "recurring subscription missing provider id"
                        ))
                    })?;

                info!(
                    %user_id,
                    %provider_subscription_id,
                    "subscriptions: scheduling cancel_at_period_end for existing recurring subscription"
                );

                self.stripe_client
                    .cancel_subscription(&provider_subscription_id)
                    .await?;

                self.subscription_repo
                    .cancel_recurring_subscription(user_id)
                    .await?;
            }
        }

        let manual_period = if billing_mode == BillingMode::Manual {
            let now = Utc::now();
            let starts_at = match current_subscription.as_ref() {
                Some(current) => {
                    let current_billing_mode = BillingMode::from_str(&current.billing_mode)
                        .unwrap_or(BillingMode::Recurring);
                    match current_billing_mode {
                        BillingMode::Recurring => current.ends_at,
                        BillingMode::Manual => {
                            if current.ends_at > now {
                                current.ends_at
                            } else {
                                now
                            }
                        }
                    }
                }
                None => now,
            };

            let ends_at = starts_at
                .checked_add_signed(Duration::days(plan.duration_days.into()))
                .context("failed to compute subscription end date")?;

            Some((starts_at, ends_at))
        } else {
            None
        };

        let price_id = Self::pick_price_id(&plan, billing_mode, payment_method)?;
        let customer_id = self
            .customer_repo
            .find_or_create_stripe_customer_id(user_id, &email)
            .await?;

        let mut metadata = HashMap::from([
            ("user_id".to_string(), user_id.to_string()),
            ("plan_id".to_string(), plan_id.to_string()),
            ("billing_mode".to_string(), billing_mode.to_string()),
            ("payment_method".to_string(), payment_method.to_string()),
        ]);

        if let Some((manual_starts_at, manual_ends_at)) = manual_period {
            metadata.insert(
                "manual_starts_at".to_string(),
                manual_starts_at.timestamp().to_string(),
            );
            metadata.insert(
                "manual_ends_at".to_string(),
                manual_ends_at.timestamp().to_string(),
            );
        }

        let payment_method_types = vec![payment_method.to_string()];
        let mode = match billing_mode {
            BillingMode::Recurring => "subscription",
            BillingMode::Manual => "payment",
        };

        info!(
            %user_id,
            %plan_id,
            billing_mode = %billing_mode,
            payment_method = %payment_method,
            "subscriptions: creating checkout session"
        );

        let checkout_url = self
            .stripe_client
            .create_checkout_session(
                &price_id,
                mode,
                payment_method_types,
                Some(customer_id),
                metadata,
            )
            .await?;

        Ok(checkout_url)
    }

    pub async fn handle_stripe_webhook(
        &self,
        payload: &[u8],
        signature: &str,
    ) -> UseCaseResult<()> {
        let event = self
            .stripe_client
            .verify_webhook_signature(payload, signature)
            .map_err(|err| {
                error!("stripe webhook verification failed: {err}");
                SubscriptionError::InvalidWebhook("signature verification failed".into())
            })?;

        let event_type = event.type_.clone();

        match event_type.as_str() {
            "checkout.session.completed" => {
                self.handle_checkout_completed(&event).await?;
            }
            "customer.subscription.deleted" => {
                self.handle_subscription_deleted(&event).await?;
            }
            "invoice.payment_failed" => {
                self.handle_invoice_status_change(&event, SubscriptionStatus::PastDue)
                    .await?;
            }
            "invoice.payment_succeeded" => {
                self.handle_invoice_status_change(&event, SubscriptionStatus::Active)
                    .await?;
            }
            _ => {
                debug!("unhandled stripe event type: {:?}", event.type_);
            }
        }

        Ok(())
    }

    pub async fn cancel_recurring_subscription(&self, user_id: Uuid) -> UseCaseResult<()> {
        let subscription = self
            .subscription_repo
            .find_current_active_non_free_subscription(user_id, self.free_plan_id)
            .await?
            .ok_or(SubscriptionError::SubscriptionNotFound)?;

        let billing_mode =
            BillingMode::from_str(&subscription.billing_mode).unwrap_or(BillingMode::Recurring);
        if billing_mode != BillingMode::Recurring {
            return Err(SubscriptionError::InvalidCombination(
                "only recurring subscriptions can be canceled".to_string(),
            ));
        }

        let provider_subscription_id = subscription
            .provider_subscription_id
            .clone()
            .ok_or(SubscriptionError::SubscriptionNotFound)?;

        info!(%user_id, "subscriptions: canceling recurring subscription at Stripe");
        self.stripe_client
            .cancel_subscription(&provider_subscription_id)
            .await?;

        self.subscription_repo
            .cancel_recurring_subscription(user_id)
            .await?;

        Ok(())
    }

    fn pick_price_id(
        plan: &PlanEntity,
        billing_mode: BillingMode,
        payment_method: PaymentMethod,
    ) -> UseCaseResult<String> {
        match billing_mode {
            BillingMode::Recurring => {
                if payment_method != PaymentMethod::Card {
                    return Err(SubscriptionError::InvalidCombination(
                        "recurring billing is card-only".to_string(),
                    ));
                }
                plan.stripe_price_recurring
                    .clone()
                    .ok_or(SubscriptionError::MissingPrice("stripe_price_recurring"))
            }
            BillingMode::Manual => {
                match payment_method {
                    PaymentMethod::Card => plan.stripe_price_one_time_card.clone().ok_or(
                        SubscriptionError::MissingPrice("stripe_price_one_time_card"),
                    ),
                    PaymentMethod::PromptPay => plan.stripe_price_one_time_promptpay.clone().ok_or(
                        SubscriptionError::MissingPrice("stripe_price_one_time_promptpay"),
                    ),
                }
            }
        }
    }

    async fn handle_checkout_completed(&self, event: &StripeEvent) -> UseCaseResult<()> {
        let session = StripeClient::extract_checkout_session(&event).ok_or_else(|| {
            SubscriptionError::InvalidWebhook("missing checkout session".to_string())
        })?;

        let metadata = session
            .metadata
            .clone()
            .ok_or_else(|| SubscriptionError::InvalidWebhook("missing metadata".to_string()))?;

        let user_id = metadata
            .get("user_id")
            .and_then(|v| Uuid::parse_str(v).ok())
            .ok_or_else(|| SubscriptionError::InvalidWebhook("missing user_id".to_string()))?;
        let plan_id = metadata
            .get("plan_id")
            .and_then(|v| Uuid::parse_str(v).ok())
            .ok_or_else(|| SubscriptionError::InvalidWebhook("missing plan_id".to_string()))?;
        let payment_method = metadata
            .get("payment_method")
            .and_then(|v| PaymentMethod::from_str(v))
            .unwrap_or(PaymentMethod::Card);

        if plan_id == self.free_plan_id {
            return Err(SubscriptionError::InvalidWebhook(
                "free plan cannot be purchased".to_string(),
            ));
        }

        let plan = self.plan_repo.find_active_plan_by_id(plan_id).await?;

        if let Some(customer) = session.customer.as_deref() {
            self.customer_repo
                .upsert_customer_ref(user_id, "stripe", customer)
                .await?;
        }

        let provider_session_ref = session.id.clone().unwrap_or_default();
        let provider_payment_id = session.payment_intent.clone();

        match session.mode.as_deref() {
            Some("subscription") => {
                let subscription_id = session.subscription.clone().ok_or_else(|| {
                    SubscriptionError::InvalidWebhook(
                        "subscription id missing on session".to_string(),
                    )
                })?;

                let subscription = self
                    .stripe_client
                    .retrieve_subscription(&subscription_id)
                    .await?;

                let starts_at = Self::ts_to_datetime(subscription.current_period_start)
                    .ok_or_else(|| {
                        SubscriptionError::InvalidWebhook(
                            "period start missing on subscription".to_string(),
                        )
                    })?;
                let ends_at =
                    Self::ts_to_datetime(subscription.current_period_end).ok_or_else(|| {
                        SubscriptionError::InvalidWebhook(
                            "period end missing on subscription".to_string(),
                        )
                    })?;

                self.subscription_repo
                    .create_or_update_subscription_after_checkout(
                        user_id,
                        plan_id,
                        BillingMode::Recurring,
                        starts_at,
                        ends_at,
                        SubscriptionStatus::Active,
                        Some(subscription_id.clone()),
                    )
                    .await?;

                let invoice_id = self
                    .invoice_repo
                    .create_invoice(crates::domain::entities::invoices::InsertInvoiceEntity {
                        user_id,
                        subscription_id: None,
                        plan_id,
                        amount_minor: plan.price_minor,
                        period_start: starts_at,
                        period_end: ends_at,
                        due_at: starts_at,
                        status: "paid".to_string(),
                        paid_at: Some(Utc::now()),
                    })
                    .await?;

                self.payment_repo
                    .record_payment(crates::domain::entities::payments::NewPaymentEntity {
                        invoice_id,
                        user_id,
                        provider: "stripe".to_string(),
                        method_type: payment_method.to_string(),
                        payment_method_id: None,
                        amount_minor: plan.price_minor,
                        status: PaymentStatus::Succeeded.to_string(),
                        provider_payment_id,
                        provider_session_ref: Some(provider_session_ref),
                        error: None,
                    })
                    .await?;
            }
            Some("payment") => {
                let (starts_at, ends_at) =
                    Self::manual_period_from_metadata(&metadata, plan.duration_days)?;

                self.subscription_repo
                    .create_or_update_subscription_after_checkout(
                        user_id,
                        plan_id,
                        BillingMode::Manual,
                        starts_at,
                        ends_at,
                        SubscriptionStatus::Active,
                        None,
                    )
                    .await?;

                let invoice_id = self
                    .invoice_repo
                    .create_invoice(crates::domain::entities::invoices::InsertInvoiceEntity {
                        user_id,
                        subscription_id: None,
                        plan_id,
                        amount_minor: plan.price_minor,
                        period_start: starts_at,
                        period_end: ends_at,
                        due_at: starts_at,
                        status: "paid".to_string(),
                        paid_at: Some(Utc::now()),
                    })
                    .await?;

                let amount_minor = session
                    .amount_total
                    .and_then(|v| i32::try_from(v).ok())
                    .unwrap_or(plan.price_minor);

                self.payment_repo
                    .record_payment(crates::domain::entities::payments::NewPaymentEntity {
                        invoice_id,
                        user_id,
                        provider: "stripe".to_string(),
                        method_type: payment_method.to_string(),
                        payment_method_id: None,
                        amount_minor,
                        status: PaymentStatus::Succeeded.to_string(),
                        provider_payment_id,
                        provider_session_ref: Some(provider_session_ref),
                        error: None,
                    })
                    .await?;
            }
            _ => {
                return Err(SubscriptionError::InvalidWebhook(
                    "unknown checkout session mode".to_string(),
                ));
            }
        }

        Ok(())
    }

    async fn handle_subscription_deleted(&self, event: &StripeEvent) -> UseCaseResult<()> {
        #[derive(Deserialize)]
        struct SubscriptionObject {
            id: Option<String>,
        }

        let subscription: SubscriptionObject = serde_json::from_value(event.data.object.clone())
            .map_err(|err| {
                error!("failed to parse subscription object from webhook: {err}");
                SubscriptionError::InvalidWebhook("invalid subscription payload".to_string())
            })?;

        let subscription_id = subscription.id.ok_or_else(|| {
            SubscriptionError::InvalidWebhook("missing subscription id".to_string())
        })?;

        info!(
            subscription_id = %subscription_id,
            "subscriptions: marking subscription expired from webhook"
        );

        self.subscription_repo
            .update_status_by_provider_subscription_id(
                &subscription_id,
                SubscriptionStatus::Expired,
            )
            .await?;

        Ok(())
    }

    async fn handle_invoice_status_change(
        &self,
        event: &StripeEvent,
        status: SubscriptionStatus,
    ) -> UseCaseResult<()> {
        #[derive(Deserialize)]
        struct InvoiceObject {
            subscription: Option<String>,
        }

        let invoice: InvoiceObject =
            serde_json::from_value(event.data.object.clone()).map_err(|err| {
                error!("failed to parse invoice object from webhook: {err}");
                SubscriptionError::InvalidWebhook("invalid invoice payload".to_string())
            })?;

        let subscription_id = invoice.subscription.ok_or_else(|| {
            SubscriptionError::InvalidWebhook("invoice missing subscription id".to_string())
        })?;

        info!(
            subscription_id = %subscription_id,
            status = %status,
            "subscriptions: updating status from invoice webhook"
        );

        self.subscription_repo
            .update_status_by_provider_subscription_id(&subscription_id, status)
            .await?;

        Ok(())
    }

    fn manual_period_from_metadata(
        metadata: &HashMap<String, String>,
        duration_days: i32,
    ) -> UseCaseResult<(DateTime<Utc>, DateTime<Utc>)> {
        let now = Utc::now();
        let starts_at = metadata
            .get("manual_starts_at")
            .and_then(|value| value.parse::<i64>().ok())
            .and_then(Self::ts_to_datetime)
            .unwrap_or(now);

        let ends_at = match metadata
            .get("manual_ends_at")
            .and_then(|value| value.parse::<i64>().ok())
            .and_then(Self::ts_to_datetime)
        {
            Some(value) => value,
            None => starts_at
                .checked_add_signed(Duration::days(duration_days.into()))
                .context("failed to compute subscription end date")?,
        };

        Ok((starts_at, ends_at))
    }

    fn ts_to_datetime(ts: i64) -> Option<DateTime<Utc>> {
        Utc.timestamp_opt(ts, 0).single()
    }
}
