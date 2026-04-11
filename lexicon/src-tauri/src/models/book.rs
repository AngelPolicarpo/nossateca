use serde::{Deserialize, Serialize};

pub const BOOK_STATUS_UNREAD: &str = "unread";
pub const BOOK_STATUS_READING: &str = "reading";
pub const BOOK_STATUS_FINISHED: &str = "finished";

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Book {
    pub id: i64,
    pub title: String,
    pub author: Option<String>,
    pub format: String,
    pub file_path: String,
    pub file_hash: Option<String>,
    pub status: String,
    pub created_at: String,
}
