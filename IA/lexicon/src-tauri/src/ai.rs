use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use tauri::State;
use tokio::task;
use uuid::Uuid;

use crate::ai::output_sanitizer::{
    sanitize_output as sanitize_model_output, OutputSanitizerConfig,
};
use crate::ai::{chunk_literary_text, EmbeddingEngine, LlmEngine, RagEngine, RagQueryResult};
use crate::db::repositories::BookRepository;
use crate::models::{AiSettings, AiSetupInfo, BookIndexProgress, ChatAnswer, ChatMessage};
use crate::reader::EpubParser;
use crate::AppState;

const PROGRESS_STATUS_CHUNKING: &str = "chunking";
const PROGRESS_STATUS_SUMMARIZING_CHAPTERS: &str = "summarizing_chapters";
const PROGRESS_STATUS_SUMMARIZING_BOOK: &str = "summarizing_book";

#[tauri::command]
pub async fn index_book(book_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let parsed_book_id = book_id
        .parse::<i64>()
        .map_err(|_| "Invalid book_id".to_string())?;

    let book_repository = BookRepository::new(&state._db_pool);
    let book = book_repository
        .find_by_id(parsed_book_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Book not found".to_string())?;

    spawn_indexing_job(parsed_book_id, book.file_path, state._db_pool.clone());
    Ok(())
}

pub fn spawn_indexing_job(book_id: i64, book_file_path: String, pool: sqlx::SqlitePool) {
    tauri::async_runtime::spawn(async move {
        if let Err(err) = run_indexing_job(book_id, &book_file_path, &pool).await {
            eprintln!(
                "[spawn_indexing_job] unexpected indexing error for book {}: {}",
                book_id, err
            );
        }
    });
}

async fn run_indexing_job(
    book_id: i64,
    book_file_path: &str,
    pool: &sqlx::SqlitePool,
) -> anyhow::Result<()> {
    set_book_status(pool, book_id, "indexing").await?;
    upsert_index_progress(
        pool,
        book_id,
        "indexing",
        0,
        0,
        None,
        "Preparando indexação...",
    )
    .await?;

    match index_book_internal(book_id, book_file_path, pool).await {
        Ok(()) => {
            set_book_status(pool, book_id, "indexed").await?;
        }
        Err(err) => {
            let message = err.to_string();
            set_book_status(pool, book_id, "error").await?;
            upsert_index_progress(pool, book_id, "error", 0, 0, None, &message).await?;
        }
    }

    Ok(())
}

pub async fn index_book_internal(
    parsed_book_id: i64,
    book_file_path: &str,
    pool: &sqlx::SqlitePool,
) -> anyhow::Result<()> {
    let settings = get_ai_settings_internal(pool).await?;

    let embedding_engine = EmbeddingEngine::new(&settings.ai_embedding_path)?;
    let llm_engine = LlmEngine::new(&settings.ai_model_path).ok();

    let parser = EpubParser::new(book_file_path);
    let spine = parser.get_spine();
    let total_chapters = spine.len();

    if total_chapters == 0 {
        anyhow::bail!("EPUB has no readable chapters for indexing");
    }

    upsert_index_progress(
        pool,
        parsed_book_id,
        PROGRESS_STATUS_CHUNKING,
        0,
        total_chapters as i64,
        None,
        &format!("Indexando trechos (capítulo 0 de {})...", total_chapters),
    )
    .await?;

    sqlx::query("DELETE FROM book_embeddings WHERE book_id = ?")
        .bind(parsed_book_id)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM book_chunks WHERE book_id = ?")
        .bind(parsed_book_id)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM chapter_summaries WHERE book_id = ?")
        .bind(parsed_book_id)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM book_summaries WHERE book_id = ?")
        .bind(parsed_book_id)
        .execute(pool)
        .await?;

    let mut inserted_chunks = 0usize;
    let mut chapter_texts: Vec<(usize, String)> = Vec::new();
    let started = Instant::now();
    let mut processed_tokens = 0usize;

    for (chapter_index, chapter_id) in spine.iter().enumerate() {
        let chapter_ordinal = chapter_index + 1;

        let chapter_html = match parser.get_chapter_content(chapter_id) {
            Ok(content) => content,
            Err(err) => {
                eprintln!(
                    "[index_book_internal] skipping unreadable chapter '{}' for book {}: {}",
                    chapter_id, parsed_book_id, err
                );
                let eta_seconds = estimate_eta_seconds(
                    &started,
                    processed_tokens,
                    chapter_ordinal,
                    total_chapters,
                );
                let message = format!(
                    "Indexando trechos (capítulo {} de {})...",
                    chapter_ordinal, total_chapters
                );
                upsert_index_progress(
                    pool,
                    parsed_book_id,
                    PROGRESS_STATUS_CHUNKING,
                    chapter_ordinal as i64,
                    total_chapters as i64,
                    eta_seconds,
                    &message,
                )
                .await?;
                continue;
            }
        };

        let chapter_text = strip_html_tags(&chapter_html);
        if chapter_text.trim().is_empty() {
            let eta_seconds =
                estimate_eta_seconds(&started, processed_tokens, chapter_ordinal, total_chapters);
            let message = format!(
                "Indexando trechos (capítulo {} de {})...",
                chapter_ordinal, total_chapters
            );
            upsert_index_progress(
                pool,
                parsed_book_id,
                PROGRESS_STATUS_CHUNKING,
                chapter_ordinal as i64,
                total_chapters as i64,
                eta_seconds,
                &message,
            )
            .await?;
            continue;
        }

        processed_tokens = processed_tokens.saturating_add(chapter_text.split_whitespace().count());
        chapter_texts.push((chapter_index, chapter_text.clone()));
        let chunks = chunk_literary_text(&chapter_text);

        for chunk in chunks {
            let inserted = sqlx::query_as::<_, (i64,)>(
                "INSERT INTO book_chunks (book_id, chapter_index, chunk_text, char_start, char_end) VALUES (?, ?, ?, ?, ?) RETURNING id",
            )
            .bind(parsed_book_id)
            .bind(chapter_index as i64)
            .bind(&chunk.chunk_text)
            .bind(chunk.char_start as i64)
            .bind(chunk.char_end as i64)
            .fetch_one(pool)
            .await?;

            let embedding_engine = embedding_engine.clone();
            let chunk_text = chunk.chunk_text.clone();
            let embedding =
                task::spawn_blocking(move || embedding_engine.embed_passage(&chunk_text))
                    .await
                    .map_err(|err| anyhow::anyhow!("failed to join embedding task: {err}"))??;
            let embedding_json = serde_json::to_string(&embedding)?;

            sqlx::query(
                "INSERT INTO book_embeddings (embedding_json, chunk_id, book_id) VALUES (?, ?, ?)",
            )
            .bind(embedding_json)
            .bind(inserted.0)
            .bind(parsed_book_id)
            .execute(pool)
            .await?;

            inserted_chunks = inserted_chunks.saturating_add(1);
        }

        let eta_seconds =
            estimate_eta_seconds(&started, processed_tokens, chapter_ordinal, total_chapters);
        let message = format!(
            "Indexando trechos (capítulo {} de {})...",
            chapter_ordinal, total_chapters
        );
        upsert_index_progress(
            pool,
            parsed_book_id,
            PROGRESS_STATUS_CHUNKING,
            chapter_ordinal as i64,
            total_chapters as i64,
            eta_seconds,
            &message,
        )
        .await?;
    }

    if inserted_chunks == 0 {
        anyhow::bail!("No readable textual chapters found for indexing");
    }

    let total_summary_targets = chapter_texts.len();
    upsert_index_progress(
        pool,
        parsed_book_id,
        PROGRESS_STATUS_SUMMARIZING_CHAPTERS,
        0,
        total_summary_targets as i64,
        None,
        &format!(
            "Gerando resumos de capítulos (0 de {})...",
            total_summary_targets
        ),
    )
    .await?;

    let saved_chapter_summaries =
        generate_hierarchical_summaries(parsed_book_id, &chapter_texts, llm_engine.as_ref(), pool)
            .await?;

    upsert_index_progress(
        pool,
        parsed_book_id,
        "indexed",
        total_chapters as i64,
        total_chapters as i64,
        Some(0.0),
        &format!(
            "Indexação concluída com {} chunks e {} resumos de capítulo",
            inserted_chunks, saved_chapter_summaries
        ),
    )
    .await?;

    Ok(())
}

#[tauri::command]
pub async fn chat_with_book(
    book_id: String,
    message: String,
    session_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<ChatAnswer, String> {
    let parsed_book_id = book_id
        .parse::<i64>()
        .map_err(|_| "Invalid book_id".to_string())?;

    let settings = get_ai_settings_internal(&state._db_pool)
        .await
        .map_err(|e| e.to_string())?;

    if !is_ai_configured_internal(&settings) {
        return Err("Configure os modelos de IA nas preferências primeiro".to_string());
    }

    let embedding_engine =
        EmbeddingEngine::new(&settings.ai_embedding_path).map_err(|e| e.to_string())?;
    let llm_engine = LlmEngine::new(&settings.ai_model_path).map_err(|e| e.to_string())?;
    let rag = RagEngine::new(&state._db_pool, embedding_engine, llm_engine);

    let rag_result = rag
        .query(&book_id, &message)
        .await
        .map_err(|e| e.to_string())?;
    let RagQueryResult {
        answer,
        source_level,
        source_label,
    } = rag_result;

    let active_session_id =
        session_id.unwrap_or_else(|| format!("book-{}-default", parsed_book_id));

    sqlx::query(
        "INSERT INTO chat_sessions (id, book_id, title) VALUES (?, ?, ?) ON CONFLICT(id) DO UPDATE SET updated_at = CURRENT_TIMESTAMP",
    )
    .bind(&active_session_id)
    .bind(parsed_book_id)
    .bind("Chat com livro")
    .execute(&state._db_pool)
    .await
    .map_err(|e| e.to_string())?;

    let user_message_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO chat_messages (id, session_id, role, content, source_level) VALUES (?, ?, 'user', ?, NULL)",
    )
    .bind(user_message_id)
    .bind(&active_session_id)
    .bind(&message)
    .execute(&state._db_pool)
    .await
    .map_err(|e| e.to_string())?;

    let assistant_message_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO chat_messages (id, session_id, role, content, source_level) VALUES (?, ?, 'assistant', ?, ?)",
    )
    .bind(assistant_message_id)
    .bind(&active_session_id)
    .bind(&answer)
    .bind(&source_level)
    .execute(&state._db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(ChatAnswer {
        answer,
        source_level,
        source_label,
    })
}

#[tauri::command]
pub async fn get_chat_history(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ChatMessage>, String> {
    let rows = sqlx::query_as::<_, ChatMessage>(
        "SELECT id, session_id, role, content, source_level, created_at FROM chat_messages WHERE session_id = ? ORDER BY created_at ASC",
    )
    .bind(session_id)
    .fetch_all(&state._db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows)
}

#[tauri::command]
pub async fn get_ai_settings(state: State<'_, AppState>) -> Result<AiSettings, String> {
    get_ai_settings_internal(&state._db_pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_ai_settings(
    settings: AiSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validate_llm_model_path(&settings.ai_model_path).map_err(|e| e.to_string())?;
    validate_embedding_model_path(&settings.ai_embedding_path).map_err(|e| e.to_string())?;

    let settings_map = HashMap::from([
        ("ai_model_path", settings.ai_model_path),
        ("ai_embedding_path", settings.ai_embedding_path),
    ]);

    for (key, value) in settings_map {
        sqlx::query(
            "INSERT INTO user_settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP",
        )
        .bind(key)
        .bind(value)
        .execute(&state._db_pool)
        .await
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn get_ai_setup_info(state: State<'_, AppState>) -> Result<AiSetupInfo, String> {
    let settings = get_ai_settings_internal(&state._db_pool)
        .await
        .map_err(|e| e.to_string())?;

    let models_dir = default_models_dir().map_err(|e| e.to_string())?;
    let (detected_gguf, detected_onnx) = detect_models_in_dir(&models_dir);

    let llm_file_size_mb = file_size_mb(Path::new(&settings.ai_model_path));
    let embedding_file_size_mb = file_size_mb(Path::new(&settings.ai_embedding_path));
    let llm_model_type = detect_model_type(&settings.ai_model_path);
    let embedding_model_type = detect_model_type(&settings.ai_embedding_path);

    let llm_configured = llm_file_size_mb.is_some() && llm_model_type.as_deref() == Some("gguf");
    let embedding_configured = embedding_file_size_mb.is_some()
        && matches!(embedding_model_type.as_deref(), Some("onnx") | Some("gguf"));

    Ok(AiSetupInfo {
        ai_model_path: settings.ai_model_path,
        ai_embedding_path: settings.ai_embedding_path,
        llm_model_type,
        embedding_model_type,
        default_models_dir: models_dir.to_string_lossy().to_string(),
        detected_gguf,
        detected_onnx,
        llm_configured,
        embedding_configured,
        llm_file_size_mb,
        embedding_file_size_mb,
    })
}

#[tauri::command]
pub async fn get_book_index_progress(
    book_id: String,
    state: State<'_, AppState>,
) -> Result<Option<BookIndexProgress>, String> {
    let parsed_book_id = book_id
        .parse::<i64>()
        .map_err(|_| "Invalid book_id".to_string())?;

    let row = sqlx::query_as::<_, BookIndexProgress>(
        "SELECT book_id, status, current_chapter, total_chapters, eta_seconds, message, updated_at
         FROM book_index_progress WHERE book_id = ? LIMIT 1",
    )
    .bind(parsed_book_id)
    .fetch_optional(&state._db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(row)
}

#[tauri::command]
pub fn ensure_models_directory() -> Result<String, String> {
    let dir = ensure_models_dir_exists().map_err(|e| e.to_string())?;
    Ok(dir.to_string_lossy().to_string())
}

async fn get_ai_settings_internal(pool: &sqlx::SqlitePool) -> anyhow::Result<AiSettings> {
    let rows = sqlx::query_as::<_, (String, String)>("SELECT key, value FROM user_settings")
        .fetch_all(pool)
        .await?;

    let mut ai_model_path = String::new();
    let mut ai_embedding_path = String::new();

    for (key, value) in rows {
        match key.as_str() {
            "ai_model_path" => ai_model_path = value,
            "ai_embedding_path" => ai_embedding_path = value,
            _ => {}
        }
    }

    Ok(AiSettings {
        ai_model_path,
        ai_embedding_path,
    })
}

fn is_ai_configured_internal(settings: &AiSettings) -> bool {
    let llm_ok = validate_llm_model_path(&settings.ai_model_path).is_ok();
    let embedding_ok = validate_embedding_model_path(&settings.ai_embedding_path).is_ok();
    llm_ok && embedding_ok
}

fn validate_llm_model_path(path: &str) -> anyhow::Result<()> {
    validate_model_path_extensions(path, &["gguf"])?;

    let file_name = Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_ascii_lowercase())
        .unwrap_or_default();

    if file_name.contains("mmproj") {
        anyhow::bail!(
            "Arquivo GGUF inválido para chat: mmproj é apenas projeção multimodal. Use um modelo LLM GGUF de texto."
        );
    }

    Ok(())
}

fn validate_embedding_model_path(path: &str) -> anyhow::Result<()> {
    validate_model_path_extensions(path, &["onnx", "gguf"])
}

fn validate_model_path_extensions(path: &str, expected_extensions: &[&str]) -> anyhow::Result<()> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        anyhow::bail!("Configure os modelos de IA nas preferências primeiro");
    }

    let file_path = Path::new(trimmed);
    if !file_path.exists() {
        anyhow::bail!("Arquivo não encontrado no caminho especificado");
    }

    if !file_path.is_file() {
        anyhow::bail!("Arquivo não encontrado no caminho especificado");
    }

    let extension = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_default();

    let is_expected = expected_extensions
        .iter()
        .any(|expected| extension == *expected);

    if !is_expected {
        let allowed = expected_extensions
            .iter()
            .map(|ext| format!(".{}", ext))
            .collect::<Vec<String>>()
            .join(" ou ");
        anyhow::bail!("Extensão inválida. Esperado arquivo {}", allowed);
    }

    Ok(())
}

fn default_models_dir() -> anyhow::Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let appdata =
            std::env::var("APPDATA").map_err(|_| anyhow::anyhow!("APPDATA não definido"))?;
        return Ok(PathBuf::from(appdata).join("lexicon").join("models"));
    }

    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").map_err(|_| anyhow::anyhow!("HOME não definido"))?;
        return Ok(PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("lexicon")
            .join("models"));
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let home = std::env::var("HOME").map_err(|_| anyhow::anyhow!("HOME não definido"))?;
        Ok(PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("lexicon")
            .join("models"))
    }
}

fn ensure_models_dir_exists() -> anyhow::Result<PathBuf> {
    let dir = default_models_dir()?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn detect_models_in_dir(dir: &Path) -> (Vec<String>, Vec<String>) {
    if !dir.exists() {
        return (Vec::new(), Vec::new());
    }

    let mut gguf = Vec::new();
    let mut onnx = Vec::new();

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return (Vec::new(), Vec::new()),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase());

        match ext.as_deref() {
            Some("gguf") => gguf.push(path.to_string_lossy().to_string()),
            Some("onnx") => onnx.push(path.to_string_lossy().to_string()),
            _ => {}
        }
    }

    gguf.sort();
    onnx.sort();

    (gguf, onnx)
}

