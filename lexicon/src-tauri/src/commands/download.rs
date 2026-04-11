use tauri::State;

use crate::models::DownloadRecord;
use crate::AppState;

#[tauri::command]
pub async fn start_download(
    source_url: String,
    file_name: Option<String>,
    state: State<'_, AppState>,
) -> Result<DownloadRecord, String> {
    state
        .download_manager
        .start_download(source_url, file_name)
        .await
}

#[tauri::command]
pub async fn pause_download(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.download_manager.pause_download(id).await
}

#[tauri::command]
pub async fn resume_download(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.download_manager.resume_download(id).await
}

#[tauri::command]
pub async fn cancel_download(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.download_manager.cancel_download(id).await
}

#[tauri::command]
pub async fn remove_download(
    id: String,
    delete_file: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.download_manager.remove_download(id, delete_file).await
}

#[tauri::command]
pub async fn list_downloads(state: State<'_, AppState>) -> Result<Vec<DownloadRecord>, String> {
    state.download_manager.list_downloads().await
}
