use serde::{Deserialize, Serialize};

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
