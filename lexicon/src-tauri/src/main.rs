// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

mod commands;
mod db;
mod download;
mod models;
mod plugins;
mod reader;
mod storage;

use download::DownloadManager;
use plugins::PluginManager;
use sqlx::SqlitePool;
use tauri::Manager;

struct AppState {
    _db_pool: SqlitePool,
    plugin_manager: Arc<Mutex<PluginManager>>,
    plugin_runtime_dir: PathBuf,
    download_manager: Arc<DownloadManager>,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn resolve_bundled_plugins_dir(app: &tauri::AppHandle) -> Option<PathBuf> {
    if let Ok(resource_dir) = app.path().resource_dir() {
        let candidate = resource_dir.join("plugins").join("dist");
        if candidate.exists() {
            return Some(candidate);
        }
    }

    let dev_candidate = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("plugins/dist");
    if dev_candidate.exists() {
        return Some(dev_candidate);
    }

    None
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let db_pool = tauri::async_runtime::block_on(db::init_db(app.handle()))
                .expect("failed to initialize SQLite database");

            let runtime_plugins = storage::resolve_plugins_dir()
                .expect("failed to resolve portable plugins directory");

            let bundled_plugins = resolve_bundled_plugins_dir(app.handle());
            if bundled_plugins.as_ref().is_some_and(|path| path.exists()) {
                let bundled_plugins = bundled_plugins.unwrap();
                if let Ok(entries) = std::fs::read_dir(&bundled_plugins) {
                    for entry in entries.flatten() {
                        let source_path = entry.path();

                        let is_wasm = source_path
                            .extension()
                            .and_then(|value| value.to_str())
                            .is_some_and(|value| value.eq_ignore_ascii_case("wasm"));

                        if !is_wasm {
                            continue;
                        }

                        let Some(file_name) = source_path.file_name() else {
                            continue;
                        };

                        let destination_path = runtime_plugins.join(file_name);

                        let should_copy = match (
                            std::fs::metadata(&source_path),
                            std::fs::metadata(&destination_path),
                        ) {
                            (Ok(source_meta), Ok(destination_meta)) => {
                                source_meta.len() != destination_meta.len()
                                    || source_meta
                                        .modified()
                                        .ok()
                                        .zip(destination_meta.modified().ok())
                                        .map(|(source_modified, destination_modified)| {
                                            source_modified > destination_modified
                                        })
                                        .unwrap_or(false)
                            }
                            (Ok(_), Err(_)) => true,
                            _ => false,
                        };

                        if !should_copy {
                            continue;
                        }

                        if let Err(err) = std::fs::copy(&source_path, &destination_path) {
                            eprintln!(
                                "[plugin-manager] failed to bootstrap plugin '{}' into runtime dir: {}",
                                source_path.display(),
                                err
                            );
                        }
                    }
                }
            }

            let plugin_dirs = vec![runtime_plugins.clone()];

            let mut plugin_manager = PluginManager::new(plugin_dirs)
                .map_err(|err| std::io::Error::other(err.to_string()))?;

            if let Err(err) = tauri::async_runtime::block_on(
                commands::addons::hydrate_addon_settings_from_db(&db_pool, &mut plugin_manager),
            ) {
                eprintln!("[addons] failed to hydrate addon settings from DB: {}", err);
            }

            if let Err(err) = plugin_manager.load_plugins() {
                eprintln!("[plugin-manager] failed loading plugins: {}", err);
            }

            println!(
                "[plugin-manager] loaded {} plugin(s)",
                plugin_manager.plugin_count()
            );

            let plugin_manager_shared = Arc::new(Mutex::new(plugin_manager));

            let download_manager = Arc::new(DownloadManager::new(
                app.handle().clone(),
                db_pool.clone(),
                plugin_manager_shared.clone(),
            ));

            app.manage(AppState {
                _db_pool: db_pool,
                plugin_manager: plugin_manager_shared,
                plugin_runtime_dir: runtime_plugins,
                download_manager,
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::library::add_book,
            commands::library::list_books,
            commands::library::remove_book,
            commands::reader::get_book_content,
            commands::reader::resolve_epub_link_target,
            commands::reader::get_pdf_document,
            commands::reader::get_cbz_page,
            commands::reader::get_reading_progress,
            commands::reader::search_epub_content,
            commands::reader::save_progress,
            commands::annotations::add_annotation,
            commands::annotations::get_annotations,
            commands::annotations::update_annotation_note,
            commands::annotations::update_annotation_color,
            commands::annotations::delete_annotation,
            commands::discover::list_discover_catalogs,
            commands::discover::list_discover_catalog_items,
            commands::discover::get_discover_item_details,
            commands::discover::search_source_downloads,
            commands::addons::list_addons,
            commands::addons::reload_addons,
            commands::addons::install_addon,
            commands::addons::remove_addon,
            commands::addons::get_addon_settings,
            commands::addons::update_addon_settings,
            commands::addons::set_addon_enabled,
            commands::download::start_download,
            commands::download::pause_download,
            commands::download::resume_download,
            commands::download::cancel_download,
            commands::download::remove_download,
            commands::download::list_downloads,
            commands::manga::list_manga_chapters
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
