wit_bindgen::generate!({
    path: "../../wit/discover-source-plugin.wit",
    world: "source-plugin",
});

use std::collections::HashSet;
use std::thread;
use std::time::Duration;

use scraper::{ElementRef, Html, Selector};

use crate::lexicon::plugin_roles::common_types::{HttpHeader, HttpRequest};
use crate::lexicon::plugin_roles::host_http;

const SOURCE_NAME: &str = "LibGen";
const SOURCE_ID: &str = "libgen";
const LIBGEN_BASE_URL: &str = "https://libgen.li";
const REQUEST_TIMEOUT_MS: u64 = 20_000;
const MAX_RESULTS: usize = 40;
const MAX_ROWS_TO_SCAN: usize = 220;
const MAX_EXPENSIVE_RESOLUTIONS: usize = 5;
const QUERY_ATTEMPT_DELAY_MS: u64 = 450;

#[derive(Debug, Clone)]
struct SearchAttempt {
    query: String,
    query_terms: Vec<String>,
}

struct LibgenSourcePlugin;

export!(LibgenSourcePlugin);

impl Guest for LibgenSourcePlugin {
    fn get_source_info() -> SourceInfo {
        SourceInfo {
            source_name: SOURCE_NAME.to_string(),
            source_id: SOURCE_ID.to_string(),
            supported_formats: vec![
                "epub".to_string(),
                "pdf".to_string(),
                "mobi".to_string(),
                "azw3".to_string(),
            ],
        }
    }

    fn find_downloads(request: SourceSearchQuery) -> Result<Vec<SourceDownloadResult>, PluginError> {
        let attempts = build_search_attempts(&request);
        if attempts.is_empty() {
            return Err(PluginError::NotFound(
                "title or isbn is required".to_string(),
            ));
        }

        let mut aggregated = Vec::new();
        let mut seen_downloads = HashSet::new();

        for (index, attempt) in attempts.iter().enumerate() {
            let attempt_results = execute_search_query(&attempt.query, &attempt.query_terms)?;

            for result in attempt_results {
                let dedup_key = normalize_download_key(&result.download_url);
                if !seen_downloads.insert(dedup_key) {
                    continue;
                }

                aggregated.push(result);
                if aggregated.len() >= MAX_RESULTS {
                    break;
                }
            }

            if aggregated.len() >= MAX_RESULTS {
                break;
            }

            if !aggregated.is_empty() {
                break;
            }

            if index + 1 < attempts.len() {
                thread::sleep(Duration::from_millis(QUERY_ATTEMPT_DELAY_MS));
            }
        }

        if let Some(isbn) = request
            .isbn
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            prioritize_isbn_matches(&mut aggregated, isbn);
        }

        if aggregated.is_empty() {
            return Err(PluginError::NotFound("no direct downloads found".to_string()));
        }

        Ok(aggregated)
    }
}

fn build_search_attempts(request: &SourceSearchQuery) -> Vec<SearchAttempt> {
    let mut attempts = Vec::new();
    let mut seen = HashSet::new();

    let title = request.title.trim();

    // ISBN-first search has proven unstable for some LibGen payloads in WASM.
    // When we already have a title, prefer title/author queries and use ISBN only
    // for result prioritization later.
    if title.is_empty() {
        for isbn in extract_isbn_candidates(request.isbn.as_deref()) {
            push_search_attempt(
                &mut attempts,
                &mut seen,
                isbn,
                // ISBN rows often do not include the literal ISBN text in the title cell.
                Vec::new(),
            );
        }
    }

    let author = request
        .author
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if !title.is_empty() {
        if let Some(author) = author {
            let combined_query = format!("{} {}", title, author);
            let combined_terms = tokenize_query(&combined_query);
            push_search_attempt(&mut attempts, &mut seen, combined_query, combined_terms);
        }

        let title_terms = tokenize_query(title);
        push_search_attempt(&mut attempts, &mut seen, title.to_string(), title_terms);
    }

    attempts
}

fn push_search_attempt(
    attempts: &mut Vec<SearchAttempt>,
    seen: &mut HashSet<String>,
    query: String,
    query_terms: Vec<String>,
) {
    let normalized_query = query.trim();
    if normalized_query.is_empty() {
        return;
    }

    let dedup_key = normalized_query.to_ascii_lowercase();
    if !seen.insert(dedup_key) {
        return;
    }

    attempts.push(SearchAttempt {
        query: normalized_query.to_string(),
        query_terms,
    });
}

