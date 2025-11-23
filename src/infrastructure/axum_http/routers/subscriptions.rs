use std::sync::Arc;

use axum::{Extension, Json, Router, extract::State, response::IntoResponse};
use uuid::Uuid;

use crate::{application::usercases::subscriptions::SubscriptionUseCase, domain::{repositories::subscriptions::SubscriptionRepository, value_objects::subscriptions::InsertSubscriptionModel}, infrastructure::postgres::{postgres_connection::PgPoolSquad, repositories::subscriptions::SubscriptionPostgres}};

pub fn routes(db_pool: Arc<PgPoolSquad>) -> Router {
    let subscriptions_repository = SubscriptionPostgres::new(Arc::clone(&db_pool));
    let subscriptions_usecase = SubscriptionUseCase::new(Arc::new(subscriptions_repository));

    Router::new()
}


pub async fn list_plans<T>(
    State(subscriptions_usecase): State<Arc<SubscriptionUseCase<T>>>,
) -> impl IntoResponse
where
    T: SubscriptionRepository + Send + Sync,
{

}

pub async fn check_current_user_subscription<T>(
    State(subscriptions_usecase): State<Arc<SubscriptionUseCase<T>>>,
    Extension(user_id): Extension<Uuid>,
) -> impl IntoResponse
where
    T: SubscriptionRepository + Send + Sync,
{

}

pub async fn subscribe<T>(
    State(subscriptions_usecase): State<Arc<SubscriptionUseCase<T>>>,
    Extension(user_id): Extension<Uuid>,
    Json(insert_subscription_model): Json<InsertSubscriptionModel>,
) -> impl IntoResponse
where
    T: SubscriptionRepository + Send + Sync,
{

}

pub async fn cancel_subscription<T>(
    State(subscriptions_usecase): State<Arc<SubscriptionUseCase<T>>>,
    Extension(user_id): Extension<Uuid>,
) -> impl IntoResponse
where
    T: SubscriptionRepository + Send + Sync,
{

}

