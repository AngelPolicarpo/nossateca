wit_bindgen::generate!({
    path: "../../wit/search-plugin.wit",
    world: "search-plugin",
});

use scraper::{ElementRef, Html, Selector};
use crate::lexicon::plugins::host_http;
use crate::lexicon::plugins::search_types::{HttpHeader, HttpRequest, HttpResponse, SearchSetting};

const SOURCE_ID: &str = "libgen-li";
const DEFAULT_BASE_URL: &str = "https://libgen.li";
const DEFAULT_TIMEOUT_MS: u64 = 15_000;
const DEFAULT_MAX_RESULTS: usize = 25;
const DEFAULT_MAX_RESOLVE_ATTEMPTS: usize = 3;

struct LibgenLiPlugin;

export!(LibgenLiPlugin);

#[derive(Debug, Clone)]
struct PluginSettings {
    base_url: String,
    format_filter: Option<String>,
    max_results: usize,
    timeout_ms: u64,
    max_resolve_attempts: usize,
}

impl Default for PluginSettings {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            format_filter: Some("epub".to_string()),
            max_results: DEFAULT_MAX_RESULTS,
            timeout_ms: DEFAULT_TIMEOUT_MS,
            max_resolve_attempts: DEFAULT_MAX_RESOLVE_ATTEMPTS,
        }
    }
}

impl PluginSettings {
    fn from_entries(entries: &[SearchSetting]) -> Self {
        let mut settings = Self::default();

        for entry in entries {
            let key = normalize_setting_key(&entry.key);
            let value = entry.value.trim();
            if value.is_empty() {
                continue;
            }

            match key.as_str() {
                "base_url" | "base" | "host" => {
                    settings.base_url = normalize_base_url(value);
                }
                "format" | "file_format" | "extension" => {
                    let normalized = value.to_ascii_lowercase();
                    settings.format_filter = match normalized.as_str() {
                        "" | "any" | "all" | "*" => None,
                        _ => Some(normalized),
                    };
                }
                "max_results" | "limit" => {
                    if let Ok(parsed) = value.parse::<usize>() {
                        settings.max_results = parsed.clamp(1usize, 100usize);
                    }
                }
                "timeout_ms" | "timeout" => {
                    if let Ok(parsed) = value.parse::<u64>() {
                        settings.timeout_ms = parsed.clamp(1_000u64, 45_000u64);
                    }
                }
                "resolve_limit" | "resolve_download_limit" | "max_resolve_attempts" => {
                    if let Ok(parsed) = value.parse::<usize>() {
                        settings.max_resolve_attempts = parsed.clamp(0usize, 50usize);
                    }
                }
                _ => {}
            }
        }

        settings
    }

    fn format_allowed(&self, value: &str) -> bool {
        match &self.format_filter {
            Some(filter) => value.eq_ignore_ascii_case(filter),
            None => true,
        }
    }
}

impl Guest for LibgenLiPlugin {
    fn search_books(request: SearchRequest) -> Vec<SearchResult> {
        let query = request.query.trim();
        if query.is_empty() {
            return Vec::new();
        }

        let settings = PluginSettings::from_entries(&request.settings);
        let encoded_query = url_encode(query);
        let search_url = format!("{}/index.php?req={}&curtab=f", settings.base_url, encoded_query);

        let response = match http_get(&search_url, settings.timeout_ms) {
            Ok(response) => response,
            Err(_) => return Vec::new(),
        };

        if response.status != 200 {
            return Vec::new();
        }

        let query_terms = tokenize_query(query);
        parse_search_results(&response.body, &settings, &query_terms)
    }
}

fn parse_search_results(
    html: &str,
    settings: &PluginSettings,
    query_terms: &[String],
) -> Vec<SearchResult> {
    let document = Html::parse_document(html);

    let row_selector = match Selector::parse("table tbody tr, table tr") {
        Ok(selector) => selector,
        Err(_) => return Vec::new(),
    };
    let cell_selector = match Selector::parse("td") {
        Ok(selector) => selector,
        Err(_) => return Vec::new(),
    };
    let edition_link_selector = match Selector::parse("a[href*=\"edition.php\"]") {
        Ok(selector) => selector,
        Err(_) => return Vec::new(),
    };
    let link_selector = match Selector::parse("a") {
        Ok(selector) => selector,
        Err(_) => return Vec::new(),
    };

    let mut results = Vec::new();
    let mut resolve_attempts = 0usize;

    for row in document.select(&row_selector) {
        if results.len() >= settings.max_results {
            break;
        }

        let cells: Vec<ElementRef<'_>> = row.select(&cell_selector).collect();
        if cells.len() < 8 {
            continue;
        }

        let first_cell = &cells[0];
        let Some(title_link) = first_cell.select(&edition_link_selector).next() else {
            continue;
        };

        let title = clean_text_from_node(&title_link);
        if title.is_empty() {
            continue;
        }

        let title_href = title_link.value().attr("href").unwrap_or("").trim();
        let Some(edition_id) = extract_query_param(title_href, "id") else {
            continue;
        };

        let author = clean_text_from_node(&cells[1]);
        let format_value = clean_text_from_node(&cells[7]);
        if format_value.is_empty() || !settings.format_allowed(&format_value) {
            continue;
        }

        let inline_candidate =
            extract_inline_download_candidate(cells.get(8), &link_selector, &settings.base_url);

        let fallback_url = inline_candidate.clone().unwrap_or_else(|| {
            format!("{}/edition.php?id={}", settings.base_url, edition_id)
        });

        let should_resolve = inline_candidate
            .as_deref()
            .map(is_intermediate_download_link)
            .unwrap_or(true);

        let download_url = if should_resolve && resolve_attempts < settings.max_resolve_attempts {
            resolve_attempts += 1;
            resolve_download_url(
                &settings.base_url,
                &edition_id,
                inline_candidate.as_deref(),
                settings.timeout_ms,
            )
            .unwrap_or(fallback_url)
        } else {
            fallback_url
        };

        if !is_download_url_usable(&download_url) {
            continue;
        }

        let score = compute_score(query_terms, &title, &author, &format_value);

        results.push(SearchResult {
            id: format!("{}-{}", SOURCE_ID, edition_id),
            title,
            author: if author.is_empty() { None } else { Some(author) },
            source: SOURCE_ID.to_string(),
            format: Some(format_value.to_ascii_lowercase()),
            download_url,
            score,
        });
    }

    results
}

