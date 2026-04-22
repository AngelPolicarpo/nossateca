use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::blocking::Client as BlockingHttpClient;
use reqwest::{Method, Url};
use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiView};

use crate::models::{
    AddonDescriptor, AddonRole, AddonSettingEntry, DiscoverCatalog, DiscoverCatalogItem,
    DiscoverCatalogPageResponse, DiscoverItemDetails, MangaChapter, MangaPageList,
    MangaSourcePluginInfo, PluginErrorKind, PluginTypedError, SourceDownloadResult,
    SourcePluginInfo,
};

const DEFAULT_PLUGIN_FUEL: u64 = 400_000_000;
const MIN_PLUGIN_FUEL: u64 = 80_000_000;
const MAX_PLUGIN_FUEL: u64 = 2_000_000_000;

mod discover_bindings {
    wasmtime::component::bindgen!({
        path: "wit/discover-source-plugin.wit",
        world: "discover-plugin",
    });
}

mod source_bindings {
    wasmtime::component::bindgen!({
        path: "wit/discover-source-plugin.wit",
        world: "source-plugin",
    });
}

mod manga_source_bindings {
    wasmtime::component::bindgen!({
        path: "wit/discover-source-plugin.wit",
        world: "manga-source-plugin",
    });
}

struct PluginHostState {
    ctx: WasiCtx,
    table: ResourceTable,
    http_client: BlockingHttpClient,
}

impl PluginHostState {
    fn new() -> Self {
        let ctx = WasiCtxBuilder::new().build();
        let http_client = BlockingHttpClient::builder()
            .user_agent("LexiconAddonHost/0.2")
            .build()
            .unwrap_or_else(|_| BlockingHttpClient::new());

        Self {
            ctx,
            table: ResourceTable::new(),
            http_client,
        }
    }

    fn execute_http_raw(
        &self,
        method: &str,
        url: &str,
        query: Vec<(String, String)>,
        headers: Vec<(String, String)>,
        body: Option<String>,
        timeout_ms: Option<u64>,
    ) -> Result<(u16, String), String> {
        let method = Method::from_bytes(method.trim().as_bytes())
            .map_err(|err| format!("invalid HTTP method '{}': {}", method, err))?;

        let mut parsed_url =
            Url::parse(url.trim()).map_err(|err| format!("invalid request URL '{}': {}", url, err))?;

        {
            let mut query_pairs = parsed_url.query_pairs_mut();
            for (key, value) in query {
                query_pairs.append_pair(&key, &value);
            }
        }

        let mut builder = self.http_client.request(method, parsed_url);

        if let Some(timeout_ms) = timeout_ms {
            if timeout_ms > 0 {
                builder = builder.timeout(Duration::from_millis(timeout_ms));
            }
        }

        for (key, value) in headers {
            let normalized = key.trim();
            if normalized.is_empty() {
                continue;
            }

            builder = builder.header(normalized, value);
        }

        if let Some(body) = body {
            builder = builder.body(body);
        }

        let response = builder
            .send()
            .map_err(|err| format!("HTTP request failed: {}", err))?;

        let status = response.status().as_u16();
        let body = response
            .text()
            .map_err(|err| format!("failed to read response body: {}", err))?;

        Ok((status, body))
    }
}

impl WasiView for PluginHostState {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }

    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

impl discover_bindings::lexicon::plugin_roles::host_http::Host for PluginHostState {
    fn send_http_request(
        &mut self,
        request: discover_bindings::lexicon::plugin_roles::common_types::HttpRequest,
    ) -> std::result::Result<
        discover_bindings::lexicon::plugin_roles::common_types::HttpResponse,
        String,
    > {
        let query = request
            .query
            .into_iter()
            .map(|item| (item.key, item.value))
            .collect::<Vec<_>>();

        let headers = request
            .headers
            .into_iter()
            .map(|item| (item.key, item.value))
            .collect::<Vec<_>>();

        let (status, body) = self.execute_http_raw(
            &request.method,
            &request.url,
            query,
            headers,
            request.body,
            request.timeout_ms,
        )?;

        Ok(discover_bindings::lexicon::plugin_roles::common_types::HttpResponse {
            status,
            body,
        })
    }
}

