use std::path::PathBuf;
use std::str::FromStr;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

use crate::storage::resolve_lexicon_data_dir;

pub async fn init_db(app: &tauri::AppHandle) -> anyhow::Result<SqlitePool> {
    let app_data_dir = resolve_lexicon_data_dir(app)?;

    let db_path: PathBuf = app_data_dir.join("lexicon.db");
    let db_url = format!("sqlite://{}", db_path.to_string_lossy());

    let connect_options = SqliteConnectOptions::from_str(&db_url)?.create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}
