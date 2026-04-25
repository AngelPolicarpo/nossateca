use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const DB_FILE_NAME: &str = "nossateca.db";
const DATA_SUBDIR: &str = "dados";
const PLUGINS_SUBDIR: &str = "plugins";
const ACERVO_SUBDIR: &str = "acervo";
const DOWNLOADS_SUBDIR: &str = "baixados";

pub fn resolve_portable_root() -> anyhow::Result<PathBuf> {
    let exe = env::current_exe()
        .map_err(|err| anyhow::anyhow!("failed to resolve current executable path: {}", err))?;
    let parent = exe
        .parent()
        .ok_or_else(|| anyhow::anyhow!("current executable has no parent directory"))?
        .to_path_buf();
    Ok(parent)
}

fn ensure_subdir(name: &str) -> anyhow::Result<PathBuf> {
    let dir = resolve_portable_root()?.join(name);
    fs::create_dir_all(&dir)
        .map_err(|err| anyhow::anyhow!("failed to create '{}' directory: {}", name, err))?;
    Ok(dir)
}

pub fn resolve_data_dir() -> anyhow::Result<PathBuf> {
    ensure_subdir(DATA_SUBDIR)
}

pub fn resolve_db_path() -> anyhow::Result<PathBuf> {
    Ok(resolve_data_dir()?.join(DB_FILE_NAME))
}

pub fn resolve_plugins_dir() -> anyhow::Result<PathBuf> {
    ensure_subdir(PLUGINS_SUBDIR)
}

pub fn resolve_acervo_dir() -> anyhow::Result<PathBuf> {
    ensure_subdir(ACERVO_SUBDIR)
}

pub fn resolve_downloads_dir() -> anyhow::Result<PathBuf> {
    ensure_subdir(DOWNLOADS_SUBDIR)
}

pub fn expand_stored_path(stored: &str) -> PathBuf {
    let path = Path::new(stored);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    match resolve_portable_root() {
        Ok(root) => root.join(path),
        Err(_) => path.to_path_buf(),
    }
}

pub fn to_relative_stored(absolute: &Path) -> String {
    if let Ok(root) = resolve_portable_root() {
        if let Ok(rel) = absolute.strip_prefix(&root) {
            return rel.to_string_lossy().to_string();
        }
    }
    absolute.to_string_lossy().to_string()
}
