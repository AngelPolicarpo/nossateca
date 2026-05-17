wit_bindgen::generate!({
    path: "../../wit/discover-source-plugin.wit",
    world: "discover-plugin",
});

use serde_json::Value;

use crate::lexicon::plugin_roles::common_types::{HttpHeader, HttpRequest};
use crate::lexicon::plugin_roles::discover_types::DiscoverItem;
use crate::lexicon::plugin_roles::host_http;

const MANGADEX_BASE_URL: &str = "https://api.mangadex.org";
const COVER_BASE_URL: &str = "https://uploads.mangadex.org/covers";
const REQUEST_TIMEOUT_MS: u64 = 15_000;
const POPULAR_CATALOG_ID: &str = "mangadex:popular";
const LATEST_CATALOG_ID: &str = "mangadex:latest";

struct MangadexDiscoverPlugin;

export!(MangadexDiscoverPlugin);

impl Guest for MangadexDiscoverPlugin {
    fn list_catalogs() -> Result<Vec<DiscoverCatalog>, PluginError> {
        Ok(vec![
            DiscoverCatalog {
                id: POPULAR_CATALOG_ID.to_string(),
                name: "Mangás Populares".to_string(),
                content_type: "manga".to_string(),
                genres: Vec::new(),
                supported_filters: vec!["year".to_string()],
            },
            DiscoverCatalog {
                id: LATEST_CATALOG_ID.to_string(),
                name: "Mangás Recentes".to_string(),
                content_type: "manga".to_string(),
                genres: Vec::new(),
                supported_filters: vec!["year".to_string()],
            },
        ])
    }

    fn list_catalog_items(request: DiscoverCatalogQuery) -> Result<DiscoverCatalogPage, PluginError> {
        let DiscoverCatalogQuery {
            catalog_id,
            skip,
            page_size,
            genre: _,
            year,
            search_query,
            language: _,
        } = request;

        let page_size = page_size.clamp(1, 100);
        let skip = skip.min(10_000);

        let normalized_search_query = search_query
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());

        let mut query: Vec<(String, String)> = Vec::new();
        query.push(("limit".to_string(), page_size.to_string()));
        query.push(("offset".to_string(), skip.to_string()));
        query.push(("includes[]".to_string(), "author".to_string()));
        query.push(("includes[]".to_string(), "cover_art".to_string()));
        query.push((
            "availableTranslatedLanguage[]".to_string(),
            "en".to_string(),
        ));
        query.push((
            "availableTranslatedLanguage[]".to_string(),
            "pt-br".to_string(),
        ));
        query.push(("contentRating[]".to_string(), "safe".to_string()));
        query.push(("contentRating[]".to_string(), "suggestive".to_string()));

        if let Some(title) = normalized_search_query {
            query.push(("title".to_string(), title.to_string()));
        } else {
            let order_key = match catalog_id.as_str() {
                LATEST_CATALOG_ID => "order[latestUploadedChapter]",
                _ => "order[followedCount]",
            };
            query.push((order_key.to_string(), "desc".to_string()));
        }

        if let Some(year_value) = year {
            query.push(("year".to_string(), year_value.to_string()));
        }

        let payload = get_json(&format!("{}/manga", MANGADEX_BASE_URL), query)?;

        let entries = payload
            .get("data")
            .and_then(Value::as_array)
            .ok_or_else(|| PluginError::ParsingFailure("missing data array".to_string()))?;

        let items = entries.iter().map(parse_item).collect::<Vec<_>>();

        let total = payload
            .get("total")
            .and_then(Value::as_u64)
            .unwrap_or(skip as u64 + entries.len() as u64);

        let has_more = (skip as u64 + entries.len() as u64) < total;

        Ok(DiscoverCatalogPage { items, has_more })
    }

    fn get_item_details(item_id: String) -> Result<DiscoverItemDetails, PluginError> {
        let manga_id = parse_manga_id(&item_id).ok_or_else(|| {
            PluginError::NotFound(format!("invalid manga id '{}'", item_id))
        })?;

        let url = format!("{}/manga/{}", MANGADEX_BASE_URL, manga_id);
        let payload = get_json(
            &url,
            vec![
                ("includes[]".to_string(), "author".to_string()),
                ("includes[]".to_string(), "cover_art".to_string()),
            ],
        )?;

        let data = payload
            .get("data")
            .ok_or_else(|| PluginError::ParsingFailure("missing data field".to_string()))?;

        let item = parse_item(data);

        let description = data
            .pointer("/attributes/description")
            .and_then(pick_localized_string);

        Ok(DiscoverItemDetails {
            id: item.id,
            title: item.title,
            author: item.author,
            description,
            cover_url: item.cover_url,
            genres: item.genres,
            year: item.year,
            page_count: None,
            format: Some("manga".to_string()),
            isbn: None,
            origin_url: Some(format!("https://mangadex.org/title/{}", manga_id)),
            rating_average: None,
            rating_count: None,
        })
    }
}