fn extract_isbn_candidates(raw_isbn: Option<&str>) -> Vec<String> {
    let Some(raw_isbn) = raw_isbn.map(str::trim).filter(|value| !value.is_empty()) else {
        return Vec::new();
    };

    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    for segment in raw_isbn.split(|ch| ch == ',' || ch == ';' || ch == '|') {
        push_isbn_candidate(&mut candidates, &mut seen, segment);
    }

    if candidates.is_empty() {
        push_isbn_candidate(&mut candidates, &mut seen, raw_isbn);
    }

    candidates
}

fn push_isbn_candidate(
    candidates: &mut Vec<String>,
    seen: &mut HashSet<String>,
    raw_value: &str,
) {
    let trimmed = raw_value.trim();
    if trimmed.is_empty() {
        return;
    }

    let compact = normalize_isbn(trimmed);
    if compact.len() >= 10 {
        if seen.insert(compact.clone()) {
            candidates.push(compact);
        }
    }

    let cleaned = trimmed.trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-');
    if cleaned.len() >= 10 {
        let key = normalize_isbn(cleaned);
        if !key.is_empty() && seen.insert(key) {
            candidates.push(cleaned.to_string());
        }
    }
}

fn normalize_isbn(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

fn tokenize_query(query: &str) -> Vec<String> {
    query
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(str::trim)
        .filter(|part| part.len() >= 2)
        .map(|part| part.to_ascii_lowercase())
        .collect()
}

fn normalize_download_key(url: &str) -> String {
    url.trim().to_ascii_lowercase()
}

fn prioritize_isbn_matches(results: &mut [SourceDownloadResult], isbn: &str) {
    let normalized_isbn = normalize_isbn(isbn);
    if normalized_isbn.is_empty() {
        return;
    }

    results.sort_by(|left, right| {
        let left_hit = left
            .quality
            .as_deref()
            .map(|value| normalize_isbn(value).contains(&normalized_isbn))
            .unwrap_or(false);
        let right_hit = right
            .quality
            .as_deref()
            .map(|value| normalize_isbn(value).contains(&normalized_isbn))
            .unwrap_or(false);

        right_hit.cmp(&left_hit)
    });
}

fn execute_search_query(
    query: &str,
    query_terms: &[String],
) -> Result<Vec<SourceDownloadResult>, PluginError> {
    let search_url = format!(
        "{}/index.php?req={}&curtab=f",
        LIBGEN_BASE_URL,
        url_encode(query)
    );

    let response = http_get(&search_url)?;

    if response.status == 429 {
        return Err(PluginError::RateLimit("libgen rate limit reached".to_string()));
    }

    if response.status != 200 {
        return Err(PluginError::NetworkFailure(format!(
            "libgen returned status {}",
            response.status
        )));
    }

    parse_search_results(&response.body, query_terms)
}

fn parse_search_results(
    html: &str,
    query_terms: &[String],
) -> Result<Vec<SourceDownloadResult>, PluginError> {
    let document = Html::parse_document(html);

    let row_selector = Selector::parse("table tbody tr, table tr")
        .map_err(|err| PluginError::ParsingFailure(format!("invalid row selector: {}", err)))?;

    let cell_selector = Selector::parse("td")
        .map_err(|err| PluginError::ParsingFailure(format!("invalid cell selector: {}", err)))?;

    let edition_link_selector = Selector::parse("a[href*='edition.php']")
        .map_err(|err| PluginError::ParsingFailure(format!("invalid edition selector: {}", err)))?;

    let link_selector = Selector::parse("a")
        .map_err(|err| PluginError::ParsingFailure(format!("invalid link selector: {}", err)))?;

    let mut results = Vec::new();
    let mut scanned_rows = 0usize;
    let mut expensive_resolutions = 0usize;
    let allow_expensive_resolution = !query_terms.is_empty();

    for row in document.select(&row_selector) {
        if results.len() >= MAX_RESULTS {
            break;
        }

        if scanned_rows >= MAX_ROWS_TO_SCAN {
            break;
        }
        scanned_rows += 1;

        let cells = row.select(&cell_selector).collect::<Vec<_>>();
        if cells.len() < 8 {
            continue;
        }

        let Some(title_cell) = cells.first() else {
            continue;
        };

        let Some(title_link) = title_cell.select(&edition_link_selector).next() else {
            continue;
        };

        let title = clean_text_from_node(&title_link);
        if title.is_empty() {
            continue;
        }

        let author = cells
            .get(1)
            .map(clean_text_from_node)
            .unwrap_or_default();

        let edition_href = title_link.value().attr("href").unwrap_or("").trim();
        let edition_id = extract_query_param(edition_href, "id").unwrap_or_default();

        let Some(format_cell) = cells.get(7) else {
            continue;
        };

        let format = clean_text_from_node(format_cell).to_ascii_lowercase();
        if format.is_empty() {
            continue;
        }

        // LibGen current table order: Language (4), Pages (5), Size (6), Ext (7).
        let language = cells
            .get(4)
            .map(clean_text_from_node)
            .and_then(|value| normalize_language_hint(&value));
        let page_count = cells
            .get(5)
            .map(clean_text_from_node)
            .and_then(|value| parse_page_count(&value));
        let size = cells.get(6).map(clean_text_from_node).filter(|value| !value.is_empty());
        let quality = build_quality_metadata(page_count, &title);

        if !query_terms.is_empty() {
            let mut haystack = title.to_ascii_lowercase();

            if !author.is_empty() {
                haystack.push(' ');
                haystack.push_str(&author.to_ascii_lowercase());
            }

            if let Some(quality) = quality.as_deref() {
                haystack.push(' ');
                haystack.push_str(&quality.to_ascii_lowercase());
            }

            if !query_terms.iter().any(|term| haystack.contains(term)) {
                continue;
            }
        }

        let inline_candidate = extract_inline_download_candidate(cells.get(8), &link_selector);

        let fallback_url = if edition_id.is_empty() {
            inline_candidate.clone().unwrap_or_default()
        } else {
            format!("{}/edition.php?id={}", LIBGEN_BASE_URL, edition_id)
        };

        let resolved = if !allow_expensive_resolution {
            if let Some(inline) = inline_candidate.as_deref() {
                if looks_like_get_link(inline) {
                    inline.to_string()
                } else {
                    fallback_url
                }
            } else {
                fallback_url
            }
        } else if edition_id.is_empty() {
            inline_candidate
                .as_deref()
                .and_then(resolve_download_candidate)
                .unwrap_or(fallback_url)
        } else if expensive_resolutions >= MAX_EXPENSIVE_RESOLUTIONS {
            if let Some(inline) = inline_candidate.as_deref() {
                resolve_download_candidate(inline).unwrap_or_else(|| inline.to_string())
            } else {
                fallback_url
            }
        } else {
            expensive_resolutions += 1;
            resolve_download_url(&edition_id, inline_candidate.as_deref()).unwrap_or(fallback_url)
        };

        if !is_direct_download_link(&resolved) {
            continue;
        }

        results.push(SourceDownloadResult {
            download_url: resolved,
            format,
            size,
            language,
            quality,
        });
    }

    Ok(results)
}

fn extract_inline_download_candidate(
    cell: Option<&ElementRef<'_>>,
    link_selector: &Selector,
) -> Option<String> {
    let cell = cell?;

    let mut ads_candidate: Option<String> = None;
    let mut fallback: Option<String> = None;

    for link in cell.select(link_selector) {
        let href = link.value().attr("href").unwrap_or("").trim();
        if href.is_empty() {
            continue;
        }

        let absolute = absolutize_url(href);

        if href.contains("get.php") {
            return Some(absolute);
        }

        if href.contains("ads.php?md5=") && ads_candidate.is_none() {
            ads_candidate = Some(absolute);
            continue;
        }

        if fallback.is_none() {
            fallback = Some(absolute);
        }
    }

    ads_candidate.or(fallback)
}

fn resolve_download_url(edition_id: &str, inline_candidate: Option<&str>) -> Option<String> {
    if let Some(candidate) = inline_candidate {
        if looks_like_get_link(candidate) {
            return Some(candidate.to_string());
        }

        if candidate.to_ascii_lowercase().contains("ads.php?md5=") {
            if let Some(url) = resolve_download_from_ads_page(candidate) {
                return Some(url);
            }
        }
    }

    let edition_url = format!("{}/edition.php?id={}", LIBGEN_BASE_URL, edition_id);
    let edition_response = http_get(&edition_url).ok()?;
    if edition_response.status != 200 {
        return inline_candidate.map(|value| value.to_string());
    }

    let ads_href = extract_href_containing(&edition_response.body, "ads.php?md5=")?;

    resolve_download_from_ads_page(&ads_href).or_else(|| inline_candidate.map(|value| value.to_string()))
}

fn resolve_download_from_ads_page(ads_url_or_href: &str) -> Option<String> {
    let ads_url = absolutize_url(ads_url_or_href);
    let ads_response = http_get(&ads_url).ok()?;
    if ads_response.status != 200 {
        return None;
    }

    let href = extract_get_href_from_html(&ads_response.body)?;
    Some(absolutize_url(&href))
}

fn extract_get_href_from_html(html: &str) -> Option<String> {
    if let Some(raw) = extract_href_containing(html, "get.php?md5=") {
        return Some(raw);
    }

    extract_href_containing(html, "get.php")
}

fn extract_href_containing(html: &str, needle: &str) -> Option<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("a[href]").ok()?;
    let needle_lc = needle.to_ascii_lowercase();

    for link in document.select(&selector) {
        let href = link.value().attr("href").unwrap_or("").trim();
        if href.is_empty() {
            continue;
        }

        if href.to_ascii_lowercase().contains(&needle_lc) {
            return Some(href.to_string());
        }
    }

    None
}