fn file_size_mb(path: &Path) -> Option<u64> {
    if !path.exists() || !path.is_file() {
        return None;
    }

    let size_bytes = path.metadata().ok()?.len();
    Some(size_bytes / (1024 * 1024))
}

fn detect_model_type(path: &str) -> Option<String> {
    let extension = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())?;

    match extension.as_str() {
        "gguf" => Some("gguf".to_string()),
        "onnx" => Some("onnx".to_string()),
        _ => None,
    }
}

fn estimate_eta_seconds(
    started: &Instant,
    processed_tokens: usize,
    processed_chapters: usize,
    total_chapters: usize,
) -> Option<f64> {
    if total_chapters <= processed_chapters {
        return Some(0.0);
    }

    if processed_tokens == 0 || processed_chapters == 0 {
        return None;
    }

    let elapsed = started.elapsed().as_secs_f64();
    if elapsed <= 0.0 {
        return None;
    }

    let tokens_per_chapter = processed_tokens as f64 / processed_chapters as f64;
    let estimated_total_tokens = tokens_per_chapter * total_chapters as f64;
    let remaining_tokens = (estimated_total_tokens - processed_tokens as f64).max(0.0);
    let seconds_per_token = elapsed / processed_tokens as f64;

    Some(remaining_tokens * seconds_per_token)
}

