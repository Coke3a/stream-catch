use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Incoming webhook envelope from the recording engine.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecordingEngineWebhook {
    pub id: Uuid,
    pub ts: DateTime<Utc>,
    #[serde(flatten)]
    pub event: RecordingEngineWebhookEvent,
}

/// Supported webhook variants (tagged by `type` in the payload).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum RecordingEngineWebhookEvent {
    TitleChange(TitleChangePayload),
    VideoFileCreate(VideoFileCreatePayload),
    VideoFileFinish(VideoFileFinishPayload),
    VideoTransmuxFinish(VideoTransmuxFinishPayload),
    LiveStart(LiveSessionPayload),
    LiveEnd(LiveSessionPayload),
    Error(WebhookErrorPayload),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RecordingEnginePlatform {
    Bigo,
    TikTok,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RecordingEngineLiveStatus {
    Live,
    Offline,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveInfoPayload {
    pub uid: String,
    pub uname: String,
    pub avatar: String,
    pub title: String,
    pub cover: String,
    pub categories: Vec<String>,
    pub status: RecordingEngineLiveStatus,
    pub live_id: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TitleChangePayload {
    pub platform: RecordingEnginePlatform,
    pub channel: String,
    pub old_live_info: LiveInfoPayload,
    pub new_live_info: LiveInfoPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoFileCreatePayload {
    pub platform: RecordingEnginePlatform,
    pub channel: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoFileFinishPayload {
    pub platform: RecordingEnginePlatform,
    pub channel: String,
    pub path: String,
    pub filesize: i64,
    pub duration: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoTransmuxFinishPayload {
    pub platform: RecordingEnginePlatform,
    pub channel: String,
    pub input: String,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveSessionPayload {
    pub platform: RecordingEnginePlatform,
    pub channel: String,
    pub url: String,
    pub live_info: LiveInfoPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WebhookErrorPayload {
    pub platform: RecordingEnginePlatform,
    pub channel: String,
    pub error: String,
}
