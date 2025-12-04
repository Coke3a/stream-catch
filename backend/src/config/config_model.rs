#[derive(Debug, Clone)]
pub struct DotEnvyConfig {
    pub backend_server: BackendServer,
    pub database: Database,
    pub supabase: Supabase,
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
