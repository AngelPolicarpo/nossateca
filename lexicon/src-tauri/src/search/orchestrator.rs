use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sha2::{Digest, Sha256};
use tokio::task::JoinSet;

use crate::models::{AddonRole, SearchBookResult};
use crate::plugins::{PluginManager, PluginRuntimeSnapshot};

pub struct SearchOrchestrator {
    plugin_manager: Arc<Mutex<PluginManager>>,
    plugin_timeout: Duration,
}

impl SearchOrchestrator {
    pub fn new(plugin_manager: Arc<Mutex<PluginManager>>) -> Self {
        Self {
            plugin_manager,
            plugin_timeout: resolve_plugin_timeout(),
        }
    }

    #[cfg(test)]
    pub fn with_timeout(
        plugin_manager: Arc<Mutex<PluginManager>>,
        plugin_timeout: Duration,
    ) -> Self {
        Self {
            plugin_manager,
            plugin_timeout,
        }
    }

    pub async fn search_books(&self, query: &str) -> Result<Vec<SearchBookResult>, String> {
        let normalized_query = query.trim();
        if normalized_query.is_empty() {
            return Ok(Vec::new());
        }

        let snapshot = self
            .plugin_manager
            .lock()
            .map_err(|_| "failed to lock plugin manager".to_string())?
            .runtime_snapshot();

        let mut aggregated = self
            .run_parallel_plugin_search(snapshot, normalized_query)
            .await;

        if aggregated.is_empty() {
            #[cfg(debug_assertions)]
            {
                aggregated = PluginManager::mock_results_for(normalized_query)
                    .into_iter()
                    .map(|result| Self::apply_ranking(result, 0.35))
                    .collect();
            }
        }

        Ok(Self::deduplicate_and_sort(aggregated))
    }

    async fn run_parallel_plugin_search(
        &self,
        snapshot: PluginRuntimeSnapshot,
        query: &str,
    ) -> Vec<SearchBookResult> {
        if snapshot.plugins.is_empty() {
            return Vec::new();
        }

        let mut join_set = JoinSet::new();

        for plugin in snapshot.plugins {
            if plugin.role != AddonRole::LegacySearch {
                continue;
            }

            let timeout = self.plugin_timeout;
            let query_owned = query.to_string();
            let plugin_id = plugin.id.clone();
            let source_weight = plugin.source_weight;

            let engine = snapshot.engine.clone();
            let fuel_per_invocation = snapshot.fuel_per_invocation;
            let plugin_descriptor = plugin.clone();

            join_set.spawn(async move {
                let plugin_id_for_success = plugin_id.clone();
                let worker = tokio::task::spawn_blocking(move || {
                    PluginManager::execute_plugin(
                        &engine,
                        fuel_per_invocation,
                        &plugin_descriptor,
                        &query_owned,
                    )
                });

                match tokio::time::timeout(timeout, worker).await {
                    Ok(Ok(Ok(results))) => Ok((plugin_id_for_success, source_weight, results)),
                    Ok(Ok(Err(err))) => Err(format!("plugin '{}' failed: {}", plugin_id, err)),
                    Ok(Err(err)) => Err(format!("plugin '{}' join error: {}", plugin_id, err)),
                    Err(_) => Err(format!(
                        "plugin '{}' timed out after {}ms",
                        plugin_id,
                        timeout.as_millis()
                    )),
                }
            });
        }

        let mut aggregated = Vec::new();

        while let Some(task_result) = join_set.join_next().await {
            match task_result {
                Ok(Ok((plugin_id, source_weight, plugin_results))) => {
                    if plugin_results.is_empty() {
                        eprintln!(
                            "[search-orchestrator] plugin '{}' returned zero results",
                            plugin_id
                        );
                    } else {
                        eprintln!(
                            "[search-orchestrator] plugin '{}' returned {} result(s)",
                            plugin_id,
                            plugin_results.len()
                        );
                    }

                    for result in plugin_results {
                        aggregated.push(Self::apply_ranking(result, source_weight));
                    }
                }
                Ok(Err(err)) => {
                    eprintln!("[search-orchestrator] {}", err);
                }
                Err(err) => {
                    eprintln!("[search-orchestrator] failed to join plugin task: {}", err);
                }
            }
        }

        aggregated
    }

    fn apply_ranking(mut result: SearchBookResult, source_weight: f32) -> SearchBookResult {
        let base_score = result.score.clamp(0.0, 1.0);
        let source_bonus = source_weight.clamp(0.0, 1.0) * 0.20;
        let format_bonus = format_bonus(result.format.as_deref());
        let protocol_bonus = if result.download_url.starts_with("magnet:") {
            0.02
        } else {
            0.0
        };

        result.score = (base_score * 0.70 + source_bonus + format_bonus + protocol_bonus).min(1.0);
        result
    }

