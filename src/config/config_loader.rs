use anyhow::Result;
use super::config_model::{DotEnvyConfig, Server, Database};

pub fn load() -> Result<DotEnvyConfig> {
    dotenvy::dotenv().ok();
    
    let server: Server = Server { 
        port: std::env::var("SERVER_PORT").expect("SERVER_PORT is invalid").parse()?, 
        body_limit: std::env::var("SERVER_LIMIT").expect("SERVER_LIMIT is invalid").parse()?, 
        timeout: std::env::var("SERVER_TIMEOUT").expect("SERVER_TIMEOUT is invalid").parse()?, 
    };
    
    let database: Database = Database { 
        url: std::env::var("SERVER_TIMEOUT").expect("SERVER_TIMEOUT is invalid"), 
    };
    
    Ok(DotEnvyConfig { server, database })
}