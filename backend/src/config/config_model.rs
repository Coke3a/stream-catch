#[derive(Debug, Clone)]
pub struct DotEnvyConfig {
    pub backend_server: BackendServer,
    pub worker_server: WorkerServer,
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
pub struct WorkerServer {
    pub port: u16,
}

#[derive(Debug, Clone)]
pub struct Database {
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct UserSecret {
    pub secret: String,
    pub refresh_secret: String,
}

#[derive(Debug, Clone)]
pub struct AdminSecret {
    pub secret: String,
    pub refresh_secret: String,
}

#[derive(Debug, Clone)]
pub struct Supabase {
    pub project_url: String,
    pub jwt_secret: String,
    pub poster_bucket: String,
    pub s3_endpoint: String,
    pub s3_region: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
    pub poster_prefix: String,
}