fn estimate_eta_by_items(
    started: &Instant,
    processed_items: usize,
    total_items: usize,
) -> Option<f64> {
    if total_items <= processed_items {
        return Some(0.0);
    }

    if processed_items == 0 {
        return None;
    }

    let elapsed = started.elapsed().as_secs_f64();
    if elapsed <= 0.0 {
        return None;
    }

    let items_per_second = processed_items as f64 / elapsed;
    if items_per_second <= 0.0 {
        return None;
    }

    let remaining_items = (total_items - processed_items) as f64;
    Some(remaining_items / items_per_second)
}

async fn set_book_status(
    pool: &sqlx::SqlitePool,
    book_id: i64,
    status: &str,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE books SET status = ? WHERE id = ?")
        .bind(status)
        .bind(book_id)
        .execute(pool)
        .await?;

    Ok(())
}

async fn upsert_index_progress(
    pool: &sqlx::SqlitePool,
    book_id: i64,
    status: &str,
    current_chapter: i64,
    total_chapters: i64,
    eta_seconds: Option<f64>,
    message: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO book_index_progress (book_id, status, current_chapter, total_chapters, eta_seconds, message)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(book_id) DO UPDATE SET
            status = excluded.status,
            current_chapter = excluded.current_chapter,
            total_chapters = excluded.total_chapters,
            eta_seconds = excluded.eta_seconds,
            message = excluded.message,
            updated_at = CURRENT_TIMESTAMP",
    )
    .bind(book_id)
    .bind(status)
    .bind(current_chapter)
    .bind(total_chapters)
    .bind(eta_seconds)
    .bind(message)
    .execute(pool)
    .await?;

    Ok(())
}

