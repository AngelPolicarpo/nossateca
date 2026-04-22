use std::time::Duration;

use tauri::State;
use tokio::task::JoinSet;

use crate::models::{
    AddonRole, MangaChapterGroup, PluginErrorKind, PluginTypedError,
};
use crate::plugins::PluginManager;
use crate::AppState;

#[tauri::command]
pub async fn list_manga_chapters(
    item_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<MangaChapterGroup>, PluginTypedError> {
    let normalized_item_id = item_id.trim().to_string();
    if normalized_item_id.is_empty() {
        return Err(not_found_error("item id is required"));
    }

    let snapshot = state
        .plugin_manager
        .lock()
        .map_err(|_| unknown_error("failed to lock plugin manager"))?
        .runtime_snapshot();

    let manga_plugins = snapshot
        .plugins
        .into_iter()
        .filter(|plugin| plugin.role == AddonRole::MangaSource && plugin.enabled)
        .collect::<Vec<_>>();

    if manga_plugins.is_empty() {
        return Ok(Vec::new());
    }

    let timeout = resolve_manga_timeout();
    let mut join_set = JoinSet::new();

    for plugin in manga_plugins {
        let engine = snapshot.engine.clone();
        let fuel_per_invocation = snapshot.fuel_per_invocation;
        let plugin_id = plugin.id.clone();
        let item_id_cloned = normalized_item_id.clone();

        join_set.spawn(async move {
            let info_worker = tokio::task::spawn_blocking({
                let engine = engine.clone();
                let plugin = plugin.clone();
                move || {
                    PluginManager::execute_manga_get_source_info(
                        &engine,
                        fuel_per_invocation,
                        &plugin,
                    )
                }
            });

            let info = match tokio::time::timeout(timeout, info_worker).await {
                Ok(Ok(Ok(info))) => info,
                Ok(Ok(Err(err))) => {
                    return MangaChapterGroup {
                        plugin_id: plugin_id.clone(),
                        source_name: plugin_id.clone(),
                        source_id: plugin_id.clone(),
                        chapters: Vec::new(),
                        error: Some(err),
                    };
                }
                Ok(Err(err)) => {
                    return MangaChapterGroup {
                        plugin_id: plugin_id.clone(),
                        source_name: plugin_id.clone(),
                        source_id: plugin_id.clone(),
                        chapters: Vec::new(),
                        error: Some(unknown_error(format!(
                            "manga plugin '{}' join error: {}",
                            plugin_id, err
                        ))),
                    };
                }
                Err(_) => {
                    return MangaChapterGroup {
                        plugin_id: plugin_id.clone(),
                        source_name: plugin_id.clone(),
                        source_id: plugin_id.clone(),
                        chapters: Vec::new(),
                        error: Some(timeout_error(format!(
                            "manga plugin '{}' timed out on info",
                            plugin_id
                        ))),
                    };
                }
            };

            let chapters_worker = tokio::task::spawn_blocking({
                let engine = engine.clone();
                let plugin = plugin.clone();
                move || {
                    PluginManager::execute_manga_list_chapters(
                        &engine,
                        fuel_per_invocation,
                        &plugin,
                        &item_id_cloned,
                    )
                }
            });

            match tokio::time::timeout(timeout, chapters_worker).await {
                Ok(Ok(Ok(chapters))) => MangaChapterGroup {
                    plugin_id: info.plugin_id.clone(),
                    source_name: info.source_name,
                    source_id: info.source_id,
                    chapters,
                    error: None,
                },
                Ok(Ok(Err(err))) => MangaChapterGroup {
                    plugin_id: info.plugin_id.clone(),
                    source_name: info.source_name,
                    source_id: info.source_id,
                    chapters: Vec::new(),
                    error: Some(err),
                },
                Ok(Err(err)) => MangaChapterGroup {
                    plugin_id: info.plugin_id.clone(),
                    source_name: info.source_name,
                    source_id: info.source_id,
                    chapters: Vec::new(),
                    error: Some(unknown_error(format!(
                        "manga plugin '{}' join error: {}",
                        plugin_id, err
                    ))),
                },
                Err(_) => MangaChapterGroup {
                    plugin_id: info.plugin_id.clone(),
                    source_name: info.source_name,
                    source_id: info.source_id,
                    chapters: Vec::new(),
                    error: Some(timeout_error(format!(
                        "manga plugin '{}' timed out",
                        plugin_id
                    ))),
                },
            }
        });
    }

    let mut groups = Vec::new();
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(group) => groups.push(group),
            Err(err) => {
                groups.push(MangaChapterGroup {
                    plugin_id: "unknown".to_string(),
                    source_name: "Unknown".to_string(),
                    source_id: "unknown".to_string(),
                    chapters: Vec::new(),
                    error: Some(unknown_error(format!("manga join error: {}", err))),
                });
            }
        }
    }

    groups.sort_by(|left, right| {
        left.source_name
            .to_ascii_lowercase()
            .cmp(&right.source_name.to_ascii_lowercase())
            .then_with(|| left.plugin_id.cmp(&right.plugin_id))
    });

    Ok(groups)
}

fn resolve_manga_timeout() -> Duration {
    let timeout_ms = std::env::var("LEXICON_MANGA_PLUGIN_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(45_000)
        .clamp(1_000, 300_000);

    Duration::from_millis(timeout_ms)
}

fn unknown_error(message: impl Into<String>) -> PluginTypedError {
    PluginTypedError {
        kind: PluginErrorKind::Unknown,
        message: message.into(),
    }
}

fn timeout_error(message: impl Into<String>) -> PluginTypedError {
    PluginTypedError {
        kind: PluginErrorKind::NetworkFailure,
        message: message.into(),
    }
}

fn not_found_error(message: impl Into<String>) -> PluginTypedError {
    PluginTypedError {
        kind: PluginErrorKind::NotFound,
        message: message.into(),
    }
}
