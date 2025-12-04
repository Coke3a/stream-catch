use crate::{
    axum_http::{
        default_routers,
        routers::{self, subscriptions::routes},
    },
    config::config_model::DotEnvyConfig,
};
use anyhow::Result;
use axum::{
    Router,
    http::{
        Method,
        header::{AUTHORIZATION, CONTENT_TYPE},
    },
    routing::get,
};
use crates::infra;
use infra::db::postgres::postgres_connection::PgPoolSquad;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower_http::{
    cors::{Any, CorsLayer},
    limit::RequestBodyLimitLayer,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::info;

pub async fn start(config: Arc<DotEnvyConfig>, db_pool: Arc<PgPoolSquad>) -> Result<()> {
    let app = Router::new()
        .fallback(default_routers::not_found)
        .nest(
            "/api/v1/live-following",
            routers::live_following::routes(Arc::clone(&db_pool)),
        )
        .nest("/api/v1/subscriptions", routes(Arc::clone(&db_pool)))
        .route("/api/v1/health-check", get(default_routers::health_check))
        .layer(TimeoutLayer::new(Duration::from_secs(
            config.backend_server.timeout,
        )))
        .layer(RequestBodyLimitLayer::new(
            (config.backend_server.body_limit * 1024 * 1024).try_into()?,
        ))
        .layer(
            CorsLayer::new()
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PATCH,
                    Method::PUT,
                    Method::DELETE,
                ])
                .allow_headers([AUTHORIZATION, CONTENT_TYPE])
                .allow_origin(Any), // TODO Add the domain later
        )
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], config.backend_server.port));
    let listener = TcpListener::bind(addr).await?;

    info!("Server is running on port {}", config.backend_server.port);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdow_signal())
        .await?;

    Ok(())
}

async fn shutdow_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
    };

    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Received ctrl+C signal"),
        _ = terminate => info!("Received terminate signal"),
    }
}
