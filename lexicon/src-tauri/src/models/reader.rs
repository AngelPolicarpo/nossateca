use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookContent {
    pub html: String,
    pub current_chapter: usize,
    pub total_chapters: usize,
    pub chapter_title: String,
    pub book_format: String,
    pub book_file_path: Option<String>,
    pub supports_annotations: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfDocumentData {
    pub bytes_base64: String,
    pub total_pages: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ReadingProgressData {
    pub current_position: String,
    pub progress_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpubSearchMatch {
    pub chapter_index: usize,
    pub chapter_title: String,
    pub snippet: String,
    pub occurrences: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpubSearchResponse {
    pub query: String,
    pub total_matches: usize,
    pub results: Vec<EpubSearchMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpubLinkTarget {
    pub chapter_index: usize,
    pub anchor_id: Option<String>,
}
