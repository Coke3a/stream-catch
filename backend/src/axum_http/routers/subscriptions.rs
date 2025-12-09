use crate::{
    axum_http::{auth::AuthUser, error_responses::ErrorResponse},
    config::config_model::DotEnvyConfig,
    usecases::subscriptions::{StripeGateway, SubscriptionError, SubscriptionUseCase},
};
use axum::{
    Json, Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use crates::{
    domain::{
        repositories::{
            invoices::InvoiceRepository,
            payment_provider_customers::PaymentProviderCustomerRepository,
            payments::PaymentRepository, plans::PlanRepository,
            subscriptions::SubscriptionRepository,
        },
        value_objects::{
            enums::{billing_modes::BillingMode, payment_methods::PaymentMethod},
            subscriptions::{CreateCheckoutRequest, CreateCheckoutResponse},
        },
    },
    infra::db::{
        postgres::postgres_connection::PgPoolSquad,
        repositories::{
            invoices::InvoicePostgres, payment_provider_customers::PaymentProviderCustomerPostgres,
            payments::PaymentPostgres, plans::PlanPostgres, subscriptions::SubscriptionPostgres,
        },
    },
    payments::stripe_client::StripeClient,
};
use std::sync::Arc;

type SubscriptionUseCaseState = SubscriptionUseCase<
    PlanPostgres,
    SubscriptionPostgres,
    PaymentPostgres,
    PaymentProviderCustomerPostgres,
    InvoicePostgres,
    StripeClient,
>;

pub fn build_usecase(
    db_pool: Arc<PgPoolSquad>,
    config: Arc<DotEnvyConfig>,
) -> Arc<SubscriptionUseCaseState> {
    let stripe_client = Arc::new(StripeClient::new(
        config.stripe.secret_key.clone(),
        config.stripe.webhook_secret.clone(),
        config.stripe.success_url.clone(),
        config.stripe.cancel_url.clone(),
    ));

    let plan_repo = Arc::new(PlanPostgres::new(Arc::clone(&db_pool)));
    let subscription_repo = Arc::new(SubscriptionPostgres::new(Arc::clone(&db_pool)));
    let payment_repo = Arc::new(PaymentPostgres::new(Arc::clone(&db_pool)));
    let customer_repo = Arc::new(PaymentProviderCustomerPostgres::new(
        Arc::clone(&db_pool),
        stripe_client.clone(),
    ));
    let invoice_repo = Arc::new(InvoicePostgres::new(Arc::clone(&db_pool)));

    Arc::new(SubscriptionUseCase::new(
        plan_repo,
        subscription_repo,
        payment_repo,
        customer_repo,
        invoice_repo,
        stripe_client,
        config.free_plan_id,
    ))
}

pub fn routes(db_pool: Arc<PgPoolSquad>, config: Arc<DotEnvyConfig>) -> Router {
    let subscription_usecase = build_usecase(Arc::clone(&db_pool), Arc::clone(&config));

    Router::new()
        .route("/plans", get(list_plans))
        .route("/current", get(check_current_user_subscription))
        .route("/checkout", post(create_checkout))
        .route("/cancel", post(cancel_subscription))
        .with_state(subscription_usecase)
}

pub fn webhook_routes(db_pool: Arc<PgPoolSquad>, config: Arc<DotEnvyConfig>) -> Router {
    let subscription_usecase = build_usecase(Arc::clone(&db_pool), Arc::clone(&config));

    Router::new()
        .route("/stripe/webhook", post(stripe_webhook))
        .with_state(subscription_usecase)
}

pub async fn list_plans<P, S, Pay, Cust, Inv, Stripe>(
    State(usecase): State<Arc<SubscriptionUseCase<P, S, Pay, Cust, Inv, Stripe>>>,
    _auth: AuthUser,
) -> impl IntoResponse
where
    P: PlanRepository + Send + Sync + 'static,
    S: SubscriptionRepository + Send + Sync + 'static,
    Pay: PaymentRepository + Send + Sync + 'static,
    Cust: PaymentProviderCustomerRepository + Send + Sync + 'static,
    Inv: InvoiceRepository + Send + Sync + 'static,
    Stripe: StripeGateway + Send + Sync + 'static,
{
    match usecase.list_plans().await {
        Ok(plans) => Json(plans).into_response(),
        Err(err) => map_error(err),
    }
}

pub async fn check_current_user_subscription<P, S, Pay, Cust, Inv, Stripe>(
    State(usecase): State<Arc<SubscriptionUseCase<P, S, Pay, Cust, Inv, Stripe>>>,
    auth: AuthUser,
) -> impl IntoResponse
where
    P: PlanRepository + Send + Sync + 'static,
    S: SubscriptionRepository + Send + Sync + 'static,
    Pay: PaymentRepository + Send + Sync + 'static,
    Cust: PaymentProviderCustomerRepository + Send + Sync + 'static,
    Inv: InvoiceRepository + Send + Sync + 'static,
    Stripe: StripeGateway + Send + Sync + 'static,
{
    match usecase.get_current_subscription(auth.user_id).await {
        Ok(Some(subscription)) => Json(subscription).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(err) => map_error(err),
    }
}

pub async fn create_checkout<P, S, Pay, Cust, Inv, Stripe>(
    State(usecase): State<Arc<SubscriptionUseCase<P, S, Pay, Cust, Inv, Stripe>>>,
    auth: AuthUser,
    Json(body): Json<CreateCheckoutRequest>,
) -> impl IntoResponse
where
    P: PlanRepository + Send + Sync + 'static,
    S: SubscriptionRepository + Send + Sync + 'static,
    Pay: PaymentRepository + Send + Sync + 'static,
    Cust: PaymentProviderCustomerRepository + Send + Sync + 'static,
    Inv: InvoiceRepository + Send + Sync + 'static,
    Stripe: StripeGateway + Send + Sync + 'static,
{
    let billing_mode = match BillingMode::from_str(&body.billing_mode) {
        Some(mode) => mode,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    code: StatusCode::BAD_REQUEST.as_u16(),
                    message: "invalid billing_mode".to_string(),
                }),
            )
                .into_response();
        }
    };

    let payment_method = match PaymentMethod::from_str(&body.payment_method) {
        Some(method) => method,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    code: StatusCode::BAD_REQUEST.as_u16(),
                    message: "invalid payment_method".to_string(),
                }),
            )
                .into_response();
        }
    };

    match usecase
        .create_checkout_session(
            auth.user_id,
            auth.email.clone(),
            body.plan_id,
            billing_mode,
            payment_method,
        )
        .await
    {
        Ok(url) => Json(CreateCheckoutResponse { checkout_url: url }).into_response(),
        Err(err) => map_error(err),
    }
}

