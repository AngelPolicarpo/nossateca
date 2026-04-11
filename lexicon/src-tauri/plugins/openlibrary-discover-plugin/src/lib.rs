wit_bindgen::generate!({
    path: "../../wit/discover-source-plugin.wit",
    world: "discover-plugin",
});

use serde_json::Value;

use crate::lexicon::plugin_roles::common_types::{HttpHeader, HttpRequest};
use crate::lexicon::plugin_roles::discover_types::DiscoverItem;
use crate::lexicon::plugin_roles::host_http;

const OPEN_LIBRARY_BASE_URL: &str = "https://openlibrary.org";
const REQUEST_TIMEOUT_MS: u64 = 15_000;

const SUBJECTS: [(&str, &str); 7] = [
    ("science_fiction", "Ficcao Cientifica"),
    ("fantasy", "Fantasia"),
    ("mystery", "Misterio"),
    ("manga", "Manga"),
    ("comics", "Quadrinhos"),
    ("biography", "Biografia"),
    ("history", "Historia"),
];

struct OpenLibraryDiscoverPlugin;

export!(OpenLibraryDiscoverPlugin);

impl Guest for OpenLibraryDiscoverPlugin {
    fn list_catalogs() -> Result<Vec<DiscoverCatalog>, PluginError> {
        Ok(vec![
            DiscoverCatalog {
                id: "openlibrary:trending:daily".to_string(),
                name: "Tendencias Diarias".to_string(),
                content_type: "trending".to_string(),
                genres: Vec::new(),
                supported_filters: vec!["year".to_string()],
            },
            DiscoverCatalog {
                id: "openlibrary:trending:weekly".to_string(),
                name: "Tendencias Semanais".to_string(),
                content_type: "trending".to_string(),
                genres: Vec::new(),
                supported_filters: vec!["year".to_string()],
            },
            DiscoverCatalog {
                id: "openlibrary:subjects".to_string(),
                name: "Navegar por Assunto".to_string(),
                content_type: "subject".to_string(),
                genres: SUBJECTS.iter().map(|(slug, _)| (*slug).to_string()).collect(),
                supported_filters: vec!["genre".to_string(), "year".to_string()],
            },
        ])
    }

    fn list_catalog_items(request: DiscoverCatalogQuery) -> Result<DiscoverCatalogPage, PluginError> {
        let page_size = request.page_size.clamp(1, 100);

        match request.catalog_id.as_str() {
            "openlibrary:trending:daily" => list_trending("daily", request.skip, page_size, request.year),
            "openlibrary:trending:weekly" => {
                list_trending("weekly", request.skip, page_size, request.year)
            }
            "openlibrary:subjects" => list_subjects(
                request.genre,
                request.skip,
                page_size,
                request.year,
            ),
            _ => Err(PluginError::NotFound(format!(
                "catalog '{}' not found",
                request.catalog_id
            ))),
        }
    }