async fn generate_hierarchical_summaries(
    book_id: i64,
    chapter_texts: &[(usize, String)],
    llm_engine: Option<&LlmEngine>,
    pool: &sqlx::SqlitePool,
) -> anyhow::Result<usize> {
    if chapter_texts.is_empty() {
        return Ok(0);
    }

    let mut summaries: Vec<(usize, String)> = Vec::new();
    let started = Instant::now();
    let total_chapters = chapter_texts.len();

    for (position, (chapter_index, chapter_text)) in chapter_texts.iter().enumerate() {
        let summary = summarize_chapter_text(*chapter_index, chapter_text, llm_engine).await;

        if summary.trim().is_empty() {
            let processed = position + 1;
            let eta_seconds = estimate_eta_by_items(&started, processed, total_chapters);
            let message = format!(
                "Gerando resumos de capítulos ({} de {})...",
                processed, total_chapters
            );

            upsert_index_progress(
                pool,
                book_id,
                PROGRESS_STATUS_SUMMARIZING_CHAPTERS,
                processed as i64,
                total_chapters as i64,
                eta_seconds,
                &message,
            )
            .await?;

            continue;
        }

        sqlx::query(
            "INSERT INTO chapter_summaries (book_id, chapter_index, summary_text)
             VALUES (?, ?, ?)
             ON CONFLICT(book_id, chapter_index) DO UPDATE SET
                summary_text = excluded.summary_text,
                updated_at = CURRENT_TIMESTAMP",
        )
        .bind(book_id)
        .bind(*chapter_index as i64)
        .bind(&summary)
        .execute(pool)
        .await?;

        summaries.push((*chapter_index, summary));

        let processed = position + 1;
        let eta_seconds = estimate_eta_by_items(&started, processed, total_chapters);
        let message = format!(
            "Gerando resumos de capítulos ({} de {})...",
            processed, total_chapters
        );

        upsert_index_progress(
            pool,
            book_id,
            PROGRESS_STATUS_SUMMARIZING_CHAPTERS,
            processed as i64,
            total_chapters as i64,
            eta_seconds,
            &message,
        )
        .await?;
    }

    let saved_chapter_summaries = summaries.len();

    if summaries.is_empty() {
        upsert_index_progress(
            pool,
            book_id,
            PROGRESS_STATUS_SUMMARIZING_BOOK,
            1,
            1,
            Some(0.0),
            "Nenhum resumo de capítulo foi gerado; pulando visão geral do livro.",
        )
        .await?;

        return Ok(0);
    }

    summaries.sort_by_key(|(chapter_index, _)| *chapter_index);
    let merged_summaries = summaries
        .iter()
        .map(|(chapter_index, summary)| format!("Capítulo {}: {}", chapter_index + 1, summary))
        .collect::<Vec<String>>()
        .join("\n\n");

    upsert_index_progress(
        pool,
        book_id,
        PROGRESS_STATUS_SUMMARIZING_BOOK,
        0,
        1,
        None,
        "Gerando visão geral do livro...",
    )
    .await?;

    let book_summary = summarize_book_text(&merged_summaries, llm_engine).await;
    if !book_summary.trim().is_empty() {
        sqlx::query(
            "INSERT INTO book_summaries (book_id, summary_text)
             VALUES (?, ?)
             ON CONFLICT(book_id) DO UPDATE SET
                summary_text = excluded.summary_text,
                updated_at = CURRENT_TIMESTAMP",
        )
        .bind(book_id)
        .bind(book_summary)
        .execute(pool)
        .await?;
    }

    upsert_index_progress(
        pool,
        book_id,
        PROGRESS_STATUS_SUMMARIZING_BOOK,
        1,
        1,
        Some(0.0),
        "Visão geral do livro concluída.",
    )
    .await?;

    Ok(saved_chapter_summaries)
}

