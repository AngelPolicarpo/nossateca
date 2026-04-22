use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AddonRole {
    Discover,
    Source,
    MangaSource,
    LegacySearch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonSettingEntry {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonDescriptor {
    pub id: String,
    pub file_name: String,
    pub file_path: String,
    pub role: AddonRole,
    pub enabled: bool,
    pub settings: Vec<AddonSettingEntry>,
}