fn extract_inline_download_candidate(
    cell: Option<&ElementRef<'_>>,
    link_selector: &Selector,
    base_url: &str,
) -> Option<String> {
    let cell = cell?;

    let mut ads_candidate: Option<String> = None;
    let mut fallback: Option<String> = None;

    for link in cell.select(link_selector) {
        let href = link.value().attr("href").unwrap_or("").trim();
        if href.is_empty() {
            continue;
        }

        let absolute = absolutize_url(base_url, href);

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

fn resolve_download_url(
    base_url: &str,
    edition_id: &str,
    inline_candidate: Option<&str>,
    timeout_ms: u64,
) -> Option<String> {
    if let Some(candidate) = inline_candidate {
        if looks_like_get_link(candidate) {
            return Some(candidate.to_string());
        }

        if candidate.to_ascii_lowercase().contains("ads.php?md5=") {
            if let Some(url) = resolve_download_from_ads_page(base_url, candidate, timeout_ms) {
                return Some(url);
            }
        }
    }

    let edition_url = format!("{}/edition.php?id={}", base_url, edition_id);
    let edition_response = http_get(&edition_url, timeout_ms).ok()?;
    if edition_response.status != 200 {
        return inline_candidate.map(|value| value.to_string());
    }

    let ads_href = extract_ads_href_from_html(&edition_response.body)?;

    resolve_download_from_ads_page(base_url, &ads_href, timeout_ms)
        .or_else(|| inline_candidate.map(|value| value.to_string()))
}

fn resolve_download_from_ads_page(
    base_url: &str,
    ads_url_or_href: &str,
    timeout_ms: u64,
) -> Option<String> {
    let ads_url = absolutize_url(base_url, ads_url_or_href);
    let ads_response = http_get(&ads_url, timeout_ms).ok()?;
    if ads_response.status != 200 {
        return None;
    }

    let href = extract_get_href_from_html(&ads_response.body)?;
    Some(absolutize_url(base_url, &href))
}

fn extract_ads_href_from_html(html: &str) -> Option<String> {
    extract_href_containing(html, "ads.php?md5=")
}

fn extract_get_href_from_html(html: &str) -> Option<String> {
    if let Some(raw) = extract_raw_link_by_prefix(html, "get.php?md5=") {
        return Some(raw);
    }

    extract_href_containing(html, "get.php")
}

fn extract_raw_link_by_prefix(html: &str, prefix: &str) -> Option<String> {
    let lower_html = html.to_ascii_lowercase();
    let lower_prefix = prefix.to_ascii_lowercase();
    let start = lower_html.find(&lower_prefix)?;
    let tail = &html[start..];

    let mut end = tail.len();
    for (index, ch) in tail.char_indices() {
        if index == 0 {
            continue;
        }

        if ch == '\'' || ch == '"' || ch == '<' || ch.is_whitespace() {
            end = index;
            break;
        }
    }

    let link = tail[..end].trim();
    if link.is_empty() {
        return None;
    }

    Some(link.to_string())
}

fn extract_href_containing(html: &str, needle: &str) -> Option<String> {
    let lower_html = html.to_ascii_lowercase();
    let lower_needle = needle.to_ascii_lowercase();
    let bytes = lower_html.as_bytes();

    let mut cursor = 0usize;
    while cursor < bytes.len() {
        let Some(found_rel) = lower_html[cursor..].find("href") else {
            break;
        };
        let href_start = cursor + found_rel;
        let mut value_cursor = href_start + 4;

        while value_cursor < bytes.len() && (bytes[value_cursor] as char).is_ascii_whitespace() {
            value_cursor += 1;
        }

        if value_cursor >= bytes.len() || bytes[value_cursor] != b'=' {
            cursor = value_cursor.saturating_add(1);
            continue;
        }
        value_cursor += 1;

        while value_cursor < bytes.len() && (bytes[value_cursor] as char).is_ascii_whitespace() {
            value_cursor += 1;
        }

        if value_cursor >= bytes.len() {
            break;
        }

        let (content_start, content_end, next_cursor) = match bytes[value_cursor] {
            b'\'' => {
                let start = value_cursor + 1;
                let end = lower_html[start..]
                    .find('\'')
                    .map(|offset| start + offset)
                    .unwrap_or(bytes.len());
                (start, end, end.saturating_add(1))
            }
            b'"' => {
                let start = value_cursor + 1;
                let end = lower_html[start..]
                    .find('"')
                    .map(|offset| start + offset)
                    .unwrap_or(bytes.len());
                (start, end, end.saturating_add(1))
            }
            _ => {
                let start = value_cursor;
                let mut end = start;
                while end < bytes.len() {
                    let ch = bytes[end] as char;
                    if ch.is_ascii_whitespace() || ch == '>' {
                        break;
                    }
                    end += 1;
                }
                (start, end, end)
            }
        };

        if content_start < content_end && content_end <= html.len() {
            let href = html[content_start..content_end].trim();
            if !href.is_empty() && href.to_ascii_lowercase().contains(&lower_needle) {
                return Some(href.to_string());
            }
        }

        cursor = next_cursor;
    }

    None
}

fn is_intermediate_download_link(url: &str) -> bool {
    let normalized = url.to_ascii_lowercase();
    normalized.contains("edition.php") || normalized.contains("ads.php?md5=")
}

fn looks_like_get_link(url: &str) -> bool {
    url.to_ascii_lowercase().contains("get.php")
}

fn is_download_url_usable(url: &str) -> bool {
    if url.trim().is_empty() {
        return false;
    }

    if is_intermediate_download_link(url) {
        return false;
    }

    if looks_like_get_link(url) && !has_query_param(url, "key") {
        return false;
    }

    true
}

fn has_query_param(url_like: &str, key: &str) -> bool {
    extract_query_param(url_like, key).is_some()
}

fn compute_score(query_terms: &[String], title: &str, author: &str, format_value: &str) -> f32 {
    let haystack = format!("{} {}", title, author).to_ascii_lowercase();

    let matched_terms = query_terms
        .iter()
        .filter(|term| haystack.contains(term.as_str()))
        .count();

    let ratio = if query_terms.is_empty() {
        0.0
    } else {
        matched_terms as f32 / query_terms.len() as f32
    };

    let format_bonus = if format_value.eq_ignore_ascii_case("epub") {
        0.05
    } else {
        0.01
    };

    (0.58 + ratio * 0.35 + format_bonus).clamp(0.35, 0.99)
}

fn http_get(
    url: &str,
    timeout_ms: u64,
) -> Result<HttpResponse, String> {
    let request = HttpRequest {
        method: "GET".to_string(),
        url: url.to_string(),
        query: Vec::new(),
        headers: default_headers(),
        body: None,
        timeout_ms: Some(timeout_ms),
    };

    host_http::send_http_request(&request)
}

fn default_headers() -> Vec<HttpHeader> {
    vec![
        HttpHeader {
            key: "User-Agent".to_string(),
            value: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
        },
        HttpHeader {
            key: "Accept".to_string(),
            value: "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8".to_string(),
        },
        HttpHeader {
            key: "Accept-Language".to_string(),
            value: "en-US,en;q=0.5".to_string(),
        },
    ]
}

fn tokenize_query(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| part.to_ascii_lowercase())
        .collect()
}

fn normalize_setting_key(key: &str) -> String {
    key.trim()
        .to_ascii_lowercase()
        .chars()
        .map(|character| if character == '-' { '_' } else { character })
        .collect()
}

fn normalize_base_url(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return DEFAULT_BASE_URL.to_string();
    }

    let with_scheme = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("https://{}", trimmed.trim_start_matches('/'))
    };

    with_scheme.trim_end_matches('/').to_string()
}

fn clean_text_from_node(node: &ElementRef<'_>) -> String {
    let joined = node.text().collect::<Vec<_>>().join(" ");
    collapse_whitespace(&joined)
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn extract_query_param(url_like: &str, key: &str) -> Option<String> {
    let start = url_like.find('?')? + 1;
    let query = &url_like[start..];

    for pair in query.split('&') {
        let mut split = pair.splitn(2, '=');
        let name = split.next()?.trim();
        if !name.eq_ignore_ascii_case(key) {
            continue;
        }

        let value = split.next().unwrap_or("").trim();
        if value.is_empty() {
            return None;
        }

        return Some(value.to_string());
    }

    None
}

fn absolutize_url(base_url: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }

    if href.starts_with("//") {
        return format!("https:{}", href);
    }

    if href.starts_with('/') {
        return format!("{}{}", base_url, href);
    }

    format!("{}/{}", base_url, href)
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