impl source_bindings::lexicon::plugin_roles::host_http::Host for PluginHostState {
    fn send_http_request(
        &mut self,
        request: source_bindings::lexicon::plugin_roles::common_types::HttpRequest,
    ) -> std::result::Result<source_bindings::lexicon::plugin_roles::common_types::HttpResponse, String>
    {
        let query = request
            .query
            .into_iter()
            .map(|item| (item.key, item.value))
            .collect::<Vec<_>>();

        let headers = request
            .headers
            .into_iter()
            .map(|item| (item.key, item.value))
            .collect::<Vec<_>>();

        let (status, body) = self.execute_http_raw(
            &request.method,
            &request.url,
            query,
            headers,
            request.body,
            request.timeout_ms,
        )?;

        Ok(source_bindings::lexicon::plugin_roles::common_types::HttpResponse {
            status,
            body,
        })
    }
}

impl manga_source_bindings::lexicon::plugin_roles::host_http::Host for PluginHostState {
    fn send_http_request(
        &mut self,
        request: manga_source_bindings::lexicon::plugin_roles::common_types::HttpRequest,
    ) -> std::result::Result<
        manga_source_bindings::lexicon::plugin_roles::common_types::HttpResponse,
        String,
    > {
        let query = request
            .query
            .into_iter()
            .map(|item| (item.key, item.value))
            .collect::<Vec<_>>();

        let headers = request
            .headers
            .into_iter()
            .map(|item| (item.key, item.value))
            .collect::<Vec<_>>();

        let (status, body) = self.execute_http_raw(
            &request.method,
            &request.url,
            query,
            headers,
            request.body,
            request.timeout_ms,
        )?;

        Ok(
            manga_source_bindings::lexicon::plugin_roles::common_types::HttpResponse {
                status,
                body,
            },
        )
    }
}

#[derive(Clone, Debug)]
pub struct PluginDescriptor {
    pub id: String,
    pub file_name: String,
    pub path: PathBuf,
    pub role: AddonRole,
    pub enabled: bool,
    pub settings: Vec<AddonSettingEntry>,
}

#[derive(Clone)]
pub struct PluginRuntimeSnapshot {
    pub engine: Engine,
    pub plugins: Vec<PluginDescriptor>,
    pub fuel_per_invocation: u64,
}

pub struct PluginManager {
    engine: Engine,
    plugin_directories: Vec<PathBuf>,
    plugins: Vec<PluginDescriptor>,
    fuel_per_invocation: u64,
    plugin_settings: HashMap<String, Vec<AddonSettingEntry>>,
}

impl PluginManager {
    pub fn new(plugin_directories: Vec<PathBuf>) -> Result<Self> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.consume_fuel(true);

        let engine = Engine::new(&config).context("failed to initialize wasmtime engine")?;

