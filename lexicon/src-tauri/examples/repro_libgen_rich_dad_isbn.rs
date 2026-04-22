#[path = "../src/models/mod.rs"]
mod models;
#[path = "../src/plugins/mod.rs"]
mod plugins;

use std::path::PathBuf;

use models::AddonRole;
use plugins::PluginManager;

fn main() -> anyhow::Result<()> {
    let mut manager = PluginManager::new(vec![PathBuf::from("plugins/dist")])?;
    manager.load_plugins()?;

    let snapshot = manager.runtime_snapshot();
    let plugin = snapshot
        .plugins
        .iter()
        .find(|plugin| {
            plugin.role == AddonRole::Source
                && (plugin.id == "libgen-source-plugin" || plugin.id.contains("libgen"))
        })
        .ok_or_else(|| anyhow::anyhow!("libgen source plugin not found"))?;

    let title = "Rich Dad, Poor Dad";
    let author = Some("Robert T. Kiyosaki".to_string());
    let isbn = Some("9781533221827".to_string());

    println!("fuel={}", snapshot.fuel_per_invocation);
    println!("title={:?}", title);
    println!("author={:?}", author);
    println!("isbn={:?}", isbn);

    let result = PluginManager::execute_source_find_downloads(
        &snapshot.engine,
        snapshot.fuel_per_invocation,
        plugin,
        title,
        author,
        isbn,
    );

    match result {
        Ok(entries) => {
            println!("ok: {}", entries.len());
            for (idx, entry) in entries.iter().take(5).enumerate() {
                println!(
                    "[{}] format={} lang={:?} size={:?} quality={:?} url={}",
                    idx,
                    entry.format,
                    entry.language,
                    entry.size,
                    entry.quality,
                    entry.download_url
                );
            }
        }
        Err(err) => {
            println!("err.kind={:?}", err.kind);
            println!("err.message={}", err.message);
        }
    }

    Ok(())
}