fn resolve_download_candidate(candidate: &str) -> Option<String> {
    if looks_like_get_link(candidate) {
        return Some(candidate.to_string());
    }

    None
}

fn is_direct_download_link(url: &str) -> bool {
    let lowered = url.to_ascii_lowercase();

    (lowered.starts_with("https://") || lowered.starts_with("http://"))
        && (lowered.contains("get.php")
            || lowered.ends_with(".epub")
            || lowered.ends_with(".pdf")
            || lowered.ends_with(".mobi")
            || lowered.ends_with(".azw3"))
}

fn looks_like_get_link(url: &str) -> bool {
    url.to_ascii_lowercase().contains("get.php")
}

fn clean_text_from_node(node: &ElementRef<'_>) -> String {
    node.text()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_language_hint(raw: &str) -> Option<String> {
    let token = raw
        .split(|ch| ch == ';' || ch == ',' || ch == '/')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())?
        .to_ascii_lowercase();

    if token == "pt" || token == "por" || token.starts_with("portugu") {
        return Some("pt".to_string());
    }

    if token == "en" || token == "eng" || token.starts_with("english") {
        return Some("en".to_string());
    }

    if token == "es" || token == "spa" || token.starts_with("spanish") {
        return Some("es".to_string());
    }

    if token == "fr" || token == "fre" || token == "fra" || token.starts_with("french") {
        return Some("fr".to_string());
    }

    if token == "de" || token == "ger" || token == "deu" || token.starts_with("german") {
        return Some("de".to_string());
    }

    if token == "it" || token == "ita" || token.starts_with("italian") {
        return Some("it".to_string());
    }

    if token == "ru" || token == "rus" || token.starts_with("russian") {
        return Some("ru".to_string());
    }

    if token == "ja" || token == "jpn" || token.starts_with("japanese") {
        return Some("ja".to_string());
    }

    if token == "zh" || token == "chi" || token == "zho" || token.starts_with("chinese") {
        return Some("zh".to_string());
    }

    Some(token.chars().take(2).collect())
}

