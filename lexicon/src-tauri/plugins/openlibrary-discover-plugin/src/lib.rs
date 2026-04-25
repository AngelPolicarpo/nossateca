wit_bindgen::generate!({
    path: "../../wit/discover-source-plugin.wit",
    world: "discover-plugin",
});

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;
use serde_json::Value;

use crate::lexicon::plugin_roles::common_types::{HttpHeader, HttpRequest};
use crate::lexicon::plugin_roles::discover_types::DiscoverItem;
use crate::lexicon::plugin_roles::host_http;

const OPEN_LIBRARY_BASE_URL: &str = "https://openlibrary.org";
const REQUEST_TIMEOUT_MS: u64 = 15_000;
const EDITIONS_PREFETCH_LIMIT: u32 = 40;
const DEFAULT_SUBJECT_FALLBACK: &str = "fantasy";
const SEARCH_FIELDS: &str =
    "key,title,author_name,cover_i,first_publish_year,subject,subtitle,isbn,isbn_10,isbn_13,language,editions,editions.title,editions.key,editions.language,editions.cover_i,editions.isbn_13,editions.isbn_10,editions.publish_date,editions.number_of_pages,availability,ebook_access,ia,public_scan_b,has_fulltext";
const EDITIONS_LOOKUP_LIMIT: u32 = 20;
const FACET_REGISTRY_JSON: &str = include_str!("../../../../src/data/discoverFacets.json");

static FACET_REGISTRY: OnceLock<Result<DiscoverFacetRegistry, String>> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct DiscoverFacetRegistry {
    subjects: Vec<SubjectFacetEntry>,
    #[serde(default)]
    languages: Vec<LanguageFacetEntry>,
}

#[derive(Debug, Deserialize)]
struct SubjectFacetEntry {
    slug: String,
    #[serde(default)]
    aliases: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct LanguageFacetEntry {
    slug: String,
    iso_code: String,
    #[serde(default)]
    aliases: Vec<String>,
}

thread_local! {
    static LOCALIZED_EDITION_CACHE: RefCell<HashMap<(String, String), Option<Value>>> =
        RefCell::new(HashMap::new());
}

struct OpenLibraryDiscoverPlugin;

export!(OpenLibraryDiscoverPlugin);

impl Guest for OpenLibraryDiscoverPlugin {
    fn list_catalogs() -> Result<Vec<DiscoverCatalog>, PluginError> {
        let subject_slugs = available_subject_slugs()?;

        Ok(vec![
            DiscoverCatalog {
                id: "openlibrary:trending:daily".to_string(),
                name: "Livros em Alta Hoje".to_string(),
                content_type: "trending".to_string(),
                genres: Vec::new(),
                supported_filters: vec!["year".to_string(), "language".to_string()],
            },
            DiscoverCatalog {
                id: "openlibrary:trending:weekly".to_string(),
                name: "Livros em Alta da Semana".to_string(),
                content_type: "trending".to_string(),
                genres: Vec::new(),
                supported_filters: vec!["year".to_string(), "language".to_string()],
            },
            DiscoverCatalog {
                id: "openlibrary:free".to_string(),
                name: "Livros Gratuitos".to_string(),
                content_type: "free".to_string(),
                genres: Vec::new(),
                supported_filters: vec!["year".to_string(), "language".to_string()],
            },
            DiscoverCatalog {
                id: "openlibrary:subjects".to_string(),
                name: "Filtrar Livros".to_string(),
                content_type: "subject".to_string(),
                genres: subject_slugs,
                supported_filters: vec![
                    "genre".to_string(),
                    "year".to_string(),
                    "format".to_string(),
                    "audience".to_string(),
                    "language".to_string(),
                ],
            },
        ])
    }

    fn list_catalog_items(request: DiscoverCatalogQuery) -> Result<DiscoverCatalogPage, PluginError> {
        let DiscoverCatalogQuery {
            catalog_id,
            skip,
            page_size,
            genre,
            year,
            search_query,
            language,
        } = request;

        let page_size = page_size.clamp(1, 100);

        let normalized_search_query = search_query
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());

        let language_code = language
            .as_deref()
            .and_then(normalize_language_code);
        let language_code_ref = language_code.as_deref();

        LOCALIZED_EDITION_CACHE.with(|cache| cache.borrow_mut().clear());

        if catalog_id == "openlibrary:free" {
            return list_public_catalog(
                normalized_search_query,
                skip,
                page_size,
                year,
                language_code_ref,
            );
        }

