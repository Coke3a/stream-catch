use std::fmt::Display;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobType {
    RecordingUpload,
    NotifyReady,
}

impl Display for JobType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let job_type = match self {
            JobType::RecordingUpload => "RecordingUpload",
            JobType::NotifyReady => "NotifyReady",
        };
        write!(f, "{}", job_type)
    }
}
