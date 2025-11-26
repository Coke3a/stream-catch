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
struct StartData {
    platform: Option<String>,
    channel: Option<String>,
    url: Option<String>,
    live_info: Option<LiveInfo>,
}

// Recording end webhook
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecordingEngineLiveEndWebhook {
    pub id: Uuid,
    pub ts: DateTime<Utc>,
    #[serde(rename = "type")]
    pub type_: String,
    pub data: EndData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct EndData {
    platform: Option<String>,
    channel: Option<String>,
    url: Option<String>,
    live_info: Option<LiveInfo>,
}


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
struct FileFinishData {
    platform: Option<String>,
    channel: Option<String>,
    path: Option<String>,
    filesize: Option<u64>,
    duration: Option<f64>,
}

// video_transmux_finish webhook
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecordingEngineTransmuxFinishWebhook {
    pub id: Uuid,
    pub ts: DateTime<Utc>,
    #[serde(rename = "type")]
    pub type_: String,
    pub data: FileFinishData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TransmuxFinishData {
    platform: Option<String>,
    channel: Option<String>,
    input: Option<String>,
    output: Option<String>,
}

// live info
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct LiveInfo {
    uid: Option<String>,
    uname: Option<String>,
    avatar: Option<String>,
    title: Option<String>,
    cover: Option<String>,
    categories: Option<Vec<String>>,
    status: Option<String>,
    live_id: Option<String>,
}
