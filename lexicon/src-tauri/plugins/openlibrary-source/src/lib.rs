wit_bindgen::generate!({
    path: "../../wit/discover-source-plugin.wit",
    world: "source-plugin",
});

use std::collections::HashSet;

use serde_json::Value;

use crate::lexicon::plugin_roles::common_types::{HttpHeader, HttpRequest};
use crate::lexicon::plugin_roles::host_http;

const SOURCE_NAME: &str = "Open Library";
const SOURCE_ID: &str = "openlibrary";
const OPEN_LIBRARY_BASE_URL: &str = "https://openlibrary.org";
const ARCHIVE_METADATA_BASE_URL: &str = "https://archive.org/metadata";
const REQUEST_TIMEOUT_MS: u64 = 15_000;
const DOWNLOAD_PROBE_TIMEOUT_MS: u64 = 8_000;
const MAX_RESULTS: usize = 20;
const MAX_DOCS_TO_SCAN: usize = 20;
const MAX_IA_IDS_PER_DOC: usize = 4;
const MAX_ARCHIVE_FILES_TO_SCAN: usize = 256;
const SEARCH_FIELDS: &str =
    "key,title,author_name,language,ia,public_scan_b,ebook_access,editions.ebook_access,has_fulltext,availability";

#[derive(Clone)]
struct ArchiveDownloadCandidate {
    download_url: String,
    format: String,
    size: Option<String>,
}

struct RankedArchiveDownloadCandidate {
    candidate: ArchiveDownloadCandidate,
    rank: u8,
}

struct OpenLibrarySourcePlugin;

export!(OpenLibrarySourcePlugin);

impl Guest for OpenLibrarySourcePlugin {
    fn get_source_info() -> SourceInfo {
        SourceInfo {
            source_name: SOURCE_NAME.to_string(),
            source_id: SOURCE_ID.to_string(),
            supported_formats: vec!["pdf".to_string(), "epub".to_string(), "mobi".to_string()],
        }
    }

    fn find_downloads(request: SourceSearchQuery) -> Result<Vec<SourceDownloadResult>, PluginError> {
        let query = build_search_query(&request);
        if query.is_empty() {
            return Err(PluginError::NotFound(
                "title or isbn is required".to_string(),
            ));
        }

        let payload = get_json(
            &format!("{}/search.json", OPEN_LIBRARY_BASE_URL),
            vec![
                ("q".to_string(), query),
                ("ebook_access".to_string(), "public".to_string()),
                ("fields".to_string(), SEARCH_FIELDS.to_string()),
                ("limit".to_string(), MAX_RESULTS.to_string()),
            ],
        )?;

        let docs = match payload.get("docs").and_then(Value::as_array) {
            Some(entries) => entries,
            None => {
                eprintln!("[openlibrary-source] search payload missing docs array");
                return Err(PluginError::NotFound(
                    "no public Open Library downloads found".to_string(),
                ));
            }
        };

        let mut results = Vec::new();
        let mut seen_urls = HashSet::new();

        for doc in docs.iter().take(MAX_DOCS_TO_SCAN) {
            if results.len() >= MAX_RESULTS {
                break;
            }

            if !doc.is_object() {
                continue;
            }

            let ia_ids = extract_ia_ids(doc);
            if ia_ids.is_empty() || !is_public_ebook(doc) {
                continue;
            }

            let language = extract_language_hint(doc);
            let title = string_field(doc, "title").unwrap_or_else(|| request.title.trim().to_string());

            for ia_id in ia_ids.into_iter().take(MAX_IA_IDS_PER_DOC) {
                if results.len() >= MAX_RESULTS {
                    break;
                }

                let candidate = resolve_archive_download_candidate(&ia_id).unwrap_or_else(|| {
                    eprintln!(
                        "[openlibrary-source] metadata lookup failed for ia '{}' ; using legacy PDF URL",
                        ia_id
                    );

                    ArchiveDownloadCandidate {
                        download_url: archive_legacy_download_url(&ia_id, "pdf"),
                        format: "pdf".to_string(),
                        size: None,
                    }
                });

                if !is_download_url_accessible(&candidate.download_url) {
                    continue;
                }

                if !seen_urls.insert(candidate.download_url.clone()) {
                    continue;
                }

                results.push(SourceDownloadResult {
                    download_url: candidate.download_url,
                    format: candidate.format,
                    size: candidate.size,
                    language: language.clone(),
                    quality: build_quality_metadata(&title),
                });
            }
        }

        if results.is_empty() {
            return Err(PluginError::NotFound(
                "no public Open Library downloads found".to_string(),
            ));
        }

        Ok(results)
    }
}

