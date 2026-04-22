use serde::{Deserialize, Serialize};

use super::discover::PluginTypedError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MangaSourcePluginInfo {
    pub plugin_id: String,
    pub source_name: String,
    pub source_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MangaChapter {
    pub id: String,
    pub chapter: Option<String>,
    pub volume: Option<String>,
    pub title: Option<String>,
    pub language: Option<String>,
    pub pages: Option<u32>,
    pub published_at: Option<String>,
    pub scanlator: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MangaChapterGroup {
    pub plugin_id: String,
    pub source_name: String,
    pub source_id: String,
    pub chapters: Vec<MangaChapter>,
    pub error: Option<PluginTypedError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MangaPageList {
    pub chapter_id: String,
    pub page_urls: Vec<String>,
}