        if let Some(search_query) = normalized_search_query {
            return list_search(search_query, skip, page_size, year, language_code_ref);
        }

        match catalog_id.as_str() {
            "openlibrary:trending:daily" => {
                list_trending("daily", skip, page_size, year, language_code_ref)
            }
            "openlibrary:trending:weekly" => {
                list_trending("weekly", skip, page_size, year, language_code_ref)
            }
            "openlibrary:subjects" => {
                list_subjects(genre, skip, page_size, year, language_code_ref)
            }
            _ => Err(PluginError::NotFound(format!(
                "catalog '{}' not found",
                catalog_id
            ))),
        }
    }

    fn get_item_details(item_id: String) -> Result<DiscoverItemDetails, PluginError> {
        let work_id = parse_work_id(&item_id).ok_or_else(|| {
            PluginError::NotFound(format!("invalid discover item id '{}'", item_id))
        })?;

        let work_url = format!("{}/works/{}.json", OPEN_LIBRARY_BASE_URL, work_id);
        let work_payload = get_json(&work_url, Vec::new())?;

        let pinned_edition_id = parse_preferred_edition_id(&item_id);

        let pinned_edition_payload = pinned_edition_id.as_ref().and_then(|edition_id| {
            get_json(
                &format!("{}/books/{}.json", OPEN_LIBRARY_BASE_URL, edition_id),
                Vec::new(),
            )
            .ok()
        });

        let preferred_edition_entry = if pinned_edition_payload.is_some() {
            None
        } else {
            resolve_preferred_edition_entry(&work_id)?
        };

        let fallback_edition_payload = preferred_edition_entry
            .as_ref()
            .and_then(resolve_edition_id)
            .and_then(|edition_id| {
                get_json(
                    &format!("{}/books/{}.json", OPEN_LIBRARY_BASE_URL, edition_id),
                    Vec::new(),
                )
                .ok()
            });

        let prefer_edition = pinned_edition_id.is_some();
        let preferred_edition_payload = pinned_edition_payload.or(fallback_edition_payload);

        let title = if prefer_edition {
            preferred_edition_payload
                .as_ref()
                .and_then(resolve_title_from_record)
                .or_else(|| string_field(&work_payload, "title"))
        } else {
            string_field(&work_payload, "title").or_else(|| {
                preferred_edition_payload
                    .as_ref()
                    .and_then(resolve_title_from_record)
            })
        }
        .or_else(|| {
            preferred_edition_entry
                .as_ref()
                .and_then(resolve_title_from_record)
        })
        .unwrap_or_else(|| "Untitled".to_string());

        let description = parse_description(work_payload.get("description"))
            .or_else(|| {
                preferred_edition_payload
                    .as_ref()
                    .and_then(|record| parse_description(record.get("description")))
            })
            .or_else(|| {
                preferred_edition_entry
                    .as_ref()
                    .and_then(|record| parse_description(record.get("description")))
            });

        let cover_url = if prefer_edition {
            preferred_edition_payload
                .as_ref()
                .and_then(resolve_cover_id)
                .or_else(|| resolve_cover_id(&work_payload))
        } else {
            preferred_edition_payload
                .as_ref()
                .and_then(resolve_cover_id)
                .or_else(|| {
                    preferred_edition_entry
                        .as_ref()
                        .and_then(resolve_cover_id)
                })
                .or_else(|| resolve_cover_id(&work_payload))
        }
        .map(cover_url)
        .unwrap_or_default();

        let mut genres = extract_subjects(&work_payload, 8);
        if genres.is_empty() {
            genres = preferred_edition_payload
                .as_ref()
                .map(|record| extract_subjects(record, 8))
                .unwrap_or_default();
        }

        let pinned_year = parse_pinned_year(&item_id);

        let year = if prefer_edition {
            preferred_edition_payload
                .as_ref()
                .and_then(resolve_publish_year)
                .or_else(|| resolve_publish_year(&work_payload))
        } else {
            pinned_year
                .or_else(|| resolve_publish_year(&work_payload))
                .or_else(|| {
                    preferred_edition_payload
                        .as_ref()
                        .and_then(resolve_publish_year)
                })
                .or_else(|| {
                    preferred_edition_entry
                        .as_ref()
                        .and_then(resolve_publish_year)
                })
        };

        let author = resolve_primary_author_name(&work_payload)
            .or_else(|| resolve_author_name(&work_payload))
            .or_else(|| {
                preferred_edition_payload
                    .as_ref()
                    .and_then(resolve_author_name)
            })
            .or_else(|| {
                preferred_edition_entry
                    .as_ref()
                    .and_then(resolve_author_name)
            })
            .unwrap_or_else(|| "Autor desconhecido".to_string());

        let isbn = preferred_edition_payload
            .as_ref()
            .and_then(resolve_isbn)
            .or_else(|| preferred_edition_entry.as_ref().and_then(resolve_isbn));

        let page_count = preferred_edition_payload
            .as_ref()
            .and_then(resolve_page_count)
            .or_else(|| {
                preferred_edition_entry
                    .as_ref()
                    .and_then(resolve_page_count)
            });

        let origin_url = pinned_edition_id
            .clone()
            .map(|edition_id| format!("{}/books/{}", OPEN_LIBRARY_BASE_URL, edition_id))
            .or_else(|| {
                preferred_edition_entry
                    .as_ref()
                    .and_then(resolve_edition_id)
                    .map(|edition_id| format!("{}/books/{}", OPEN_LIBRARY_BASE_URL, edition_id))
            })
            .or_else(|| Some(format!("{}/works/{}", OPEN_LIBRARY_BASE_URL, work_id)));

        Ok(DiscoverItemDetails {
            id: item_id,
            title,
            author,
            description,
            cover_url,
            genres,
            year,
            page_count,
            format: None,
            isbn,
            origin_url,
        })
    }
}

