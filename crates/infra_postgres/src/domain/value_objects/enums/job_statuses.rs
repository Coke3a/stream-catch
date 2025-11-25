use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    #[default]
    Queued,
    Running,
    Done,
    Failed,
    Dead,
}

impl Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let job_type = match self {
            JobStatus::Queued => "queued",
            JobStatus::Running => "running",
            JobStatus::Done => "done",
            JobStatus::Failed => "failed",
            JobStatus::Dead => "dead",
        };
        write!(f, "{}", job_type)
    }
}
