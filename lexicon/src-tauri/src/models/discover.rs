use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginErrorKind {
    NetworkFailure,
    ParsingFailure,
    RateLimit,
    NotFound,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginTypedError {
    pub kind: PluginErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverCatalog {
    pub plugin_id: String,
    pub id: String,
    pub name: String,
    pub content_type: String,
    pub genres: Vec<String>,
    pub supported_filters: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverCatalogItem {
    pub plugin_id: String,
    pub catalog_id: String,
    pub id: String,
    pub title: String,
    pub author: String,
    pub cover_url: String,
    pub genres: Vec<String>,
    pub year: Option<u32>,
    pub page_count: Option<u32>,
    pub short_description: Option<String>,
    pub format: Option<String>,
    pub isbn: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverCatalogPageResponse {
    pub plugin_id: String,
    pub catalog_id: String,
    pub items: Vec<DiscoverCatalogItem>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverItemDetails {
    pub plugin_id: String,
    pub id: String,
    pub title: String,
    pub author: String,
    pub description: Option<String>,
    pub cover_url: String,
    pub genres: Vec<String>,
    pub year: Option<u32>,
    pub page_count: Option<u32>,
    pub format: Option<String>,
    pub isbn: Option<String>,
    pub origin_url: Option<String>,
    pub rating_average: Option<f64>,
    pub rating_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcePluginInfo {
    pub plugin_id: String,
    pub source_name: String,
    pub source_id: String,
    pub supported_formats: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceDownloadResult {
    pub download_url: String,
    pub format: String,
    pub size: Option<String>,
    pub language: Option<String>,
    pub quality: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSearchResultGroup {
    pub plugin_id: String,
    pub source_name: String,
    pub source_id: String,
    pub supported_formats: Vec<String>,
    pub results: Vec<SourceDownloadResult>,
    pub error: Option<PluginTypedError>,
}
