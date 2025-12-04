#[derive(Debug, Clone)]
pub struct DotEnvyConfig {
    pub worker_server: WorkerServer,
    pub database: Database,
    pub supabase: Supabase,
}

#[derive(Debug, Clone)]
pub struct WorkerServer {
    pub port: u16,
    pub timeout: u64,
    pub body_limit: u64,
}

#[derive(Debug, Clone)]
pub struct Database {
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct Supabase {
    pub project_url: String,
    pub poster_bucket: String,
    pub s3_endpoint: String,
    pub s3_region: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
    pub poster_prefix: String,
}
