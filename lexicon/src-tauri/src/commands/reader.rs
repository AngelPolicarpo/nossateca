use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use lopdf::Document as PdfDocument;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use tauri::State;

use crate::db::repositories::BookRepository;
use crate::models::{
    BookContent, EpubLinkTarget, EpubSearchMatch, EpubSearchResponse, PdfDocumentData,
    ReadingProgressData,
    BOOK_STATUS_FINISHED, BOOK_STATUS_READING,
};
use crate::reader::EpubParser;
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CbzPageData {
    pub bytes_base64: String,
    pub mime_type: String,
    pub page_index: usize,
    pub total_pages: usize,
}

#[tauri::command]
pub async fn get_book_content(
    book_id: String,
    chapter_index: usize,
    state: State<'_, AppState>,
) -> Result<BookContent, String> {
    let parsed_book_id = book_id
        .parse::<i64>()
        .map_err(|_| "Invalid book_id".to_string())?;

    let repository = BookRepository::new(&state._db_pool);
    let book = repository
        .find_by_id(parsed_book_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Book not found".to_string())?;

    let normalized_format = book.format.trim().to_ascii_lowercase();

    if normalized_format == "pdf" {
        let total_pages = count_pdf_pages(&book.file_path)?;
        let safe_index = chapter_index.min(total_pages.saturating_sub(1));

        return Ok(BookContent {
            html: String::new(),
            current_chapter: safe_index,
            total_chapters: total_pages,
            chapter_title: book.title,
            book_format: normalized_format,
            book_file_path: Some(book.file_path),
            supports_annotations: false,
        });
    }

    if normalized_format == "cbz" {
        let total_pages = list_cbz_pages(&book.file_path)?.len();
        if total_pages == 0 {
            return Err("CBZ has no readable pages".to_string());
        }
        let safe_index = chapter_index.min(total_pages.saturating_sub(1));

        return Ok(BookContent {
            html: String::new(),
            current_chapter: safe_index,
            total_chapters: total_pages,
            chapter_title: book.title,
            book_format: normalized_format,
            book_file_path: Some(book.file_path),
            supports_annotations: false,
        });
    }

    if normalized_format != "epub" {
        return Err(format!("Reader does not support format: {}", book.format));
    }

    let parser = EpubParser::new(&book.file_path);
    let spine = parser.get_spine();

    if spine.is_empty() {
        return Err("EPUB has no readable chapters".to_string());
    }

    if chapter_index >= spine.len() {
        return Err("Chapter index out of bounds".to_string());
    }

    let chapter_id = &spine[chapter_index];
    let html = parser
        .get_chapter_content(chapter_id)
        .map_err(|e| e.to_string())?;

    let chapter_title = parser
        .get_toc()
        .into_iter()
        .find_map(|(title, id)| if id == *chapter_id { Some(title) } else { None })
        .unwrap_or_else(|| format!("Capítulo {}", chapter_index + 1));

    Ok(BookContent {
        html,
        current_chapter: chapter_index,
        total_chapters: spine.len(),
        chapter_title,
        book_format: normalized_format,
        book_file_path: Some(book.file_path),
        supports_annotations: true,
    })
}

#[tauri::command]
pub async fn get_pdf_document(
    book_id: String,
    state: State<'_, AppState>,
) -> Result<PdfDocumentData, String> {
    let parsed_book_id = book_id
        .parse::<i64>()
        .map_err(|_| "Invalid book_id".to_string())?;

    let repository = BookRepository::new(&state._db_pool);
    let book = repository
        .find_by_id(parsed_book_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Book not found".to_string())?;

    let normalized_format = book.format.trim().to_ascii_lowercase();
    if normalized_format != "pdf" {
        return Err("Requested book is not a PDF".to_string());
    }

    let total_pages = count_pdf_pages(&book.file_path)?;

    let bytes = tokio::fs::read(&book.file_path)
        .await
        .map_err(|e| format!("Failed to read PDF file '{}': {}", book.file_path, e))?;

    if bytes.is_empty() {
        return Err("PDF file is empty".to_string());
    }

    Ok(PdfDocumentData {
        bytes_base64: BASE64_STANDARD.encode(bytes),
        total_pages,
    })
}

#[tauri::command]
pub async fn get_reading_progress(
    book_id: String,
    state: State<'_, AppState>,
) -> Result<Option<ReadingProgressData>, String> {
    let parsed_book_id = book_id
        .parse::<i64>()
        .map_err(|_| "Invalid book_id".to_string())?;

    let repository = BookRepository::new(&state._db_pool);
    let book_exists = repository
        .find_by_id(parsed_book_id)
        .await
        .map_err(|e| e.to_string())?
        .is_some();

    if !book_exists {
        return Err("Book not found".to_string());
    }

    let progress = sqlx::query_as::<_, ReadingProgressData>(
        "SELECT current_position, progress_percent FROM reading_progress WHERE book_id = ?",
    )
    .bind(parsed_book_id)
    .fetch_optional(&state._db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(progress)
}

#[tauri::command]
pub async fn search_epub_content(
    book_id: String,
    query: String,
    state: State<'_, AppState>,
) -> Result<EpubSearchResponse, String> {
    let parsed_book_id = book_id
        .parse::<i64>()
        .map_err(|_| "Invalid book_id".to_string())?;

    let normalized_query = query.trim();
    if normalized_query.is_empty() {
        return Ok(EpubSearchResponse {
            query: String::new(),
            total_matches: 0,
            results: Vec::new(),
        });
    }

    let repository = BookRepository::new(&state._db_pool);
    let book = repository
        .find_by_id(parsed_book_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Book not found".to_string())?;

    let normalized_format = book.format.trim().to_ascii_lowercase();
    if normalized_format != "epub" {
        return Err("Whole-book search is currently available only for EPUB".to_string());
    }

    let parser = EpubParser::new(&book.file_path);
    let spine = parser.get_spine();
    if spine.is_empty() {
        return Ok(EpubSearchResponse {
            query: normalized_query.to_string(),
            total_matches: 0,
            results: Vec::new(),
        });
    }

    let toc_titles: HashMap<String, String> = parser
        .get_toc()
        .into_iter()
        .map(|(title, id)| (id, title))
        .collect();

    let mut total_matches = 0usize;
    let mut results: Vec<EpubSearchMatch> = Vec::new();

    for (chapter_index, chapter_id) in spine.iter().enumerate() {
        let chapter_html = parser
            .get_chapter_content(chapter_id)
            .map_err(|e| format!("Failed to parse chapter {}: {}", chapter_index + 1, e))?;

        let plain_text = html_to_plain_text(&chapter_html);
        let (occurrences, snippet) = count_matches_and_snippet(&plain_text, normalized_query);

        if occurrences == 0 {
            continue;
        }

        total_matches += occurrences;
        results.push(EpubSearchMatch {
            chapter_index,
            chapter_title: toc_titles
                .get(chapter_id)
                .cloned()
                .unwrap_or_else(|| format!("Capítulo {}", chapter_index + 1)),
            snippet,
            occurrences,
        });
    }

    Ok(EpubSearchResponse {
        query: normalized_query.to_string(),
        total_matches,
        results,
    })
}

#[tauri::command]
pub async fn resolve_epub_link_target(
    book_id: String,
    chapter_index: usize,
    href: String,
    state: State<'_, AppState>,
) -> Result<EpubLinkTarget, String> {
    let parsed_book_id = book_id
        .parse::<i64>()
        .map_err(|_| "Invalid book_id".to_string())?;

    let repository = BookRepository::new(&state._db_pool);
    let book = repository
        .find_by_id(parsed_book_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Book not found".to_string())?;

    let normalized_format = book.format.trim().to_ascii_lowercase();
    if normalized_format != "epub" {
        return Err("Internal link resolution is currently available only for EPUB".to_string());
    }

    let parser = EpubParser::new(&book.file_path);
    let resolved = parser
        .resolve_internal_link(chapter_index, href.as_str())
        .map_err(|e| e.to_string())?;

    let Some((resolved_chapter_index, anchor_id)) = resolved else {
        return Err("Unable to resolve EPUB link target".to_string());
    };

    Ok(EpubLinkTarget {
        chapter_index: resolved_chapter_index,
        anchor_id,
    })
}

#[tauri::command]
pub async fn save_progress(
    book_id: String,
    chapter_index: usize,
    scroll_position: Option<f64>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let parsed_book_id = book_id
        .parse::<i64>()
        .map_err(|_| "Invalid book_id".to_string())?;

    let repository = BookRepository::new(&state._db_pool);
    let book = repository
        .find_by_id(parsed_book_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Book not found".to_string())?;

    let normalized_format = book.format.trim().to_ascii_lowercase();

    let (current_position, percent) = if normalized_format == "epub" {
        let parser = EpubParser::new(&book.file_path);
        let total_chapters = parser.get_spine().len().max(1);
        let computed_percent =
            (((chapter_index + 1) as f64 / total_chapters as f64) * 100.0).min(100.0);

        let position = match scroll_position {
            Some(scroll) => format!("chapter:{};scroll:{:.4}", chapter_index, scroll),
            None => format!("chapter:{}", chapter_index),
        };

        (position, computed_percent)
    } else if normalized_format == "pdf" {
        let total_pages = count_pdf_pages(&book.file_path)?.max(1);
        let safe_index = chapter_index.min(total_pages.saturating_sub(1));
        let current_page = safe_index.saturating_add(1);
        let computed_percent = ((current_page as f64 / total_pages as f64) * 100.0).min(100.0);

        (format!("page:{}", current_page), computed_percent)
    } else if normalized_format == "cbz" {
        let total_pages = list_cbz_pages(&book.file_path)?.len().max(1);
        let safe_index = chapter_index.min(total_pages.saturating_sub(1));
        let current_page = safe_index.saturating_add(1);
        let computed_percent = ((current_page as f64 / total_pages as f64) * 100.0).min(100.0);

        (format!("page:{}", current_page), computed_percent)
    } else {
        return Err(format!("Reader does not support format: {}", book.format));
    };

    sqlx::query(
        "INSERT INTO reading_progress (book_id, current_position, progress_percent) VALUES (?, ?, ?) ON CONFLICT(book_id) DO UPDATE SET current_position = excluded.current_position, progress_percent = excluded.progress_percent",
    )
    .bind(parsed_book_id)
    .bind(current_position)
    .bind(percent)
    .execute(&state._db_pool)
    .await
    .map_err(|e| e.to_string())?;

    let current_status = book.status.trim().to_ascii_lowercase();
    let next_status = if percent >= 99.5 || current_status == BOOK_STATUS_FINISHED {
        BOOK_STATUS_FINISHED
    } else {
        BOOK_STATUS_READING
    };

    if current_status != next_status {
        repository
            .update_status(parsed_book_id, next_status)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn count_pdf_pages(file_path: &str) -> Result<usize, String> {
    let document = PdfDocument::load(file_path)
        .map_err(|e| format!("Failed to open PDF '{}': {}", file_path, e))?;

    let total_pages = document.get_pages().len();
    if total_pages == 0 {
        return Err("PDF has no readable pages".to_string());
    }

    Ok(total_pages)
}

fn list_cbz_pages(file_path: &str) -> Result<Vec<String>, String> {
    let file = File::open(file_path)
        .map_err(|err| format!("Failed to open CBZ '{}': {}", file_path, err))?;
    let mut archive = zip::ZipArchive::new(BufReader::new(file))
        .map_err(|err| format!("Failed to read CBZ archive '{}': {}", file_path, err))?;

    let mut entries: Vec<String> = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|err| format!("Failed to inspect zip entry {}: {}", index, err))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        if is_image_entry(&name) {
            entries.push(name);
        }
    }

    entries.sort_by(|left, right| natural_sort_key(left).cmp(&natural_sort_key(right)));
    Ok(entries)
}

fn is_image_entry(name: &str) -> bool {
    let lowered = name.to_ascii_lowercase();
    matches!(
        Path::new(&lowered)
            .extension()
            .and_then(|ext| ext.to_str()),
        Some("jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp" | "avif")
    )
}

fn natural_sort_key(value: &str) -> Vec<(u8, u64, String)> {
    let mut parts: Vec<(u8, u64, String)> = Vec::new();
    let mut current_text = String::new();
    let mut current_number = String::new();

    let push_text = |buffer: &mut String, parts: &mut Vec<(u8, u64, String)>| {
        if !buffer.is_empty() {
            parts.push((0, 0, std::mem::take(buffer).to_ascii_lowercase()));
        }
    };
    let push_number = |buffer: &mut String, parts: &mut Vec<(u8, u64, String)>| {
        if !buffer.is_empty() {
            let n = buffer.parse::<u64>().unwrap_or(0);
            parts.push((1, n, std::mem::take(buffer)));
        }
    };

    for ch in value.chars() {
        if ch.is_ascii_digit() {
            push_text(&mut current_text, &mut parts);
            current_number.push(ch);
        } else {
            push_number(&mut current_number, &mut parts);
            current_text.push(ch);
        }
    }
    push_text(&mut current_text, &mut parts);
    push_number(&mut current_number, &mut parts);
    parts
}

fn cbz_mime_type(name: &str) -> &'static str {
    let lowered = name.to_ascii_lowercase();
    match Path::new(&lowered).extension().and_then(|ext| ext.to_str()) {
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        Some("bmp") => "image/bmp",
        Some("avif") => "image/avif",
        _ => "image/jpeg",
    }
}

#[tauri::command]
pub async fn get_cbz_page(
    book_id: String,
    page_index: usize,
    state: State<'_, AppState>,
) -> Result<CbzPageData, String> {
    let parsed_book_id = book_id
        .parse::<i64>()
        .map_err(|_| "Invalid book_id".to_string())?;

    let repository = BookRepository::new(&state._db_pool);
    let book = repository
        .find_by_id(parsed_book_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Book not found".to_string())?;

    let normalized_format = book.format.trim().to_ascii_lowercase();
    if normalized_format != "cbz" {
        return Err("Requested book is not a CBZ".to_string());
    }

    let file_path = book.file_path.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<CbzPageData, String> {
        let entries = list_cbz_pages(&file_path)?;
        if entries.is_empty() {
            return Err("CBZ has no readable pages".to_string());
        }

        let total_pages = entries.len();
        let safe_index = page_index.min(total_pages - 1);
        let entry_name = &entries[safe_index];

        let file = File::open(&file_path)
            .map_err(|err| format!("Failed to open CBZ '{}': {}", file_path, err))?;
        let mut archive = zip::ZipArchive::new(BufReader::new(file))
            .map_err(|err| format!("Failed to read CBZ archive '{}': {}", file_path, err))?;

        let mut entry = archive
            .by_name(entry_name)
            .map_err(|err| format!("Failed to read entry '{}': {}", entry_name, err))?;

        let mut buffer = Vec::with_capacity(entry.size() as usize);
        entry
            .read_to_end(&mut buffer)
            .map_err(|err| format!("Failed to extract entry '{}': {}", entry_name, err))?;

        Ok(CbzPageData {
            bytes_base64: BASE64_STANDARD.encode(&buffer),
            mime_type: cbz_mime_type(entry_name).to_string(),
            page_index: safe_index,
            total_pages,
        })
    })
    .await
    .map_err(|err| format!("CBZ task join error: {}", err))??;

    Ok(result)
}

fn html_to_plain_text(input: &str) -> String {
    let mut text = String::with_capacity(input.len());
    let mut in_tag = false;

    for ch in input.chars() {
        if ch == '<' {
            in_tag = true;
            if !text.ends_with(' ') {
                text.push(' ');
            }
            continue;
        }

        if ch == '>' {
            in_tag = false;
            continue;
        }

        if !in_tag {
            text.push(ch);
        }
    }

    collapse_whitespace(&decode_html_entities(&text))
}

fn decode_html_entities(value: &str) -> String {
    value
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn collapse_whitespace(value: &str) -> String {
    value
        .split_whitespace()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn count_matches_and_snippet(text: &str, query: &str) -> (usize, String) {
    let lower_text = text.to_lowercase();
    let lower_query = query.to_lowercase();

    if lower_query.is_empty() || lower_text.is_empty() {
        return (0, String::new());
    }

    let mut match_count = 0usize;
    let mut first_match_byte_index: Option<usize> = None;
    let mut search_start = 0usize;

    while search_start < lower_text.len() {
        let search_slice = &lower_text[search_start..];
        let Some(offset) = search_slice.find(&lower_query) else {
            break;
        };

        let absolute_index = search_start + offset;
        if first_match_byte_index.is_none() {
            first_match_byte_index = Some(absolute_index);
        }

        match_count += 1;
        search_start = absolute_index + lower_query.len();
    }

    let Some(first_match) = first_match_byte_index else {
        return (0, String::new());
    };

    let snippet = build_snippet(text, first_match, query.len());
    (match_count, snippet)
}

fn build_snippet(text: &str, first_match_start: usize, query_len: usize) -> String {
    let context = 80usize;
    let start_raw = first_match_start.min(text.len()).saturating_sub(context);
    let end_raw = (first_match_start.min(text.len()) + query_len + context).min(text.len());
    let start = previous_char_boundary(text, start_raw);
    let end = next_char_boundary(text, end_raw);

    let mut snippet = text[start..end].trim().to_string();
    if start > 0 {
        snippet = format!("...{}", snippet);
    }

    if end < text.len() {
        snippet.push_str("...");
    }

    snippet
}

fn previous_char_boundary(text: &str, mut index: usize) -> usize {
    index = index.min(text.len());
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn next_char_boundary(text: &str, mut index: usize) -> usize {
    index = index.min(text.len());
    while index < text.len() && !text.is_char_boundary(index) {
        index += 1;
    }
    index
}
