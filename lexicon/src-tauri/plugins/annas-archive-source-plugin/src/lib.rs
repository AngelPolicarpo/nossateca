wit_bindgen::generate!({
    path: "../../wit/discover-source-plugin.wit",
    world: "source-plugin",
});

use std::collections::BTreeSet;

use scraper::{Html, Selector};

use crate::lexicon::plugin_roles::common_types::{HttpHeader, HttpRequest};
use crate::lexicon::plugin_roles::host_http;

const SOURCE_NAME: &str = "Anna's Archive";
const SOURCE_ID: &str = "annas-archive";
const ANNAS_BASE_URL: &str = "https://annas-archive.org";
const REQUEST_TIMEOUT_MS: u64 = 20_000;
const MAX_RESULTS: usize = 30;
const MAX_DETAIL_PAGES: usize = 8;

struct AnnasArchiveSourcePlugin;

export!(AnnasArchiveSourcePlugin);

impl Guest for AnnasArchiveSourcePlugin {
    fn get_source_info() -> SourceInfo {
        SourceInfo {
            source_name: SOURCE_NAME.to_string(),
            source_id: SOURCE_ID.to_string(),
            supported_formats: vec![
                "epub".to_string(),
                "pdf".to_string(),
                "mobi".to_string(),
                "azw3".to_string(),
                "djvu".to_string(),
            ],
        }
    }

    fn find_downloads(request: SourceSearchQuery) -> Result<Vec<SourceDownloadResult>, PluginError> {
        let query = build_query(&request);
        if query.is_empty() {
            return Err(PluginError::NotFound(
                "title or isbn is required".to_string(),
            ));
        }

        let search_url = format!("{}/search?q={}", ANNAS_BASE_URL, url_encode(&query));
        let search_response = http_get(&search_url)?;

        if search_response.status == 429 || search_response.status == 403 {
            return Err(PluginError::RateLimit(
                "anna's archive temporarily rate limited this request".to_string(),
            ));
        }

        if search_response.status != 200 {
            return Err(PluginError::NetworkFailure(format!(
                "anna's archive returned status {}",
                search_response.status
            )));
        }

        let detail_pages = extract_detail_pages(&search_response.body)?;
        if detail_pages.is_empty() {
            return Err(PluginError::NotFound(
                "no matching documents found".to_string(),
            ));
        }

        let mut seen = BTreeSet::new();
        let mut results = Vec::new();
        let query_lc = query.to_ascii_lowercase();

        for detail_url in detail_pages.into_iter().take(MAX_DETAIL_PAGES) {
            if results.len() >= MAX_RESULTS {
                break;
            }

            let detail_response = match http_get(&detail_url) {
                Ok(response) => response,
                Err(_) => continue,
            };

            if detail_response.status == 429 || detail_response.status == 403 {
                continue;
            }

            if detail_response.status != 200 {
                continue;
            }

            let candidates = extract_direct_download_candidates(&detail_response.body)?;
            for candidate in candidates {
                if !is_direct_download_link(&candidate.url) {
                    continue;
                }

                if !seen.insert(candidate.url.clone()) {
                    continue;
                }

                let mut quality = candidate.label;
                if let Some(author) = request.author.as_ref().map(|value| value.trim()).filter(|value| !value.is_empty()) {
                    quality = Some(format!("{} | {}", quality.unwrap_or_else(|| "mirror".to_string()), author));
                }

                let format = detect_format(&candidate.url);
                let language = detect_language_hint(&detail_response.body);

                results.push(SourceDownloadResult {
                    download_url: candidate.url,
                    format,
                    size: candidate.size,
                    language,
                    quality,
                });

                if results.len() >= MAX_RESULTS {
                    break;
                }
            }
        }

        if let Some(isbn) = request
            .isbn
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let isbn_lc = isbn.to_ascii_lowercase();
            results.sort_by(|left, right| {
                let left_match = left
                    .quality
                    .as_deref()
                    .map(|value| value.to_ascii_lowercase().contains(&isbn_lc))
                    .unwrap_or(false)
                    || left.download_url.to_ascii_lowercase().contains(&isbn_lc);

                let right_match = right
                    .quality
                    .as_deref()
                    .map(|value| value.to_ascii_lowercase().contains(&isbn_lc))
                    .unwrap_or(false)
                    || right.download_url.to_ascii_lowercase().contains(&isbn_lc);

                right_match.cmp(&left_match)
            });
        } else {
            results.sort_by(|left, right| {
                let left_score = relevance_score(&left.download_url, &query_lc);
                let right_score = relevance_score(&right.download_url, &query_lc);
                right_score.cmp(&left_score)
            });
        }

