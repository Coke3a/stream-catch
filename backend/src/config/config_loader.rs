use crate::config::stage::Stage;

use super::config_model::{BackendServer, DotEnvyConfig};
use anyhow::Result;


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

    let database = super::config_model::Database {
        url: std::env::var("DATABASE_URL").expect("DATABASE_URL is invalid"),
    };

    Ok(DotEnvyConfig {
        backend_server,
        database,
        supabase,
    })
}

pub fn get_stage() -> Stage {
    dotenvy::dotenv().ok();

    let stage_str = std::env::var("STAGE").unwrap_or("".to_string());
    Stage::try_from(&stage_str).unwrap_or_default()
}

