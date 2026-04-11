use std::time::Duration;

use tauri::State;
use tokio::task::JoinSet;

use crate::models::{
    AddonRole, DiscoverCatalog, DiscoverCatalogPageResponse, DiscoverItemDetails, PluginErrorKind,
    PluginTypedError, SourceSearchResultGroup,
};
use crate::plugins::PluginManager;
use crate::AppState;

const DEFAULT_PAGE_SIZE: u32 = 24;
const MAX_PAGE_SIZE: u32 = 100;

#[tauri::command]
pub async fn list_discover_catalogs(
    state: State<'_, AppState>,
) -> Result<Vec<DiscoverCatalog>, PluginTypedError> {
    let snapshot = state
        .plugin_manager
        .lock()
        .map_err(|_| unknown_error("failed to lock plugin manager"))?
        .runtime_snapshot();

    let discover_plugins = snapshot
        .plugins
        .into_iter()
        .filter(|plugin| plugin.role == AddonRole::Discover)
        .collect::<Vec<_>>();

    if discover_plugins.is_empty() {
        return Ok(Vec::new());
    }

    let timeout = resolve_discover_timeout();
    let mut join_set = JoinSet::new();

    for plugin in discover_plugins {
        let engine = snapshot.engine.clone();
        let fuel_per_invocation = snapshot.fuel_per_invocation;
        let plugin_id = plugin.id.clone();

        join_set.spawn(async move {
            let worker = tokio::task::spawn_blocking(move || {
                PluginManager::execute_discover_list_catalogs(&engine, fuel_per_invocation, &plugin)
            });

            match tokio::time::timeout(timeout, worker).await {
                Ok(Ok(Ok(catalogs))) => Ok(catalogs),
                Ok(Ok(Err(err))) => {
                    eprintln!(
                        "[discover] plugin '{}' list catalogs failed: {}",
                        plugin_id, err.message
                    );
                    Err(err)
                }
                Ok(Err(err)) => Err(unknown_error(format!(
                    "discover plugin '{}' join error: {}",
                    plugin_id, err
                ))),
                Err(_) => Err(timeout_error(format!(
                    "discover plugin '{}' timed out",
                    plugin_id
                ))),
            }
        });
    }

    let mut catalogs = Vec::new();

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(mut plugin_catalogs)) => catalogs.append(&mut plugin_catalogs),
            Ok(Err(err)) => {
                eprintln!(
                    "[discover] catalog collection ignored plugin failure: {}",
                    err.message
                );
            }
            Err(err) => {
                eprintln!("[discover] failed to join catalog task: {}", err);
            }
        }
    }

    catalogs.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
            .then_with(|| left.plugin_id.cmp(&right.plugin_id))
    });

    Ok(catalogs)
}

#[tauri::command]
pub async fn list_discover_catalog_items(
    plugin_id: String,
    catalog_id: String,
    skip: Option<u32>,
    page_size: Option<u32>,
    genre: Option<String>,
    year: Option<u32>,
    state: State<'_, AppState>,
) -> Result<DiscoverCatalogPageResponse, PluginTypedError> {
    let normalized_plugin_id = plugin_id.trim();
    if normalized_plugin_id.is_empty() {
        return Err(not_found_error("discover plugin id is required"));
    }

    let normalized_catalog_id = catalog_id.trim();
    if normalized_catalog_id.is_empty() {
        return Err(not_found_error("catalog id is required"));
    }

    let normalized_page_size = page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);

    let (engine, fuel_per_invocation, plugin) = {
        let manager = state
            .plugin_manager
            .lock()
            .map_err(|_| unknown_error("failed to lock plugin manager"))?;

        let plugin = manager
            .plugin_by_id(normalized_plugin_id)
            .ok_or_else(|| not_found_error(format!("discover plugin '{}' not found", normalized_plugin_id)))?;

        if plugin.role != AddonRole::Discover {
            return Err(not_found_error(format!(
                "plugin '{}' is not a discover plugin",
                normalized_plugin_id
            )));
        }

        let snapshot = manager.runtime_snapshot();
        (snapshot.engine, snapshot.fuel_per_invocation, plugin)
    };

    let timeout = resolve_discover_timeout();
    let catalog_id_owned = normalized_catalog_id.to_string();
    let genre_owned = genre.map(|value| value.trim().to_string()).filter(|value| !value.is_empty());

    let worker = tokio::task::spawn_blocking(move || {
        PluginManager::execute_discover_list_catalog_items(
            &engine,
            fuel_per_invocation,
            &plugin,
            &catalog_id_owned,
            skip.unwrap_or(0),
            normalized_page_size,
            genre_owned,
            year,
        )
    });

    match tokio::time::timeout(timeout, worker).await {
        Ok(Ok(result)) => result,
        Ok(Err(err)) => Err(unknown_error(format!("discover worker failed: {}", err))),
        Err(_) => Err(timeout_error("discover catalog query timed out")),
    }
}