fn parse_item(entry: &Value) -> DiscoverItem {
    let manga_id = entry
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .trim()
        .to_string();

    let title = entry
        .pointer("/attributes/title")
        .and_then(pick_localized_string)
        .or_else(|| {
            entry
                .pointer("/attributes/altTitles")
                .and_then(Value::as_array)
                .and_then(|titles| titles.iter().find_map(pick_localized_string))
        })
        .unwrap_or_else(|| "Sem título".to_string());

    let author = resolve_author_name(entry).unwrap_or_else(|| "Autor desconhecido".to_string());

    let cover_url = resolve_cover_url(&manga_id, entry).unwrap_or_default();

    let genres = extract_genres(entry, 6);

    let year = entry
        .pointer("/attributes/year")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .filter(|value| *value > 0);

    let short_description = entry
        .pointer("/attributes/description")
        .and_then(pick_localized_string)
        .map(|value| truncate_description(&value, 240));

    DiscoverItem {
        id: format!("mangadex:manga:{}", manga_id),
        title,
        author,
        cover_url,
        genres,
        year,
        page_count: None,
        short_description,
        format: Some("manga".to_string()),
        isbn: None,
    }
}

fn pick_localized_string(value: &Value) -> Option<String> {
    let object = value.as_object()?;
    const PREFERRED_LOCALES: &[&str] = &["pt-br", "pt", "en", "en-us", "ja-ro", "ja"];

    for locale in PREFERRED_LOCALES {
        if let Some(text) = object.get(*locale).and_then(Value::as_str).and_then(non_empty) {
            return Some(text);
        }
    }

    object
        .values()
        .filter_map(Value::as_str)
        .find_map(non_empty)
}

fn resolve_author_name(entry: &Value) -> Option<String> {
    let relationships = entry.get("relationships").and_then(Value::as_array)?;

    for relationship in relationships {
        let rel_type = relationship
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();

        if rel_type != "author" {
            continue;
        }

        if let Some(name) = relationship
            .pointer("/attributes/name")
            .and_then(Value::as_str)
            .and_then(non_empty)
        {
            return Some(name);
        }
    }

    for relationship in relationships {
        let rel_type = relationship
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();

        if rel_type != "artist" {
            continue;
        }

        if let Some(name) = relationship
            .pointer("/attributes/name")
            .and_then(Value::as_str)
            .and_then(non_empty)
        {
            return Some(name);
        }
    }

    None
}

fn resolve_cover_url(manga_id: &str, entry: &Value) -> Option<String> {
    let relationships = entry.get("relationships").and_then(Value::as_array)?;

    for relationship in relationships {
        let rel_type = relationship
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();

        if rel_type != "cover_art" {
            continue;
        }

        if let Some(file_name) = relationship
            .pointer("/attributes/fileName")
            .and_then(Value::as_str)
            .and_then(non_empty)
        {
            return Some(format!(
                "{}/{}/{}.512.jpg",
                COVER_BASE_URL, manga_id, file_name
            ));
        }
    }

    None
}

fn extract_genres(entry: &Value, limit: usize) -> Vec<String> {
    let tags = match entry.pointer("/attributes/tags").and_then(Value::as_array) {
        Some(value) => value,
        None => return Vec::new(),
    };

    let mut results = Vec::new();

    for tag in tags {
        let group = tag
            .pointer("/attributes/group")
            .and_then(Value::as_str)
            .unwrap_or_default();

        if group != "genre" && group != "theme" {
            continue;
        }

        if let Some(name) = tag
            .pointer("/attributes/name")
            .and_then(pick_localized_string)
        {
            results.push(name);
            if results.len() >= limit {
                break;
            }
        }
    }

    results
}

fn truncate_description(value: &str, limit: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= limit {
        return trimmed.to_string();
    }

    let mut truncated = String::new();
    for (index, ch) in trimmed.chars().enumerate() {
        if index >= limit {
            break;
        }
        truncated.push(ch);
    }

    format!("{}…", truncated.trim_end())
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

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

fn get_json(url: &str, query: Vec<(String, String)>) -> Result<Value, PluginError> {
    let response = host_http::send_http_request(&HttpRequest {
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
