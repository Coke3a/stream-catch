use std::sync::Arc;

use axum::{Json, Router, extract::State, response::IntoResponse, routing::{get, post}};
use uuid::Uuid;

use crate::auth::AuthUser;

use application::usercases::subscriptions::SubscriptionUseCase;
use domain::{
    repositories::subscriptions::SubscriptionRepository,
    value_objects::subscriptions::InsertSubscriptionModel,
};
use infra::postgres::{
    postgres_connection::PgPoolSquad,
    repositories::subscriptions::SubscriptionPostgres,
};

pub fn routes(db_pool: Arc<PgPoolSquad>) -> Router {
    let subscriptions_repository = SubscriptionPostgres::new(Arc::clone(&db_pool));
    let subscriptions_usecase = SubscriptionUseCase::new(Arc::new(subscriptions_repository));

    Router::new()
        .route("/plans", get(list_plans))
        .route("/current", get(check_current_user_subscription))
        .route("/subscribe", post(subscribe))
        .route("/cancel", post(cancel_subscription))
        .with_state(Arc::new(subscriptions_usecase))
}

pub async fn list_plans<T>(
    State(subscriptions_usecase): State<Arc<SubscriptionUseCase<T>>>,
    _auth: AuthUser,
) -> impl IntoResponse
where
    T: SubscriptionRepository + Send + Sync,
{
}

pub async fn check_current_user_subscription<T>(
    State(subscriptions_usecase): State<Arc<SubscriptionUseCase<T>>>,
    auth: AuthUser,
) -> impl IntoResponse
where
    T: SubscriptionRepository + Send + Sync,
{
}

pub async fn subscribe<T>(
    State(subscriptions_usecase): State<Arc<SubscriptionUseCase<T>>>,
    auth: AuthUser,
    Json(insert_subscription_model): Json<InsertSubscriptionModel>,
) -> impl IntoResponse
where
    T: SubscriptionRepository + Send + Sync,
{
}

pub async fn cancel_subscription<T>(
    State(subscriptions_usecase): State<Arc<SubscriptionUseCase<T>>>,
    auth: AuthUser,
) -> impl IntoResponse
where
    T: SubscriptionRepository + Send + Sync,
{
}