        let requested_fuel = env::var("LEXICON_PLUGIN_FUEL")
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok());

        let fuel_per_invocation = match requested_fuel {
            Some(value) => {
                let clamped = value.clamp(MIN_PLUGIN_FUEL, MAX_PLUGIN_FUEL);
                if clamped != value {
                    eprintln!(
                        "[plugin-manager] adjusted LEXICON_PLUGIN_FUEL from {} to {} (allowed range: {}..={})",
                        value, clamped, MIN_PLUGIN_FUEL, MAX_PLUGIN_FUEL
                    );
                }
                clamped
            }
            None => DEFAULT_PLUGIN_FUEL,
        };

        Ok(Self {
            engine,
            plugin_directories,
            plugins: Vec::new(),
            fuel_per_invocation,
            plugin_settings: HashMap::new(),
        })
    }

    pub fn load_plugins(&mut self) -> Result<usize> {
        self.plugins.clear();

        let directories = self.plugin_directories.clone();
        for directory in directories {
            self.load_plugins_from_directory(&directory)?;
        }

        Ok(self.plugins.len())
    }

    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    pub fn runtime_snapshot(&self) -> PluginRuntimeSnapshot {
        PluginRuntimeSnapshot {
            engine: self.engine.clone(),
            plugins: self.plugins.clone(),
            fuel_per_invocation: self.fuel_per_invocation,
        }
    }

    pub fn list_plugins(&self) -> Vec<AddonDescriptor> {
        self.plugins
            .iter()
            .map(|plugin| AddonDescriptor {
                id: plugin.id.clone(),
                file_name: plugin.file_name.clone(),
                file_path: plugin.path.to_string_lossy().to_string(),
                role: plugin.role.clone(),
                enabled: plugin.enabled,
                settings: plugin.settings.clone(),
            })
            .collect()
    }

    pub fn plugin_by_id(&self, id: &str) -> Option<PluginDescriptor> {
        self.plugins.iter().find(|plugin| plugin.id == id).cloned()
    }

    pub fn set_all_plugin_settings(&mut self, settings: HashMap<String, Vec<AddonSettingEntry>>) {
        self.plugin_settings = settings;

        for plugin in &mut self.plugins {
            plugin.settings = self
                .plugin_settings
                .get(&plugin.id)
                .cloned()
                .unwrap_or_default();
            plugin.enabled = resolve_plugin_enabled(&plugin.settings);
            plugin.role = resolve_plugin_role(&plugin.id, &plugin.settings);
        }
    }

    pub fn set_plugin_settings(&mut self, plugin_id: &str, settings: Vec<AddonSettingEntry>) {
        self.plugin_settings
            .insert(plugin_id.to_string(), settings.clone());

        if let Some(plugin) = self.plugins.iter_mut().find(|plugin| plugin.id == plugin_id) {
            plugin.settings = settings;
            plugin.enabled = resolve_plugin_enabled(&plugin.settings);
            plugin.role = resolve_plugin_role(&plugin.id, &plugin.settings);
        }
    }

    pub fn clear_plugin_settings(&mut self, plugin_id: &str) {
        self.plugin_settings.remove(plugin_id);

        if let Some(plugin) = self.plugins.iter_mut().find(|plugin| plugin.id == plugin_id) {
            plugin.settings.clear();
            plugin.enabled = true;
            plugin.role = resolve_plugin_role(&plugin.id, &plugin.settings);
        }
    }

    pub fn execute_discover_list_catalogs(
        engine: &Engine,
        fuel_per_invocation: u64,
        plugin: &PluginDescriptor,
    ) -> Result<Vec<DiscoverCatalog>, PluginTypedError> {
        let component = Component::from_file(engine, &plugin.path)
            .map_err(|err| unknown_error(format!("failed to load discover plugin: {}", err)))?;

        let mut linker = Linker::<PluginHostState>::new(engine);
        wasmtime_wasi::add_to_linker_sync(&mut linker)
            .map_err(|err| unknown_error(format!("failed to wire WASI imports: {}", err)))?;

        discover_bindings::lexicon::plugin_roles::host_http::add_to_linker(&mut linker, |state| {
            state
        })
        .map_err(|err| unknown_error(format!("failed to wire host-http imports: {}", err)))?;

        let mut store = Store::new(engine, PluginHostState::new());
        let _ = store.set_fuel(fuel_per_invocation);

        let (exports, _instance) = discover_bindings::DiscoverPlugin::instantiate(
            &mut store,
            &component,
            &linker,
        )
        .map_err(|err| unknown_error(format!("failed to instantiate discover plugin: {}", err)))?;

        let response = exports
            .call_list_catalogs(&mut store)
            .map_err(|err| unknown_error(format!("discover list_catalogs failed: {}", err)))?;

        match response {
            Ok(catalogs) => Ok(catalogs
                .into_iter()
                .map(|catalog| DiscoverCatalog {
                    plugin_id: plugin.id.clone(),
                    id: catalog.id,
                    name: catalog.name,
                    content_type: catalog.content_type,
                    genres: catalog.genres,
                    supported_filters: catalog.supported_filters,
                })
                .collect()),
            Err(err) => Err(map_discover_plugin_error(err)),
        }
    }

    pub fn execute_discover_list_catalog_items(
        engine: &Engine,
        fuel_per_invocation: u64,
        plugin: &PluginDescriptor,
        catalog_id: &str,
        skip: u32,
        page_size: u32,
        genre: Option<String>,
        year: Option<u32>,
        search_query: Option<String>,
    ) -> Result<DiscoverCatalogPageResponse, PluginTypedError> {
        let component = Component::from_file(engine, &plugin.path)
            .map_err(|err| unknown_error(format!("failed to load discover plugin: {}", err)))?;

        let mut linker = Linker::<PluginHostState>::new(engine);
        wasmtime_wasi::add_to_linker_sync(&mut linker)
            .map_err(|err| unknown_error(format!("failed to wire WASI imports: {}", err)))?;

        discover_bindings::lexicon::plugin_roles::host_http::add_to_linker(&mut linker, |state| {
            state
        })
        .map_err(|err| unknown_error(format!("failed to wire host-http imports: {}", err)))?;

        let mut store = Store::new(engine, PluginHostState::new());
        let _ = store.set_fuel(fuel_per_invocation);

        let (exports, _instance) = discover_bindings::DiscoverPlugin::instantiate(
            &mut store,
            &component,
            &linker,
        )
        .map_err(|err| unknown_error(format!("failed to instantiate discover plugin: {}", err)))?;

        let request = discover_bindings::lexicon::plugin_roles::discover_types::DiscoverCatalogQuery {
            catalog_id: catalog_id.to_string(),
            skip,
            page_size,
            genre,
            year,
            search_query,
        };

        let response = exports
            .call_list_catalog_items(&mut store, &request)
            .map_err(|err| unknown_error(format!("discover list_catalog_items failed: {}", err)))?;

        match response {
            Ok(page) => {
                let items = page
                    .items
                    .into_iter()
                    .map(|item| DiscoverCatalogItem {
                        plugin_id: plugin.id.clone(),
                        catalog_id: catalog_id.to_string(),
                        id: item.id,
                        title: item.title,
                        author: item.author,
                        cover_url: item.cover_url,
                        genres: item.genres,
                        year: item.year,
                        page_count: item.page_count,
                        short_description: item.short_description,
                        format: item.format,
                        isbn: item.isbn,
                    })
                    .collect::<Vec<_>>();

                Ok(DiscoverCatalogPageResponse {
                    plugin_id: plugin.id.clone(),
                    catalog_id: catalog_id.to_string(),
                    items,
                    has_more: page.has_more,
                })
            }
            Err(err) => Err(map_discover_plugin_error(err)),
        }
    }

    pub fn execute_discover_get_item_details(
        engine: &Engine,
        fuel_per_invocation: u64,
        plugin: &PluginDescriptor,
        item_id: &str,
    ) -> Result<DiscoverItemDetails, PluginTypedError> {
        let component = Component::from_file(engine, &plugin.path)
            .map_err(|err| unknown_error(format!("failed to load discover plugin: {}", err)))?;

        let mut linker = Linker::<PluginHostState>::new(engine);
        wasmtime_wasi::add_to_linker_sync(&mut linker)
            .map_err(|err| unknown_error(format!("failed to wire WASI imports: {}", err)))?;

        discover_bindings::lexicon::plugin_roles::host_http::add_to_linker(&mut linker, |state| {
            state
        })
        .map_err(|err| unknown_error(format!("failed to wire host-http imports: {}", err)))?;

        let mut store = Store::new(engine, PluginHostState::new());
        let _ = store.set_fuel(fuel_per_invocation);

        let (exports, _instance) = discover_bindings::DiscoverPlugin::instantiate(
            &mut store,
            &component,
            &linker,
        )
        .map_err(|err| unknown_error(format!("failed to instantiate discover plugin: {}", err)))?;

        let response = exports
            .call_get_item_details(&mut store, item_id)
            .map_err(|err| unknown_error(format!("discover get_item_details failed: {}", err)))?;

        match response {
            Ok(details) => Ok(DiscoverItemDetails {
                plugin_id: plugin.id.clone(),
                id: details.id,
                title: details.title,
                author: details.author,
                description: details.description,
                cover_url: details.cover_url,
                genres: details.genres,
                year: details.year,
                page_count: details.page_count,
                format: details.format,
                isbn: details.isbn,
                origin_url: details.origin_url,
            }),
            Err(err) => Err(map_discover_plugin_error(err)),
        }
    }

    pub fn execute_source_get_info(
        engine: &Engine,
        fuel_per_invocation: u64,
        plugin: &PluginDescriptor,
    ) -> Result<SourcePluginInfo, PluginTypedError> {
        let component = Component::from_file(engine, &plugin.path)
            .map_err(|err| unknown_error(format!("failed to load source plugin: {}", err)))?;

        let mut linker = Linker::<PluginHostState>::new(engine);
        wasmtime_wasi::add_to_linker_sync(&mut linker)
            .map_err(|err| unknown_error(format!("failed to wire WASI imports: {}", err)))?;

        source_bindings::lexicon::plugin_roles::host_http::add_to_linker(&mut linker, |state| {
            state
        })
        .map_err(|err| unknown_error(format!("failed to wire host-http imports: {}", err)))?;

        let mut store = Store::new(engine, PluginHostState::new());
        let _ = store.set_fuel(fuel_per_invocation);

        let (exports, _instance) = source_bindings::SourcePlugin::instantiate(
            &mut store,
            &component,
            &linker,
        )
        .map_err(|err| unknown_error(format!("failed to instantiate source plugin: {}", err)))?;

        let info = exports
            .call_get_source_info(&mut store)
            .map_err(|err| unknown_error(format!("source get_source_info failed: {}", err)))?;

        Ok(SourcePluginInfo {
            plugin_id: plugin.id.clone(),
            source_name: info.source_name,
            source_id: info.source_id,
            supported_formats: info.supported_formats,
        })
    }

    pub fn execute_source_find_downloads(
        engine: &Engine,
        fuel_per_invocation: u64,
        plugin: &PluginDescriptor,
        title: &str,
        author: Option<String>,
        isbn: Option<String>,
    ) -> Result<Vec<SourceDownloadResult>, PluginTypedError> {
        let component = Component::from_file(engine, &plugin.path)
            .map_err(|err| unknown_error(format!("failed to load source plugin: {}", err)))?;

        let mut linker = Linker::<PluginHostState>::new(engine);
        wasmtime_wasi::add_to_linker_sync(&mut linker)
            .map_err(|err| unknown_error(format!("failed to wire WASI imports: {}", err)))?;

        source_bindings::lexicon::plugin_roles::host_http::add_to_linker(&mut linker, |state| {
            state
        })
        .map_err(|err| unknown_error(format!("failed to wire host-http imports: {}", err)))?;

        let mut store = Store::new(engine, PluginHostState::new());
        let _ = store.set_fuel(fuel_per_invocation);

        let (exports, _instance) = source_bindings::SourcePlugin::instantiate(
            &mut store,
            &component,
            &linker,
        )
        .map_err(|err| unknown_error(format!("failed to instantiate source plugin: {}", err)))?;

        let request = source_bindings::lexicon::plugin_roles::source_types::SourceSearchQuery {
            title: title.to_string(),
            author,
            isbn,
        };

        let response = exports
            .call_find_downloads(&mut store, &request)
            .map_err(|err| unknown_error(format!("source find_downloads failed: {}", err)))?;

        match response {
            Ok(results) => Ok(results
                .into_iter()
                .filter(|entry| is_download_url_usable(&entry.download_url))
                .map(|entry| SourceDownloadResult {
                    download_url: entry.download_url,
                    format: entry.format,
                    size: entry.size,
                    language: entry.language,
                    quality: entry.quality,
                })
                .collect()),
            Err(err) => Err(map_source_plugin_error(err)),
        }
    }

    pub fn execute_manga_get_source_info(
        engine: &Engine,
        fuel_per_invocation: u64,
        plugin: &PluginDescriptor,
    ) -> Result<MangaSourcePluginInfo, PluginTypedError> {
        let component = Component::from_file(engine, &plugin.path)
            .map_err(|err| unknown_error(format!("failed to load manga plugin: {}", err)))?;

        let mut linker = Linker::<PluginHostState>::new(engine);
        wasmtime_wasi::add_to_linker_sync(&mut linker)
            .map_err(|err| unknown_error(format!("failed to wire WASI imports: {}", err)))?;

        manga_source_bindings::lexicon::plugin_roles::host_http::add_to_linker(
            &mut linker,
            |state| state,
        )
        .map_err(|err| unknown_error(format!("failed to wire host-http imports: {}", err)))?;

        let mut store = Store::new(engine, PluginHostState::new());
        let _ = store.set_fuel(fuel_per_invocation);

        let (exports, _instance) = manga_source_bindings::MangaSourcePlugin::instantiate(
            &mut store,
            &component,
            &linker,
        )
        .map_err(|err| unknown_error(format!("failed to instantiate manga plugin: {}", err)))?;

        let info = exports
            .call_get_manga_source_info(&mut store)
            .map_err(|err| unknown_error(format!("manga get_source_info failed: {}", err)))?;

        Ok(MangaSourcePluginInfo {
            plugin_id: plugin.id.clone(),
            source_name: info.source_name,
            source_id: info.source_id,
        })
    }

    pub fn execute_manga_list_chapters(
        engine: &Engine,
        fuel_per_invocation: u64,
        plugin: &PluginDescriptor,
        manga_id: &str,
    ) -> Result<Vec<MangaChapter>, PluginTypedError> {
        let component = Component::from_file(engine, &plugin.path)
            .map_err(|err| unknown_error(format!("failed to load manga plugin: {}", err)))?;

        let mut linker = Linker::<PluginHostState>::new(engine);
        wasmtime_wasi::add_to_linker_sync(&mut linker)
            .map_err(|err| unknown_error(format!("failed to wire WASI imports: {}", err)))?;

        manga_source_bindings::lexicon::plugin_roles::host_http::add_to_linker(
            &mut linker,
            |state| state,
        )
        .map_err(|err| unknown_error(format!("failed to wire host-http imports: {}", err)))?;

        let mut store = Store::new(engine, PluginHostState::new());
        let _ = store.set_fuel(fuel_per_invocation);

        let (exports, _instance) = manga_source_bindings::MangaSourcePlugin::instantiate(
            &mut store,
            &component,
            &linker,
        )
        .map_err(|err| unknown_error(format!("failed to instantiate manga plugin: {}", err)))?;

        let response = exports
            .call_list_chapters(&mut store, manga_id)
            .map_err(|err| unknown_error(format!("manga list_chapters failed: {}", err)))?;

        match response {
            Ok(chapters) => Ok(chapters
                .into_iter()
                .map(|chapter| MangaChapter {
                    id: chapter.id,
                    chapter: chapter.chapter,
                    volume: chapter.volume,
                    title: chapter.title,
                    language: chapter.language,
                    pages: chapter.pages,
                    published_at: chapter.published_at,
                    scanlator: chapter.scanlator,
                })
                .collect()),
            Err(err) => Err(map_manga_plugin_error(err)),
        }
    }

    pub fn execute_manga_get_chapter_pages(
        engine: &Engine,
        fuel_per_invocation: u64,
        plugin: &PluginDescriptor,
        chapter_id: &str,
    ) -> Result<MangaPageList, PluginTypedError> {
        let component = Component::from_file(engine, &plugin.path)
            .map_err(|err| unknown_error(format!("failed to load manga plugin: {}", err)))?;

        let mut linker = Linker::<PluginHostState>::new(engine);
        wasmtime_wasi::add_to_linker_sync(&mut linker)
            .map_err(|err| unknown_error(format!("failed to wire WASI imports: {}", err)))?;

        manga_source_bindings::lexicon::plugin_roles::host_http::add_to_linker(
            &mut linker,
            |state| state,
        )
        .map_err(|err| unknown_error(format!("failed to wire host-http imports: {}", err)))?;

        let mut store = Store::new(engine, PluginHostState::new());
        let _ = store.set_fuel(fuel_per_invocation);

        let (exports, _instance) = manga_source_bindings::MangaSourcePlugin::instantiate(
            &mut store,
            &component,
            &linker,
        )
        .map_err(|err| unknown_error(format!("failed to instantiate manga plugin: {}", err)))?;

        let response = exports
            .call_get_chapter_pages(&mut store, chapter_id)
            .map_err(|err| unknown_error(format!("manga get_chapter_pages failed: {}", err)))?;

        match response {
            Ok(page_list) => Ok(MangaPageList {
                chapter_id: page_list.chapter_id,
                page_urls: page_list.page_urls,
            }),
            Err(err) => Err(map_manga_plugin_error(err)),
        }
    }

    fn load_plugins_from_directory(&mut self, directory: &Path) -> Result<()> {
        if !directory.exists() || !directory.is_dir() {
            return Ok(());
        }

        for entry in fs::read_dir(directory)
            .with_context(|| format!("failed to read plugin directory {}", directory.display()))?
        {
            let entry = entry.with_context(|| {
                format!(
                    "failed to read entry in plugin directory {}",
                    directory.display()
                )
            })?;
            let path = entry.path();

            let is_wasm = path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("wasm"));

            if !is_wasm {
                continue;
            }

            Component::from_file(&self.engine, &path)
                .with_context(|| format!("failed to load plugin component {}", path.display()))?;

            let id = path
                .file_stem()
                .and_then(|value| value.to_str())
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| "plugin".to_string());

            let file_name = path
                .file_name()
                .and_then(|value| value.to_str())
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| format!("{}.wasm", id));

            let settings = self.plugin_settings.get(&id).cloned().unwrap_or_default();
            let enabled = resolve_plugin_enabled(&settings);
            let role = resolve_plugin_role(&id, &settings);

            self.plugins.push(PluginDescriptor {
                id,
                file_name,
                path,
                role,
                enabled,
                settings,
            });
        }

        self.plugins.sort_by(|left, right| left.id.cmp(&right.id));

        Ok(())
    }
}