    fn get_item_details(item_id: String) -> Result<DiscoverItemDetails, PluginError> {
        let work_id = parse_work_id(&item_id).ok_or_else(|| {
            PluginError::NotFound(format!("invalid discover item id '{}'", item_id))
        })?;

        let work_url = format!("{}/works/{}.json", OPEN_LIBRARY_BASE_URL, work_id);
        let work_payload = get_json(&work_url, Vec::new())?;

        let title = string_field(&work_payload, "title").unwrap_or_else(|| "Untitled".to_string());
        let description = parse_description(work_payload.get("description"));
        let cover_url = work_payload
            .get("covers")
            .and_then(Value::as_array)
            .and_then(|covers| covers.first())
            .and_then(Value::as_i64)
            .map(cover_url)
            .unwrap_or_default();

        let genres = work_payload
            .get("subjects")
            .and_then(Value::as_array)
            .map(|subjects| {
                subjects
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                    .take(8)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let year = string_field(&work_payload, "first_publish_date")
            .and_then(|value| value.chars().take(4).collect::<String>().parse::<u32>().ok());

        let author = resolve_primary_author_name(&work_payload).unwrap_or_else(|| "Autor desconhecido".to_string());
        let isbn = resolve_first_isbn(&work_id)?;

        Ok(DiscoverItemDetails {
            id: item_id,
            title,
            author,
            description,
            cover_url,
            genres,
            year,
            format: None,
            isbn,
            origin_url: Some(format!("{}/works/{}", OPEN_LIBRARY_BASE_URL, work_id)),
        })
    }
}

fn list_trending(
    period: &str,
    skip: u32,
    page_size: u32,
    year_filter: Option<u32>,
) -> Result<DiscoverCatalogPage, PluginError> {
    let url = format!("{}/trending/{}.json", OPEN_LIBRARY_BASE_URL, period);
    let payload = get_json(&url, Vec::new())?;

    let works = payload
        .get("works")
        .and_then(Value::as_array)
        .ok_or_else(|| PluginError::ParsingFailure("missing works array".to_string()))?;

    let total_len = works.len();
    let mut items = works
        .iter()
        .skip(skip as usize)
        .map(parse_item)
        .collect::<Vec<_>>();

    if let Some(year) = year_filter {
        items.retain(|item| item.year == Some(year));
    }

    if items.len() > page_size as usize {
        items.truncate(page_size as usize);
    }

    let has_more = (skip as usize + page_size as usize) < total_len;

    Ok(DiscoverCatalogPage { items, has_more })
}

fn list_subjects(
    genre_filter: Option<String>,
    skip: u32,
    page_size: u32,
    year_filter: Option<u32>,
) -> Result<DiscoverCatalogPage, PluginError> {
    let subject = normalize_subject_slug(genre_filter.as_deref().unwrap_or("fantasy"));

    let url = format!("{}/subjects/{}.json", OPEN_LIBRARY_BASE_URL, subject);
    let payload = get_json(
        &url,
        vec![
            ("limit".to_string(), (page_size + 1).to_string()),
            ("offset".to_string(), skip.to_string()),
        ],
    )?;

    let works = payload
        .get("works")
        .and_then(Value::as_array)
        .ok_or_else(|| PluginError::ParsingFailure("missing works array".to_string()))?;

    let mut items = works.iter().map(parse_item).collect::<Vec<_>>();

    if let Some(year) = year_filter {
        items.retain(|item| item.year == Some(year));
    }

    let has_more = items.len() > page_size as usize;
    if has_more {
        items.truncate(page_size as usize);
    }

    Ok(DiscoverCatalogPage { items, has_more })
}

fn parse_item(value: &Value) -> DiscoverItem {
    let work_key = value
        .get("key")
        .and_then(Value::as_str)
        .unwrap_or("/works/unknown")
        .trim();

    let work_id = work_key
        .strip_prefix("/works/")
        .or_else(|| work_key.strip_prefix("works/"))
        .unwrap_or(work_key);

    let title = string_field(value, "title").unwrap_or_else(|| "Untitled".to_string());

    let author = value
        .get("author_name")
        .and_then(Value::as_array)
        .and_then(|authors| authors.first())
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            value
                .get("authors")
                .and_then(Value::as_array)
                .and_then(|authors| authors.first())
                .and_then(|author| author.get("name"))
                .and_then(Value::as_str)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| "Autor desconhecido".to_string());

    let cover_url = value
        .get("cover_i")
        .and_then(Value::as_i64)
        .or_else(|| value.get("cover_id").and_then(Value::as_i64))
        .map(cover_url)
        .unwrap_or_default();

    let genres = value
        .get("subject")
        .and_then(Value::as_array)
        .map(|subjects| {
            subjects
                .iter()
                .filter_map(Value::as_str)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .take(5)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let year = value
        .get("first_publish_year")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok());

    let short_description = string_field(value, "subtitle");

    let isbn = value
        .pointer("/editions/docs")
        .and_then(Value::as_array)
        .and_then(|docs| docs.first())
        .and_then(|doc| {
            doc.get("isbn")
                .and_then(Value::as_array)
                .and_then(|isbns| isbns.first())
                .and_then(Value::as_str)
        })
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    DiscoverItem {
        id: format!("openlibrary:work:{}", work_id),
        title,
        author,
        cover_url,
        genres,
        year,
        short_description,
        format: string_field(value, "ebook_access"),
        isbn,
    }
}

fn resolve_primary_author_name(work_payload: &Value) -> Option<String> {
    let author_key = work_payload
        .get("authors")
        .and_then(Value::as_array)
        .and_then(|authors| authors.first())
        .and_then(|entry| entry.pointer("/author/key"))
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())?;

    let author_url = format!("{}{}.json", OPEN_LIBRARY_BASE_URL, author_key);
    let author_payload = get_json(&author_url, Vec::new()).ok()?;

    string_field(&author_payload, "name")
}

fn resolve_first_isbn(work_id: &str) -> Result<Option<String>, PluginError> {
    let url = format!("{}/works/{}/editions.json", OPEN_LIBRARY_BASE_URL, work_id);
    let payload = get_json(
        &url,
        vec![
            ("limit".to_string(), "5".to_string()),
            ("offset".to_string(), "0".to_string()),
        ],
    )?;

    let entries = payload
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    for entry in entries {
        if let Some(isbn) = entry
            .get("isbn_13")
            .and_then(Value::as_array)
            .and_then(|isbns| isbns.first())
            .and_then(Value::as_str)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        {
            return Ok(Some(isbn));
        }

        if let Some(isbn) = entry
            .get("isbn_10")
            .and_then(Value::as_array)
            .and_then(|isbns| isbns.first())
            .and_then(Value::as_str)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        {
            return Ok(Some(isbn));
        }
    }

    Ok(None)
}

fn normalize_subject_slug(raw: &str) -> String {
    let normalized = raw
        .trim()
        .to_ascii_lowercase()
        .replace(' ', "_")
        .replace('-', "_");

    if SUBJECTS.iter().any(|(slug, _)| *slug == normalized) {
        return normalized;
    }

    match normalized.as_str() {
        "ficcao_cientifica" | "ficção_científica" => "science_fiction".to_string(),
        "misterio" | "mistério" => "mystery".to_string(),
        "quadrinhos" => "comics".to_string(),
        "historia" | "história" => "history".to_string(),
        _ => "fantasy".to_string(),
    }
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
            "openlibrary returned status {}",
            response.status
        )));
    }

    serde_json::from_str::<Value>(&response.body)
        .map_err(|err| PluginError::ParsingFailure(format!("invalid json payload: {}", err)))
}

fn parse_work_id(item_id: &str) -> Option<String> {
    let trimmed = item_id.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.starts_with("OL") && trimmed.ends_with('W') {
        return Some(trimmed.to_string());
    }

    if let Some(work_id) = trimmed.rsplit(':').next() {
        let value = work_id.trim();
        if value.starts_with("OL") && value.ends_with('W') {
            return Some(value.to_string());
        }
    }

    None
}

fn parse_description(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(content)) => non_empty(content),
        Some(Value::Object(content)) => content
            .get("value")
            .and_then(Value::as_str)
            .and_then(non_empty),
        _ => None,
    }
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .and_then(non_empty)
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.to_string())
}

fn cover_url(cover_id: i64) -> String {
    format!("https://covers.openlibrary.org/b/id/{}-L.jpg", cover_id)
}
