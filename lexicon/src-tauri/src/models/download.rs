use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRecord {
    pub id: String,
    pub source_url: String,
    pub source_type: String,
    pub file_name: String,
    pub file_path: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub total_bytes: Option<i64>,
    pub downloaded_bytes: i64,
    pub speed_bps: Option<i64>,
    pub torrent_info_hash: Option<String>,
    pub torrent_peers: Option<i64>,
    pub torrent_seeds: Option<i64>,
    pub progress_percent: f32,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartDownloadRequest {
    pub source_url: String,
    pub file_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgressEvent {
    pub id: String,
    pub file_name: String,
    pub status: String,
    pub downloaded_bytes: i64,
    pub total_bytes: Option<i64>,
    pub speed_bps: Option<i64>,
    pub progress_percent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadStateEvent {
    pub id: String,
    pub file_name: String,
    pub status: String,
    pub file_path: Option<String>,
    pub error_message: Option<String>,
    pub downloaded_bytes: i64,
    pub total_bytes: Option<i64>,
    pub speed_bps: Option<i64>,
    pub progress_percent: f32,
}

impl DownloadStateEvent {
    pub fn from_record(record: &DownloadRecord) -> Self {
        Self {
            id: record.id.clone(),
            file_name: record.file_name.clone(),
            status: record.status.clone(),
            file_path: record.file_path.clone(),
            error_message: record.error_message.clone(),
            downloaded_bytes: record.downloaded_bytes,
            total_bytes: record.total_bytes,
            speed_bps: record.speed_bps,
            progress_percent: record.progress_percent,
        }
    }
}