pub async fn cancel_subscription<P, S, Pay, Cust, Inv, Stripe>(
    State(usecase): State<Arc<SubscriptionUseCase<P, S, Pay, Cust, Inv, Stripe>>>,
    auth: AuthUser,
) -> impl IntoResponse
where
    P: PlanRepository + Send + Sync + 'static,
    S: SubscriptionRepository + Send + Sync + 'static,
    Pay: PaymentRepository + Send + Sync + 'static,
    Cust: PaymentProviderCustomerRepository + Send + Sync + 'static,
    Inv: InvoiceRepository + Send + Sync + 'static,
    Stripe: StripeGateway + Send + Sync + 'static,
{
    match usecase.cancel_recurring_subscription(auth.user_id).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => map_error(err),
    }
}

pub async fn stripe_webhook<P, S, Pay, Cust, Inv, Stripe>(
    State(usecase): State<Arc<SubscriptionUseCase<P, S, Pay, Cust, Inv, Stripe>>>,
    headers: HeaderMap,
    payload: Bytes,
) -> impl IntoResponse
where
    P: PlanRepository + Send + Sync + 'static,
    S: SubscriptionRepository + Send + Sync + 'static,
    Pay: PaymentRepository + Send + Sync + 'static,
    Cust: PaymentProviderCustomerRepository + Send + Sync + 'static,
    Inv: InvoiceRepository + Send + Sync + 'static,
    Stripe: StripeGateway + Send + Sync + 'static,
{
    let signature = match headers.get("stripe-signature") {
        Some(value) => match value.to_str() {
            Ok(v) => v.to_string(),
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        code: StatusCode::BAD_REQUEST.as_u16(),
                        message: "invalid stripe-signature header".to_string(),
                    }),
                )
                    .into_response();
            }
        },
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    code: StatusCode::BAD_REQUEST.as_u16(),
                    message: "missing stripe-signature header".to_string(),
                }),
            )
                .into_response();
        }
    };

    match usecase
        .handle_stripe_webhook(payload.as_ref(), &signature)
        .await
    {
        Ok(_) => StatusCode::OK.into_response(),
        Err(err) => map_error(err),
    }
}

fn map_error(err: SubscriptionError) -> axum::response::Response {
    let status = err.status_code();
    let body = Json(ErrorResponse {
        code: status.as_u16(),
        message: err.to_string(),
    });
    (status, body).into_response()
}