        if results.is_empty() {
            return Err(PluginError::NotFound(
                "no direct downloads resolved from anna's archive".to_string(),
            ));
        }

        Ok(results)
    }
}

fn build_query(request: &SourceSearchQuery) -> String {
    if let Some(isbn) = request
        .isbn
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return isbn.to_string();
    }

    let mut chunks = Vec::new();

    let title = request.title.trim();
    if !title.is_empty() {
        chunks.push(title.to_string());
    }

    if let Some(author) = request
        .author
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        chunks.push(author.to_string());
    }

    chunks.join(" ")
}

fn extract_detail_pages(html: &str) -> Result<Vec<String>, PluginError> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("a[href*='/md5/']")
        .map_err(|err| PluginError::ParsingFailure(format!("invalid selector: {}", err)))?;

    let mut urls = Vec::new();
    let mut seen = BTreeSet::new();

    for link in document.select(&selector) {
        let href = link.value().attr("href").unwrap_or("").trim();
        if href.is_empty() {
            continue;
        }

        let absolute = absolutize_url(href);
        if seen.insert(absolute.clone()) {
            urls.push(absolute);
        }
    }

    Ok(urls)
}

#[derive(Debug, Clone)]
struct CandidateDownload {
    url: String,
    label: Option<String>,
    size: Option<String>,
}

fn extract_direct_download_candidates(html: &str) -> Result<Vec<CandidateDownload>, PluginError> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("a[href]")
        .map_err(|err| PluginError::ParsingFailure(format!("invalid selector: {}", err)))?;

    let mut results = Vec::new();

    for link in document.select(&selector) {
        let href = link.value().attr("href").unwrap_or("").trim();
        if href.is_empty() {
            continue;
        }

        let lower_href = href.to_ascii_lowercase();
        if !lower_href.contains("get.php")
            && !lower_href.ends_with(".epub")
            && !lower_href.ends_with(".pdf")
            && !lower_href.ends_with(".mobi")
            && !lower_href.ends_with(".azw3")
            && !lower_href.ends_with(".djvu")
        {
            continue;
        }

        let label = link
            .text()
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();

        let normalized_label = if label.is_empty() { None } else { Some(label) };

        results.push(CandidateDownload {
            url: absolutize_url(href),
            label: normalized_label,
            size: None,
        });
    }

    Ok(results)
}

fn detect_format(url: &str) -> String {
    let lowered = url.to_ascii_lowercase();

    if lowered.ends_with(".epub") {
        return "epub".to_string();
    }

    if lowered.ends_with(".pdf") {
        return "pdf".to_string();
    }

    if lowered.ends_with(".mobi") {
        return "mobi".to_string();
    }

    if lowered.ends_with(".azw3") {
        return "azw3".to_string();
    }

    if lowered.ends_with(".djvu") {
        return "djvu".to_string();
    }

    "unknown".to_string()
}

fn detect_language_hint(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();

    if lower.contains("language") {
        if lower.contains("english") {
            return Some("en".to_string());
        }

        if lower.contains("portuguese") || lower.contains("portugues") {
            return Some("pt".to_string());
        }

        if lower.contains("spanish") {
            return Some("es".to_string());
        }
    }

    None
}

fn relevance_score(url: &str, query_lc: &str) -> usize {
    query_lc
        .split_whitespace()
        .filter(|term| !term.is_empty())
        .filter(|term| url.to_ascii_lowercase().contains(*term))
        .count()
}

fn is_direct_download_link(url: &str) -> bool {
    let normalized = url.trim().to_ascii_lowercase();

    normalized.starts_with("https://")
        && (normalized.contains("get.php")
            || normalized.ends_with(".epub")
            || normalized.ends_with(".pdf")
            || normalized.ends_with(".mobi")
            || normalized.ends_with(".azw3")
            || normalized.ends_with(".djvu"))
}

fn absolutize_url(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        return url.to_string();
    }

    format!("{}{}{}", ANNAS_BASE_URL, if url.starts_with('/') { "" } else { "/" }, url)
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
                key: "accept".to_string(),
                value: "text/html,application/xhtml+xml".to_string(),
            },
            HttpHeader {
                key: "accept-language".to_string(),
                value: "en-US,en;q=0.9".to_string(),
            },
        ],
        body: None,
        timeout_ms: Some(REQUEST_TIMEOUT_MS),
    })
    .map_err(|err| PluginError::NetworkFailure(format!("request failed: {}", err)))
}
