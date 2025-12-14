use crate::{
    axum_http::{
        default_routers,
        routers::{self},
    },
    config::config_model::DotEnvyConfig,
    usecases::cleanup_expired_recordings::CleanupExpiredRecordingsUseCase,
    usecases::recording_engine_webhook::RecordingEngineWebhookUseCase,
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
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower_http::{
    cors::CorsLayer, limit::RequestBodyLimitLayer, timeout::TimeoutLayer, trace::TraceLayer,
};
use tracing::info;

pub async fn start(
    config: Arc<DotEnvyConfig>,
    usecase: Arc<RecordingEngineWebhookUseCase>,
    cleanup_usecase: Arc<CleanupExpiredRecordingsUseCase>,
) -> Result<()> {
    let allowed_origins = vec![
        "http://localhost".parse()?,
        "http://127.0.0.1".parse()?,
        "http://localhost:3000".parse()?,
        "http://127.0.0.1:3000".parse()?,
    ];

    let app = Router::new()
        .fallback(default_routers::not_found)
        .nest(
            "/internal/recording-engine",
            routers::recording_engine_webhook::routes(usecase),
        )
        .nest(
            "/internal/v1/cleanup",
            routers::cleanup_recordings::routes(Arc::clone(&config), cleanup_usecase),
        )
        .route("/health-check", get(default_routers::health_check))
        .layer(TimeoutLayer::new(Duration::from_secs(
            config.worker_server.timeout,
        )))
        .layer(RequestBodyLimitLayer::new(
            (config.worker_server.body_limit * 1024 * 1024).try_into()?,
        ))
        .layer(
            CorsLayer::new()
                .allow_methods([Method::POST])
                .allow_headers([AUTHORIZATION, CONTENT_TYPE])
                .allow_origin(allowed_origins),
        )
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], config.worker_server.port));
    let listener = TcpListener::bind(addr).await?;
    info!("Worker HTTP server running on {}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
    };

    // Issue #5: Support SIGTERM for graceful shutdown in Docker/K8s/Fly (Unix only).
    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};
        let mut sigterm =
            signal(SignalKind::terminate()).expect("Failed to install SIGTERM signal handler");
        sigterm.recv().await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Received ctrl+C signal"),
        _ = terminate => info!("Received terminate signal"),
    }
}
