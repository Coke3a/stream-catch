use crate::config::{
    config_model::{AdminSecret, UserSecret},
    stage::Stage,
};
use anyhow::{Ok, Result};

use super::config_model::DotEnvyConfig;

pub fn load() -> Result<DotEnvyConfig> {
    dotenvy::dotenv().ok();

    let backend_server = super::config_model::BackendServer {
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

    let worker_server = super::config_model::WorkerServer {
        port: std::env::var("SERVER_PORT_WORKER")
            .expect("SERVER_PORT_WORKER is invalid")
            .parse()?,
    };

    let database = super::config_model::Database {
        url: std::env::var("DATABASE_URL").expect("DATABASE_URL is invalid"),
    };

    let supabase = super::config_model::Supabase {
        project_url: std::env::var("SUPABASE_PROJECT_URL")
            .expect("SUPABASE_PROJECT_URL is invalid"),
        jwt_secret: std::env::var("SUPABASE_JWT_SECRET").expect("SUPABASE_JWT_SECRET is invalid"),
        poster_bucket: std::env::var("SUPABASE_POSTER_BUCKET")
            .unwrap_or_else(|_| "recording_cover".to_string()),
    };

    Ok(DotEnvyConfig {
        backend_server,
        worker_server,
        database,
        supabase,
    })
}

pub fn get_stage() -> Stage {
    dotenvy::dotenv().ok();

    let stage_str = std::env::var("STAGE").unwrap_or("".to_string());
    Stage::try_from(&stage_str).unwrap_or_default()
}

pub fn get_user_secret() -> Result<UserSecret> {
    dotenvy::dotenv().ok();

    Ok(UserSecret {
        secret: std::env::var("JWT_USER_SECRET").expect("JWT_USER_SECRET is invalid"),
        refresh_secret: std::env::var("JWT_USER_REFRESH_SECRET")
            .expect("JWT_USER_REFRESH_SECRET is invalid"),
    })
}

pub fn get_admin_secret() -> Result<AdminSecret> {
    dotenvy::dotenv().ok();

    Ok(AdminSecret {
        secret: std::env::var("JWT_ADMIN_SECRET").expect("JWT_ADMIN_SECRET is invalid"),
        refresh_secret: std::env::var("JWT_ADMIN_REFRESH_SECRET")
            .expect("JWT_ADMIN_REFRESH_SECRET is invalid"),
    })
}
