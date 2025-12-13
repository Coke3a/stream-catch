use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecordingStatus {
    #[default]
    LiveRecording,
    LiveEnd,
    WaitingUpload,
    Uploading,
    Ready,
    Failed,
    ExpiredDeleted,
}

impl Display for RecordingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let follow_status = match self {
            RecordingStatus::LiveRecording => "live_recording",
            RecordingStatus::LiveEnd => "live_end",
            RecordingStatus::WaitingUpload => "waiting_upload",
            RecordingStatus::Uploading => "uploading",
            RecordingStatus::Ready => "ready",
            RecordingStatus::Failed => "failed",
            RecordingStatus::ExpiredDeleted => "expired_deleted",
        };
        write!(f, "{}", follow_status)
    }
}
