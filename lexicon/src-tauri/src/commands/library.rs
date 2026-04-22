use std::path::Path;
use std::{fs::File, io::Read};

use sha2::{Digest, Sha256};
use tauri::State;

use crate::db::repositories::BookRepository;
use crate::models::{Book, BOOK_STATUS_UNREAD};
use crate::reader::EpubParser;
use crate::AppState;

struct BookImportMetadata {
    title: String,
    author: Option<String>,
}

#[tauri::command]
pub async fn add_book(file_path: String, state: State<'_, AppState>) -> Result<Book, String> {
    let path = Path::new(&file_path);

    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    if !path.is_file() {
        return Err(format!("Path is not a file: {}", file_path));
    }

    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .ok_or_else(|| "File extension is missing".to_string())?;

    if extension != "epub" && extension != "pdf" && extension != "cbz" {
        return Err("Only .epub, .pdf and .cbz files are supported right now".to_string());
    }

    let metadata = extract_book_metadata(&file_path, &extension)?;
    let file_hash = compute_sha256(&file_path).map_err(|e| e.to_string())?;

    let repository = BookRepository::new(&state._db_pool);

    if repository
        .find_by_hash(&file_hash)
        .await
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return Err("Book already exists".to_string());
    }

    let new_book = Book {
        id: 0,
        title: metadata.title,
        author: metadata.author,
        format: extension,
        file_path,
        file_hash: Some(file_hash.clone()),
        status: BOOK_STATUS_UNREAD.to_string(),
        created_at: String::new(),
    };

    repository
        .insert(&new_book)
        .await
        .map_err(|e| e.to_string())?;

    let inserted_book = repository
        .find_by_hash(&file_hash)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Book inserted but could not be fetched".to_string())?;

    Ok(inserted_book)
}

fn extract_book_metadata(file_path: &str, extension: &str) -> Result<BookImportMetadata, String> {
    if extension == "epub" {
        let metadata = EpubParser::extract_metadata(file_path).map_err(|e| e.to_string())?;
        let _language = metadata.language.clone();
        let _isbn = metadata.isbn.clone();
        return Ok(BookImportMetadata {
            title: metadata.title,
            author: metadata.author,
        });
    }

    let fallback_title = fallback_title_from_path(Path::new(file_path));

    let author = if extension == "cbz" {
        cbz_parent_directory_label(Path::new(file_path))
    } else {
        None
    };

    Ok(BookImportMetadata {
        title: fallback_title,
        author,
    })
}

fn cbz_parent_directory_label(path: &Path) -> Option<String> {
    let parent_name = path.parent()?.file_name()?.to_str()?.trim().to_string();
    if parent_name.is_empty() {
        return None;
    }
    Some(parent_name)
}

fn fallback_title_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Unknown Title".to_string())
}

fn compute_sha256(file_path: &str) -> anyhow::Result<String> {
    let mut file = File::open(file_path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

#[tauri::command]
pub async fn list_books(state: State<'_, AppState>) -> Result<Vec<Book>, String> {
    let repository = BookRepository::new(&state._db_pool);
    let mut books = repository.list_all().await.map_err(|e| e.to_string())?;

    for book in &mut books {
        let is_unknown =
            book.title.trim().is_empty() || book.title.eq_ignore_ascii_case("Unknown Title");

        if is_unknown {
            let fallback = Path::new(&book.file_path)
                .file_stem()
                .and_then(|value| value.to_str())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "Unknown Title".to_string());

            book.title = fallback;
        }
    }

    Ok(books)
}

#[tauri::command]
pub async fn remove_book(
    book_id: i64,
    delete_file: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let repository = BookRepository::new(&state._db_pool);

    let existing = repository
        .find_by_id(book_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Book not found".to_string())?;

    repository
        .delete_by_id(book_id)
        .await
        .map_err(|e| e.to_string())?;

    if delete_file {
        let file_path = Path::new(&existing.file_path);

        if file_path.exists() {
            if !file_path.is_file() {
                return Err("Book removed from library, but target path is not a file".to_string());
            }

            std::fs::remove_file(file_path).map_err(|error| {
                format!(
                    "Book removed from library, but failed to delete file '{}': {}",
                    existing.file_path, error
                )
            })?;
        }
    }

    Ok(())
}