async fn summarize_chapter_text(
    _chapter_index: usize,
    chapter_text: &str,
    llm_engine: Option<&LlmEngine>,
) -> String {
    let truncated = trim_to_chars(chapter_text, 9000);
    let prompt = format!(
        "Você é um sistema de sumarização de textos literários.\n\nTarefa: Gerar um resumo fiel e preciso do capítulo fornecido.\n\nRegras obrigatórias:\n- Use APENAS as informações presentes no texto.\n- NÃO invente eventos, personagens ou detalhes ausentes.\n- NÃO repita frases ou ideias — cada sentença deve acrescentar informação nova.\n- Se o texto estiver corrompido, incoerente ou ilegível, responda exatamente: TEXTO_INVÁLIDO\n- Ignore trechos claramente quebrados (OCR corrompido, símbolos aleatórios).\n- Termine sempre com ponto final.\n\nFormato:\n- 1 parágrafo coeso\n- Máximo 200 palavras\n- Linguagem clara e objetiva\n- Mesmo idioma do texto original\n- Responda diretamente com o resumo, sem introduções, rótulos ou raciocínio interno.\n\nTexto do capítulo:\n{}",
        truncated,
    );

    if let Some(llm) = llm_engine {
        if let Ok(summary) = generate_with_llm(llm, prompt, 260).await {
            if !summary.trim().is_empty() {
                let clean_summary = sanitize_summary_for_storage(&summary);
                if clean_summary.trim().is_empty() {
                    return extractive_summary(chapter_text, 200);
                }

                if is_low_quality_summary(&clean_summary) {
                    return extractive_summary(chapter_text, 200);
                }

                return clean_summary;
            }
        }
    }

    extractive_summary(chapter_text, 200)
}