#[tauri::command]
pub async fn get_discover_item_details(
    plugin_id: String,
    item_id: String,
    state: State<'_, AppState>,
) -> Result<DiscoverItemDetails, PluginTypedError> {
    let normalized_plugin_id = plugin_id.trim();
    if normalized_plugin_id.is_empty() {
        return Err(not_found_error("discover plugin id is required"));
    }

    let normalized_item_id = item_id.trim();
    if normalized_item_id.is_empty() {
        return Err(not_found_error("item id is required"));
    }

    let (engine, fuel_per_invocation, plugin) = {
        let manager = state
            .plugin_manager
            .lock()
            .map_err(|_| unknown_error("failed to lock plugin manager"))?;

        let plugin = manager
            .plugin_by_id(normalized_plugin_id)
            .ok_or_else(|| not_found_error(format!("discover plugin '{}' not found", normalized_plugin_id)))?;

        if plugin.role != AddonRole::Discover {
            return Err(not_found_error(format!(
                "plugin '{}' is not a discover plugin",
                normalized_plugin_id
            )));
        }

        let snapshot = manager.runtime_snapshot();
        (snapshot.engine, snapshot.fuel_per_invocation, plugin)
    };

    let timeout = resolve_discover_timeout();
    let item_id_owned = normalized_item_id.to_string();

    let worker = tokio::task::spawn_blocking(move || {
        PluginManager::execute_discover_get_item_details(
            &engine,
            fuel_per_invocation,
            &plugin,
            &item_id_owned,
        )
    });

    match tokio::time::timeout(timeout, worker).await {
        Ok(Ok(result)) => result,
        Ok(Err(err)) => Err(unknown_error(format!("discover worker failed: {}", err))),
        Err(_) => Err(timeout_error("discover details query timed out")),
    }
}

