#[derive(Debug, Clone)]
pub struct DotEnvyConfig {
    pub backend_server: BackendServer,
    pub database: Database,
    pub supabase: Supabase,
    pub watch_url: WatchUrl,
}

#[derive(Debug, Clone)]
pub struct BackendServer {
    pub port: u16,
    pub body_limit: u64,
    pub timeout: u64,
}

#[derive(Debug, Clone)]
pub struct Database {
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct Supabase {
    pub jwt_secret: String,
}

#[derive(Debug, Clone)]
pub struct WatchUrl {
    pub jwt_secret: String,
    pub base_url: String,
    pub ttl_seconds: u64,
}
