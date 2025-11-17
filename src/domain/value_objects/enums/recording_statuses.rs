use std::fmt::Display;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecordingStatus {
    #[default]
    Processing,
    Ready,
    Failed,
}

impl Display for RecordingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let follow_status = match self {
            RecordingStatus::Processing => "processing",
            RecordingStatus::Ready => "ready",
            RecordingStatus::Failed => "failed",
        };
        write!(f, "{}", follow_status)
    }
}
