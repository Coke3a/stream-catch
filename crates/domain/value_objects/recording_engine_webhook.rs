use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Recording start webhook
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecordingEngineLiveStartWebhook {
    pub id: Uuid,
    pub ts: DateTime<Utc>,
    #[serde(rename = "type")]
    pub type_: String,
    pub data: StartData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StartData {
    pub platform: Option<String>,
    pub channel: Option<String>,
    pub url: Option<String>,
    pub live_info: Option<LiveInfo>,
}

// // Recording end webhook // no use for now
// #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
// pub struct RecordingEngineLiveEndWebhook {
//     pub id: Uuid,
//     pub ts: DateTime<Utc>,
//     #[serde(rename = "type")]
//     pub type_: String,
//     pub data: EndData,
// }

// #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
// struct EndData {
//     platform: Option<String>,
//     channel: Option<String>,
//     url: Option<String>,
//     live_info: Option<LiveInfo>,
// }

// video_file_finish webhook
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecordingEngineFileFinishWebhook {
    pub id: Uuid,
    pub ts: DateTime<Utc>,
    #[serde(rename = "type")]
    pub type_: String,
    pub data: FileFinishData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileFinishData {
    pub platform: Option<String>,
    pub channel: Option<String>,
    pub path: Option<String>,
    pub filesize: Option<u64>,
    pub duration: Option<f64>,
}

// video_transmux_finish webhook
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecordingEngineTransmuxFinishWebhook {
    pub id: Uuid,
    pub ts: DateTime<Utc>,
    #[serde(rename = "type")]
    pub type_: String,
    pub data: TransmuxFinishData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransmuxFinishData {
    pub platform: Option<String>,
    pub channel: Option<String>,
    pub input: Option<String>,
    pub output: Option<String>,
}

// error webhook
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecordingEngineErrorWebhook {
    pub id: Uuid,
    pub ts: DateTime<Utc>,
    #[serde(rename = "type")]
    pub type_: String,
    pub data: ErrorData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ErrorData {
    pub platform: Option<String>,
    pub channel: Option<String>,
    pub error: Option<String>,
}

// live info
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveInfo {
    pub uid: Option<String>,
    pub uname: Option<String>,
    pub avatar: Option<String>,
    pub title: Option<String>,
    pub cover: Option<String>,
    pub categories: Option<Vec<String>>,
    pub status: Option<String>,
    pub live_id: Option<String>,
}