async fn summarize_book_text(merged_summaries: &str, llm_engine: Option<&LlmEngine>) -> String {
    let truncated = trim_to_chars(merged_summaries, 12000);
    let prompt = format!(
        "Com base nos resumos de capítulos abaixo, gere uma visão geral do livro em até 500 tokens, preservando linha narrativa e temas centrais.\n\n{}",
        truncated,
    );

    if let Some(llm) = llm_engine {
        if let Ok(summary) = generate_with_llm(llm, prompt, 500).await {
            let clean_summary = sanitize_summary_for_storage(&summary);
            if !clean_summary.trim().is_empty() {
                return clean_summary;
            }
        }
    }

    extractive_summary(merged_summaries, 500)
}

async fn generate_with_llm(
    llm: &LlmEngine,
    prompt: String,
    max_tokens: i32,
) -> anyhow::Result<String> {
    let llm = llm.clone();
    let answer = task::spawn_blocking(move || llm.generate(&prompt, max_tokens))
        .await
        .map_err(|err| anyhow::anyhow!("failed to join LLM task: {err}"))??;

    Ok(answer)
}

fn extractive_summary(text: &str, max_tokens: usize) -> String {
    let sentences = split_sentences(text);
    if sentences.is_empty() {
        return trim_to_chars(text, max_tokens * 6);
    }

    let mut used_tokens = 0usize;
    let mut selected = Vec::new();

    for sentence in sentences {
        let tokens = sentence.split_whitespace().count();
        if used_tokens + tokens > max_tokens && !selected.is_empty() {
            break;
        }

        used_tokens += tokens;
        selected.push(sentence);
    }

    selected.join(" ")
}