    fn deduplicate_and_sort(results: Vec<SearchBookResult>) -> Vec<SearchBookResult> {
        let mut deduplicated: HashMap<String, SearchBookResult> = HashMap::new();

        for result in results {
            let key = dedup_key(&result);
            match deduplicated.get(&key) {
                Some(existing) if existing.score >= result.score => {}
                _ => {
                    deduplicated.insert(key, result);
                }
            }
        }

        let mut ranked: Vec<SearchBookResult> = deduplicated.into_values().collect();
        ranked.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.title.cmp(&right.title))
        });

        ranked
    }
}

fn resolve_plugin_timeout() -> Duration {
    let default_timeout = Duration::from_secs(15);

    let Some(value) = std::env::var("LEXICON_SEARCH_PLUGIN_TIMEOUT_MS").ok() else {
        return default_timeout;
    };

    let Ok(timeout_ms) = value.trim().parse::<u64>() else {
        return default_timeout;
    };

    let clamped_timeout_ms = timeout_ms.clamp(1_000, 120_000);
    Duration::from_millis(clamped_timeout_ms)
}

fn dedup_key(result: &SearchBookResult) -> String {
    let normalized_title = normalize_for_dedup(&result.title);
    let normalized_author = normalize_for_dedup(result.author.as_deref().unwrap_or(""));

    let mut hasher = Sha256::new();
    hasher.update(normalized_title.as_bytes());
    hasher.update(b"|");
    hasher.update(normalized_author.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn normalize_for_dedup(value: &str) -> String {
    let lowered = value
        .chars()
        .flat_map(|character| character.to_lowercase())
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else if character.is_whitespace() {
                ' '
            } else {
                ' '
            }
        })
        .collect::<String>();

    lowered.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn format_bonus(format: Option<&str>) -> f32 {
    match format.map(|value| value.to_ascii_lowercase()).as_deref() {
        Some("epub") => 0.10,
        Some("pdf") => 0.06,
        Some("mobi") => 0.05,
        Some("azw3") => 0.05,
        Some(_) => 0.03,
        None => 0.02,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    #[test]
    fn deduplication_prefers_higher_ranked_result() {
        let low = SearchBookResult {
            id: "a1".to_string(),
            title: "Pride & Prejudice".to_string(),
            author: Some("Jane Austen".to_string()),
            source: "source-a".to_string(),
            format: Some("epub".to_string()),
            download_url: "https://example.com/a.epub".to_string(),
            score: 0.50,
        };

        let high = SearchBookResult {
            id: "a2".to_string(),
            title: "pride prejudice".to_string(),
            author: Some("Jane Austen".to_string()),
            source: "source-b".to_string(),
            format: Some("epub".to_string()),
            download_url: "https://example.com/b.epub".to_string(),
            score: 0.90,
        };

        let result = SearchOrchestrator::deduplicate_and_sort(vec![low, high]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "a2");
    }

    #[test]
    fn ranking_boosts_preferred_formats() {
        let epub = SearchBookResult {
            id: "epub".to_string(),
            title: "Book".to_string(),
            author: None,
            source: "opds".to_string(),
            format: Some("epub".to_string()),
            download_url: "https://example.com/book.epub".to_string(),
            score: 0.6,
        };

        let pdf = SearchBookResult {
            id: "pdf".to_string(),
            title: "Book".to_string(),
            author: None,
            source: "opds".to_string(),
            format: Some("pdf".to_string()),
            download_url: "https://example.com/book.pdf".to_string(),
            score: 0.6,
        };

        let ranked_epub = SearchOrchestrator::apply_ranking(epub, 0.8);
        let ranked_pdf = SearchOrchestrator::apply_ranking(pdf, 0.8);

        assert!(ranked_epub.score > ranked_pdf.score);
    }

    #[tokio::test]
    async fn aggregates_multiple_plugins_and_keeps_unique_results() {
        let plugin_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("plugins/dist");
        if !plugin_dir.exists() {
            return;
        }

        let mut manager =
            PluginManager::new(vec![plugin_dir]).expect("plugin manager should initialize");
        manager
            .load_plugins()
            .expect("plugins from dist should load");

        if manager.plugin_count() < 2 {
            return;
        }

        let orchestrator =
            SearchOrchestrator::with_timeout(Arc::new(Mutex::new(manager)), Duration::from_secs(5));

        let results = orchestrator
            .search_books("pride prejudice")
            .await
            .expect("search should work with multiple plugins");

        assert!(!results.is_empty());
        assert!(results
            .windows(2)
            .all(|window| window[0].score >= window[1].score));

        let mut keys = HashSet::new();
        for result in &results {
            assert!(keys.insert(dedup_key(result)));
        }
    }
}
