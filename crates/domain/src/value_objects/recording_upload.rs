use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingUploadPayload {
    pub recording_id: Uuid,
    pub local_path: String,
}