fn is_low_quality_summary(summary: &str) -> bool {
    let text = summary.trim();
    if text.is_empty() {
        return true;
    }

    let lower = text.to_ascii_lowercase();
    if lower.contains("<think")
        || lower.contains("thinking process")
        || lower.contains("analyze the request")
        || lower.contains("token count check")
    {
        return true;
    }

    let tokens: Vec<String> = text
        .split_whitespace()
        .map(normalize_token)
        .filter(|token| !token.is_empty())
        .collect();

    if tokens.is_empty() {
        return true;
    }

    let total_tokens = tokens.len();
    let unique_tokens = tokens.iter().cloned().collect::<HashSet<String>>().len();
    let unique_ratio = unique_tokens as f32 / total_tokens as f32;

    if total_tokens >= 120 && unique_ratio < 0.22 {
        return true;
    }

    if total_tokens >= 60 && unique_ratio < 0.18 {
        return true;
    }

    let mut frequency: HashMap<String, usize> = HashMap::new();
    for token in &tokens {
        *frequency.entry(token.clone()).or_insert(0) += 1;
    }

    let max_frequency = frequency.values().copied().max().unwrap_or(0);
    if total_tokens >= 30 && max_frequency >= (total_tokens / 3).max(8) {
        return true;
    }

    let numeric_lines = text
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty()
                && trimmed.chars().all(|ch| {
                    ch.is_ascii_digit() || ch == '.' || ch == ',' || ch == '-' || ch == ':'
                })
        })
        .count();

    numeric_lines >= 3
}

fn sanitize_summary_for_storage(summary: &str) -> String {
    let trimmed = summary.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if has_reasoning_leak_markers(trimmed) {
        let cleaned = sanitize_model_output(
            trimmed,
            OutputSanitizerConfig {
                requires_thinking_filter: true,
            },
        );

        return cleaned.trim().to_string();
    }

    trimmed.to_string()
}

fn has_reasoning_leak_markers(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("<think")
        || lower.contains("thinking process")
        || lower.contains("processo de pensamento")
        || lower.contains("analyze the request")
        || lower.contains("análise do pedido")
        || lower.contains("analise do pedido")
        || lower.contains("token count check")
        || lower.contains("contagem de tokens")
}

fn normalize_token(token: &str) -> String {
    token
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase()
}

fn split_sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut chars = text.char_indices().peekable();

    while let Some((idx, ch)) = chars.next() {
        if !matches!(ch, '.' | '!' | '?' | ';' | ':') {
            continue;
        }

        let end = idx + ch.len_utf8();
        let next_is_boundary = chars
            .peek()
            .map(|(_, next)| next.is_whitespace())
            .unwrap_or(true);

        if !next_is_boundary {
            continue;
        }

        let sentence = text[start..end].trim();
        if !sentence.is_empty() {
            out.push(sentence.to_string());
        }
        start = end;
    }

    if start < text.len() {
        let sentence = text[start..].trim();
        if !sentence.is_empty() {
            out.push(sentence.to_string());
        }
    }

    out
}

fn trim_to_chars(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        return text.to_string();
    }

    text.chars().take(max_chars).collect::<String>()
}

fn strip_html_tags(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut cursor = 0usize;

    while cursor < input.len() {
        let remaining = &input[cursor..];

        if starts_with_ascii_case_insensitive(remaining, "<!--") {
            if let Some(end) = find_ascii_case_insensitive(remaining, "-->") {
                cursor += end + 3;
            } else {
                break;
            }
            continue;
        }

        if starts_with_ascii_case_insensitive(remaining, "<script") {
            if let Some(open_end) = remaining.find('>') {
                cursor += open_end + 1;
            } else {
                break;
            }

            let rest_after_open = &input[cursor..];
            if let Some(close_rel) = find_ascii_case_insensitive(rest_after_open, "</script>") {
                cursor += close_rel + "</script>".len();
            } else {
                break;
            }

            out.push(' ');
            continue;
        }

        if starts_with_ascii_case_insensitive(remaining, "<style") {
            if let Some(open_end) = remaining.find('>') {
                cursor += open_end + 1;
            } else {
                break;
            }

            let rest_after_open = &input[cursor..];
            if let Some(close_rel) = find_ascii_case_insensitive(rest_after_open, "</style>") {
                cursor += close_rel + "</style>".len();
            } else {
                break;
            }

            out.push(' ');
            continue;
        }

        if remaining.starts_with('<') {
            if let Some(end) = remaining.find('>') {
                cursor += end + 1;
                out.push(' ');
            } else {
                break;
            }
            continue;
        }

        let Some(ch) = remaining.chars().next() else {
            break;
        };

        out.push(ch);
        cursor += ch.len_utf8();
    }

    decode_html_entities(&out)
}

