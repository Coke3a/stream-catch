use std::fmt::Display;
use serde::{Deserialize, Serialize};

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
            JobStatus::Queued => "Queued",
            JobStatus::Running => "Running",
            JobStatus::Done => "Done",
            JobStatus::Failed => "Failed",
            JobStatus::Dead => "Dead",
        };
        write!(f, "{}", job_type)
    }
}
