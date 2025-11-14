use std::fmt::Display;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecordingStatus {
    #[default]
    Recording,
    Uploading,
    Ready,
    Failed,
}

impl Display for RecordingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let follow_status = match self {
            RecordingStatus::Recording => "Recording",
            RecordingStatus::Uploading => "Uploading",
            RecordingStatus::Ready => "Ready",
            RecordingStatus::Failed => "Error",
        };
        write!(f, "{}", follow_status)
    }
}
