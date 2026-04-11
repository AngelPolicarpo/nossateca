use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tauri::State;
use tokio::fs;

use crate::models::{AddonDescriptor, AddonSettingEntry};
use crate::plugins::PluginManager;
use crate::AppState;

const ADDON_SETTING_PREFIX: &str = "addon::";

#[tauri::command]
pub async fn list_addons(state: State<'_, AppState>) -> Result<Vec<AddonDescriptor>, String> {
    let plugin_manager = state
        .plugin_manager
        .lock()
        .map_err(|_| "failed to lock plugin manager".to_string())?;

    Ok(plugin_manager.list_plugins())
}

#[tauri::command]
pub async fn reload_addons(state: State<'_, AppState>) -> Result<Vec<AddonDescriptor>, String> {
    let mut plugin_manager = state
        .plugin_manager
        .lock()
        .map_err(|_| "failed to lock plugin manager".to_string())?;

    plugin_manager
        .load_plugins()
        .map_err(|err| format!("failed to reload addons: {}", err))?;

    Ok(plugin_manager.list_plugins())
}

#[tauri::command]
pub async fn install_addon(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<AddonDescriptor, String> {
    let normalized_path = file_path.trim();
    if normalized_path.is_empty() {
        return Err("addon file path is required".to_string());
    }

    let source_path = PathBuf::from(normalized_path);
    if !source_path.exists() {
        return Err(format!("addon file not found: {}", source_path.display()));
    }

    if !source_path.is_file() {
        return Err("addon path must be a file".to_string());
    }

    let target_file_name = derive_target_file_name(&source_path)?;
    let target_path = state.plugin_runtime_dir.join(target_file_name);

    if source_path != target_path {
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|err| format!("failed to prepare addon directory: {}", err))?;
        }

        fs::copy(&source_path, &target_path)
            .await
            .map_err(|err| format!("failed to install addon: {}", err))?;
    }

    let addon_id = target_path
        .file_stem()
        .and_then(|value| value.to_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| "failed to resolve addon id".to_string())?;

    let mut plugin_manager = state
        .plugin_manager
        .lock()
        .map_err(|_| "failed to lock plugin manager".to_string())?;

    plugin_manager
        .load_plugins()
        .map_err(|err| format!("failed to reload addons after install: {}", err))?;

    plugin_manager
        .list_plugins()
        .into_iter()
        .find(|addon| addon.id == addon_id)
        .ok_or_else(|| "addon installed but not detected by runtime".to_string())
}

#[tauri::command]
pub async fn remove_addon(addon_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let normalized_id = addon_id.trim().to_string();
    if normalized_id.is_empty() {
        return Err("addon id is required".to_string());
    }

    let addon_path = {
        let plugin_manager = state
            .plugin_manager
            .lock()
            .map_err(|_| "failed to lock plugin manager".to_string())?;

        plugin_manager
            .plugin_by_id(&normalized_id)
            .map(|plugin| plugin.path)
            .ok_or_else(|| format!("addon '{}' not found", normalized_id))?
    };

    if !addon_path.starts_with(&state.plugin_runtime_dir) {
        return Err("addon is outside runtime addons directory".to_string());
    }

    if addon_path.exists() {
        fs::remove_file(&addon_path)
            .await
            .map_err(|err| format!("failed to remove addon file: {}", err))?;
    }

    delete_addon_settings(&state._db_pool, &normalized_id)
        .await
        .map_err(|err| err.to_string())?;

    let mut plugin_manager = state
        .plugin_manager
        .lock()
        .map_err(|_| "failed to lock plugin manager".to_string())?;

    plugin_manager.clear_plugin_settings(&normalized_id);
    plugin_manager
        .load_plugins()
        .map_err(|err| format!("failed to reload addons after removal: {}", err))?;

    Ok(())
}