#[tauri::command]
pub async fn search_source_downloads(
    title: String,
    author: Option<String>,
    isbn: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<SourceSearchResultGroup>, PluginTypedError> {
    let normalized_title = title.trim().to_string();
    if normalized_title.is_empty() {
        return Err(not_found_error("title is required"));
    }

    let normalized_author = author
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let normalized_isbn = isbn
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let snapshot = state
        .plugin_manager
        .lock()
        .map_err(|_| unknown_error("failed to lock plugin manager"))?
        .runtime_snapshot();

    let source_plugins = snapshot
        .plugins
        .into_iter()
        .filter(|plugin| plugin.role == AddonRole::Source)
        .collect::<Vec<_>>();

    if source_plugins.is_empty() {
        return Ok(Vec::new());
    }

    let timeout = resolve_source_timeout();
    let mut join_set = JoinSet::new();

    for plugin in source_plugins {
        let engine = snapshot.engine.clone();
        let fuel_per_invocation = snapshot.fuel_per_invocation;
        let plugin_id = plugin.id.clone();
        let title = normalized_title.clone();
        let author = normalized_author.clone();
        let isbn = normalized_isbn.clone();

        join_set.spawn(async move {
            let info_worker = tokio::task::spawn_blocking({
                let engine = engine.clone();
                let plugin = plugin.clone();
                move || PluginManager::execute_source_get_info(&engine, fuel_per_invocation, &plugin)
            });

            let info = match tokio::time::timeout(timeout, info_worker).await {
                Ok(Ok(Ok(info))) => info,
                Ok(Ok(Err(err))) => {
                    return SourceSearchResultGroup {
                        plugin_id: plugin_id.clone(),
                        source_name: plugin_id.clone(),
                        source_id: plugin_id.clone(),
                        supported_formats: Vec::new(),
                        results: Vec::new(),
                        error: Some(err),
                    };
                }
                Ok(Err(err)) => {
                    return SourceSearchResultGroup {
                        plugin_id: plugin_id.clone(),
                        source_name: plugin_id.clone(),
                        source_id: plugin_id.clone(),
                        supported_formats: Vec::new(),
                        results: Vec::new(),
                        error: Some(unknown_error(format!(
                            "source plugin '{}' join error: {}",
                            plugin_id, err
                        ))),
                    };
                }
                Err(_) => {
                    return SourceSearchResultGroup {
                        plugin_id: plugin_id.clone(),
                        source_name: plugin_id.clone(),
                        source_id: plugin_id.clone(),
                        supported_formats: Vec::new(),
                        results: Vec::new(),
                        error: Some(timeout_error(format!(
                            "source plugin '{}' timed out while loading metadata",
                            plugin_id
                        ))),
                    };
                }
            };

            let downloads_worker = tokio::task::spawn_blocking({
                let engine = engine.clone();
                let plugin = plugin.clone();
                move || {
                    PluginManager::execute_source_find_downloads(
                        &engine,
                        fuel_per_invocation,
                        &plugin,
                        &title,
                        author,
                        isbn,
                    )
                }
            });

            match tokio::time::timeout(timeout, downloads_worker).await {
                Ok(Ok(Ok(results))) => SourceSearchResultGroup {
                    plugin_id: info.plugin_id.clone(),
                    source_name: info.source_name,
                    source_id: info.source_id,
                    supported_formats: info.supported_formats,
                    results,
                    error: None,
                },
                Ok(Ok(Err(err))) => SourceSearchResultGroup {
                    plugin_id: info.plugin_id.clone(),
                    source_name: info.source_name,
                    source_id: info.source_id,
                    supported_formats: info.supported_formats,
                    results: Vec::new(),
                    error: Some(err),
                },
                Ok(Err(err)) => SourceSearchResultGroup {
                    plugin_id: info.plugin_id.clone(),
                    source_name: info.source_name,
                    source_id: info.source_id,
                    supported_formats: info.supported_formats,
                    results: Vec::new(),
                    error: Some(unknown_error(format!(
                        "source plugin '{}' join error: {}",
                        plugin_id, err
                    ))),
                },
                Err(_) => SourceSearchResultGroup {
                    plugin_id: info.plugin_id.clone(),
                    source_name: info.source_name,
                    source_id: info.source_id,
                    supported_formats: info.supported_formats,
                    results: Vec::new(),
                    error: Some(timeout_error(format!(
                        "source plugin '{}' timed out",
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
                groups.push(SourceSearchResultGroup {
                    plugin_id: "unknown".to_string(),
                    source_name: "Unknown Source".to_string(),
                    source_id: "unknown".to_string(),
                    supported_formats: Vec::new(),
                    results: Vec::new(),
                    error: Some(unknown_error(format!("failed to join source task: {}", err))),
                });
            }
        }
    }

    groups.sort_by(|left, right| {
        left
            .source_name
            .to_ascii_lowercase()
            .cmp(&right.source_name.to_ascii_lowercase())
            .then_with(|| left.plugin_id.cmp(&right.plugin_id))
    });

    Ok(groups)
}

fn resolve_discover_timeout() -> Duration {
    resolve_timeout_from_env("LEXICON_DISCOVER_PLUGIN_TIMEOUT_MS", 15_000)
}

fn resolve_source_timeout() -> Duration {
    resolve_timeout_from_env("LEXICON_SOURCE_PLUGIN_TIMEOUT_MS", 15_000)
}

fn resolve_timeout_from_env(env_key: &str, fallback_ms: u64) -> Duration {
    let timeout_ms = std::env::var(env_key)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(fallback_ms)
        .clamp(1_000, 120_000);

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
