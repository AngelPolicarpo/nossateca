use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ChatMessage {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub source_level: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSettings {
    pub ai_model_path: String,
    pub ai_embedding_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSetupInfo {
    pub ai_model_path: String,
    pub ai_embedding_path: String,
    pub llm_model_type: Option<String>,
    pub embedding_model_type: Option<String>,
    pub default_models_dir: String,
    pub detected_gguf: Vec<String>,
    pub detected_onnx: Vec<String>,
    pub llm_configured: bool,
    pub embedding_configured: bool,
    pub llm_file_size_mb: Option<u64>,
    pub embedding_file_size_mb: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatAnswer {
    pub answer: String,
    pub source_level: String,
    pub source_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct BookIndexProgress {
    pub book_id: i64,
    pub status: String,
    pub current_chapter: i64,
    pub total_chapters: i64,
    pub eta_seconds: Option<f64>,
    pub message: String,
    pub updated_at: String,
}