fn resolve_plugin_enabled(settings: &[AddonSettingEntry]) -> bool {
    let Some(value) = settings
        .iter()
        .find(|entry| normalize_setting_key(&entry.key) == "enabled")
        .map(|entry| entry.value.trim().to_ascii_lowercase())
    else {
        return true;
    };

    if value.is_empty() {
        return true;
    }

    !matches!(value.as_str(), "0" | "false" | "off" | "no" | "disabled")
}

fn resolve_plugin_role(id: &str, settings: &[AddonSettingEntry]) -> AddonRole {
    if let Some(explicit_role) = settings
        .iter()
        .find(|entry| {
            let key = normalize_setting_key(&entry.key);
            key == "plugin_role" || key == "role"
        })
        .map(|entry| entry.value.trim().to_ascii_lowercase())
    {
        return match explicit_role.as_str() {
            "discover" => AddonRole::Discover,
            "source" => AddonRole::Source,
            "manga_source" | "manga-source" => AddonRole::MangaSource,
            "legacy_search" | "legacy-search" | "legacy" => AddonRole::LegacySearch,
            _ => infer_role_from_id(id),
        };
    }

    infer_role_from_id(id)
}

fn infer_role_from_id(id: &str) -> AddonRole {
    let normalized = id.to_ascii_lowercase();

    if normalized.contains("discover-plugin") {
        return AddonRole::Discover;
    }

    if normalized.contains("source-plugin") {
        if normalized.contains("manga") {
            return AddonRole::MangaSource;
        }
        return AddonRole::Source;
    }

    AddonRole::LegacySearch
}