fn build_search_query(request: &SourceSearchQuery) -> String {
    if let Some(isbn) = request
        .isbn
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return format!("isbn:{}", normalize_isbn(isbn));
    }

    let title = request.title.trim();
    let author = request
        .author
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    match (title.is_empty(), author) {
        (true, Some(author)) => author.to_string(),
        (true, None) => String::new(),
        (false, Some(author)) => format!("{} {}", title, author),
        (false, None) => title.to_string(),
    }
}

fn normalize_isbn(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}

fn is_public_ebook(doc: &Value) -> bool {
    let direct_access = string_field(doc, "ebook_access")
        .map(|value| value.eq_ignore_ascii_case("public"))
        .unwrap_or(false);

    if direct_access {
        return true;
    }

    let editions_access = doc
        .pointer("/editions/docs")
        .and_then(Value::as_array)
        .map(|entries| {
            entries.iter().any(|entry| {
                string_field(entry, "ebook_access")
                    .map(|value| value.eq_ignore_ascii_case("public"))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);

    if editions_access {
        return true;
    }

    let public_scan = doc
        .get("public_scan_b")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if public_scan {
        return true;
    }

    let availability_open = doc
        .pointer("/availability/status")
        .and_then(Value::as_str)
        .map(|status| status.eq_ignore_ascii_case("open"))
        .unwrap_or(false);

    if availability_open {
        return true;
    }

    doc.pointer("/availability/is_readable")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && !doc
            .pointer("/availability/is_lendable")
            .and_then(Value::as_bool)
            .unwrap_or(false)
}

fn extract_ia_ids(doc: &Value) -> Vec<String> {
    let mut ids = Vec::new();

    if let Some(array) = doc.get("ia").and_then(Value::as_array) {
        ids.extend(
            array
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned),
        );
    }

    if let Some(single) = doc.get("ia").and_then(Value::as_str) {
        let normalized = single.trim();
        if !normalized.is_empty() {
            ids.push(normalized.to_string());
        }
    }

    if let Some(identifier) = doc
        .pointer("/availability/identifier")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        ids.push(identifier.to_string());
    }

    ids.sort();
    ids.dedup();
    ids
}

fn extract_language_hint(doc: &Value) -> Option<String> {
    doc.get("language")
        .and_then(Value::as_array)
        .and_then(|entries| entries.first())
        .and_then(Value::as_str)
        .and_then(|value| {
            let normalized = value.trim().to_ascii_lowercase();
            if normalized.is_empty() {
                return None;
            }

            if normalized == "por" || normalized.starts_with("pt") {
                return Some("pt".to_string());
            }

            if normalized == "eng" || normalized.starts_with("en") {
                return Some("en".to_string());
            }

            Some(normalized.chars().take(2).collect())
        })
}

fn resolve_archive_download_candidate(ia_id: &str) -> Option<ArchiveDownloadCandidate> {
    let payload = get_json(
        &format!("{}/{}", ARCHIVE_METADATA_BASE_URL, ia_id),
        Vec::new(),
    )
    .ok()?;

    let files = payload.get("files")?.as_array()?;
    let mut best: Option<RankedArchiveDownloadCandidate> = None;

    for file in files.iter().take(MAX_ARCHIVE_FILES_TO_SCAN) {
        let Some(candidate) = parse_archive_file_candidate(ia_id, file) else {
            continue;
        };

        match &best {
            Some(current) if current.rank <= candidate.rank => {}
            _ => best = Some(candidate),
        }

        if best.as_ref().is_some_and(|entry| entry.rank == 0) {
            break;
        }
    }

    if files.len() > MAX_ARCHIVE_FILES_TO_SCAN {
        eprintln!(
            "[openlibrary-source] ia '{}' has {} metadata files; truncated to {}",
            ia_id,
            files.len(),
            MAX_ARCHIVE_FILES_TO_SCAN
        );
    }

    best.map(|entry| entry.candidate)
}

fn parse_archive_file_candidate(ia_id: &str, file: &Value) -> Option<RankedArchiveDownloadCandidate> {
    let file_name = file
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?;

    if file_name.contains('/') {
        return None;
    }

    let format_hint = file.get("format").and_then(Value::as_str).unwrap_or_default();

    if is_encrypted_archive_asset(file_name, format_hint) {
        return None;
    }

    let (normalized_format, rank) = infer_archive_file_format(file_name, format_hint)?;

    Some(RankedArchiveDownloadCandidate {
        candidate: ArchiveDownloadCandidate {
            download_url: archive_file_download_url(ia_id, file_name),
            format: normalized_format,
            size: parse_archive_file_size(file),
        },
        rank,
    })
}

fn is_encrypted_archive_asset(file_name: &str, format_hint: &str) -> bool {
    let lower_name = file_name.trim().to_ascii_lowercase();
    let lower_format_hint = format_hint.trim().to_ascii_lowercase();

    if lower_name.contains(".lcp")
        || lower_name.contains("_lcp")
        || lower_name.contains("encrypted")
        || lower_name.ends_with(".acsm")
    {
        return true;
    }

    has_format_token(&lower_format_hint, "lcp")
        || has_format_token(&lower_format_hint, "encrypted")
        || has_format_token(&lower_format_hint, "drm")
}

fn infer_archive_file_format(file_name: &str, format_hint: &str) -> Option<(String, u8)> {
    let lower_name = file_name.trim().to_ascii_lowercase();

    if lower_name.ends_with(".pdf") {
        return Some(("pdf".to_string(), 0));
    }

    if lower_name.ends_with(".epub") {
        return Some(("epub".to_string(), 1));
    }

    if lower_name.ends_with(".mobi") {
        return Some(("mobi".to_string(), 2));
    }

    if lower_name.ends_with(".azw3") {
        return Some(("azw3".to_string(), 3));
    }

    let lower_format_hint = format_hint.trim().to_ascii_lowercase();

    if has_format_token(&lower_format_hint, "pdf") {
        return Some(("pdf".to_string(), 0));
    }

    if has_format_token(&lower_format_hint, "epub") {
        return Some(("epub".to_string(), 1));
    }

    if has_format_token(&lower_format_hint, "kindle")
        || has_format_token(&lower_format_hint, "mobi")
    {
        return Some(("mobi".to_string(), 2));
    }

    None
}

fn has_format_token(format_hint: &str, token: &str) -> bool {
    format_hint
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|chunk| chunk == token)
}

