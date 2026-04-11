use tauri::State;

use crate::models::SearchBookResult;
use crate::search::SearchOrchestrator;
use crate::AppState;

#[tauri::command]
pub async fn search_books(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<SearchBookResult>, String> {
    let normalized_query = query.trim().to_string();
    if normalized_query.is_empty() {
        return Ok(Vec::new());
    }

    let orchestrator = SearchOrchestrator::new(state.plugin_manager.clone());

    orchestrator.search_books(&normalized_query).await
}
