use crate::config::stage::Stage;

use super::config_model::{BackendServer, DotEnvyConfig, StripeConfig};
use anyhow::Result;
use uuid::Uuid;

pub fn load() -> Result<DotEnvyConfig> {
    dotenvy::dotenv().ok();

    let backend_server = BackendServer {
        port: std::env::var("SERVER_PORT_BACKEND")
            .expect("SERVER_PORT_BACKEND is invalid")
            .parse()?,
        body_limit: std::env::var("SERVER_BODY_LIMIT")
            .expect("SERVER_BODY_LIMIT is invalid")
            .parse()?,
        timeout: std::env::var("SERVER_TIMEOUT")
            .expect("SERVER_TIMEOUT is invalid")
            .parse()?,
    };

    let supabase = super::config_model::Supabase {
        jwt_secret: std::env::var("SUPABASE_JWT_SECRET").expect("SUPABASE_JWT_SECRET is invalid"),
    };

    let watch_url = super::config_model::WatchUrl {
        jwt_secret: std::env::var("WATCH_URL_JWT_SECRET").expect("WATCH_URL_JWT_SECRET is invalid"),
        base_url: std::env::var("WATCH_URL_BASE_URL").expect("WATCH_URL_BASE_URL is invalid"),
        ttl_seconds: std::env::var("WATCH_URL_TTL_SECONDS")
            .ok()
            .map(|v| v.parse())
            .transpose()?
            .unwrap_or(600),
    };

    let database = super::config_model::Database {
        url: std::env::var("DATABASE_URL").expect("DATABASE_URL is invalid"),
    };

    let stripe = StripeConfig {
        secret_key: std::env::var("STRIPE_SECRET_KEY").expect("STRIPE_SECRET_KEY is invalid"),
        webhook_secret: std::env::var("STRIPE_WEBHOOK_SECRET")
            .expect("STRIPE_WEBHOOK_SECRET is invalid"),
        success_url: std::env::var("STRIPE_SUCCESS_URL").unwrap_or_else(|_| {
            "https://example.com/checkout/success?session_id={CHECKOUT_SESSION_ID}".to_string()
        }),
        cancel_url: std::env::var("STRIPE_CANCEL_URL")
            .unwrap_or_else(|_| "https://example.com/checkout/cancel".to_string()),
    };

    let free_plan_id =
        std::env::var("STREAMCATCH_FREE_PLAN_ID").unwrap_or_else(|_| Uuid::nil().to_string());
    let free_plan_id = Uuid::parse_str(&free_plan_id).expect("STREAMCATCH_FREE_PLAN_ID is invalid");

    Ok(DotEnvyConfig {
        backend_server,
        database,
        supabase,
        watch_url,
        stripe,
        free_plan_id,
    })
}

pub fn get_stage() -> Stage {
    dotenvy::dotenv().ok();

    let stage_str = std::env::var("STAGE").unwrap_or("".to_string());
    Stage::try_from(&stage_str).unwrap_or_default()
}