fn list_trending(
    period: &str,
    skip: u32,
    page_size: u32,
    year_filter: Option<u32>,
    language_filter: Option<&str>,
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
        .filter_map(|raw| {
            let parsed = parse_item(raw);
            apply_language_filter(raw, parsed, language_filter)
        })
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
    language_filter: Option<&str>,
) -> Result<DiscoverCatalogPage, PluginError> {
    let subject = normalize_subject_slug(genre_filter.as_deref().unwrap_or(DEFAULT_SUBJECT_FALLBACK));
    let url = format!(
        "{}/subjects/{}.json",
        OPEN_LIBRARY_BASE_URL,
        subject_path_segment(&subject)
    );
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

    let raw_len = works.len();
    let mut items = works
        .iter()
        .filter_map(|raw| {
            let parsed = parse_item(raw);
            apply_language_filter(raw, parsed, language_filter)
        })
        .collect::<Vec<_>>();

    if let Some(year) = year_filter {
        items.retain(|item| item.year == Some(year));
    }

    let has_more = raw_len > page_size as usize;
    if items.len() > page_size as usize {
        items.truncate(page_size as usize);
    }

    Ok(DiscoverCatalogPage { items, has_more })
}

fn list_search(
    search_query: &str,
    skip: u32,
    page_size: u32,
    year_filter: Option<u32>,
    language_filter: Option<&str>,
) -> Result<DiscoverCatalogPage, PluginError> {
    let payload = get_json(
        &format!("{}/search.json", OPEN_LIBRARY_BASE_URL),
        vec![
            ("q".to_string(), search_query.to_string()),
            ("fields".to_string(), SEARCH_FIELDS.to_string()),
            ("limit".to_string(), page_size.to_string()),
            ("offset".to_string(), skip.to_string()),
        ],
    )?;

    let docs = payload
        .get("docs")
        .and_then(Value::as_array)
        .ok_or_else(|| PluginError::ParsingFailure("missing docs array".to_string()))?;

    let mut items = docs
        .iter()
        .filter_map(|raw| {
            let parsed = parse_item(raw);
            apply_language_filter(raw, parsed, language_filter)
        })
        .collect::<Vec<_>>();

    if let Some(year) = year_filter {
        items.retain(|item| item.year == Some(year));
    }

    let total_found = payload
        .get("numFound")
        .and_then(Value::as_u64)
        .unwrap_or(skip as u64 + docs.len() as u64);

    let has_more = (skip as u64 + docs.len() as u64) < total_found;

    Ok(DiscoverCatalogPage { items, has_more })
}