fn normalize_setting_key(key: &str) -> String {
    key.trim().to_ascii_lowercase().replace('-', "_")
}

fn map_discover_plugin_error(
    err: discover_bindings::lexicon::plugin_roles::common_types::PluginError,
) -> PluginTypedError {
    match err {
        discover_bindings::lexicon::plugin_roles::common_types::PluginError::NetworkFailure(
            message,
        ) => PluginTypedError {
            kind: PluginErrorKind::NetworkFailure,
            message,
        },
        discover_bindings::lexicon::plugin_roles::common_types::PluginError::ParsingFailure(
            message,
        ) => PluginTypedError {
            kind: PluginErrorKind::ParsingFailure,
            message,
        },
        discover_bindings::lexicon::plugin_roles::common_types::PluginError::RateLimit(message) => {
            PluginTypedError {
                kind: PluginErrorKind::RateLimit,
                message,
            }
        }
        discover_bindings::lexicon::plugin_roles::common_types::PluginError::NotFound(message) => {
            PluginTypedError {
                kind: PluginErrorKind::NotFound,
                message,
            }
        }
        discover_bindings::lexicon::plugin_roles::common_types::PluginError::Unknown(message) => {
            PluginTypedError {
                kind: PluginErrorKind::Unknown,
                message,
            }
        }
    }
}

