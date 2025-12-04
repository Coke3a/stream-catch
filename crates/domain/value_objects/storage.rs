#[derive(Debug, Clone)]
pub struct UploadResult {
    pub remote_prefix: String,
    pub size_bytes: i64,
    pub duration_sec: i32,
}