#[tauri::command]
pub async fn get_addon_settings(
    addon_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<AddonSettingEntry>, String> {
    let normalized_id = addon_id.trim();
    if normalized_id.is_empty() {
        return Err("addon id is required".to_string());
    }

    let mut all_settings = load_all_addon_settings(&state._db_pool)
        .await
        .map_err(|err| err.to_string())?;

    Ok(all_settings.remove(normalized_id).unwrap_or_default())
}

#[tauri::command]
pub async fn update_addon_settings(
    addon_id: String,
    settings: Vec<AddonSettingEntry>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let normalized_id = addon_id.trim().to_string();
    if normalized_id.is_empty() {
        return Err("addon id is required".to_string());
    }

    let normalized_settings = normalize_settings(settings);

    persist_addon_settings(&state._db_pool, &normalized_id, &normalized_settings)
        .await
        .map_err(|err| err.to_string())?;

    let mut plugin_manager = state
        .plugin_manager
        .lock()
        .map_err(|_| "failed to lock plugin manager".to_string())?;

    plugin_manager.set_plugin_settings(&normalized_id, normalized_settings);

    Ok(())
}

pub async fn hydrate_addon_settings_from_db(
    pool: &sqlx::SqlitePool,
    plugin_manager: &mut PluginManager,
) -> Result<(), sqlx::Error> {
    let settings = load_all_addon_settings(pool).await?;
    plugin_manager.set_all_plugin_settings(settings);
    Ok(())
}

async fn load_all_addon_settings(
    pool: &sqlx::SqlitePool,
) -> Result<HashMap<String, Vec<AddonSettingEntry>>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT key, value FROM user_settings WHERE key LIKE ?",
    )
    .bind(format!("{}%", ADDON_SETTING_PREFIX))
    .fetch_all(pool)
    .await?;

    let mut grouped: HashMap<String, Vec<AddonSettingEntry>> = HashMap::new();

    for (key, value) in rows {
        let Some((addon_id, setting_key)) = parse_storage_key(&key) else {
            continue;
        };

        grouped.entry(addon_id).or_default().push(AddonSettingEntry {
            key: setting_key,
            value,
        });
    }

    for settings in grouped.values_mut() {
        settings.sort_by(|left, right| left.key.cmp(&right.key));
    }

    Ok(grouped)
}

async fn persist_addon_settings(
    pool: &sqlx::SqlitePool,
    addon_id: &str,
    settings: &[AddonSettingEntry],
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    let delete_pattern = format!("{}{}::%", ADDON_SETTING_PREFIX, addon_id);
    sqlx::query("DELETE FROM user_settings WHERE key LIKE ?")
        .bind(delete_pattern)
        .execute(&mut *tx)
        .await?;

    for setting in settings {
        let key = build_storage_key(addon_id, &setting.key);
        sqlx::query(
            "INSERT INTO user_settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP",
        )
        .bind(key)
        .bind(setting.value.trim())
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await
}

async fn delete_addon_settings(pool: &sqlx::SqlitePool, addon_id: &str) -> Result<(), sqlx::Error> {
    let delete_pattern = format!("{}{}::%", ADDON_SETTING_PREFIX, addon_id);
    sqlx::query("DELETE FROM user_settings WHERE key LIKE ?")
        .bind(delete_pattern)
        .execute(pool)
        .await?;

    Ok(())
}

fn parse_storage_key(key: &str) -> Option<(String, String)> {
    let stripped = key.strip_prefix(ADDON_SETTING_PREFIX)?;
    let (addon_id, setting_key) = stripped.split_once("::")?;

    let addon_id = addon_id.trim();
    let setting_key = setting_key.trim();

    if addon_id.is_empty() || setting_key.is_empty() {
        return None;
    }

    Some((addon_id.to_string(), setting_key.to_string()))
}

fn build_storage_key(addon_id: &str, key: &str) -> String {
    format!("{}{}::{}", ADDON_SETTING_PREFIX, addon_id.trim(), key.trim())
}

fn derive_target_file_name(path: &Path) -> Result<String, String> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "addon file must have .wasm extension".to_string())?;

    if !extension.eq_ignore_ascii_case("wasm") {
        return Err("addon file must have .wasm extension".to_string());
    }

    let file_stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "addon file name is invalid".to_string())?;

    let sanitized_stem = file_stem
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else if ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();

    let normalized = sanitized_stem.trim_matches('-');
    if normalized.is_empty() {
        return Err("addon file name is invalid".to_string());
    }

    Ok(format!("{}.wasm", normalized))
}

fn normalize_settings(settings: Vec<AddonSettingEntry>) -> Vec<AddonSettingEntry> {
    let mut by_key: HashMap<String, String> = HashMap::new();

    for setting in settings {
        let key = setting.key.trim().to_string();
        if key.is_empty() {
            continue;
        }

        by_key.insert(key, setting.value.trim().to_string());
    }

    let mut normalized = by_key
        .into_iter()
        .map(|(key, value)| AddonSettingEntry { key, value })
        .collect::<Vec<_>>();

    normalized.sort_by(|left, right| left.key.cmp(&right.key));
    normalized
}
