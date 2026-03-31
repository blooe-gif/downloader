use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            TaskStatus::Queued => "queued",
            TaskStatus::Running => "running",
            TaskStatus::Paused => "paused",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct DownloadTask {
    pub id: i64,
    pub url: String,
    pub output_path: String,
    pub file_size: i64,
    pub supports_ranges: i64,
    pub priority: f64,
    pub status: String,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ChunkState {
    pub id: i64,
    pub task_id: i64,
    pub start_offset: i64,
    pub end_offset: i64,
    pub downloaded: i64,
    pub status: String,
    pub retries: i64,
}

#[derive(Debug, Clone)]
pub struct ChunkWork {
    pub chunk_id: i64,
    pub start: u64,
    pub end: u64,
    pub downloaded: u64,
    pub retries: u32,
}
