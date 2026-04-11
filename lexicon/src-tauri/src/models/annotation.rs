use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Annotation {
    pub id: String,
    pub book_id: i64,
    pub annotation_type: String,
    pub position: String,
    pub position_end: Option<String>,
    pub selected_text: Option<String>,
    pub note_text: Option<String>,
    pub color: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewAnnotation {
    pub annotation_type: String,
    pub position: String,
    pub position_end: Option<String>,
    pub selected_text: Option<String>,
    pub note_text: Option<String>,
    pub color: Option<String>,
}
