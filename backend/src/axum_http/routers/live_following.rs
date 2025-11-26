use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    response::IntoResponse, routing::{delete, get, post},
};
use uuid::Uuid;

use crate::config::stage::Stage;
use application::usercases::live_following::LiveFollowingUseCase;
use domain::{
    repositories::live_following::LiveFollowingRepository,
    value_objects::live_following::{InsertFollowLiveAccountModel, ListFollowsFilter},
};
use infra::postgres::{
    postgres_connection::PgPoolSquad,
    repositories::live_following::LiveFollowingPostgres,
};

pub fn routes(db_pool: Arc<PgPoolSquad>) -> Router {
    let live_following_repository = LiveFollowingPostgres::new(Arc::clone(&db_pool));
    let live_following_usecase = LiveFollowingUseCase::new(Arc::new(live_following_repository));

    Router::new()
        .route("/", post(follow))
        .route("/", delete(unfollow))
        .route("/", get(list))
        .with_state(Arc::new(live_following_usecase))
}

pub async fn follow<T>(
    State(live_following_usecase): State<Arc<LiveFollowingUseCase<T>>>,
    Extension(user_id): Extension<Uuid>,
    Path(url): Path<String>,
) -> impl IntoResponse
where
    T: LiveFollowingRepository + Send + Sync,
{
    // convert base64 url to url
    // Detect which streaming platform this URL corresponds to
    // send to recording engine to check is url is exising
    // if exising
    // -	add follow
    // -	response ok
    // if not exising
    // -	response not found
}

pub async fn unfollow<T>(
    State(live_following_usecase): State<Arc<LiveFollowingUseCase<T>>>,
    Extension(user_id): Extension<Uuid>,
    Path(follow_id): Path<Uuid>,
) -> impl IntoResponse
where
    T: LiveFollowingRepository + Send + Sync,
{
    // find follow by user_id and follow_id
    // If the update time is less than 1 day
    // -	return a failed response
    // else set follow status to Inactive
}

pub async fn list<T>(
    State(live_following_usecase): State<Arc<LiveFollowingUseCase<T>>>,
    Extension(user_id): Extension<Uuid>,
) -> impl IntoResponse
where
    T: LiveFollowingRepository + Send + Sync,
{
    // get (join recording with follow) by user_id
    // response
}