fn starts_with_ascii_case_insensitive(input: &str, prefix: &str) -> bool {
    let input_bytes = input.as_bytes();
    let prefix_bytes = prefix.as_bytes();

    if input_bytes.len() < prefix_bytes.len() {
        return false;
    }

    input_bytes
        .iter()
        .zip(prefix_bytes.iter())
        .take(prefix_bytes.len())
        .all(|(a, b)| a.eq_ignore_ascii_case(b))
}

fn find_ascii_case_insensitive(input: &str, needle: &str) -> Option<usize> {
    let input_bytes = input.as_bytes();
    let needle_bytes = needle.as_bytes();

    if needle_bytes.is_empty() || input_bytes.len() < needle_bytes.len() {
        return None;
    }

    for start in 0..=(input_bytes.len() - needle_bytes.len()) {
        if input_bytes[start..start + needle_bytes.len()]
            .iter()
            .zip(needle_bytes.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
        {
            return Some(start);
        }
    }

    None
}

fn decode_html_entities(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut cursor = 0usize;

    while cursor < input.len() {
        let remaining = &input[cursor..];
        if !remaining.starts_with('&') {
            let Some(ch) = remaining.chars().next() else {
                break;
            };
            out.push(ch);
            cursor += ch.len_utf8();
            continue;
        }

        let Some(end_rel) = remaining.find(';') else {
            out.push('&');
            cursor += 1;
            continue;
        };

        if end_rel > 12 {
            out.push('&');
            cursor += 1;
            continue;
        }

        let entity = &remaining[1..end_rel];
        if let Some(decoded) = decode_single_entity(entity) {
            out.push_str(&decoded);
            cursor += end_rel + 1;
            continue;
        }

        out.push('&');
        cursor += 1;
    }

    normalize_whitespace(&out)
}

fn decode_single_entity(entity: &str) -> Option<String> {
    match entity {
        "nbsp" | "ensp" | "emsp" => return Some(" ".to_string()),
        "amp" => return Some("&".to_string()),
        "quot" => return Some("\"".to_string()),
        "apos" => return Some("'".to_string()),
        "lt" => return Some("<".to_string()),
        "gt" => return Some(">".to_string()),
        "#39" | "#x27" | "#X27" => return Some("'".to_string()),
        _ => {}
    }

    if let Some(hex) = entity
        .strip_prefix("#x")
        .or_else(|| entity.strip_prefix("#X"))
    {
        let value = u32::from_str_radix(hex, 16).ok()?;
        return char::from_u32(value).map(|ch| ch.to_string());
    }

    if let Some(dec) = entity.strip_prefix('#') {
        let value = dec.parse::<u32>().ok()?;
        return char::from_u32(value).map(|ch| ch.to_string());
    }

    None
}

fn normalize_whitespace(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut previous_was_space = false;

    for ch in input.chars() {
        if ch.is_whitespace() {
            if !previous_was_space {
                out.push(' ');
                previous_was_space = true;
            }
        } else {
            out.push(ch);
            previous_was_space = false;
        }
    }

    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::{is_low_quality_summary, sanitize_summary_for_storage, strip_html_tags};

    #[test]
    fn strips_html_script_style_and_decodes_entities() {
        let html = "<p>Anne&nbsp;&amp;&nbsp;Kitty</p><style>.x{color:red;}</style><script>alert(1)</script><!--note--><p>fim</p>";
        let text = strip_html_tags(html);
        assert_eq!(text, "Anne & Kitty fim");
    }

    #[test]
    fn sanitizes_summary_before_storage() {
        let raw = "Resumo válido do capítulo. <think>rascunho interno sem fechamento";
        let cleaned = sanitize_summary_for_storage(raw);
        assert_eq!(cleaned, "Resumo válido do capítulo.");
    }

    #[test]
    fn keeps_literary_summary_as_high_quality() {
        let summary = "O capítulo acompanha Anne em um dia de tensão no esconderijo, destacando conflitos domésticos, medo de descoberta e pequenas rotinas que sustentam sua esperança.";
        assert!(!is_low_quality_summary(summary));
    }
}
