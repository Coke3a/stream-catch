use crate::config::stage::Stage;

use super::config_model::{Database, DotEnvyConfig, Supabase, WorkerServer};
use anyhow::Result;
use crates::infra::storages::wasabi::WasabiStorageConfig;

pub fn load() -> Result<DotEnvyConfig> {
    dotenvy::dotenv().ok();

    let worker_server = WorkerServer {
        port: std::env::var("SERVER_PORT_WORKER")
            .expect("SERVER_PORT_WORKER is invalid")
            .parse()?,
        body_limit: std::env::var("SERVER_BODY_LIMIT")
            .expect("SERVER_BODY_LIMIT is invalid")
            .parse()?,
        timeout: std::env::var("SERVER_TIMEOUT")
            .expect("SERVER_TIMEOUT is invalid")
            .parse()?,
    };

    let supabase = Supabase {
        project_url: std::env::var("SUPABASE_PROJECT_URL")
            .expect("SUPABASE_PROJECT_URL is invalid"),
        poster_bucket: std::env::var("SUPABASE_POSTER_BUCKET")
            .unwrap_or_else(|_| "recording_cover".to_string()),
        s3_endpoint: std::env::var("SUPABASE_S3_ENDPOINT").unwrap_or_else(|_| {
            format!(
                "{}/storage/v1/s3",
                std::env::var("SUPABASE_PROJECT_URL")
                    .expect("SUPABASE_PROJECT_URL is invalid")
                    .trim_end_matches('/')
            )
        }),
        s3_region: std::env::var("SUPABASE_S3_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
        s3_access_key: std::env::var("SUPABASE_S3_ACCESS_KEY_ID")
            .expect("SUPABASE_S3_ACCESS_KEY_ID is invalid"),
        s3_secret_key: std::env::var("SUPABASE_S3_SECRET_ACCESS_KEY")
            .expect("SUPABASE_S3_SECRET_ACCESS_KEY is invalid"),
        poster_prefix: std::env::var("SUPABASE_POSTER_PREFIX")
            .unwrap_or_else(|_| "recordings".to_string()),
    };

    let database = Database {
        url: std::env::var("DATABASE_URL").expect("DATABASE_URL is invalid"),
    };

    let video_storage = WasabiStorageConfig {
        endpoint: std::env::var("VIDEO_STORAGE_S3_ENDPOINT")
            .expect("VIDEO_STORAGE_S3_ENDPOINT is invalid"),
        region: std::env::var("VIDEO_STORAGE_S3_REGION")
            .expect("VIDEO_STORAGE_S3_REGION is invalid"),
        bucket: std::env::var("VIDEO_STORAGE_S3_BUCKET")
            .expect("VIDEO_STORAGE_S3_BUCKET is invalid"),
        access_key_id: std::env::var("VIDEO_STORAGE_S3_ACCESS_KEY_ID")
            .expect("VIDEO_STORAGE_S3_ACCESS_KEY_ID is invalid"),
        secret_access_key: std::env::var("VIDEO_STORAGE_S3_SECRET_ACCESS_KEY")
            .expect("VIDEO_STORAGE_S3_SECRET_ACCESS_KEY is invalid"),
        key_prefix: std::env::var("VIDEO_STORAGE_S3_KEY_PREFIX")
            .unwrap_or_else(|_| "recordings".to_string()),
    };

    Ok(DotEnvyConfig {
        worker_server,
        database,
        supabase,
        video_storage,
    })
}

pub fn get_stage() -> Stage {
    dotenvy::dotenv().ok();

    let stage_str = std::env::var("STAGE").unwrap_or("".to_string());
    Stage::try_from(&stage_str).unwrap_or_default()
}