fn list_public_catalog(
    search_query: Option<&str>,
    skip: u32,
    page_size: u32,
    year_filter: Option<u32>,
    language_filter: Option<&str>,
) -> Result<DiscoverCatalogPage, PluginError> {
    let effective_query = search_query.unwrap_or("ebook_access:public");

    let payload = get_json(
        &format!("{}/search.json", OPEN_LIBRARY_BASE_URL),
        vec![
            ("q".to_string(), effective_query.to_string()),
            ("ebook_access".to_string(), "public".to_string()),
            ("fields".to_string(), SEARCH_FIELDS.to_string()),
            ("limit".to_string(), page_size.to_string()),
            ("offset".to_string(), skip.to_string()),
        ],
    )?;

    let docs = payload
        .get("docs")
        .and_then(Value::as_array)
        .ok_or_else(|| PluginError::ParsingFailure("missing docs array".to_string()))?;

    let mut items = docs
        .iter()
        .filter(|doc| is_public_ebook(doc))
        .filter_map(|raw| {
            let parsed = parse_item(raw);
            apply_language_filter(raw, parsed, language_filter)
        })
        .collect::<Vec<_>>();

    if let Some(year) = year_filter {
        items.retain(|item| item.year == Some(year));
    }

    let total_found = payload
        .get("numFound")
        .and_then(Value::as_u64)
        .unwrap_or(skip as u64 + docs.len() as u64);

    let has_more = (skip as u64 + docs.len() as u64) < total_found;

    Ok(DiscoverCatalogPage { items, has_more })
}

