use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result as AnyResult, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Duration, TimeZone, Utc};
use crates::{
    domain::{
        entities::{plans::PlanEntity, subscriptions::SubscriptionEntity},
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
            subscriptions::{CurrentSubscriptionDto, PlanDto},
        },
    },
    payments::stripe_client::{
        StripeCheckoutSession, StripeClient, StripeEvent, StripeSubscription,
    },
};
use serde::Deserialize;
use thiserror::Error;
use tracing::{error, info, warn};
use uuid::Uuid;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait StripeGateway: Send + Sync {
    async fn create_checkout_session(
        &self,
        price_id: &str,
        mode: &str,
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
        customer_id: Option<String>,
        metadata: HashMap<String, String>,
    ) -> AnyResult<String> {
        self.create_checkout_session(price_id, mode, customer_id, metadata)
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

struct InvoiceContext {
    invoice_id: Option<String>,
    subscription_id: String,
    customer_id: Option<String>,
    status: Option<String>,
    payment_intent_id: Option<String>,
    currency: Option<String>,
    period_start: Option<DateTime<Utc>>,
    period_end: Option<DateTime<Utc>>,
    amount_due: Option<i64>,
    amount_paid: Option<i64>,
}

impl InvoiceContext {
    fn period(&self) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
        self.period_start.zip(self.period_end)
    }

    fn amount_minor(&self) -> Option<i32> {
        self.amount_paid
            .or(self.amount_due)
            .and_then(|value| i32::try_from(value).ok())
    }
}

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
        info!("subscriptions: listing active plans");
        let plans = self.plan_repo.list_active_plans().await.map_err(|err| {
            error!(db_error = ?err, "subscriptions: failed to list active plans");
            SubscriptionError::Internal(err)
        })?;
        let plan_count = plans.len();
        info!(plan_count, "subscriptions: active plans loaded");
        Ok(plans.into_iter().map(PlanDto::from).collect())
    }

    pub async fn get_current_subscription(
        &self,
        user_id: Uuid,
    ) -> UseCaseResult<Option<CurrentSubscriptionDto>> {
        info!(
            %user_id,
            "subscriptions: loading current subscription for user"
        );
        let subscription = match self
            .subscription_repo
            .find_current_active_non_free_subscription(user_id, self.free_plan_id)
            .await
            .map_err(|err| {
                error!(
                    %user_id,
                    db_error = ?err,
                    "subscriptions: failed to load current subscription"
                );
                SubscriptionError::Internal(err)
            })? {
            Some(sub) => sub,
            None => {
                info!(%user_id, "subscriptions: no active subscription");
                return Ok(None);
            }
        };

        let plan = self
            .plan_repo
            .find_active_plan_by_id(subscription.plan_id)
            .await
            .map_err(|err| {
                error!(
                    %user_id,
                    plan_id = %subscription.plan_id,
                    db_error = ?err,
                    "subscriptions: failed to load active plan"
                );
                SubscriptionError::Internal(err)
            })?;

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
        info!(
            %user_id,
            %plan_id,
            billing_mode = %billing_mode,
            payment_method = %payment_method,
            "subscriptions: create checkout session requested"
        );

        let email = match user_email {
            Some(value) => value,
            None => {
                let err = SubscriptionError::MissingEmail;
                warn!(
                    %user_id,
                    %plan_id,
                    status = err.status_code().as_u16(),
                    "subscriptions: missing email for checkout"
                );
                return Err(err);
            }
        };

        if plan_id == self.free_plan_id {
            let err = SubscriptionError::InvalidCombination(
                "free plan does not require checkout".to_string(),
            );
            warn!(
                %user_id,
                %plan_id,
                status = err.status_code().as_u16(),
                "subscriptions: free plan checkout attempted"
            );
            return Err(err);
        }

        let plan = self
            .plan_repo
            .find_active_plan_by_id(plan_id)
            .await
            .map_err(|err| {
                error!(
                    %user_id,
                    %plan_id,
                    db_error = ?err,
                    "subscriptions: failed to load plan for checkout"
                );
                SubscriptionError::Internal(err)
            })?;
        let current_subscription = self
            .subscription_repo
            .find_current_active_non_free_subscription(user_id, self.free_plan_id)
            .await
            .map_err(|err| {
                error!(
                    %user_id,
                    db_error = ?err,
                    "subscriptions: failed to load current subscription before checkout"
                );
                SubscriptionError::Internal(err)
            })?;

        if let Some(current_subscription) = current_subscription.as_ref() {
            let current_billing_mode = BillingMode::from_str(&current_subscription.billing_mode)
                .unwrap_or(BillingMode::Recurring);

            if current_billing_mode == BillingMode::Recurring
                && (billing_mode == BillingMode::Recurring || billing_mode == BillingMode::OneTime)
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
                    .await
                    .map_err(|err| {
                        error!(
                            %user_id,
                            %provider_subscription_id,
                            error = ?err,
                            "subscriptions: failed to cancel provider subscription before checkout"
                        );
                        err
                    })?;

                self.subscription_repo
                    .cancel_recurring_subscription(user_id)
                    .await
                    .map_err(|err| {
                        error!(
                            %user_id,
                            %provider_subscription_id,
                            db_error = ?err,
                            "subscriptions: failed to mark recurring subscription canceled"
                        );
                        SubscriptionError::Internal(err)
                    })?;
            }
        }

        let one_time_period = if billing_mode == BillingMode::OneTime {
            let now = Utc::now();
            let starts_at = match current_subscription.as_ref() {
                Some(current) => {
                    let current_billing_mode = BillingMode::from_str(&current.billing_mode)
                        .unwrap_or(BillingMode::Recurring);
                    match current_billing_mode {
                        BillingMode::Recurring => current.ends_at,
                        BillingMode::OneTime => {
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
            .await
            .map_err(|err| {
                error!(
                    %user_id,
                    %plan_id,
                    error = ?err,
                    "subscriptions: failed to resolve stripe customer id"
                );
                SubscriptionError::Internal(err)
            })?;

        let mut metadata = HashMap::from([
            ("user_id".to_string(), user_id.to_string()),
            ("plan_id".to_string(), plan_id.to_string()),
            ("billing_mode".to_string(), billing_mode.to_string()),
            ("payment_method".to_string(), payment_method.to_string()),
        ]);

        if let Some((one_time_starts_at, one_time_ends_at)) = one_time_period {
            metadata.insert(
                "one_time_starts_at".to_string(),
                one_time_starts_at.timestamp().to_string(),
            );
            metadata.insert(
                "one_time_ends_at".to_string(),
                one_time_ends_at.timestamp().to_string(),
            );
        }

        let mode = match billing_mode {
            BillingMode::Recurring => "subscription",
            BillingMode::OneTime => "payment",
        };

        info!(
            %user_id,
            %plan_id,
            billing_mode = %billing_mode,
            payment_method = %payment_method,
            metadata = ?metadata,
            price_id = %price_id,
            customer_id = %customer_id,
            "subscriptions: creating checkout session"
        );

        let checkout_url = self
            .stripe_client
            .create_checkout_session(&price_id, mode, Some(customer_id.clone()), metadata)
            .await
            .map_err(|err| {
                error!(
                    %user_id,
                    %plan_id,
                    price_id = %price_id,
                    billing_mode = %billing_mode,
                    payment_method = %payment_method,
                    customer_id = %customer_id,
                    error = ?err,
                    "subscriptions: stripe checkout session creation failed"
                );
                SubscriptionError::Internal(err)
            })?;

        info!(
            %user_id,
            %plan_id,
            checkout_url = %checkout_url,
            "subscriptions: checkout session created successfully"
        );

        Ok(checkout_url)
    }

    pub async fn handle_stripe_webhook(
        &self,
        payload: &[u8],
        signature: &str,
    ) -> UseCaseResult<()> {
        info!(
            payload = %String::from_utf8_lossy(payload),
            signature,
            "subscriptions: stripe webhook payload received"
        );
        let event = self
            .stripe_client
            .verify_webhook_signature(payload, signature)
            .map_err(|err| {
                error!(
                    error = %err,
                    status = SubscriptionError::InvalidWebhook("".into()).status_code().as_u16(),
                    "stripe webhook verification failed"
                );
                SubscriptionError::InvalidWebhook("signature verification failed".into())
            })?;

        let event_type = event.type_.clone();
        info!(
            stripe_event_id = ?event.id,
            event_type = %event_type,
            created = ?event.created,
            livemode = ?event.livemode,
            api_version = ?event.api_version,
            request = ?event.request,
            "subscriptions: stripe webhook verified"
        );

        match event_type.as_str() {
            "checkout.session.completed" => self.handle_checkout_completed(&event).await?,
            "checkout.session.expired" => self.handle_checkout_expired(&event).await?,
            "customer.subscription.deleted" => self.handle_subscription_deleted(&event).await?,
            "invoice.payment_succeeded" => {
                self.handle_invoice_payment_succeeded(&event).await?
            }
            "invoice.payment_failed" => self.handle_invoice_payment_failed(&event).await?,
            "payment_intent.succeeded" => self.handle_payment_intent_succeeded(&event).await?,
            "payment_intent.payment_failed" => {
                self.handle_payment_intent_failed(&event, PaymentStatus::Failed, "failed")
                    .await?
            }
            "payment_intent.canceled" => {
                self.handle_payment_intent_failed(&event, PaymentStatus::Canceled, "void")
                    .await?
            }
            _ => {
                error!(
                    stripe_event_id = ?event.id,
                    event_type = %event.type_,
                    created = ?event.created,
                    livemode = ?event.livemode,
                    api_version = ?event.api_version,
                    request = ?event.request,
                    "subscriptions: unhandled stripe event type"
                );
            }
        }

        Ok(())
    }

    pub async fn cancel_recurring_subscription(&self, user_id: Uuid) -> UseCaseResult<()> {
        let subscription = self
            .subscription_repo
            .find_current_active_non_free_subscription(user_id, self.free_plan_id)
            .await
            .map_err(|err| {
                error!(
                    %user_id,
                    db_error = ?err,
                    "subscriptions: failed to load current subscription for cancel"
                );
                SubscriptionError::Internal(err)
            })?
            .ok_or_else(|| {
                let err = SubscriptionError::SubscriptionNotFound;
                warn!(
                    %user_id,
                    status = err.status_code().as_u16(),
                    "subscriptions: no active recurring subscription to cancel"
                );
                err
            })?;

        let billing_mode =
            BillingMode::from_str(&subscription.billing_mode).unwrap_or(BillingMode::Recurring);
        if billing_mode != BillingMode::Recurring {
            let err = SubscriptionError::InvalidCombination(
                "only recurring subscriptions can be canceled".to_string(),
            );
            warn!(
                %user_id,
                status = err.status_code().as_u16(),
                billing_mode = %subscription.billing_mode,
                "subscriptions: attempted to cancel non-recurring subscription"
            );
            return Err(err);
        }

        let provider_subscription_id =
            subscription
                .provider_subscription_id
                .clone()
                .ok_or_else(|| {
                    let err = SubscriptionError::SubscriptionNotFound;
                    warn!(
                        %user_id,
                        status = err.status_code().as_u16(),
                        "subscriptions: recurring subscription missing provider id"
                    );
                    err
                })?;

        info!(%user_id, "subscriptions: canceling recurring subscription at Stripe");
        self.stripe_client
            .cancel_subscription(&provider_subscription_id)
            .await
            .map_err(|err| {
                error!(
                    %user_id,
                    %provider_subscription_id,
                    error = ?err,
                    "subscriptions: stripe cancel subscription failed"
                );
                SubscriptionError::Internal(err)
            })?;

        self.subscription_repo
            .cancel_recurring_subscription(user_id)
            .await
            .map_err(|err| {
                error!(
                    %user_id,
                    %provider_subscription_id,
                    db_error = ?err,
                    "subscriptions: failed to mark recurring subscription canceled"
                );
                SubscriptionError::Internal(err)
            })?;

        info!(
            %user_id,
            %provider_subscription_id,
            "subscriptions: recurring subscription cancellation completed"
        );

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
                    let err = SubscriptionError::InvalidCombination(
                        "recurring billing is card-only".to_string(),
                    );
                    warn!(
                        status = err.status_code().as_u16(),
                        billing_mode = %billing_mode,
                        payment_method = %payment_method,
                        "subscriptions: invalid recurring payment combination"
                    );
                    return Err(err);
                }
                plan.stripe_price_recurring.clone().ok_or_else(|| {
                    let err = SubscriptionError::MissingPrice("stripe_price_recurring");
                    warn!(
                        status = err.status_code().as_u16(),
                        plan_id = %plan.id,
                        "subscriptions: missing recurring price"
                    );
                    err
                })
            }
            BillingMode::OneTime => match payment_method {
                PaymentMethod::Card => plan.stripe_price_one_time_card.clone().ok_or_else(|| {
                    let err = SubscriptionError::MissingPrice("stripe_price_one_time_card");
                    warn!(
                        status = err.status_code().as_u16(),
                        plan_id = %plan.id,
                        "subscriptions: missing one-time card price"
                    );
                    err
                }),
                PaymentMethod::PromptPay => {
                    plan.stripe_price_one_time_promptpay.clone().ok_or_else(|| {
                        let err =
                            SubscriptionError::MissingPrice("stripe_price_one_time_promptpay");
                        warn!(
                            status = err.status_code().as_u16(),
                            plan_id = %plan.id,
                            "subscriptions: missing promptpay price"
                        );
                        err
                    })
                }
            },
        }
    }

    async fn handle_checkout_completed(&self, event: &StripeEvent) -> UseCaseResult<()> {
        info!(
            stripe_event_id = ?event.id,
            event_type = %event.type_,
            payload = ?event.data.object,
            created = ?event.created,
            livemode = ?event.livemode,
            api_version = ?event.api_version,
            request = ?event.request,
            "subscriptions: processing checkout completed webhook"
        );

        let session = StripeClient::extract_checkout_session(event).ok_or_else(|| {
            let err = SubscriptionError::InvalidWebhook("missing checkout session".to_string());
            error!(
                stripe_event_id = ?event.id,
                status = err.status_code().as_u16(),
                "subscriptions: checkout session missing in webhook"
            );
            err
        })?;

        let metadata = session.metadata.clone().ok_or_else(|| {
            let err = SubscriptionError::InvalidWebhook("missing metadata".to_string());
            error!(
                stripe_event_id = ?event.id,
                status = err.status_code().as_u16(),
                "subscriptions: missing metadata on checkout session"
            );
            err
        })?;

        let user_id = metadata
            .get("user_id")
            .and_then(|v| Uuid::parse_str(v).ok())
            .ok_or_else(|| {
                let err = SubscriptionError::InvalidWebhook("missing user_id".to_string());
                error!(
                    stripe_event_id = ?event.id,
                    status = err.status_code().as_u16(),
                    "subscriptions: missing user_id in checkout metadata"
                );
                err
            })?;
        let plan_id = metadata
            .get("plan_id")
            .and_then(|v| Uuid::parse_str(v).ok())
            .ok_or_else(|| {
                let err = SubscriptionError::InvalidWebhook("missing plan_id".to_string());
                error!(
                    stripe_event_id = ?event.id,
                    %user_id,
                    status = err.status_code().as_u16(),
                    "subscriptions: missing plan_id in checkout metadata"
                );
                err
            })?;
        let payment_method = metadata
            .get("payment_method")
            .and_then(|v| PaymentMethod::from_str(v))
            .unwrap_or(PaymentMethod::Card);

        if plan_id == self.free_plan_id {
            let err =
                SubscriptionError::InvalidWebhook("free plan cannot be purchased".to_string());
            error!(
                stripe_event_id = ?event.id,
                %user_id,
                %plan_id,
                status = err.status_code().as_u16(),
                "subscriptions: free plan cannot be purchased from webhook"
            );
            return Err(err);
        }

        let plan = self
            .plan_repo
            .find_active_plan_by_id(plan_id)
            .await
            .map_err(|err| {
                error!(
                    %user_id,
                    %plan_id,
                    db_error = ?err,
                    "subscriptions: failed to load plan during webhook"
                );
                SubscriptionError::Internal(err)
            })?;

        if let Some(customer) = session.customer.as_deref() {
            self.customer_repo
                .upsert_customer_ref(user_id, "stripe", customer)
                .await
                .map_err(|err| {
                    error!(
                        %user_id,
                        %plan_id,
                        customer_id = customer,
                        db_error = ?err,
                        "subscriptions: failed to upsert stripe customer ref"
                    );
                    SubscriptionError::Internal(err)
                })?;
        }

        match session.mode.as_deref() {
            Some("payment") => {
                self.handle_checkout_completed_one_time(
                    event,
                    &session,
                    &metadata,
                    user_id,
                    &plan,
                    payment_method,
                )
                .await?;
            }
            Some("subscription") => {
                self.handle_checkout_completed_recurring(event, &session, user_id, &plan)
                    .await?;
            }
            _ => {
                let err =
                    SubscriptionError::InvalidWebhook("unknown checkout session mode".to_string());
                error!(
                    stripe_event_id = ?event.id,
                    %user_id,
                    %plan_id,
                    status = err.status_code().as_u16(),
                    mode = ?session.mode,
                    "subscriptions: unknown checkout session mode"
                );
                return Err(err);
            }
        }

        Ok(())
    }

    async fn handle_checkout_completed_one_time(
        &self,
        event: &StripeEvent,
        session: &StripeCheckoutSession,
        metadata: &HashMap<String, String>,
        user_id: Uuid,
        plan: &PlanEntity,
        payment_method: PaymentMethod,
    ) -> UseCaseResult<()> {
        let (starts_at, ends_at) =
            Self::one_time_period_from_metadata(metadata, plan.duration_days)?;
        let currency = session.currency.clone().unwrap_or_else(|| "thb".to_string());
        let amount_minor = session
            .amount_total
            .and_then(|value| i32::try_from(value).ok())
            .unwrap_or(plan.price_minor);

        let provider_session_ref = session.id.clone().unwrap_or_default();
        let provider_payment_id = session.payment_intent.clone();
        let provider_reference = session
            .payment_intent
            .clone()
            .or_else(|| session.id.clone())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                let err =
                    SubscriptionError::InvalidWebhook("missing payment reference".to_string());
                error!(
                    stripe_event_id = ?event.id,
                    status = err.status_code().as_u16(),
                    "subscriptions: missing payment reference in checkout"
                );
                err
            })?;

        let existing_subscription = self
            .subscription_repo
            .find_by_provider_subscription_id(&provider_reference)
            .await
            .map_err(|err| {
                error!(
                    stripe_event_id = ?event.id,
                    %user_id,
                    db_error = ?err,
                    "subscriptions: failed to load one-time subscription after checkout"
                );
                SubscriptionError::Internal(err)
            })?;

        let subscription = match existing_subscription {
            Some(existing) => existing,
            None => {
                let subscription_id = self
                    .subscription_repo
                    .create_or_update_subscription_after_checkout(
                        user_id,
                        plan.id,
                        BillingMode::OneTime,
                        starts_at,
                        ends_at,
                        SubscriptionStatus::Pending,
                        Some(provider_reference.clone()),
                    )
                    .await
                    .map_err(|err| {
                        error!(
                            stripe_event_id = ?event.id,
                            %user_id,
                            db_error = ?err,
                            "subscriptions: failed to upsert one-time subscription after checkout"
                        );
                        SubscriptionError::Internal(err)
                    })?;

                SubscriptionEntity {
                    id: subscription_id,
                    user_id,
                    plan_id: plan.id,
                    starts_at,
                    ends_at,
                    billing_mode: BillingMode::OneTime.to_string(),
                    default_payment_method_id: None,
                    cancel_at_period_end: false,
                    canceled_at: None,
                    provider_subscription_id: Some(provider_reference.clone()),
                    status: SubscriptionStatus::Pending.to_string(),
                    created_at: Utc::now(),
                }
            }
        };

        let invoice_id = self
            .ensure_invoice_for_subscription(
                &subscription,
                starts_at,
                ends_at,
                amount_minor,
                currency.clone(),
            )
            .await?;

        if let Some(payment_intent_id) = provider_payment_id.as_deref() {
            let payment_exists = self
                .payment_repo
                .exists_by_provider_payment_id(payment_intent_id)
                .await
                .map_err(|err| {
                    error!(
                        stripe_event_id = ?event.id,
                        %user_id,
                        payment_intent_id = %payment_intent_id,
                        db_error = ?err,
                        "subscriptions: failed to check payment for one-time checkout"
                    );
                    SubscriptionError::Internal(err)
                })?;

            if !payment_exists {
                self.payment_repo
                    .record_payment(crates::domain::entities::payments::NewPaymentEntity {
                        invoice_id,
                        user_id,
                        provider: "stripe".to_string(),
                        method_type: payment_method.to_string(),
                        payment_method_id: None,
                        amount_minor,
                        currency: currency.clone(),
                        status: PaymentStatus::Processing.to_string(),
                        provider_payment_id: provider_payment_id.clone(),
                        provider_session_ref: Some(provider_session_ref),
                        error: None,
                    })
                    .await
                    .map_err(|err| {
                        error!(
                            stripe_event_id = ?event.id,
                            %user_id,
                            db_error = ?err,
                            "subscriptions: failed to record payment for one-time checkout"
                        );
                        SubscriptionError::Internal(err)
                    })?;
            }
        } else {
            warn!(
                stripe_event_id = ?event.id,
                %user_id,
                "subscriptions: missing payment intent for one-time checkout"
            );
        }

        if Self::checkout_paid(session.payment_status.as_deref()) {
            info!(
                stripe_event_id = ?event.id,
                %user_id,
                payment_status = ?session.payment_status,
                "subscriptions: checkout session paid; activating one-time subscription"
            );

            self.subscription_repo
                .update_status_by_provider_subscription_id(
                    &provider_reference,
                    SubscriptionStatus::Active,
                )
                .await
                .map_err(|err| {
                    error!(
                        stripe_event_id = ?event.id,
                        %user_id,
                        provider_reference = %provider_reference,
                        db_error = ?err,
                        "subscriptions: failed to activate one-time subscription after paid checkout"
                    );
                    SubscriptionError::Internal(err)
                })?;

            self.invoice_repo
                .mark_invoice_paid(invoice_id)
                .await
                .map_err(|err| {
                    error!(
                        stripe_event_id = ?event.id,
                        %user_id,
                        invoice_id = %invoice_id,
                        db_error = ?err,
                        "subscriptions: failed to mark invoice paid after paid checkout"
                    );
                    SubscriptionError::Internal(err)
                })?;

            if let Some(payment_intent_id) = provider_payment_id.as_deref() {
                self.payment_repo
                    .update_status_by_provider_payment_id(
                        payment_intent_id,
                        PaymentStatus::Succeeded,
                    )
                    .await
                    .map_err(|err| {
                        error!(
                            stripe_event_id = ?event.id,
                            %user_id,
                            payment_intent_id = %payment_intent_id,
                            db_error = ?err,
                            "subscriptions: failed to update payment after paid checkout"
                        );
                        SubscriptionError::Internal(err)
                    })?;
            }
        }

        info!(
            stripe_event_id = ?event.id,
            %user_id,
            "subscriptions: processed one-time checkout webhook"
        );

        Ok(())
    }

    async fn handle_checkout_completed_recurring(
        &self,
        event: &StripeEvent,
        session: &StripeCheckoutSession,
        user_id: Uuid,
        plan: &PlanEntity,
    ) -> UseCaseResult<()> {
        let subscription_id = session.subscription.clone().ok_or_else(|| {
            SubscriptionError::InvalidWebhook("subscription id missing on session".to_string())
        })?;

        info!(
            stripe_event_id = ?event.id,
            %user_id,
            %subscription_id,
            "subscriptions: retrieving subscription from stripe"
        );

        let subscription = self
            .stripe_client
            .retrieve_subscription(&subscription_id)
            .await
            .map_err(|err| {
                error!(
                    stripe_event_id = ?event.id,
                    %user_id,
                    %subscription_id,
                    error = ?err,
                    "subscriptions: failed to retrieve subscription from stripe"
                );
                SubscriptionError::Internal(err)
            })?;

        let starts_at = subscription
            .period_start()
            .and_then(Self::ts_to_datetime)
            .ok_or_else(|| {
                SubscriptionError::InvalidWebhook(
                    "period start missing on subscription".to_string(),
                )
            })?;
        let ends_at = subscription
            .period_end()
            .and_then(Self::ts_to_datetime)
            .ok_or_else(|| {
                SubscriptionError::InvalidWebhook("period end missing on subscription".to_string())
            })?;
        let currency = session.currency.clone().unwrap_or_else(|| "thb".to_string());
        let amount_minor = session
            .amount_total
            .and_then(|value| i32::try_from(value).ok())
            .unwrap_or(plan.price_minor);

        let existing_subscription = self
            .subscription_repo
            .find_by_provider_subscription_id(&subscription_id)
            .await
            .map_err(|err| {
                error!(
                    stripe_event_id = ?event.id,
                    %user_id,
                    %subscription_id,
                    db_error = ?err,
                    "subscriptions: failed to load subscription after checkout"
                );
                SubscriptionError::Internal(err)
            })?;

        let subscription = match existing_subscription {
            Some(existing) => existing,
            None => {
                let subscription_id_local = self
                    .subscription_repo
                    .create_or_update_subscription_after_checkout(
                        user_id,
                        plan.id,
                        BillingMode::Recurring,
                        starts_at,
                        ends_at,
                        SubscriptionStatus::Pending,
                        Some(subscription_id.clone()),
                    )
                    .await
                    .map_err(|err| {
                        error!(
                            stripe_event_id = ?event.id,
                            %user_id,
                            %subscription_id,
                            db_error = ?err,
                            "subscriptions: failed to upsert subscription after checkout"
                        );
                        SubscriptionError::Internal(err)
                    })?;

                SubscriptionEntity {
                    id: subscription_id_local,
                    user_id,
                    plan_id: plan.id,
                    starts_at,
                    ends_at,
                    billing_mode: BillingMode::Recurring.to_string(),
                    default_payment_method_id: None,
                    cancel_at_period_end: false,
                    canceled_at: None,
                    provider_subscription_id: Some(subscription_id.clone()),
                    status: SubscriptionStatus::Pending.to_string(),
                    created_at: Utc::now(),
                }
            }
        };

        let invoice_id = self
            .ensure_invoice_for_subscription(
                &subscription,
                starts_at,
                ends_at,
                amount_minor,
                currency,
            )
            .await?;

        if Self::checkout_paid(session.payment_status.as_deref()) {
            info!(
                stripe_event_id = ?event.id,
                %user_id,
                %subscription_id,
                payment_status = ?session.payment_status,
                "subscriptions: checkout session paid; activating subscription"
            );

            self.subscription_repo
                .update_status_by_provider_subscription_id(
                    &subscription_id,
                    SubscriptionStatus::Active,
                )
                .await
                .map_err(|err| {
                    error!(
                        stripe_event_id = ?event.id,
                        %user_id,
                        %subscription_id,
                        db_error = ?err,
                        "subscriptions: failed to activate subscription after paid checkout"
                    );
                    SubscriptionError::Internal(err)
                })?;

            self.invoice_repo
                .mark_invoice_paid(invoice_id)
                .await
                .map_err(|err| {
                    error!(
                        stripe_event_id = ?event.id,
                        %user_id,
                        invoice_id = %invoice_id,
                        db_error = ?err,
                        "subscriptions: failed to mark invoice paid after paid checkout"
                    );
                    SubscriptionError::Internal(err)
                })?;
        }

        info!(
            stripe_event_id = ?event.id,
            %user_id,
            %subscription_id,
            "subscriptions: processed subscription checkout webhook"
        );

        Ok(())
    }

    async fn handle_checkout_expired(&self, event: &StripeEvent) -> UseCaseResult<()> {
        let session = StripeClient::extract_checkout_session(event).ok_or_else(|| {
            let err = SubscriptionError::InvalidWebhook("missing checkout session".to_string());
            error!(
                stripe_event_id = ?event.id,
                status = err.status_code().as_u16(),
                "subscriptions: expired checkout missing session"
            );
            err
        })?;

        let provider_reference = session
            .payment_intent
            .clone()
            .or(session.id.clone())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                let err =
                    SubscriptionError::InvalidWebhook("missing payment reference".to_string());
                error!(
                    stripe_event_id = ?event.id,
                    status = err.status_code().as_u16(),
                    "subscriptions: expired checkout missing payment reference"
                );
                err
            })?;

        info!(
            stripe_event_id = ?event.id,
            provider_reference = %provider_reference,
            "subscriptions: checkout expired; expiring pending subscription if present"
        );

        let subscription = self
            .subscription_repo
            .find_by_provider_subscription_id(&provider_reference)
            .await
            .map_err(|err| {
                error!(
                    stripe_event_id = ?event.id,
                    provider_reference = %provider_reference,
                    db_error = ?err,
                    "subscriptions: failed to load subscription for checkout expiration"
                );
                SubscriptionError::Internal(err)
            })?;

        let Some(subscription) = subscription else {
            info!(
                stripe_event_id = ?event.id,
                provider_reference = %provider_reference,
                "subscriptions: no subscription found for checkout expiration"
            );
            return Ok(());
        };

        if SubscriptionStatus::from_str(&subscription.status) != SubscriptionStatus::Pending {
            info!(
                stripe_event_id = ?event.id,
                provider_reference = %provider_reference,
                status = %subscription.status,
                "subscriptions: checkout expired for non-pending subscription"
            );
            return Ok(());
        }

        self.subscription_repo
            .update_status_by_provider_subscription_id(
                &provider_reference,
                SubscriptionStatus::Expired,
            )
            .await
            .map_err(|err| {
                error!(
                    stripe_event_id = ?event.id,
                    provider_reference = %provider_reference,
                    db_error = ?err,
                    "subscriptions: failed to expire subscription after checkout expiration"
                );
                SubscriptionError::Internal(err)
            })?;

        if let Some(invoice) = self
            .invoice_repo
            .find_by_subscription_and_period_start(subscription.id, subscription.starts_at)
            .await
            .map_err(|err| {
                error!(
                    stripe_event_id = ?event.id,
                    provider_reference = %provider_reference,
                    db_error = ?err,
                    "subscriptions: failed to load invoice for checkout expiration"
                );
                SubscriptionError::Internal(err)
            })?
        {
            self.invoice_repo
                .update_status_by_id(invoice.id, "void")
                .await
                .map_err(|err| {
                    error!(
                        stripe_event_id = ?event.id,
                        invoice_id = %invoice.id,
                        db_error = ?err,
                        "subscriptions: failed to void invoice after checkout expiration"
                    );
                    SubscriptionError::Internal(err)
                })?;
        }

        if let Some(payment_intent_id) = session.payment_intent.as_deref() {
            self.payment_repo
                .update_status_by_provider_payment_id(
                    payment_intent_id,
                    PaymentStatus::Canceled,
                )
                .await
                .map_err(|err| {
                    error!(
                        stripe_event_id = ?event.id,
                        payment_intent_id = %payment_intent_id,
                        db_error = ?err,
                        "subscriptions: failed to cancel payment after checkout expiration"
                    );
                    SubscriptionError::Internal(err)
                })?;
        }

        Ok(())
    }

    async fn handle_payment_intent_succeeded(
        &self,
        event: &StripeEvent,
    ) -> UseCaseResult<()> {
        let (payment_intent_id, amount_minor, currency, payment_method) =
            Self::parse_payment_intent_event(event)?;

        info!(
            stripe_event_id = ?event.id,
            payment_intent_id = %payment_intent_id,
            "subscriptions: payment_intent succeeded"
        );

        let subscription = self
            .subscription_repo
            .find_by_provider_subscription_id(&payment_intent_id)
            .await
            .map_err(|err| {
                error!(
                    stripe_event_id = ?event.id,
                    payment_intent_id = %payment_intent_id,
                    db_error = ?err,
                    "subscriptions: failed to load subscription after payment_intent"
                );
                SubscriptionError::Internal(err)
            })?;

        let Some(subscription) = subscription else {
            info!(
                stripe_event_id = ?event.id,
                payment_intent_id = %payment_intent_id,
                "subscriptions: payment_intent succeeded without local subscription"
            );
            return Ok(());
        };

        if BillingMode::from_str(&subscription.billing_mode) != Some(BillingMode::OneTime) {
            info!(
                stripe_event_id = ?event.id,
                payment_intent_id = %payment_intent_id,
                billing_mode = %subscription.billing_mode,
                "subscriptions: ignoring payment_intent for non one-time subscription"
            );
            return Ok(());
        }

        self.subscription_repo
            .update_status_by_provider_subscription_id(
                &payment_intent_id,
                SubscriptionStatus::Active,
            )
            .await
            .map_err(|err| {
                error!(
                    stripe_event_id = ?event.id,
                    payment_intent_id = %payment_intent_id,
                    db_error = ?err,
                    "subscriptions: failed to activate subscription after payment_intent"
                );
                SubscriptionError::Internal(err)
            })?;

        let currency = currency.unwrap_or_else(|| "thb".to_string());

        let plan_price_minor = match amount_minor {
            Some(value) => value,
            None => {
                let plan = self
                    .plan_repo
                    .find_active_plan_by_id(subscription.plan_id)
                    .await
                    .map_err(|err| {
                        error!(
                            stripe_event_id = ?event.id,
                            plan_id = %subscription.plan_id,
                            db_error = ?err,
                            "subscriptions: failed to load plan for payment_intent"
                        );
                        SubscriptionError::Internal(err)
                    })?;
                plan.price_minor
            }
        };

        let invoice_id = self
            .ensure_invoice_for_subscription(
                &subscription,
                subscription.starts_at,
                subscription.ends_at,
                plan_price_minor,
                currency.clone(),
            )
            .await?;

        self.invoice_repo
            .mark_invoice_paid(invoice_id)
            .await
            .map_err(|err| {
                error!(
                    stripe_event_id = ?event.id,
                    invoice_id = %invoice_id,
                    db_error = ?err,
                    "subscriptions: failed to mark invoice paid after payment_intent"
                );
                SubscriptionError::Internal(err)
            })?;

        let payment_exists = self
            .payment_repo
            .exists_by_provider_payment_id(&payment_intent_id)
            .await
            .map_err(|err| {
                error!(
                    stripe_event_id = ?event.id,
                    payment_intent_id = %payment_intent_id,
                    db_error = ?err,
                    "subscriptions: failed to check payment after payment_intent"
                );
                SubscriptionError::Internal(err)
            })?;

        if payment_exists {
            self.payment_repo
                .update_status_by_provider_payment_id(
                    &payment_intent_id,
                    PaymentStatus::Succeeded,
                )
                .await
                .map_err(|err| {
                    error!(
                        stripe_event_id = ?event.id,
                        payment_intent_id = %payment_intent_id,
                        db_error = ?err,
                        "subscriptions: failed to update payment after payment_intent"
                    );
                    SubscriptionError::Internal(err)
                })?;
        } else {
            let amount_minor = amount_minor.unwrap_or(plan_price_minor);
            self.payment_repo
                .record_payment(crates::domain::entities::payments::NewPaymentEntity {
                    invoice_id,
                    user_id: subscription.user_id,
                    provider: "stripe".to_string(),
                    method_type: payment_method.to_string(),
                    payment_method_id: None,
                    amount_minor,
                    currency: currency.clone(),
                    status: PaymentStatus::Succeeded.to_string(),
                    provider_payment_id: Some(payment_intent_id.clone()),
                    provider_session_ref: None,
                    error: None,
                })
                .await
                .map_err(|err| {
                    error!(
                        stripe_event_id = ?event.id,
                        payment_intent_id = %payment_intent_id,
                        db_error = ?err,
                        "subscriptions: failed to record payment after payment_intent"
                    );
                    SubscriptionError::Internal(err)
                })?;
        }

        Ok(())
    }

    async fn handle_payment_intent_failed(
        &self,
        event: &StripeEvent,
        payment_status: PaymentStatus,
        invoice_status: &'static str,
    ) -> UseCaseResult<()> {
        let (payment_intent_id, amount_minor, currency, payment_method) =
            Self::parse_payment_intent_event(event)?;

        error!(
            stripe_event_id = ?event.id,
            payment_intent_id = %payment_intent_id,
            "subscriptions: payment_intent failed"
        );

        let subscription = self
            .subscription_repo
            .find_by_provider_subscription_id(&payment_intent_id)
            .await
            .map_err(|err| {
                error!(
                    stripe_event_id = ?event.id,
                    payment_intent_id = %payment_intent_id,
                    db_error = ?err,
                    "subscriptions: failed to load subscription after payment_intent failure"
                );
                SubscriptionError::Internal(err)
            })?;

        let Some(subscription) = subscription else {
            info!(
                stripe_event_id = ?event.id,
                payment_intent_id = %payment_intent_id,
                "subscriptions: payment_intent failed without local subscription"
            );
            return Ok(());
        };

        if BillingMode::from_str(&subscription.billing_mode) != Some(BillingMode::OneTime) {
            info!(
                stripe_event_id = ?event.id,
                payment_intent_id = %payment_intent_id,
                billing_mode = %subscription.billing_mode,
                "subscriptions: ignoring payment_intent failure for non one-time subscription"
            );
            return Ok(());
        }

        self.subscription_repo
            .update_status_by_provider_subscription_id(
                &payment_intent_id,
                SubscriptionStatus::Expired,
            )
            .await
            .map_err(|err| {
                error!(
                    stripe_event_id = ?event.id,
                    payment_intent_id = %payment_intent_id,
                    db_error = ?err,
                    "subscriptions: failed to expire subscription after payment_intent failure"
                );
                SubscriptionError::Internal(err)
            })?;

        let currency = currency.unwrap_or_else(|| "thb".to_string());

        let plan_price_minor = match amount_minor {
            Some(value) => value,
            None => {
                let plan = self
                    .plan_repo
                    .find_active_plan_by_id(subscription.plan_id)
                    .await
                    .map_err(|err| {
                        error!(
                            stripe_event_id = ?event.id,
                            plan_id = %subscription.plan_id,
                            db_error = ?err,
                            "subscriptions: failed to load plan for payment_intent failure"
                        );
                        SubscriptionError::Internal(err)
                    })?;
                plan.price_minor
            }
        };

        let invoice_id = self
            .ensure_invoice_for_subscription(
                &subscription,
                subscription.starts_at,
                subscription.ends_at,
                plan_price_minor,
                currency.clone(),
            )
            .await?;

        self.invoice_repo
            .update_status_by_id(invoice_id, invoice_status)
            .await
            .map_err(|err| {
                error!(
                    stripe_event_id = ?event.id,
                    invoice_id = %invoice_id,
                    db_error = ?err,
                    "subscriptions: failed to update invoice after payment_intent failure"
                );
                SubscriptionError::Internal(err)
            })?;

        let payment_exists = self
            .payment_repo
            .exists_by_provider_payment_id(&payment_intent_id)
            .await
            .map_err(|err| {
                error!(
                    stripe_event_id = ?event.id,
                    payment_intent_id = %payment_intent_id,
                    db_error = ?err,
                    "subscriptions: failed to check payment after payment_intent failure"
                );
                SubscriptionError::Internal(err)
            })?;

        if payment_exists {
            self.payment_repo
                .update_status_by_provider_payment_id(&payment_intent_id, payment_status)
                .await
                .map_err(|err| {
                    error!(
                        stripe_event_id = ?event.id,
                        payment_intent_id = %payment_intent_id,
                        db_error = ?err,
                        "subscriptions: failed to update payment after payment_intent failure"
                    );
                    SubscriptionError::Internal(err)
                })?;
        } else {
            let amount_minor = amount_minor.unwrap_or(plan_price_minor);
            self.payment_repo
                .record_payment(crates::domain::entities::payments::NewPaymentEntity {
                    invoice_id,
                    user_id: subscription.user_id,
                    provider: "stripe".to_string(),
                    method_type: payment_method.to_string(),
                    payment_method_id: None,
                    amount_minor,
                    currency: currency.clone(),
                    status: payment_status.to_string(),
                    provider_payment_id: Some(payment_intent_id.clone()),
                    provider_session_ref: None,
                    error: None,
                })
                .await
                .map_err(|err| {
                    error!(
                        stripe_event_id = ?event.id,
                        payment_intent_id = %payment_intent_id,
                        db_error = ?err,
                        "subscriptions: failed to record payment after payment_intent failure"
                    );
                    SubscriptionError::Internal(err)
                })?;
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
                error!(
                    stripe_event_id = ?event.id,
                    error = %err,
                    status = SubscriptionError::InvalidWebhook("".into()).status_code().as_u16(),
                    "subscriptions: invalid subscription payload in webhook"
                );
                SubscriptionError::InvalidWebhook("invalid subscription payload".to_string())
            })?;

        let subscription_id = subscription.id.ok_or_else(|| {
            let err = SubscriptionError::InvalidWebhook("missing subscription id".to_string());
            error!(
                stripe_event_id = ?event.id,
                status = err.status_code().as_u16(),
                "subscriptions: subscription id missing in webhook payload"
            );
            err
        })?;

        error!(
            stripe_event_id = ?event.id,
            subscription_id = %subscription_id,
            "subscriptions: marking subscription expired from webhook"
        );

        self.subscription_repo
            .update_status_by_provider_subscription_id(
                &subscription_id,
                SubscriptionStatus::Expired,
            )
            .await
            .map_err(|err| {
                error!(
                    subscription_id = %subscription_id,
                    db_error = ?err,
                    "subscriptions: failed to update subscription status from webhook"
                );
                SubscriptionError::Internal(err)
            })?;

        Ok(())
    }

    async fn handle_invoice_payment_succeeded(
        &self,
        event: &StripeEvent,
    ) -> UseCaseResult<()> {
        let context = Self::parse_invoice_context(event)?;

        info!(
            stripe_event_id = ?event.id,
            invoice_id = ?context.invoice_id,
            subscription_id = %context.subscription_id,
            "subscriptions: invoice payment succeeded"
        );

        let subscription = self
            .subscription_repo
            .find_by_provider_subscription_id(&context.subscription_id)
            .await
            .map_err(|err| {
                error!(
                    stripe_event_id = ?event.id,
                    subscription_id = %context.subscription_id,
                    db_error = ?err,
                    "subscriptions: failed to load subscription from invoice webhook"
                );
                SubscriptionError::Internal(err)
            })?;

        let Some(subscription) = subscription else {
            warn!(
                stripe_event_id = ?event.id,
                subscription_id = %context.subscription_id,
                "subscriptions: invoice payment succeeded without local subscription"
            );
            return Err(SubscriptionError::Internal(anyhow!(
                "subscription not ready for invoice webhook"
            )));
        };

        if BillingMode::from_str(&subscription.billing_mode) != Some(BillingMode::Recurring) {
            info!(
                stripe_event_id = ?event.id,
                subscription_id = %context.subscription_id,
                billing_mode = %subscription.billing_mode,
                "subscriptions: ignoring invoice for non-recurring subscription"
            );
            return Ok(());
        }

        let period = self
            .resolve_invoice_period(event, &context.subscription_id, context.period())
            .await?;

        self.subscription_repo
            .update_status_and_period_by_provider_subscription_id(
                &context.subscription_id,
                SubscriptionStatus::Active,
                period.0,
                period.1,
            )
            .await
            .map_err(|err| {
                error!(
                    stripe_event_id = ?event.id,
                    subscription_id = %context.subscription_id,
                    db_error = ?err,
                    "subscriptions: failed to update subscription period from invoice webhook"
                );
                SubscriptionError::Internal(err)
            })?;

        let currency = context.currency.clone().unwrap_or_else(|| "thb".to_string());

        let plan_price_minor = match context.amount_minor() {
            Some(value) => value,
            None => {
                let plan = self
                    .plan_repo
                    .find_active_plan_by_id(subscription.plan_id)
                    .await
                    .map_err(|err| {
                        error!(
                            stripe_event_id = ?event.id,
                            plan_id = %subscription.plan_id,
                            db_error = ?err,
                            "subscriptions: failed to load plan for invoice webhook"
                        );
                        SubscriptionError::Internal(err)
                    })?;
                plan.price_minor
            }
        };

        let invoice_id = self
            .ensure_invoice_for_subscription(
                &subscription,
                period.0,
                period.1,
                plan_price_minor,
                currency.clone(),
            )
            .await?;

        self.invoice_repo
            .mark_invoice_paid(invoice_id)
            .await
            .map_err(|err| {
                error!(
                    stripe_event_id = ?event.id,
                    invoice_id = %invoice_id,
                    db_error = ?err,
                    "subscriptions: failed to mark invoice paid from invoice webhook"
                );
                SubscriptionError::Internal(err)
            })?;

        let payment_reference = context
            .payment_intent_id
            .clone()
            .or_else(|| context.invoice_id.clone());

        if let Some(provider_payment_id) = payment_reference.as_deref() {
            let payment_exists = self
                .payment_repo
                .exists_by_provider_payment_id(provider_payment_id)
                .await
                .map_err(|err| {
                    error!(
                        stripe_event_id = ?event.id,
                        provider_payment_id = %provider_payment_id,
                        db_error = ?err,
                        "subscriptions: failed to check payment existence from invoice webhook"
                    );
                    SubscriptionError::Internal(err)
                })?;

            if payment_exists {
                self.payment_repo
                    .update_status_by_provider_payment_id(
                        provider_payment_id,
                        PaymentStatus::Succeeded,
                    )
                    .await
                    .map_err(|err| {
                        error!(
                            stripe_event_id = ?event.id,
                            provider_payment_id = %provider_payment_id,
                            db_error = ?err,
                            "subscriptions: failed to update payment from invoice webhook"
                        );
                        SubscriptionError::Internal(err)
                    })?;
            } else {
                let amount_minor = context.amount_minor().unwrap_or(plan_price_minor);
                self.payment_repo
                    .record_payment(crates::domain::entities::payments::NewPaymentEntity {
                        invoice_id,
                        user_id: subscription.user_id,
                        provider: "stripe".to_string(),
                        method_type: PaymentMethod::Card.to_string(),
                        payment_method_id: None,
                        amount_minor,
                        currency: currency.clone(),
                        status: PaymentStatus::Succeeded.to_string(),
                        provider_payment_id: Some(provider_payment_id.to_string()),
                        provider_session_ref: None,
                        error: None,
                    })
                    .await
                    .map_err(|err| {
                        error!(
                            stripe_event_id = ?event.id,
                            provider_payment_id = %provider_payment_id,
                            db_error = ?err,
                            "subscriptions: failed to record payment from invoice webhook"
                        );
                        SubscriptionError::Internal(err)
                    })?;
            }
        } else {
            warn!(
                stripe_event_id = ?event.id,
                subscription_id = %context.subscription_id,
                "subscriptions: invoice webhook missing payment reference"
            );
        }

        Ok(())
    }

    async fn handle_invoice_payment_failed(
        &self,
        event: &StripeEvent,
    ) -> UseCaseResult<()> {
        let context = Self::parse_invoice_context(event)?;

        error!(
            stripe_event_id = ?event.id,
            invoice_id = ?context.invoice_id,
            subscription_id = %context.subscription_id,
            "subscriptions: invoice payment failed"
        );

        let subscription = self
            .subscription_repo
            .find_by_provider_subscription_id(&context.subscription_id)
            .await
            .map_err(|err| {
                error!(
                    stripe_event_id = ?event.id,
                    subscription_id = %context.subscription_id,
                    db_error = ?err,
                    "subscriptions: failed to load subscription from invoice webhook"
                );
                SubscriptionError::Internal(err)
            })?;

        let Some(subscription) = subscription else {
            warn!(
                stripe_event_id = ?event.id,
                subscription_id = %context.subscription_id,
                "subscriptions: invoice payment failed without local subscription"
            );
            return Err(SubscriptionError::Internal(anyhow!(
                "subscription not ready for invoice webhook"
            )));
        };

        if BillingMode::from_str(&subscription.billing_mode) != Some(BillingMode::Recurring) {
            info!(
                stripe_event_id = ?event.id,
                subscription_id = %context.subscription_id,
                billing_mode = %subscription.billing_mode,
                "subscriptions: ignoring invoice failure for non-recurring subscription"
            );
            return Ok(());
        }

        let period = self
            .resolve_invoice_period(event, &context.subscription_id, context.period())
            .await?;

        self.subscription_repo
            .update_status_by_provider_subscription_id(
                &context.subscription_id,
                SubscriptionStatus::PastDue,
            )
            .await
            .map_err(|err| {
                error!(
                    stripe_event_id = ?event.id,
                    subscription_id = %context.subscription_id,
                    db_error = ?err,
                    "subscriptions: failed to update subscription status from invoice webhook"
                );
                SubscriptionError::Internal(err)
            })?;

        let currency = context.currency.clone().unwrap_or_else(|| "thb".to_string());

        let plan_price_minor = match context.amount_minor() {
            Some(value) => value,
            None => {
                let plan = self
                    .plan_repo
                    .find_active_plan_by_id(subscription.plan_id)
                    .await
                    .map_err(|err| {
                        error!(
                            stripe_event_id = ?event.id,
                            plan_id = %subscription.plan_id,
                            db_error = ?err,
                            "subscriptions: failed to load plan for invoice failure"
                        );
                        SubscriptionError::Internal(err)
                    })?;
                plan.price_minor
            }
        };

        let invoice_id = self
            .ensure_invoice_for_subscription(
                &subscription,
                period.0,
                period.1,
                plan_price_minor,
                currency.clone(),
            )
            .await?;

        self.invoice_repo
            .update_status_by_id(invoice_id, "past_due")
            .await
            .map_err(|err| {
                error!(
                    stripe_event_id = ?event.id,
                    invoice_id = %invoice_id,
                    db_error = ?err,
                    "subscriptions: failed to mark invoice past due from invoice webhook"
                );
                SubscriptionError::Internal(err)
            })?;

        let payment_reference = context
            .payment_intent_id
            .clone()
            .or_else(|| context.invoice_id.clone());

        if let Some(provider_payment_id) = payment_reference.as_deref() {
            let payment_exists = self
                .payment_repo
                .exists_by_provider_payment_id(provider_payment_id)
                .await
                .map_err(|err| {
                    error!(
                        stripe_event_id = ?event.id,
                        provider_payment_id = %provider_payment_id,
                        db_error = ?err,
                        "subscriptions: failed to check payment existence from invoice webhook"
                    );
                    SubscriptionError::Internal(err)
                })?;

            if payment_exists {
                self.payment_repo
                    .update_status_by_provider_payment_id(
                        provider_payment_id,
                        PaymentStatus::Failed,
                    )
                    .await
                    .map_err(|err| {
                        error!(
                            stripe_event_id = ?event.id,
                            provider_payment_id = %provider_payment_id,
                            db_error = ?err,
                            "subscriptions: failed to update payment from invoice webhook"
                        );
                        SubscriptionError::Internal(err)
                    })?;
            } else {
                let amount_minor = context.amount_minor().unwrap_or(plan_price_minor);
                self.payment_repo
                    .record_payment(crates::domain::entities::payments::NewPaymentEntity {
                        invoice_id,
                        user_id: subscription.user_id,
                        provider: "stripe".to_string(),
                        method_type: PaymentMethod::Card.to_string(),
                        payment_method_id: None,
                        amount_minor,
                        currency: currency.clone(),
                        status: PaymentStatus::Failed.to_string(),
                        provider_payment_id: Some(provider_payment_id.to_string()),
                        provider_session_ref: None,
                        error: None,
                    })
                    .await
                    .map_err(|err| {
                        error!(
                            stripe_event_id = ?event.id,
                            provider_payment_id = %provider_payment_id,
                            db_error = ?err,
                            "subscriptions: failed to record payment from invoice webhook"
                        );
                        SubscriptionError::Internal(err)
                    })?;
            }
        } else {
            warn!(
                stripe_event_id = ?event.id,
                subscription_id = %context.subscription_id,
                "subscriptions: invoice webhook missing payment reference"
            );
        }

        Ok(())
    }

    fn parse_payment_intent_event(
        event: &StripeEvent,
    ) -> UseCaseResult<(String, Option<i32>, Option<String>, PaymentMethod)> {
        #[derive(Deserialize)]
        struct PaymentIntentObject {
            id: Option<String>,
            amount: Option<i64>,
            amount_received: Option<i64>,
            currency: Option<String>,
            payment_method_types: Option<Vec<String>>,
        }

        let payment_intent: PaymentIntentObject =
            serde_json::from_value(event.data.object.clone()).map_err(|err| {
                error!(
                    stripe_event_id = ?event.id,
                    error = %err,
                    status = SubscriptionError::InvalidWebhook("".into()).status_code().as_u16(),
                    "subscriptions: invalid payment_intent payload in webhook"
                );
                SubscriptionError::InvalidWebhook("invalid payment_intent payload".to_string())
            })?;

        let payment_intent_id = payment_intent.id.ok_or_else(|| {
            let err =
                SubscriptionError::InvalidWebhook("missing payment_intent id".to_string());
            error!(
                stripe_event_id = ?event.id,
                status = err.status_code().as_u16(),
                "subscriptions: payment_intent id missing in webhook"
            );
            err
        })?;

        let amount_minor = payment_intent
            .amount_received
            .or(payment_intent.amount)
            .and_then(|value| i32::try_from(value).ok());

        let payment_method = payment_intent
            .payment_method_types
            .as_ref()
            .and_then(|types| {
                types
                    .iter()
                    .find(|value| value.as_str() == "promptpay")
                    .map(|_| PaymentMethod::PromptPay)
            })
            .unwrap_or(PaymentMethod::Card);

        Ok((payment_intent_id, amount_minor, payment_intent.currency, payment_method))
    }

    fn parse_invoice_context(event: &StripeEvent) -> UseCaseResult<InvoiceContext> {
        #[derive(Deserialize)]
        struct InvoiceObject {
            id: Option<String>,
            subscription: Option<String>,
            customer: Option<String>,
            status: Option<String>,
            period_start: Option<i64>,
            period_end: Option<i64>,
            amount_due: Option<i64>,
            amount_paid: Option<i64>,
            currency: Option<String>,
            payment_intent: Option<String>,
            parent: Option<InvoiceParent>,
            lines: Option<InvoiceLines>,
        }

        #[derive(Deserialize)]
        struct InvoiceParent {
            subscription_details: Option<InvoiceSubscriptionDetails>,
        }

        #[derive(Deserialize)]
        struct InvoiceSubscriptionDetails {
            subscription: Option<String>,
        }

        #[derive(Deserialize)]
        struct InvoiceLines {
            #[serde(default)]
            data: Vec<InvoiceLine>,
        }

        #[derive(Deserialize)]
        struct InvoiceLine {
            period: Option<InvoiceLinePeriod>,
            currency: Option<String>,
            parent: Option<InvoiceLineParent>,
        }

        #[derive(Deserialize)]
        struct InvoiceLinePeriod {
            start: Option<i64>,
            end: Option<i64>,
        }

        #[derive(Deserialize)]
        struct InvoiceLineParent {
            subscription_item_details: Option<InvoiceLineSubscriptionItemDetails>,
        }

        #[derive(Deserialize)]
        struct InvoiceLineSubscriptionItemDetails {
            subscription: Option<String>,
        }

        let invoice: InvoiceObject =
            serde_json::from_value(event.data.object.clone()).map_err(|err| {
                error!(
                    stripe_event_id = ?event.id,
                    error = %err,
                    status = SubscriptionError::InvalidWebhook("".into()).status_code().as_u16(),
                    "subscriptions: invalid invoice payload in webhook"
                );
                SubscriptionError::InvalidWebhook("invalid invoice payload".to_string())
            })?;

        let subscription_id = invoice
            .subscription
            .clone()
            .or_else(|| {
                invoice.parent.as_ref().and_then(|parent| {
                    parent
                        .subscription_details
                        .as_ref()
                        .and_then(|details| details.subscription.clone())
                })
            })
            .or_else(|| {
                invoice.lines.as_ref().and_then(|lines| {
                    lines.data.iter().find_map(|line| {
                        line.parent.as_ref().and_then(|parent| {
                            parent
                                .subscription_item_details
                                .as_ref()
                                .and_then(|details| details.subscription.clone())
                        })
                    })
                })
            })
            .ok_or_else(|| {
                let err = SubscriptionError::InvalidWebhook(
                    "invoice missing subscription id".to_string(),
                );
                error!(
                    stripe_event_id = ?event.id,
                    status = err.status_code().as_u16(),
                    "subscriptions: invoice webhook missing subscription id"
                );
                err
            })?;

        let line_period = invoice.lines.as_ref().and_then(|lines| {
            lines.data.iter().find_map(|line| {
                if let Some(parent) = line.parent.as_ref() {
                    if let Some(details) = parent.subscription_item_details.as_ref() {
                        if let Some(line_subscription_id) = details.subscription.as_ref() {
                            if line_subscription_id != &subscription_id {
                                return None;
                            }
                        }
                    }
                }
                let period = line.period.as_ref()?;
                let starts_at = period.start.and_then(Self::ts_to_datetime)?;
                let ends_at = period.end.and_then(Self::ts_to_datetime)?;
                Some((starts_at, ends_at))
            })
        });

        let line_currency = invoice.lines.as_ref().and_then(|lines| {
            lines.data.iter().find_map(|line| {
                if let Some(parent) = line.parent.as_ref() {
                    if let Some(details) = parent.subscription_item_details.as_ref() {
                        if let Some(line_subscription_id) = details.subscription.as_ref() {
                            if line_subscription_id != &subscription_id {
                                return None;
                            }
                        }
                    }
                }
                line.currency.clone()
            })
        });

        let invoice_period = line_period.or_else(|| {
            invoice
                .period_start
                .and_then(Self::ts_to_datetime)
                .zip(invoice.period_end.and_then(Self::ts_to_datetime))
        });

        Ok(InvoiceContext {
            invoice_id: invoice.id,
            subscription_id,
            customer_id: invoice.customer,
            status: invoice.status,
            payment_intent_id: invoice.payment_intent,
            currency: invoice.currency.or(line_currency),
            period_start: invoice_period.map(|value| value.0),
            period_end: invoice_period.map(|value| value.1),
            amount_due: invoice.amount_due,
            amount_paid: invoice.amount_paid,
        })
    }

    async fn resolve_invoice_period(
        &self,
        event: &StripeEvent,
        subscription_id: &str,
        period: Option<(DateTime<Utc>, DateTime<Utc>)>,
    ) -> UseCaseResult<(DateTime<Utc>, DateTime<Utc>)> {
        if let Some(period) = period {
            return Ok(period);
        }

        info!(
            stripe_event_id = ?event.id,
            subscription_id = %subscription_id,
            "subscriptions: invoice missing period data; retrieving subscription from stripe"
        );

        let subscription = self
            .stripe_client
            .retrieve_subscription(subscription_id)
            .await
            .map_err(|err| {
                error!(
                    stripe_event_id = ?event.id,
                    subscription_id = %subscription_id,
                    error = ?err,
                    "subscriptions: failed to retrieve subscription for invoice period"
                );
                SubscriptionError::Internal(err)
            })?;

        let starts_at = subscription
            .period_start()
            .and_then(Self::ts_to_datetime)
            .ok_or_else(|| {
                SubscriptionError::InvalidWebhook(
                    "subscription period start missing".to_string(),
                )
            })?;
        let ends_at = subscription
            .period_end()
            .and_then(Self::ts_to_datetime)
            .ok_or_else(|| {
                SubscriptionError::InvalidWebhook("subscription period end missing".to_string())
            })?;

        Ok((starts_at, ends_at))
    }

    async fn ensure_invoice_for_subscription(
        &self,
        subscription: &SubscriptionEntity,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        amount_minor: i32,
        currency: String,
    ) -> UseCaseResult<Uuid> {
        let existing = self
            .invoice_repo
            .find_by_subscription_and_period_start(subscription.id, period_start)
            .await
            .map_err(|err| {
                error!(
                    subscription_id = %subscription.id,
                    db_error = ?err,
                    "subscriptions: failed to load invoice for subscription"
                );
                SubscriptionError::Internal(err)
            })?;

        if let Some(invoice) = existing {
            return Ok(invoice.id);
        }

        let invoice_id = self
            .invoice_repo
            .create_invoice(crates::domain::entities::invoices::InsertInvoiceEntity {
                user_id: subscription.user_id,
                subscription_id: Some(subscription.id),
                plan_id: subscription.plan_id,
                amount_minor,
                currency,
                period_start,
                period_end,
                due_at: period_start,
                status: "pending".to_string(),
                paid_at: None,
            })
            .await
            .map_err(|err| {
                error!(
                    subscription_id = %subscription.id,
                    db_error = ?err,
                    "subscriptions: failed to create invoice for subscription"
                );
                SubscriptionError::Internal(err)
            })?;

        Ok(invoice_id)
    }

    fn checkout_paid(status: Option<&str>) -> bool {
        matches!(status, Some("paid") | Some("no_payment_required"))
    }

    fn one_time_period_from_metadata(
        metadata: &HashMap<String, String>,
        duration_days: i32,
    ) -> UseCaseResult<(DateTime<Utc>, DateTime<Utc>)> {
        let now = Utc::now();
        let starts_at = metadata
            .get("one_time_starts_at")
            .and_then(|value| value.parse::<i64>().ok())
            .and_then(Self::ts_to_datetime)
            .unwrap_or(now);

        let ends_at = match metadata
            .get("one_time_ends_at")
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
