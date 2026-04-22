wit_bindgen::generate!({
    path: "../../wit/discover-source-plugin.wit",
    world: "manga-source-plugin",
});

use serde_json::Value;

use crate::lexicon::plugin_roles::common_types::{HttpHeader, HttpRequest, HttpResponse};
use crate::lexicon::plugin_roles::host_http;

const MANGADEX_API: &str = "https://api.mangadex.org";
const SOURCE_ID: &str = "mangadex";
const SOURCE_NAME: &str = "MangaDex";
const REQUEST_TIMEOUT_MS: u64 = 20_000;
const CHAPTER_PAGE_LIMIT: u32 = 500;
const MAX_CHAPTER_PAGES: u32 = 20;
const EXCLUDED_EXTERNAL_HOSTS: &[&str] = &["blinktoon.com"];
const PREFERRED_LANGUAGES: &[&str] = &["en", "pt-br"];

struct MangadexSourcePlugin;

export!(MangadexSourcePlugin);

impl Guest for MangadexSourcePlugin {
    fn get_manga_source_info() -> MangaSourceInfo {
        MangaSourceInfo {
            source_name: SOURCE_NAME.to_string(),
            source_id: SOURCE_ID.to_string(),
        }
    }

    fn list_chapters(manga_id: String) -> Result<Vec<MangaChapter>, PluginError> {
        let uuid = parse_manga_id(&manga_id)
            .ok_or_else(|| PluginError::NotFound(format!("invalid manga id '{}'", manga_id)))?;

        let mut chapters: Vec<MangaChapter> = Vec::new();
        let mut offset: u32 = 0;
        let mut fetched_pages: u32 = 0;

        loop {
            let query = build_chapter_query(offset);
            let url = format!("{}/manga/{}/feed", MANGADEX_API, uuid);
            let payload = get_json(&url, query)?;

            let data = payload
                .get("data")
                .and_then(Value::as_array)
                .ok_or_else(|| PluginError::ParsingFailure("missing data array".to_string()))?;

            for entry in data {
                if let Some(chapter) = parse_chapter(entry) {
                    chapters.push(chapter);
                }
            }

            let total = payload
                .get("total")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32;

            offset += CHAPTER_PAGE_LIMIT;
            fetched_pages += 1;

            if offset >= total || data.is_empty() || fetched_pages >= MAX_CHAPTER_PAGES {
                break;
            }
        }

        Ok(chapters)
    }

    fn get_chapter_pages(chapter_id: String) -> Result<MangaPageList, PluginError> {
        let trimmed = chapter_id.trim().to_string();
        if trimmed.is_empty() {
            return Err(PluginError::NotFound("chapter id is required".to_string()));
        }

        let url = format!("{}/at-home/server/{}", MANGADEX_API, trimmed);
        let payload = get_json(&url, Vec::new())?;

        let base_url = payload
            .get("baseUrl")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| PluginError::ParsingFailure("missing baseUrl".to_string()))?;

        let hash = payload
            .pointer("/chapter/hash")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| PluginError::ParsingFailure("missing chapter.hash".to_string()))?;

        let files = payload
            .pointer("/chapter/data")
            .and_then(Value::as_array)
            .ok_or_else(|| PluginError::ParsingFailure("missing chapter.data array".to_string()))?;

        let page_urls = files
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|file| format!("{}/data/{}/{}", base_url, hash, file))
            .collect::<Vec<_>>();

        if page_urls.is_empty() {
            return Err(PluginError::NotFound(
                "no pages available for chapter".to_string(),
            ));
        }

        Ok(MangaPageList {
            chapter_id: trimmed,
            page_urls,
        })
    }
}

fn parse_manga_id(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(rest) = trimmed.strip_prefix("mangadex:manga:") {
        let candidate = rest.trim();
        if !candidate.is_empty() {
            return Some(candidate.to_string());
        }
    }

    if trimmed.matches('-').count() == 4 {
        return Some(trimmed.to_string());
    }

    None
}

fn build_chapter_query(offset: u32) -> Vec<(String, String)> {
    let mut query = Vec::new();
    query.push(("limit".to_string(), CHAPTER_PAGE_LIMIT.to_string()));
    query.push(("offset".to_string(), offset.to_string()));
    query.push(("includes[]".to_string(), "scanlation_group".to_string()));
    query.push(("includes[]".to_string(), "user".to_string()));
    query.push(("order[volume]".to_string(), "desc".to_string()));
    query.push(("order[chapter]".to_string(), "desc".to_string()));

    query.push(("contentRating[]".to_string(), "safe".to_string()));
    query.push(("contentRating[]".to_string(), "suggestive".to_string()));
    query.push(("contentRating[]".to_string(), "erotica".to_string()));
    query.push(("contentRating[]".to_string(), "pornographic".to_string()));

    query.push(("includeUnavailable".to_string(), "1".to_string()));

    for lang in PREFERRED_LANGUAGES {
        query.push(("translatedLanguage[]".to_string(), (*lang).to_string()));
    }

    for host in EXCLUDED_EXTERNAL_HOSTS {
        query.push(("excludeExternalUrl".to_string(), (*host).to_string()));
    }

    query
}

fn parse_chapter(entry: &Value) -> Option<MangaChapter> {
    let id = entry
        .get("id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?
        .to_string();

    let attributes = entry.get("attributes")?;

    let pages = attributes
        .get("pages")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .filter(|value| *value > 0);

    let chapter = attributes
        .get("chapter")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let volume = attributes
        .get("volume")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let title = attributes
        .get("title")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let language = attributes
        .get("translatedLanguage")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let published_at = attributes
        .get("publishAt")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let scanlator = resolve_scanlator(entry);

    Some(MangaChapter {
        id,
        chapter,
        volume,
        title,
        language,
        pages,
        published_at,
        scanlator,
    })
}

fn resolve_scanlator(entry: &Value) -> Option<String> {
    let relationships = entry.get("relationships").and_then(Value::as_array)?;

    for relationship in relationships {
        let rel_type = relationship
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();

        if rel_type != "scanlation_group" {
            continue;
        }

        if let Some(name) = relationship
            .pointer("/attributes/name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(name.to_string());
        }
    }

    None
}

fn get_json(url: &str, query: Vec<(String, String)>) -> Result<Value, PluginError> {
    let response: HttpResponse = host_http::send_http_request(&HttpRequest {
        method: "GET".to_string(),
        url: url.to_string(),
        query: query
            .into_iter()
            .map(|(key, value)| HttpHeader { key, value })
            .collect(),
        headers: vec![HttpHeader {
            key: "accept".to_string(),
            value: "application/json".to_string(),
        }],
        body: None,
        timeout_ms: Some(REQUEST_TIMEOUT_MS),
    })
    .map_err(|err| PluginError::NetworkFailure(format!("request failed: {}", err)))?;

    if response.status == 429 {
        return Err(PluginError::RateLimit("rate limit reached".to_string()));
    }

    if response.status == 404 {
        return Err(PluginError::NotFound(format!("resource not found: {}", url)));
    }

    if !(200..300).contains(&response.status) {
        return Err(PluginError::NetworkFailure(format!(
            "mangadex returned status {}",
            response.status
        )));
    }

    serde_json::from_str::<Value>(&response.body)
        .map_err(|err| PluginError::ParsingFailure(format!("invalid json payload: {}", err)))
}