fn is_public_ebook(value: &Value) -> bool {
    let direct_access = string_field(value, "ebook_access")
        .map(|entry| entry.eq_ignore_ascii_case("public"))
        .unwrap_or(false);

    if direct_access {
        return true;
    }

    value
        .pointer("/editions/docs")
        .and_then(Value::as_array)
        .map(|editions| {
            editions.iter().any(|edition| {
                string_field(edition, "ebook_access")
                    .map(|entry| entry.eq_ignore_ascii_case("public"))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
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

    let preferred_inline_edition = value
        .pointer("/editions/docs")
        .and_then(Value::as_array)
        .and_then(|docs| choose_preferred_edition(docs));

    let title = preferred_inline_edition
        .and_then(resolve_title_from_record)
        .or_else(|| string_field(value, "title"))
        .unwrap_or_else(|| "Untitled".to_string());

    let author = resolve_author_name(value)
        .or_else(|| preferred_inline_edition.and_then(resolve_author_name))
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

    let cover_url = preferred_inline_edition
        .and_then(resolve_cover_id)
        .or_else(|| resolve_cover_id(value))
        .map(cover_url)
        .unwrap_or_default();

    let genres = extract_subjects(value, 5);

    let year = preferred_inline_edition
        .and_then(resolve_publish_year)
        .or_else(|| resolve_publish_year(value));

    let short_description = string_field(value, "subtitle")
        .or_else(|| preferred_inline_edition.and_then(|entry| string_field(entry, "subtitle")));

    let isbn = preferred_inline_edition
        .and_then(resolve_isbn)
        .or_else(|| {
            value
                .pointer("/editions/docs")
                .and_then(Value::as_array)
                .and_then(|docs| docs.first())
                .and_then(resolve_isbn)
        })
        .or_else(|| resolve_isbn(value));

    let page_count = preferred_inline_edition
        .and_then(resolve_page_count)
        .or_else(|| resolve_page_count(value));

    let id = match preferred_inline_edition.and_then(resolve_edition_id) {
        Some(edition_id) => format!("openlibrary:work:{}:edition:{}", work_id, edition_id),
        None => match year {
            Some(y) => format!("openlibrary:work:{}:year:{}", work_id, y),
            None => format!("openlibrary:work:{}", work_id),
        },
    };

    DiscoverItem {
        id,
        title,
        author,
        cover_url,
        genres,
        year,
        page_count,
        short_description,
        format: None,
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

fn resolve_preferred_edition_entry(work_id: &str) -> Result<Option<Value>, PluginError> {
    let url = format!("{}/works/{}/editions.json", OPEN_LIBRARY_BASE_URL, work_id);
    let payload = get_json(
        &url,
        vec![
            ("limit".to_string(), EDITIONS_PREFETCH_LIMIT.to_string()),
            ("offset".to_string(), "0".to_string()),
        ],
    )?;

    let entries = payload
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    Ok(choose_preferred_edition(&entries).cloned())
}

fn choose_preferred_edition(entries: &[Value]) -> Option<&Value> {
    entries.iter().min_by_key(|entry| {
        let has_title = resolve_title_from_record(entry).is_some();
        let has_page_count = resolve_page_count(entry).is_some();
        let has_isbn = resolve_isbn(entry).is_some();

        (u8::from(!has_title), u8::from(!has_page_count), u8::from(!has_isbn))
    })
}

fn resolve_title_from_record(value: &Value) -> Option<String> {
    string_field(value, "title").or_else(|| string_field(value, "full_title"))
}

fn resolve_author_name(value: &Value) -> Option<String> {
    value
        .get("author_name")
        .and_then(Value::as_array)
        .and_then(|authors| authors.first())
        .and_then(Value::as_str)
        .and_then(non_empty)
        .or_else(|| {
            value
                .get("authors")
                .and_then(Value::as_array)
                .and_then(|authors| authors.first())
                .and_then(|author| author.get("name"))
                .and_then(Value::as_str)
                .and_then(non_empty)
        })
}

fn resolve_cover_id(value: &Value) -> Option<i64> {
    value
        .get("cover_i")
        .and_then(Value::as_i64)
        .filter(|cover_id| *cover_id > 0)
        .or_else(|| {
            value
                .get("cover_id")
                .and_then(Value::as_i64)
                .filter(|cover_id| *cover_id > 0)
        })
        .or_else(|| {
            value
                .get("covers")
                .and_then(Value::as_array)
                .and_then(|covers| {
                    covers
                        .iter()
                        .filter_map(Value::as_i64)
                        .find(|cover_id| *cover_id > 0)
                })
        })
}

fn extract_subjects(value: &Value, limit: usize) -> Vec<String> {
    value
        .get("subject")
        .and_then(Value::as_array)
        .or_else(|| value.get("subjects").and_then(Value::as_array))
        .map(|subjects| {
            subjects
                .iter()
                .filter_map(Value::as_str)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .take(limit)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn resolve_publish_year(value: &Value) -> Option<u32> {
    value
        .get("first_publish_year")
        .and_then(Value::as_u64)
        .and_then(|year| u32::try_from(year).ok())
        .or_else(|| string_field(value, "first_publish_date").and_then(|date| parse_year_from_text(&date)))
        .or_else(|| string_field(value, "publish_date").and_then(|date| parse_year_from_text(&date)))
}

fn parse_year_from_text(value: &str) -> Option<u32> {
    let bytes = value.as_bytes();
    if bytes.len() < 4 {
        return None;
    }

    for window in bytes.windows(4) {
        if window.iter().all(u8::is_ascii_digit) {
            let year = std::str::from_utf8(window).ok()?.parse::<u32>().ok()?;
            if (1000..=2100).contains(&year) {
                return Some(year);
            }
        }
    }

    None
}

fn resolve_isbn(value: &Value) -> Option<String> {
    first_string_from_array(value, "isbn_13")
        .or_else(|| first_string_from_array(value, "isbn_10"))
        .or_else(|| first_string_from_array(value, "isbn"))
        .or_else(|| {
            value
                .pointer("/availability/isbn")
                .and_then(Value::as_str)
                .and_then(non_empty)
        })
}

fn first_string_from_array(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .and_then(|values| values.first())
        .and_then(Value::as_str)
        .and_then(non_empty)
}

fn resolve_page_count(value: &Value) -> Option<u32> {
    value
        .get("number_of_pages")
        .and_then(Value::as_u64)
        .and_then(|count| u32::try_from(count).ok())
        .or_else(|| {
            value
                .get("number_of_pages")
                .and_then(Value::as_str)
                .and_then(parse_number_from_text)
        })
        .or_else(|| {
            value
                .get("number_of_pages_median")
                .and_then(Value::as_u64)
                .and_then(|count| u32::try_from(count).ok())
        })
        .or_else(|| string_field(value, "pagination").and_then(|value| parse_number_from_text(&value)))
}

fn parse_number_from_text(value: &str) -> Option<u32> {
    let mut current = String::new();
    let mut best: Option<u32> = None;

    for ch in value.chars() {
        if ch.is_ascii_digit() {
            current.push(ch);
            continue;
        }

        if !current.is_empty() {
            if let Ok(parsed) = current.parse::<u32>() {
                best = Some(best.map_or(parsed, |previous| previous.max(parsed)));
            }
            current.clear();
        }
    }

    if !current.is_empty() {
        if let Ok(parsed) = current.parse::<u32>() {
            best = Some(best.map_or(parsed, |previous| previous.max(parsed)));
        }
    }

    best.filter(|count| *count > 0)
}

fn resolve_edition_id(value: &Value) -> Option<String> {
    value
        .get("key")
        .and_then(Value::as_str)
        .and_then(parse_edition_id)
}

fn parse_edition_id(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.starts_with("OL") && trimmed.ends_with('M') {
        return Some(trimmed.to_string());
    }

    let suffix = trimmed.rsplit('/').next().unwrap_or(trimmed).trim();
    if suffix.starts_with("OL") && suffix.ends_with('M') {
        return Some(suffix.to_string());
    }

    None
}

fn normalize_subject_slug(raw: &str) -> String {
    let normalized = normalize_subject_token(raw);

    if let Ok(registry) = facet_registry() {
        for subject in &registry.subjects {
            let canonical = normalize_subject_token(&subject.slug);
            if canonical == normalized {
                return subject.slug.clone();
            }

            if subject
                .aliases
                .iter()
                .any(|alias| normalize_subject_token(alias) == normalized)
            {
                return subject.slug.clone();
            }
        }

        if normalized.starts_with("place_") {
            let candidate = normalized.replacen("place_", "place:", 1);
            if registry
                .subjects
                .iter()
                .any(|subject| normalize_subject_token(&subject.slug) == candidate)
            {
                return candidate;
            }
        }
    }

    DEFAULT_SUBJECT_FALLBACK.to_string()
}

fn subject_path_segment(subject: &str) -> String {
    subject
        .trim()
        .replace(' ', "_")
        .replace(':', "%3A")
}

fn normalize_subject_token(raw: &str) -> String {
    let decoded = raw.trim().replace("%3A", ":").replace("%3a", ":");
    fold_portuguese_diacritics(&decoded)
        .to_ascii_lowercase()
        .replace(' ', "_")
        .replace('-', "_")
        .replace("__", "_")
}

fn fold_portuguese_diacritics(input: &str) -> String {
    input
        .chars()
        .map(|ch| match ch {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            _ => ch,
        })
        .collect()
}

fn normalize_language_code(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let lower = trimmed.to_ascii_lowercase();

    let registry = match facet_registry() {
        Ok(registry) => registry,
        Err(_) => return None,
    };

    if registry
        .languages
        .iter()
        .any(|entry| entry.iso_code.eq_ignore_ascii_case(&lower))
    {
        return Some(lower);
    }

    let token = normalize_subject_token(&lower);
    for entry in &registry.languages {
        if normalize_subject_token(&entry.slug) == token
            || entry
                .aliases
                .iter()
                .any(|alias| normalize_subject_token(alias) == token)
        {
            return Some(entry.iso_code.to_ascii_lowercase());
        }
    }

    None
}

fn apply_language_filter(
    raw_work: &Value,
    item: DiscoverItem,
    language_code: Option<&str>,
) -> Option<DiscoverItem> {
    let Some(code) = language_code else {
        return Some(item);
    };

    if !work_supports_language(raw_work, code) {
        if let Some(language_array) = raw_work.get("language").and_then(Value::as_array) {
            if !language_array.is_empty() {
                return None;
            }
        }
    }

    if let Some(edition) = find_inline_localized_edition(raw_work, code) {
        return Some(merge_edition_into_item(item, edition));
    }

    let work_id = parse_work_id_from_value(raw_work)?;
    let edition = fetch_localized_edition(&work_id, code)?;
    Some(merge_edition_into_item(item, &edition))
}

fn work_supports_language(raw_work: &Value, code: &str) -> bool {
    raw_work
        .get("language")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .any(|entry| entry.eq_ignore_ascii_case(code))
        })
        .unwrap_or(false)
}

fn edition_has_language(edition: &Value, code: &str) -> bool {
    if let Some(structured) = edition.get("languages").and_then(Value::as_array) {
        let matched = structured.iter().any(|entry| match entry {
            Value::Object(_) => entry
                .get("key")
                .and_then(Value::as_str)
                .map(|key| {
                    key.trim()
                        .strip_prefix("/languages/")
                        .map(|value| value.eq_ignore_ascii_case(code))
                        .unwrap_or(false)
                })
                .unwrap_or(false),
            Value::String(value) => value.eq_ignore_ascii_case(code),
            _ => false,
        });
        if matched {
            return true;
        }
    }

    if let Some(flat) = edition.get("language").and_then(Value::as_array) {
        return flat
            .iter()
            .filter_map(Value::as_str)
            .any(|entry| entry.eq_ignore_ascii_case(code));
    }

    false
}

fn find_inline_localized_edition<'a>(raw_work: &'a Value, code: &str) -> Option<&'a Value> {
    raw_work
        .pointer("/editions/docs")
        .and_then(Value::as_array)
        .and_then(|docs| docs.iter().find(|edition| edition_has_language(edition, code)))
}

fn fetch_localized_edition(work_id: &str, code: &str) -> Option<Value> {
    let cache_key = (work_id.to_string(), code.to_string());
    if let Some(cached) = LOCALIZED_EDITION_CACHE
        .with(|cache| cache.borrow().get(&cache_key).cloned())
    {
        return cached;
    }

    let url = format!("{}/works/{}/editions.json", OPEN_LIBRARY_BASE_URL, work_id);
    let payload = get_json(
        &url,
        vec![
            ("limit".to_string(), EDITIONS_LOOKUP_LIMIT.to_string()),
            ("offset".to_string(), "0".to_string()),
        ],
    )
    .ok();

    let resolved = payload.and_then(|payload| {
        payload
            .get("entries")
            .and_then(Value::as_array)
            .and_then(|entries| {
                entries
                    .iter()
                    .find(|entry| edition_has_language(entry, code))
                    .cloned()
            })
    });

    LOCALIZED_EDITION_CACHE.with(|cache| {
        cache.borrow_mut().insert(cache_key, resolved.clone());
    });

    resolved
}

fn merge_edition_into_item(mut item: DiscoverItem, edition: &Value) -> DiscoverItem {
    if let Some(title) = resolve_title_from_record(edition) {
        item.title = title;
    }

    if let Some(cover_id) = resolve_cover_id(edition) {
        item.cover_url = cover_url(cover_id);
    }

    if let Some(isbn) = resolve_isbn(edition) {
        item.isbn = Some(isbn);
    }

    if let Some(year) = resolve_publish_year(edition) {
        item.year = Some(year);
    }

    if let Some(pages) = resolve_page_count(edition) {
        item.page_count = Some(pages);
    }

    if let Some(subtitle) = string_field(edition, "subtitle") {
        item.short_description = Some(subtitle);
    }

    if let Some(edition_id) = resolve_edition_id(edition) {
        if let Some(rest) = item.id.strip_prefix("openlibrary:work:") {
            let work_id = rest.split(':').next().unwrap_or(rest);
            item.id = format!("openlibrary:work:{}:edition:{}", work_id, edition_id);
        }
    }

    item
}

fn parse_work_id_from_value(value: &Value) -> Option<String> {
    let key = value.get("key").and_then(Value::as_str)?.trim();
    let stripped = key
        .strip_prefix("/works/")
        .or_else(|| key.strip_prefix("works/"))
        .unwrap_or(key)
        .trim();

    if stripped.is_empty() {
        None
    } else {
        Some(stripped.to_string())
    }
}

fn available_subject_slugs() -> Result<Vec<String>, PluginError> {
    let registry = facet_registry()?;
    Ok(registry
        .subjects
        .iter()
        .map(|subject| subject.slug.clone())
        .collect())
}

fn facet_registry() -> Result<&'static DiscoverFacetRegistry, PluginError> {
    let parsed = FACET_REGISTRY.get_or_init(|| {
        serde_json::from_str::<DiscoverFacetRegistry>(FACET_REGISTRY_JSON)
            .map_err(|err| format!("failed to parse facet registry: {}", err))
    });

    match parsed {
        Ok(registry) => Ok(registry),
        Err(err) => Err(PluginError::ParsingFailure(err.clone())),
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
    item_id
        .trim()
        .split(':')
        .map(str::trim)
        .find(|seg| seg.starts_with("OL") && seg.ends_with('W'))
        .map(str::to_string)
}

fn parse_preferred_edition_id(item_id: &str) -> Option<String> {
    item_id
        .trim()
        .split(':')
        .map(str::trim)
        .find(|seg| seg.starts_with("OL") && seg.ends_with('M'))
        .map(str::to_string)
}

fn parse_pinned_year(item_id: &str) -> Option<u32> {
    let segments: Vec<&str> = item_id.trim().split(':').map(str::trim).collect();
    for window in segments.windows(2) {
        if window[0] == "year" {
            if let Ok(value) = window[1].parse::<u32>() {
                return Some(value);
            }
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
