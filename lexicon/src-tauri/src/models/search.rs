use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchBookResult {
    pub id: String,
    pub title: String,
    pub author: Option<String>,
    pub source: String,
    pub format: Option<String>,
    pub download_url: String,
    pub score: f32,
}