fn parse_page_count(raw: &str) -> Option<u32> {
    let digits = raw
        .chars()
        .filter(|ch| ch.is_ascii_digit())
        .collect::<String>();

    digits.parse::<u32>().ok().filter(|value| *value > 0)
}

fn build_quality_metadata(page_count: Option<u32>, title: &str) -> Option<String> {
    let mut fields = Vec::new();

    if let Some(page_count) = page_count {
        fields.push(format!("pages:{}", page_count));
    }

    let normalized_title = sanitize_quality_value(title);
    if !normalized_title.is_empty() {
        fields.push(format!("name:{}", normalized_title));
    }

    if fields.is_empty() {
        return None;
    }

    Some(fields.join("|"))
}

fn sanitize_quality_value(raw: &str) -> String {
    raw.replace('|', " ")
        .replace('\n', " ")
        .replace('\r', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_query_param(url: &str, key: &str) -> Option<String> {
    let mut parts = url.splitn(2, '?');
    let _path = parts.next()?;
    let query = parts.next()?;

    for pair in query.split('&') {
        let mut key_value = pair.splitn(2, '=');
        let raw_key = key_value.next()?.trim();
        let raw_value = key_value.next().unwrap_or("").trim();

        if raw_key.eq_ignore_ascii_case(key) && !raw_value.is_empty() {
            return Some(raw_value.to_string());
        }
    }

    None
}

fn absolutize_url(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        return url.to_string();
    }

    format!("{}{}{}", LIBGEN_BASE_URL, if url.starts_with('/') { "" } else { "/" }, url)
}

fn url_encode(value: &str) -> String {
    let mut encoded = String::new();

    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char)
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{:02X}", byte)),
        }
    }

    encoded
}

fn http_get(url: &str) -> Result<crate::lexicon::plugin_roles::common_types::HttpResponse, PluginError> {
    host_http::send_http_request(&HttpRequest {
        method: "GET".to_string(),
        url: url.to_string(),
        query: Vec::new(),
        headers: vec![
            HttpHeader {
                key: "User-Agent".to_string(),
                value:
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
                        .to_string(),
            },
            HttpHeader {
                key: "Accept".to_string(),
                value: "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"
                    .to_string(),
            },
            HttpHeader {
                key: "Accept-Language".to_string(),
                value: "en-US,en;q=0.5".to_string(),
            },
        ],
        body: None,
        timeout_ms: Some(REQUEST_TIMEOUT_MS),
    })
    .map_err(|err| PluginError::NetworkFailure(format!("request failed: {}", err)))
}