fn map_source_plugin_error(
    err: source_bindings::lexicon::plugin_roles::common_types::PluginError,
) -> PluginTypedError {
    match err {
        source_bindings::lexicon::plugin_roles::common_types::PluginError::NetworkFailure(
            message,
        ) => PluginTypedError {
            kind: PluginErrorKind::NetworkFailure,
            message,
        },
        source_bindings::lexicon::plugin_roles::common_types::PluginError::ParsingFailure(
            message,
        ) => PluginTypedError {
            kind: PluginErrorKind::ParsingFailure,
            message,
        },
        source_bindings::lexicon::plugin_roles::common_types::PluginError::RateLimit(message) => {
            PluginTypedError {
                kind: PluginErrorKind::RateLimit,
                message,
            }
        }
        source_bindings::lexicon::plugin_roles::common_types::PluginError::NotFound(message) => {
            PluginTypedError {
                kind: PluginErrorKind::NotFound,
                message,
            }
        }
        source_bindings::lexicon::plugin_roles::common_types::PluginError::Unknown(message) => {
            PluginTypedError {
                kind: PluginErrorKind::Unknown,
                message,
            }
        }
    }
}

fn map_manga_plugin_error(
    err: manga_source_bindings::lexicon::plugin_roles::common_types::PluginError,
) -> PluginTypedError {
    match err {
        manga_source_bindings::lexicon::plugin_roles::common_types::PluginError::NetworkFailure(
            message,
        ) => PluginTypedError {
            kind: PluginErrorKind::NetworkFailure,
            message,
        },
        manga_source_bindings::lexicon::plugin_roles::common_types::PluginError::ParsingFailure(
            message,
        ) => PluginTypedError {
            kind: PluginErrorKind::ParsingFailure,
            message,
        },
        manga_source_bindings::lexicon::plugin_roles::common_types::PluginError::RateLimit(
            message,
        ) => PluginTypedError {
            kind: PluginErrorKind::RateLimit,
            message,
        },
        manga_source_bindings::lexicon::plugin_roles::common_types::PluginError::NotFound(
            message,
        ) => PluginTypedError {
            kind: PluginErrorKind::NotFound,
            message,
        },
        manga_source_bindings::lexicon::plugin_roles::common_types::PluginError::Unknown(
            message,
        ) => PluginTypedError {
            kind: PluginErrorKind::Unknown,
            message,
        },
    }
}

fn unknown_error(message: String) -> PluginTypedError {
    PluginTypedError {
        kind: PluginErrorKind::Unknown,
        message,
    }
}

fn is_download_url_usable(url: &str) -> bool {
    let normalized = url.trim();
    if normalized.is_empty() || normalized.contains(' ') {
        return false;
    }

    if normalized.starts_with("magnet:") {
        return true;
    }

    if !(normalized.starts_with("http://") || normalized.starts_with("https://")) {
        return false;
    }

    let lowered = normalized.to_ascii_lowercase();

    !lowered.ends_with(".html") && !lowered.contains("/search")
}
