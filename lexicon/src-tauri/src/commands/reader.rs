use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use lopdf::Document as PdfDocument;
use tauri::State;

use crate::db::repositories::BookRepository;
use crate::models::{BookContent, PdfDocumentData, BOOK_STATUS_FINISHED, BOOK_STATUS_READING};
use crate::reader::EpubParser;
use crate::AppState;

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