fn parse_archive_file_size(file: &Value) -> Option<String> {
    let bytes = file
        .get("size")
        .and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_str().and_then(|entry| entry.parse::<u64>().ok()))
        })
        .filter(|value| *value > 0)?;

    Some(format_bytes(bytes))
}

fn format_bytes(bytes: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit_index = 0usize;

    while value >= 1024.0 && unit_index < units.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, units[unit_index])
    } else {
        format!("{:.2} {}", value, units[unit_index])
    }
}

fn archive_file_download_url(ia_id: &str, file_name: &str) -> String {
    format!(
        "https://archive.org/download/{}/{}",
        ia_id,
        encode_archive_file_name(file_name)
    )
}

fn archive_legacy_download_url(ia_id: &str, extension: &str) -> String {
    format!("https://archive.org/download/{0}/{0}.{1}", ia_id, extension)
}

fn is_download_url_accessible(url: &str) -> bool {
    let response = host_http::send_http_request(&HttpRequest {
        method: "HEAD".to_string(),
        url: url.to_string(),
        query: Vec::new(),
        headers: vec![HttpHeader {
            key: "accept".to_string(),
            value: "*/*".to_string(),
        }],
        body: None,
        timeout_ms: Some(DOWNLOAD_PROBE_TIMEOUT_MS),
    });

    match response {
        Ok(response) => (200..300).contains(&response.status),
        Err(_) => false,
    }
}

fn encode_archive_file_name(raw: &str) -> String {
    let mut encoded = String::with_capacity(raw.len());

    for byte in raw.bytes() {
        let is_unreserved = byte.is_ascii_alphanumeric()
            || matches!(byte, b'-' | b'_' | b'.' | b'~');

        if is_unreserved {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{:02X}", byte));
        }
    }

    encoded
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
    .map_err(|err| {
        eprintln!("[openlibrary-source] GET {} failed: {}", url, err);
        PluginError::NetworkFailure(format!("request failed: {}", err))
    })?;

    if response.status == 429 {
        eprintln!("[openlibrary-source] GET {} returned 429", url);
        return Err(PluginError::RateLimit(
            "open library rate limit reached".to_string(),
        ));
    }

    if response.status == 404 {
        eprintln!("[openlibrary-source] GET {} returned 404", url);
        return Err(PluginError::NotFound("resource not found".to_string()));
    }

    if !(200..300).contains(&response.status) {
        eprintln!(
            "[openlibrary-source] GET {} returned status {}",
            url, response.status
        );
        return Err(PluginError::NetworkFailure(format!(
            "upstream returned status {}",
            response.status
        )));
    }

    serde_json::from_str::<Value>(&response.body).map_err(|err| {
        eprintln!("[openlibrary-source] invalid JSON payload from {}: {}", url, err);
        PluginError::ParsingFailure(format!("invalid json payload: {}", err))
    })
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(ToOwned::to_owned)
}

fn build_quality_metadata(title: &str) -> Option<String> {
    let normalized_title = sanitize_quality_value(title);
    if normalized_title.is_empty() {
        return None;
    }

    Some(format!("name:{}", normalized_title))
}

fn sanitize_quality_value(raw: &str) -> String {
    raw.replace('|', " ")
        .replace('\n', " ")
        .replace('\r', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
