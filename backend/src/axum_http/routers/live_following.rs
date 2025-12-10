use crate::usecases::{live_following::LiveFollowingUseCase, plan_resolver::PlanResolver};
use crate::{axum_http::auth::AuthUser, config::config_model::DotEnvyConfig};
use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
};
use crates::{
    domain::repositories::{
        live_following::LiveFollowingRepository, plans::PlanRepository,
        subscriptions::SubscriptionRepository,
    },
    infra::db::{
        postgres::postgres_connection::PgPoolSquad,
        repositories::{
            live_following::LiveFollowingPostgres, plans::PlanPostgres,
            subscriptions::SubscriptionPostgres,
        },
    },
};
use std::sync::Arc;
use tracing::info;

pub fn routes(db_pool: Arc<PgPoolSquad>, config: Arc<DotEnvyConfig>) -> Router {
    let live_following_repository = LiveFollowingPostgres::new(Arc::clone(&db_pool));
    let plan_repository = PlanPostgres::new(Arc::clone(&db_pool));
    let subscription_repository = SubscriptionPostgres::new(Arc::clone(&db_pool));

    let plan_resolver = PlanResolver::new(
        Arc::new(plan_repository),
        Arc::new(subscription_repository),
        config.free_plan_id,
    );

    let live_following_usecase =
        LiveFollowingUseCase::new(Arc::new(live_following_repository), Arc::new(plan_resolver));

    Router::new()
        .route("/:value", post(follow))
        .with_state(Arc::new(live_following_usecase))
}

pub async fn follow<L, P, S>(
    State(live_following_usecase): State<Arc<LiveFollowingUseCase<L, P, S>>>,
    auth: AuthUser,
    Path(url): Path<String>,
) -> impl IntoResponse
where
    L: LiveFollowingRepository + Send + Sync + 'static,
    P: PlanRepository + Send + Sync + 'static,
    S: SubscriptionRepository + Send + Sync + 'static,
{
    use base64::{Engine as _, engine::general_purpose};

    info!(
        %auth.user_id,
        raw_value = %url,
        "live_following: follow request received"
    );

    // Decode base64url
    let decoded_url_bytes = match general_purpose::URL_SAFE_NO_PAD.decode(&url) {
        Ok(bytes) => bytes,
        Err(_) => {
            info!(
                %auth.user_id,
                status = StatusCode::BAD_REQUEST.as_u16(),
                "live_following: invalid base64url payload"
            );
            return (StatusCode::BAD_REQUEST, "Invalid base64url").into_response();
        }
    };

    let decoded_url = match String::from_utf8(decoded_url_bytes) {
        Ok(s) => s,
        Err(_) => {
            info!(
                %auth.user_id,
                status = StatusCode::BAD_REQUEST.as_u16(),
                "live_following: invalid UTF-8 sequence in payload"
            );
            return (StatusCode::BAD_REQUEST, "Invalid UTF-8 sequence").into_response();
        }
    };

    match live_following_usecase
        .follow(auth.user_id, decoded_url)
        .await
    {
        Ok(_) => {
            info!(
                %auth.user_id,
                status = StatusCode::OK.as_u16(),
                "live_following: follow processed successfully"
            );
            (StatusCode::OK, "Followed successfully").into_response()
        }
        Err(e) => {
            let error_message = e.to_string();
            if error_message.contains("Follow already exists") {
                let status = StatusCode::CONFLICT;
                info!(
                    %auth.user_id,
                    status = status.as_u16(),
                    error = %error_message,
                    "live_following: follow already exists"
                );
                (status, "Follow already exists").into_response()
            } else if error_message.contains("Invalid URL")
                || error_message.contains("Unsupported platform")
            {
                let status = StatusCode::BAD_REQUEST;
                info!(
                    %auth.user_id,
                    status = status.as_u16(),
                    error = %error_message,
                    "live_following: invalid follow request"
                );
                (status, error_message).into_response()
            } else if error_message.contains("follow limit reached") {
                let status = StatusCode::FORBIDDEN;
                info!(
                    %auth.user_id,
                    status = status.as_u16(),
                    error = %error_message,
                    "live_following: follow limit reached"
                );
                (status, error_message).into_response()
            } else {
                let status = StatusCode::INTERNAL_SERVER_ERROR;
                info!(
                    %auth.user_id,
                    status = status.as_u16(),
                    error = %error_message,
                    "live_following: unexpected follow failure"
                );
                (status, error_message).into_response()
            }
        }
    }
}
