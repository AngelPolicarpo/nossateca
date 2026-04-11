use tauri::State;
use uuid::Uuid;

use crate::db::repositories::{AnnotationRepository, BookRepository};
use crate::models::{Annotation, NewAnnotation};
use crate::AppState;

#[tauri::command]
pub async fn add_annotation(
    book_id: String,
    annotation: NewAnnotation,
    state: State<'_, AppState>,
) -> Result<Annotation, String> {
    let parsed_book_id = book_id
        .parse::<i64>()
        .map_err(|_| "Invalid book_id".to_string())?;

    let book_repository = BookRepository::new(&state._db_pool);
    let exists = book_repository
        .find_by_id(parsed_book_id)
        .await
        .map_err(|e| e.to_string())?
        .is_some();

    if !exists {
        return Err("Book not found".to_string());
    }

    let annotation_repository = AnnotationRepository::new(&state._db_pool);
    let annotation_id = Uuid::new_v4().to_string();

    annotation_repository
        .insert(&annotation_id, parsed_book_id, &annotation)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_annotations(
    book_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<Annotation>, String> {
    let parsed_book_id = book_id
        .parse::<i64>()
        .map_err(|_| "Invalid book_id".to_string())?;

    let annotation_repository = AnnotationRepository::new(&state._db_pool);

    annotation_repository
        .list_by_book(parsed_book_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_annotation_note(
    id: String,
    note_text: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let annotation_repository = AnnotationRepository::new(&state._db_pool);

    annotation_repository
        .update_note(&id, &note_text)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_annotation_color(
    id: String,
    color: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let allowed = ["yellow", "green", "blue", "pink", "purple"];
    if !allowed.contains(&color.as_str()) {
        return Err("Invalid highlight color".to_string());
    }

    let annotation_repository = AnnotationRepository::new(&state._db_pool);

    annotation_repository
        .update_color(&id, &color)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_annotation(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let annotation_repository = AnnotationRepository::new(&state._db_pool);

    annotation_repository
        .delete(&id)
        .await
        .map_err(|e| e.to_string())
}
