use std::fs;
use std::path::{Path, PathBuf};

use tauri::Manager;

const LEGACY_DATA_DIR_NAME: &str = "com.god.lexicon";
const DATA_DIR_NAME: &str = "lexicon";

pub fn resolve_lexicon_data_dir(app: &tauri::AppHandle) -> anyhow::Result<PathBuf> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|err| anyhow::anyhow!("failed to resolve app data dir: {}", err))?;

    let canonical_data_dir = canonicalize_data_dir(&app_data_dir);

    if canonical_data_dir != app_data_dir {
        migrate_legacy_data_dir(&app_data_dir, &canonical_data_dir)?;
    }

    fs::create_dir_all(&canonical_data_dir)?;

    Ok(canonical_data_dir)
}

fn canonicalize_data_dir(app_data_dir: &Path) -> PathBuf {
    match app_data_dir.file_name().and_then(|name| name.to_str()) {
        Some(LEGACY_DATA_DIR_NAME) => app_data_dir
            .parent()
            .map(|parent| parent.join(DATA_DIR_NAME))
            .unwrap_or_else(|| app_data_dir.to_path_buf()),
        _ => app_data_dir.to_path_buf(),
    }
}

fn migrate_legacy_data_dir(legacy_dir: &Path, canonical_dir: &Path) -> anyhow::Result<()> {
    if !legacy_dir.exists() {
        return Ok(());
    }

    if canonical_dir.exists() {
        move_selected_legacy_entries(legacy_dir, canonical_dir)?;
        return Ok(());
    }

    if let Some(parent) = canonical_dir.parent() {
        fs::create_dir_all(parent)?;
    }

    match fs::rename(legacy_dir, canonical_dir) {
        Ok(()) => Ok(()),
        Err(_) => {
            copy_dir_recursive(legacy_dir, canonical_dir)?;
            fs::remove_dir_all(legacy_dir)?;
            Ok(())
        }
    }
}

fn move_selected_legacy_entries(legacy_dir: &Path, canonical_dir: &Path) -> anyhow::Result<()> {
    for name in ["lexicon.db", "downloads", "plugins"] {
        let source_path = legacy_dir.join(name);
        if !source_path.exists() {
            continue;
        }

        let target_path = canonical_dir.join(name);
        if target_path.exists() {
            continue;
        }

        move_path_with_fallback(&source_path, &target_path)?;
    }

    Ok(())
}

fn move_path_with_fallback(source: &Path, target: &Path) -> anyhow::Result<()> {
    match fs::rename(source, target) {
        Ok(()) => Ok(()),
        Err(_) => {
            if source.is_dir() {
                copy_dir_recursive(source, target)?;
                fs::remove_dir_all(source)?;
            } else {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(source, target)?;
                fs::remove_file(source)?;
            }

            Ok(())
        }
    }
}

fn copy_dir_recursive(source: &Path, target: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(target)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());

        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
            continue;
        }

        fs::copy(&source_path, &target_path)?;
    }

    Ok(())
}
