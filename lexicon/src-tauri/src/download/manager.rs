use std::collections::{HashMap, VecDeque};
use std::env;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use librqbit::api::TorrentIdOrHash;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, Session as TorrentSession, SessionOptions,
    TorrentStats, TorrentStatsState,
};
use reqwest::header::{ACCEPT_ENCODING, RANGE};
use reqwest::StatusCode;
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter};
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::models::{
    AddonRole, DownloadProgressEvent, DownloadRecord, DownloadStateEvent, StartDownloadRequest,
};
use crate::plugins::PluginManager;

const DOWNLOAD_PROGRESS_EVENT: &str = "download:progress";
const DOWNLOAD_STATE_EVENT: &str = "download:state";
const HTTP_CONNECT_TIMEOUT_SECONDS: u64 = 15;
const HTTP_READ_TIMEOUT_SECONDS: u64 = 30;
const HTTP_RETRY_ATTEMPTS: usize = 3;
const HTTP_RETRY_BASE_DELAY_MS: u64 = 750;
const DOWNLOAD_HTTP_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
const HTML_CONTENT_TYPES: [&str; 2] = ["text/html", "application/xhtml+xml"];
const TORRENT_PROGRESS_POLL_INTERVAL_MS: u64 = 700;
const TORRENT_SCAN_FILE_LIMIT: usize = 2048;

pub struct DownloadManager {
    tx: mpsc::Sender<ManagerCommand>,
    pool: SqlitePool,
}

impl DownloadManager {
    pub fn new(
        app_handle: AppHandle,
        pool: SqlitePool,
        plugin_manager: Arc<Mutex<PluginManager>>,
    ) -> Self {
        let (tx, rx) = mpsc::channel(128);
        let torrent_session = initialize_torrent_session(&app_handle);
        let actor = DownloadActor::new(
            app_handle,
            pool.clone(),
            tx.clone(),
            torrent_session,
            plugin_manager,
        );
        tauri::async_runtime::spawn(actor.run(rx));

        Self { tx, pool }
    }

    pub async fn start_download(
        &self,
        source_url: String,
        file_name: Option<String>,
        subfolder: Option<String>,
    ) -> Result<DownloadRecord, String> {
        let request = StartDownloadRequest {
            source_url,
            file_name,
            subfolder,
        };

        let (responder, receiver) = oneshot::channel();
        self.tx
            .send(ManagerCommand::Start { request, responder })
            .await
            .map_err(|_| "download manager unavailable".to_string())?;

        receiver
            .await
            .map_err(|_| "download manager failed to respond".to_string())?
    }

    pub async fn pause_download(&self, id: String) -> Result<(), String> {
        let (responder, receiver) = oneshot::channel();
        self.tx
            .send(ManagerCommand::Pause { id, responder })
            .await
            .map_err(|_| "download manager unavailable".to_string())?;

        receiver
            .await
            .map_err(|_| "download manager failed to respond".to_string())?
    }

    pub async fn resume_download(&self, id: String) -> Result<(), String> {
        let (responder, receiver) = oneshot::channel();
        self.tx
            .send(ManagerCommand::Resume { id, responder })
            .await
            .map_err(|_| "download manager unavailable".to_string())?;

        receiver
            .await
            .map_err(|_| "download manager failed to respond".to_string())?
    }

    pub async fn cancel_download(&self, id: String) -> Result<(), String> {
        let (responder, receiver) = oneshot::channel();
        self.tx
            .send(ManagerCommand::Cancel { id, responder })
            .await
            .map_err(|_| "download manager unavailable".to_string())?;

        receiver
            .await
            .map_err(|_| "download manager failed to respond".to_string())?
    }

    pub async fn remove_download(&self, id: String, delete_file: bool) -> Result<(), String> {
        let (responder, receiver) = oneshot::channel();
        self.tx
            .send(ManagerCommand::Remove {
                id,
                delete_file,
                responder,
            })
            .await
            .map_err(|_| "download manager unavailable".to_string())?;

        receiver
            .await
            .map_err(|_| "download manager failed to respond".to_string())?
    }

    pub async fn list_downloads(&self) -> Result<Vec<DownloadRecord>, String> {
        list_downloads_internal(&self.pool)
            .await
            .map_err(|err| err.to_string())
    }
}

fn initialize_torrent_session(app_handle: &AppHandle) -> Option<Arc<TorrentSession>> {
    let output_dir = match tauri::async_runtime::block_on(resolve_downloads_dir(app_handle)) {
        Ok(path) => path,
        Err(err) => {
            eprintln!(
                "[download-manager] failed to initialize integrated torrent session (output dir): {}",
                err
            );
            return None;
        }
    };

    let options = SessionOptions::default();

    match tauri::async_runtime::block_on(TorrentSession::new_with_opts(output_dir, options)) {
        Ok(session) => Some(session),
        Err(err) => {
            eprintln!(
                "[download-manager] failed to initialize integrated torrent session: {}",
                err
            );
            None
        }
    }
}

fn log_actor_snapshot(
    phase: &str,
    id: Option<&str>,
    detail: &str,
    pending: &VecDeque<String>,
    active: &HashMap<String, ActiveDownload>,
) {
    let pending_preview = pending
        .iter()
        .take(3)
        .cloned()
        .collect::<Vec<_>>()
        .join(",");

    eprintln!(
        "[download-manager][actor][{}] id={} pending={} active={} pending_head=[{}] {}",
        phase,
        id.unwrap_or("-"),
        pending.len(),
        active.len(),
        pending_preview,
        detail
    );
}

fn log_worker_event(id: &str, source_type: &str, message: &str) {
    eprintln!(
        "[download-manager][worker][{}][{}] {}",
        source_type, id, message
    );
}

enum ManagerCommand {
    Start {
        request: StartDownloadRequest,
        responder: oneshot::Sender<Result<DownloadRecord, String>>,
    },
    Pause {
        id: String,
        responder: oneshot::Sender<Result<(), String>>,
    },
    Resume {
        id: String,
        responder: oneshot::Sender<Result<(), String>>,
    },
    Cancel {
        id: String,
        responder: oneshot::Sender<Result<(), String>>,
    },
    Remove {
        id: String,
        delete_file: bool,
        responder: oneshot::Sender<Result<(), String>>,
    },
    WorkerFinished {
        id: String,
    },
}

#[derive(Clone)]
struct ActiveDownload {
    control_tx: mpsc::UnboundedSender<WorkerControl>,
}

#[derive(Debug)]
enum WorkerControl {
    Pause,
    Cancel,
}

struct DownloadActor {
    app_handle: AppHandle,
    pool: SqlitePool,
    pending: VecDeque<String>,
    active: HashMap<String, ActiveDownload>,
    max_concurrent: usize,
    tx: mpsc::Sender<ManagerCommand>,
    torrent_session: Option<Arc<TorrentSession>>,
    plugin_manager: Arc<Mutex<PluginManager>>,
}

impl DownloadActor {
    fn new(
        app_handle: AppHandle,
        pool: SqlitePool,
        tx: mpsc::Sender<ManagerCommand>,
        torrent_session: Option<Arc<TorrentSession>>,
        plugin_manager: Arc<Mutex<PluginManager>>,
    ) -> Self {
        let max_concurrent = env::var("LEXICON_MAX_CONCURRENT_DOWNLOADS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(2)
            .max(1);

        Self {
            app_handle,
            pool,
            pending: VecDeque::new(),
            active: HashMap::new(),
            max_concurrent,
            tx,
            torrent_session,
            plugin_manager,
        }
    }

    async fn run(mut self, mut rx: mpsc::Receiver<ManagerCommand>) {
        while let Some(command) = rx.recv().await {
            match command {
                ManagerCommand::Start { request, responder } => {
                    log_actor_snapshot(
                        "command:start",
                        None,
                        &format!(
                            "source_type={} source_url={} file_name={}",
                            detect_source_type(&request.source_url),
                            request.source_url,
                            request.file_name.as_deref().unwrap_or("<auto>")
                        ),
                        &self.pending,
                        &self.active,
                    );
                    let result = self.handle_start(request).await;
                    log_actor_snapshot(
                        "result:start",
                        result.as_ref().ok().map(|r| r.id.as_str()),
                        &format!("ok={}", result.is_ok()),
                        &self.pending,
                        &self.active,
                    );
                    let _ = responder.send(result);
                }
                ManagerCommand::Pause { id, responder } => {
                    log_actor_snapshot("command:pause", Some(&id), "received", &self.pending, &self.active);
                    let result = self.handle_pause(&id).await;
                    log_actor_snapshot(
                        "result:pause",
                        Some(&id),
                        &format!("ok={}", result.is_ok()),
                        &self.pending,
                        &self.active,
                    );
                    let _ = responder.send(result);
                }
                ManagerCommand::Resume { id, responder } => {
                    log_actor_snapshot("command:resume", Some(&id), "received", &self.pending, &self.active);
                    let result = self.handle_resume(&id).await;
                    log_actor_snapshot(
                        "result:resume",
                        Some(&id),
                        &format!("ok={}", result.is_ok()),
                        &self.pending,
                        &self.active,
                    );
                    let _ = responder.send(result);
                }
                ManagerCommand::Cancel { id, responder } => {
                    log_actor_snapshot("command:cancel", Some(&id), "received", &self.pending, &self.active);
                    let result = self.handle_cancel(&id).await;
                    log_actor_snapshot(
                        "result:cancel",
                        Some(&id),
                        &format!("ok={}", result.is_ok()),
                        &self.pending,
                        &self.active,
                    );
                    let _ = responder.send(result);
                }
                ManagerCommand::Remove {
                    id,
                    delete_file,
                    responder,
                } => {
                    log_actor_snapshot(
                        "command:remove",
                        Some(&id),
                        &format!("delete_file={}", delete_file),
                        &self.pending,
                        &self.active,
                    );
                    let result = self.handle_remove(&id, delete_file).await;
                    log_actor_snapshot(
                        "result:remove",
                        Some(&id),
                        &format!("ok={}", result.is_ok()),
                        &self.pending,
                        &self.active,
                    );
                    let _ = responder.send(result);
                }
                ManagerCommand::WorkerFinished { id } => {
                    self.active.remove(&id);
                    log_actor_snapshot(
                        "worker:finished",
                        Some(&id),
                        "removed from active; scheduling next",
                        &self.pending,
                        &self.active,
                    );
                    if let Err(err) = self.try_start_next().await {
                        eprintln!(
                            "[download-manager] failed to schedule next download: {}",
                            err
                        );
                    }
                }
            }
        }
    }

    async fn handle_start(
        &mut self,
        request: StartDownloadRequest,
    ) -> Result<DownloadRecord, String> {
        let normalized_url = request.source_url.trim().to_string();
        if normalized_url.is_empty() {
            return Err("source_url is required".to_string());
        }

        if !is_supported_source_url(&normalized_url) {
            return Err("Unsupported source_url: use http(s), magnet, or .torrent".to_string());
        }

        let source_type = detect_source_type(&normalized_url).to_string();
        let id = Uuid::new_v4().to_string();
        let file_name = request
            .file_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(sanitize_file_name)
            .unwrap_or_else(|| derive_file_name(&normalized_url, &id));

        let downloads_dir = resolve_downloads_dir(&self.app_handle)
            .await
            .map_err(|err| format!("failed to resolve downloads directory: {}", err))?;

        let subfolder = request
            .subfolder
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(sanitize_file_name);

        let initial_file_path = match subfolder {
            Some(folder) => downloads_dir.join(folder).join(&file_name),
            None => downloads_dir.join(&file_name),
        }
        .to_string_lossy()
        .to_string();

        sqlx::query(
            "INSERT INTO downloads (id, source_url, source_type, file_name, file_path, status, downloaded_bytes, speed_bps, max_retries)
             VALUES (?, ?, ?, ?, ?, 'queued', 0, NULL, 3)",
        )
        .bind(&id)
        .bind(&normalized_url)
        .bind(&source_type)
        .bind(&file_name)
        .bind(&initial_file_path)
        .execute(&self.pool)
        .await
        .map_err(|err| err.to_string())?;

        self.pending.push_back(id.clone());
        self.try_start_next().await.map_err(|err| err.to_string())?;

        fetch_download_by_id(&self.pool, &id)
            .await
            .map_err(|err| err.to_string())?
            .ok_or_else(|| "failed to fetch created download".to_string())
    }

    async fn handle_pause(&mut self, id: &str) -> Result<(), String> {
        log_actor_snapshot("pause:enter", Some(id), "begin", &self.pending, &self.active);

        if let Some(active) = self.active.get(id) {
            if active.control_tx.send(WorkerControl::Pause).is_ok() {
                log_actor_snapshot(
                    "pause:active-control",
                    Some(id),
                    "sent control=Pause to active worker",
                    &self.pending,
                    &self.active,
                );
                return Ok(());
            }

            // Worker likely exited before the actor processed WorkerFinished.
            self.active.remove(id);
            log_actor_snapshot(
                "pause:active-stale",
                Some(id),
                "control channel closed; removed stale active entry",
                &self.pending,
                &self.active,
            );
        }

        let download = fetch_download_by_id(&self.pool, id)
            .await
            .map_err(|err| err.to_string())?;

        let Some(download) = download else {
            log_actor_snapshot(
                "pause:not-found",
                Some(id),
                "download missing",
                &self.pending,
                &self.active,
            );
            return Ok(());
        };

        if remove_pending(&mut self.pending, id) {
            update_download_status(&self.pool, id, "paused", None, None, Some(0), None, false)
                .await
                .map_err(|err| err.to_string())?;
            emit_state_for_download(&self.app_handle, &self.pool, id).await;
            log_actor_snapshot(
                "pause:pending",
                Some(id),
                "removed from pending and marked paused",
                &self.pending,
                &self.active,
            );
            return Ok(());
        }

        if download.source_type == "torrent" && download.status == "downloading" {
            if let Some(session) = self.torrent_session.as_ref() {
                if let Some(torrent_key) = torrent_key_from_download(&download) {
                    if let Some(handle) = session.get(torrent_key) {
                        if !handle.is_paused() {
                            session.pause(&handle).await.map_err(|err| {
                                format!("failed to pause torrent in session: {}", err)
                            })?;
                        }

                        let stats = handle.stats();
                        update_download_status(
                            &self.pool,
                            id,
                            "paused",
                            u64_to_i64_option(stats.total_bytes),
                            Some(u64_to_i64(stats.progress_bytes)),
                            Some(0),
                            download.file_path.clone(),
                            false,
                        )
                        .await
                        .map_err(|err| err.to_string())?;

                        emit_state_for_download(&self.app_handle, &self.pool, id).await;
                        log_actor_snapshot(
                            "pause:session",
                            Some(id),
                            "paused via torrent session fallback",
                            &self.pending,
                            &self.active,
                        );
                        return Ok(());
                    }
                }
            }
        }

        log_actor_snapshot(
            "pause:no-op",
            Some(id),
            &format!(
                "no matching branch source_type={} status={}",
                download.source_type, download.status
            ),
            &self.pending,
            &self.active,
        );

        Ok(())
    }

    async fn handle_resume(&mut self, id: &str) -> Result<(), String> {
        log_actor_snapshot("resume:enter", Some(id), "begin", &self.pending, &self.active);

        let download = fetch_download_by_id(&self.pool, id)
            .await
            .map_err(|err| err.to_string())?
            .ok_or_else(|| "download not found".to_string())?;

        log_actor_snapshot(
            "resume:state",
            Some(id),
            &format!(
                "db_status={} source_type={} progress={:.2}",
                download.status, download.source_type, download.progress_percent
            ),
            &self.pending,
            &self.active,
        );

        if download.status == "completed" || download.status == "cancelled" {
            return Err(format!(
                "cannot resume download with status '{}'",
                download.status
            ));
        }

        if self.active.contains_key(id) {
            // Allow "resume" to be requested while pause/cancel transition is still draining.
            // This avoids losing the click when worker is still marked active for a short window.
            if matches!(download.status.as_str(), "paused" | "queued" | "failed") {
                update_download_status(&self.pool, id, "queued", None, None, Some(0), None, false)
                    .await
                    .map_err(|err| err.to_string())?;

                if !self.pending.iter().any(|pending_id| pending_id == id) {
                    self.pending.push_back(id.to_string());
                }

                emit_state_for_download(&self.app_handle, &self.pool, id).await;
                log_actor_snapshot(
                    "resume:deferred",
                    Some(id),
                    "active transition in-flight; requeued for deferred resume",
                    &self.pending,
                    &self.active,
                );
            }

            return Ok(());
        }

        update_download_status(&self.pool, id, "queued", None, None, Some(0), None, false)
            .await
            .map_err(|err| err.to_string())?;

        if !self.pending.iter().any(|pending_id| pending_id == id) {
            self.pending.push_back(id.to_string());
        }

        emit_state_for_download(&self.app_handle, &self.pool, id).await;
        log_actor_snapshot(
            "resume:queued",
            Some(id),
            "marked queued and scheduling",
            &self.pending,
            &self.active,
        );
        self.try_start_next().await.map_err(|err| err.to_string())
    }

    async fn handle_cancel(&mut self, id: &str) -> Result<(), String> {
        log_actor_snapshot("cancel:enter", Some(id), "begin", &self.pending, &self.active);

        let download = fetch_download_by_id(&self.pool, id)
            .await
            .map_err(|err| err.to_string())?;

        let Some(download) = download else {
            log_actor_snapshot(
                "cancel:not-found",
                Some(id),
                "download missing",
                &self.pending,
                &self.active,
            );
            return Ok(());
        };

        if let Some(active) = self.active.get(id) {
            if active.control_tx.send(WorkerControl::Cancel).is_ok() {
                log_actor_snapshot(
                    "cancel:active-control",
                    Some(id),
                    "sent control=Cancel to active worker",
                    &self.pending,
                    &self.active,
                );
                return Ok(());
            }

            // Worker likely exited before the actor processed WorkerFinished.
            self.active.remove(id);
            log_actor_snapshot(
                "cancel:active-stale",
                Some(id),
                "control channel closed; removed stale active entry",
                &self.pending,
                &self.active,
            );
        }

        let was_pending = remove_pending(&mut self.pending, id);

        if download.source_type == "torrent" {
            cancel_torrent_in_session(
                self.torrent_session.as_ref(),
                &download,
                true,
                "cancel",
            )
            .await;
        } else if let Some(path) = download
            .file_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let _ = remove_path_if_exists(Path::new(path)).await;
        }

        if was_pending
            || download_exists(&self.pool, id)
                .await
                .map_err(|err| err.to_string())?
        {
            update_download_status(&self.pool, id, "cancelled", None, None, Some(0), None, true)
                .await
                .map_err(|err| err.to_string())?;
            emit_state_for_download(&self.app_handle, &self.pool, id).await;
            log_actor_snapshot(
                "cancel:finalized",
                Some(id),
                "marked cancelled",
                &self.pending,
                &self.active,
            );
        }

        Ok(())
    }

    async fn handle_remove(&mut self, id: &str, delete_file: bool) -> Result<(), String> {
        if self.active.contains_key(id) {
            return Err("Cannot remove an active download. Pause or cancel it first.".to_string());
        }

        remove_pending(&mut self.pending, id);

        let Some(download) = fetch_download_by_id(&self.pool, id)
            .await
            .map_err(|err| err.to_string())?
        else {
            return Ok(());
        };

        if download.source_type == "torrent" {
            cancel_torrent_in_session(
                self.torrent_session.as_ref(),
                &download,
                delete_file,
                "remove",
            )
            .await;
        }

        if delete_file {
            remove_download_artifact(&download).await?;
        }

        sqlx::query("DELETE FROM downloads WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|err| err.to_string())?;

        Ok(())
    }

    async fn try_start_next(&mut self) -> Result<(), sqlx::Error> {
        while self.active.len() < self.max_concurrent {
            let Some(id) = self.pending.pop_front() else {
                log_actor_snapshot(
                    "schedule:empty",
                    None,
                    "pending queue empty",
                    &self.pending,
                    &self.active,
                );
                break;
            };

            log_actor_snapshot(
                "schedule:pick",
                Some(&id),
                "picked pending item",
                &self.pending,
                &self.active,
            );

            let Some(download) = fetch_download_by_id(&self.pool, &id).await? else {
                log_actor_snapshot(
                    "schedule:missing",
                    Some(&id),
                    "download record not found anymore",
                    &self.pending,
                    &self.active,
                );
                continue;
            };

            if matches!(download.status.as_str(), "cancelled" | "completed") {
                log_actor_snapshot(
                    "schedule:skip",
                    Some(&id),
                    &format!("skip status={}", download.status),
                    &self.pending,
                    &self.active,
                );
                continue;
            }

            update_download_status(
                &self.pool,
                &id,
                "downloading",
                download.total_bytes,
                Some(download.downloaded_bytes),
                Some(0),
                None,
                false,
            )
            .await?;

            let updated = fetch_download_by_id(&self.pool, &id).await?;
            if let Some(updated) = updated {
                let _ = self.app_handle.emit(
                    DOWNLOAD_STATE_EVENT,
                    DownloadStateEvent::from_record(&updated),
                );

                let active = spawn_worker(
                    updated,
                    self.pool.clone(),
                    self.app_handle.clone(),
                    self.tx.clone(),
                    self.torrent_session.clone(),
                    self.plugin_manager.clone(),
                );
                let active_id = id.clone();
                self.active.insert(id, active);
                log_actor_snapshot(
                    "schedule:spawned",
                    Some(active_id.as_str()),
                    "worker spawned and inserted in active map",
                    &self.pending,
                    &self.active,
                );
            }
        }

        Ok(())
    }
}

fn spawn_worker(
    download: DownloadRecord,
    pool: SqlitePool,
    app_handle: AppHandle,
    tx: mpsc::Sender<ManagerCommand>,
    torrent_session: Option<Arc<TorrentSession>>,
    plugin_manager: Arc<Mutex<PluginManager>>,
) -> ActiveDownload {
    let (control_tx, control_rx) = mpsc::unbounded_channel();
    tauri::async_runtime::spawn(async move {
        let id = download.id.clone();
        log_worker_event(
            &id,
            &download.source_type,
            &format!("spawned status={} file={}", download.status, download.file_name),
        );

        match download.source_type.as_str() {
            "http" | "opds" => {
                run_http_download(download, pool.clone(), app_handle.clone(), control_rx).await;
            }
            "torrent" => {
                run_torrent_download(
                    download,
                    pool.clone(),
                    app_handle.clone(),
                    torrent_session,
                    control_rx,
                )
                .await;
            }
            "manga-cbz" => {
                run_manga_cbz_download(
                    download,
                    pool.clone(),
                    app_handle.clone(),
                    plugin_manager,
                    control_rx,
                )
                .await;
            }
            other => {
                mark_download_failed(
                    &pool,
                    &app_handle,
                    &id,
                    format!("unsupported source_type '{}'", other),
                )
                .await;
            }
        }

        log_worker_event(&id, "any", "finished; notifying actor");

        let _ = tx.send(ManagerCommand::WorkerFinished { id }).await;
    });

    ActiveDownload { control_tx }
}

async fn run_http_download(
    download: DownloadRecord,
    pool: SqlitePool,
    app_handle: AppHandle,
    mut control_rx: mpsc::UnboundedReceiver<WorkerControl>,
) {
    let destination_path = match resolve_target_path(&app_handle, &download).await {
        Ok(path) => path,
        Err(err) => {
            mark_download_failed(&pool, &app_handle, &download.id, err).await;
            return;
        }
    };

    let mut existing_bytes = fs::metadata(&destination_path)
        .await
        .map(|metadata| metadata.len() as i64)
        .unwrap_or(0);

    let client = match reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(HTTP_CONNECT_TIMEOUT_SECONDS))
        .read_timeout(Duration::from_secs(HTTP_READ_TIMEOUT_SECONDS))
        .http1_only()
        .user_agent(DOWNLOAD_HTTP_USER_AGENT)
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            mark_download_failed(
                &pool,
                &app_handle,
                &download.id,
                format!("http client error: {}", err),
            )
            .await;
            return;
        }
    };

    let mut retry_attempt = 0usize;

    'download_attempt: loop {
        let mut request = client
            .get(&download.source_url)
            .header(ACCEPT_ENCODING, "identity");

        if existing_bytes > 0 {
            request = request.header(RANGE, format!("bytes={}-", existing_bytes));
        }

        let response = match request.send().await {
            Ok(response) => response,
            Err(err) => {
                if should_retry_http_error(&err) && retry_attempt + 1 < HTTP_RETRY_ATTEMPTS {
                    retry_attempt += 1;
                    let delay = retry_backoff_delay(retry_attempt);
                    eprintln!(
                        "[download-manager] request attempt {} failed for {}: {}; retrying in {}ms",
                        retry_attempt,
                        download.source_url,
                        format_http_error(&err),
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                    continue 'download_attempt;
                }

                mark_download_failed(
                    &pool,
                    &app_handle,
                    &download.id,
                    format!("request failed: {}", format_http_error(&err)),
                )
                .await;
                return;
            }
        };

        let response_status = response.status();

        if !(response_status.is_success() || response_status == StatusCode::PARTIAL_CONTENT) {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<no-body>".to_string());
            mark_download_failed(
                &pool,
                &app_handle,
                &download.id,
                format!("http status {}: {}", response_status, body),
            )
            .await;
            return;
        }

        if existing_bytes == 0 && response_status == StatusCode::OK {
            let content_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .map(str::to_ascii_lowercase)
                .unwrap_or_default();

            let is_html = HTML_CONTENT_TYPES
                .iter()
                .any(|mime| content_type.starts_with(mime));

            if is_html {
                mark_download_failed(
                    &pool,
                    &app_handle,
                    &download.id,
                    format!(
                        "download URL returned HTML page instead of file (content-type: {})",
                        content_type
                    ),
                )
                .await;
                return;
            }
        }

        let append_mode = existing_bytes > 0 && response_status == StatusCode::PARTIAL_CONTENT;
        if existing_bytes > 0 && response_status == StatusCode::OK {
            existing_bytes = 0;
        }

        let total_bytes = response
            .content_length()
            .map(|value| value as i64 + existing_bytes);

        if let Err(err) = update_download_status(
            &pool,
            &download.id,
            "downloading",
            total_bytes,
            Some(existing_bytes),
            Some(0),
            Some(destination_path.to_string_lossy().to_string()),
            false,
        )
        .await
        {
            eprintln!(
                "[download-manager] failed updating download status: {}",
                err
            );
        }

        if let Err(err) = ensure_parent_dir(&destination_path).await {
            mark_download_failed(&pool, &app_handle, &download.id, err).await;
            return;
        }

        let mut options = OpenOptions::new();
        options.create(true).write(true);
        if append_mode {
            options.append(true);
        } else {
            options.truncate(true);
        }

        let mut file = match options.open(&destination_path).await {
            Ok(file) => file,
            Err(err) => {
                mark_download_failed(
                    &pool,
                    &app_handle,
                    &download.id,
                    format!("failed to open target file: {}", err),
                )
                .await;
                return;
            }
        };

        let mut downloaded_bytes = existing_bytes;
        let mut response = response;
        let mut last_progress_emit = Instant::now();
        let mut bytes_since_last_emit: i64 = 0;

        loop {
            tokio::select! {
                control = control_rx.recv() => {
                    match control {
                        Some(WorkerControl::Pause) => {
                            finalize_paused(
                                &pool,
                                &app_handle,
                                &download,
                                &destination_path,
                                total_bytes,
                                downloaded_bytes,
                            ).await;
                            return;
                        }
                        Some(WorkerControl::Cancel) => {
                            finalize_cancelled(
                                &pool,
                                &app_handle,
                                &download,
                                &destination_path,
                            ).await;
                            return;
                        }
                        None => {}
                    }
                }
                next = response.chunk() => {
                    match next {
                        Ok(Some(chunk)) => {
                            if let Err(err) = file.write_all(&chunk).await {
                                mark_download_failed(
                                    &pool,
                                    &app_handle,
                                    &download.id,
                                    format!("write failed: {}", err),
                                ).await;
                                return;
                            }

                            let chunk_size = chunk.len() as i64;
                            downloaded_bytes += chunk_size;
                            bytes_since_last_emit += chunk_size;

                            if last_progress_emit.elapsed() >= Duration::from_millis(400) {
                                let elapsed = last_progress_emit.elapsed().as_secs_f64().max(0.001);
                                let speed_bps = (bytes_since_last_emit as f64 / elapsed).round() as i64;
                                bytes_since_last_emit = 0;
                                last_progress_emit = Instant::now();

                                if let Err(err) = persist_progress(
                                    &pool,
                                    &download.id,
                                    total_bytes,
                                    downloaded_bytes,
                                    Some(speed_bps),
                                ).await {
                                    eprintln!("[download-manager] failed persisting progress: {}", err);
                                }

                                emit_progress(
                                    &app_handle,
                                    &download,
                                    "downloading",
                                    downloaded_bytes,
                                    total_bytes,
                                    Some(speed_bps),
                                );
                            }
                        }
                        Ok(None) => {
                            break;
                        }
                        Err(err) => {
                            if should_retry_http_error(&err) && retry_attempt + 1 < HTTP_RETRY_ATTEMPTS {
                                retry_attempt += 1;
                                existing_bytes = downloaded_bytes;
                                let delay = retry_backoff_delay(retry_attempt);
                                eprintln!(
                                    "[download-manager] stream attempt {} failed for {} at byte {}: {}; retrying in {}ms",
                                    retry_attempt,
                                    download.source_url,
                                    downloaded_bytes,
                                    format_http_error(&err),
                                    delay.as_millis()
                                );
                                let _ = file.flush().await;
                                tokio::time::sleep(delay).await;
                                continue 'download_attempt;
                            }

                            mark_download_failed(
                                &pool,
                                &app_handle,
                                &download.id,
                                format!("stream read failed: {}", format_http_error(&err)),
                            ).await;
                            return;
                        }
                    }
                }
            }
        }

        if let Err(err) = file.flush().await {
            mark_download_failed(
                &pool,
                &app_handle,
                &download.id,
                format!("flush failed: {}", err),
            )
            .await;
            return;
        }

        if let Err(err) = sqlx::query(
            "UPDATE downloads SET status = 'completed', downloaded_bytes = ?, total_bytes = COALESCE(total_bytes, ?), speed_bps = 0, error_message = NULL, file_path = ?, completed_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(downloaded_bytes)
        .bind(downloaded_bytes)
        .bind(destination_path.to_string_lossy().to_string())
        .bind(&download.id)
        .execute(&pool)
        .await
        {
            eprintln!("[download-manager] failed marking completed: {}", err);
        }

        emit_progress(
            &app_handle,
            &download,
            "completed",
            downloaded_bytes,
            Some(downloaded_bytes),
            Some(0),
        );
        emit_state_for_download(&app_handle, &pool, &download.id).await;
        return;
    }
}

async fn run_torrent_download(
    download: DownloadRecord,
    pool: SqlitePool,
    app_handle: AppHandle,
    torrent_session: Option<Arc<TorrentSession>>,
    mut control_rx: mpsc::UnboundedReceiver<WorkerControl>,
) {
    log_worker_event(
        &download.id,
        "torrent",
        &format!(
            "enter run_torrent_download status={} source={}",
            download.status, download.source_url
        ),
    );

    let Some(torrent_session) = torrent_session else {
        log_worker_event(
            &download.id,
            "torrent",
            "abort: torrent session unavailable",
        );
        mark_download_failed(
            &pool,
            &app_handle,
            &download.id,
            "integrated torrent session unavailable".to_string(),
        )
        .await;
        return;
    };

    let output_dir = match resolve_downloads_dir(&app_handle).await {
        Ok(path) => path,
        Err(err) => {
            mark_download_failed(&pool, &app_handle, &download.id, err).await;
            return;
        }
    };

    let torrent_subfolder = format!("torrent-{}", download.id);
    let output_path_hint = output_dir.join(&torrent_subfolder);
    let output_path_string = output_path_hint.to_string_lossy().to_string();

    let existing_handle = torrent_key_from_download(&download)
        .and_then(|torrent_key| torrent_session.get(torrent_key));

    let handle = if let Some(existing_handle) = existing_handle {
        log_worker_event(
            &download.id,
            "torrent",
            "reusing existing session handle from torrent_info_hash",
        );
        existing_handle
    } else {
        let add_target = if is_http_or_magnet_url(&download.source_url) {
            AddTorrent::from_url(download.source_url.clone())
        } else {
            let torrent_bytes = match fs::read(&download.source_url).await {
                Ok(bytes) => bytes,
                Err(err) => {
                    mark_download_failed(
                        &pool,
                        &app_handle,
                        &download.id,
                        format!("failed to read torrent file '{}': {}", download.source_url, err),
                    )
                    .await;
                    return;
                }
            };
            AddTorrent::from_bytes(torrent_bytes)
        };

        let add_options = AddTorrentOptions {
            sub_folder: Some(torrent_subfolder.clone()),
            overwrite: true,
            ..Default::default()
        };

        let mut add_future = Box::pin(torrent_session.add_torrent(add_target, Some(add_options)));

        let add_response = loop {
            tokio::select! {
                add_response = &mut add_future => {
                    break match add_response {
                        Ok(response) => response,
                        Err(err) => {
                            log_worker_event(
                                &download.id,
                                "torrent",
                                &format!("add_torrent failed: {}", err),
                            );
                            mark_download_failed(
                                &pool,
                                &app_handle,
                                &download.id,
                                format!("failed to initialize integrated torrent: {}", err),
                            )
                            .await;
                            return;
                        }
                    };
                }
                control = control_rx.recv() => {
                    match control {
                        Some(WorkerControl::Pause) => {
                            log_worker_event(
                                &download.id,
                                "torrent",
                                "received control=Pause before handle acquisition",
                            );

                            if let Err(err) = update_download_status(
                                &pool,
                                &download.id,
                                "paused",
                                download.total_bytes,
                                Some(download.downloaded_bytes),
                                Some(0),
                                Some(output_path_string.clone()),
                                false,
                            ).await {
                                eprintln!("[download-manager] failed pausing torrent before handle acquisition: {}", err);
                            }

                            emit_progress(
                                &app_handle,
                                &download,
                                "paused",
                                download.downloaded_bytes,
                                download.total_bytes,
                                Some(0),
                            );
                            emit_state_for_download(&app_handle, &pool, &download.id).await;
                            return;
                        }
                        Some(WorkerControl::Cancel) => {
                            log_worker_event(
                                &download.id,
                                "torrent",
                                "received control=Cancel before handle acquisition",
                            );

                            cancel_torrent_in_session(
                                Some(&torrent_session),
                                &download,
                                true,
                                "cancel-pre-handle",
                            )
                            .await;

                            if let Err(err) = remove_path_if_exists(&output_path_hint).await {
                                eprintln!("[download-manager] failed removing cancelled pre-handle torrent target: {}", err);
                            }

                            if let Err(err) = update_download_status(
                                &pool,
                                &download.id,
                                "cancelled",
                                None,
                                None,
                                Some(0),
                                None,
                                true,
                            ).await {
                                eprintln!("[download-manager] failed cancelling torrent before handle acquisition: {}", err);
                            }

                            emit_state_for_download(&app_handle, &pool, &download.id).await;
                            return;
                        }
                        None => {}
                    }
                }
            }
        };

        match add_response {
            AddTorrentResponse::Added(_, handle)
            | AddTorrentResponse::AlreadyManaged(_, handle) => {
                log_worker_event(
                    &download.id,
                    "torrent",
                    "handle acquired (added or already managed)",
                );
                handle
            }
            AddTorrentResponse::ListOnly(_) => {
                log_worker_event(
                    &download.id,
                    "torrent",
                    "unexpected list-only response",
                );
                mark_download_failed(
                    &pool,
                    &app_handle,
                    &download.id,
                    "unexpected list-only response from integrated torrent engine".to_string(),
                )
                .await;
                return;
            }
        }
    };

    if handle.is_paused() {
        log_worker_event(
            &download.id,
            "torrent",
            "handle initially paused; attempting unpause",
        );
        if let Err(err) = torrent_session.unpause(&handle).await {
            log_worker_event(
                &download.id,
                "torrent",
                &format!("initial unpause failed: {}", err),
            );
            mark_download_failed(
                &pool,
                &app_handle,
                &download.id,
                format!("failed to start integrated torrent: {}", err),
            )
            .await;
            return;
        }
    }

    let torrent_key = TorrentIdOrHash::Id(handle.id());
    let info_hash = handle.info_hash().as_string();

    let initial_stats = handle.stats();
    if let Err(err) = persist_torrent_progress(
        &pool,
        &download.id,
        u64_to_i64_option(initial_stats.total_bytes),
        u64_to_i64(initial_stats.progress_bytes),
        torrent_speed_bps_from_stats(&initial_stats),
        torrent_peers_from_stats(&initial_stats),
        Some(info_hash.as_str()),
        Some(output_path_string.as_str()),
    )
    .await
    {
        eprintln!(
            "[download-manager] failed persisting initial torrent tracking: {}",
            err
        );
    }

    let mut progress_tick = tokio::time::interval(Duration::from_millis(
        TORRENT_PROGRESS_POLL_INTERVAL_MS,
    ));
    progress_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            control = control_rx.recv() => {
                match control {
                    Some(WorkerControl::Pause) => {
                        log_worker_event(&download.id, "torrent", "received control=Pause");
                        if !handle.is_paused() {
                            if let Err(err) = torrent_session.pause(&handle).await {
                                eprintln!("[download-manager] failed pausing integrated torrent: {}", err);
                                log_worker_event(
                                    &download.id,
                                    "torrent",
                                    &format!("pause failed; keep worker alive: {}", err),
                                );

                                // Keep worker alive if pause failed so we don't lose lifecycle control.
                                let stats = handle.stats();
                                let total_bytes = u64_to_i64_option(stats.total_bytes);
                                let downloaded_bytes = u64_to_i64(stats.progress_bytes);
                                let speed_bps = torrent_speed_bps_from_stats(&stats);
                                let peers = torrent_peers_from_stats(&stats);

                                if let Err(err) = persist_torrent_progress(
                                    &pool,
                                    &download.id,
                                    total_bytes,
                                    downloaded_bytes,
                                    speed_bps,
                                    peers,
                                    Some(info_hash.as_str()),
                                    Some(output_path_string.as_str()),
                                ).await {
                                    eprintln!("[download-manager] failed syncing torrent state after pause error: {}", err);
                                }

                                emit_progress(
                                    &app_handle,
                                    &download,
                                    "downloading",
                                    downloaded_bytes,
                                    total_bytes,
                                    Some(speed_bps),
                                );

                                emit_state_for_download(&app_handle, &pool, &download.id).await;
                                continue;
                            }
                        }

                        let stats = handle.stats();
                        let total_bytes = u64_to_i64_option(stats.total_bytes);
                        let downloaded_bytes = u64_to_i64(stats.progress_bytes);
                        let peers = torrent_peers_from_stats(&stats);

                        if let Err(err) = update_download_status(
                            &pool,
                            &download.id,
                            "paused",
                            total_bytes,
                            Some(downloaded_bytes),
                            Some(0),
                            Some(output_path_string.clone()),
                            false,
                        ).await {
                            eprintln!("[download-manager] failed pausing torrent: {}", err);
                        }

                        if let Err(err) = persist_torrent_tracking(
                            &pool,
                            &download.id,
                            Some(info_hash.as_str()),
                            peers,
                            None,
                        ).await {
                            eprintln!("[download-manager] failed updating torrent tracking on pause: {}", err);
                        }

                        emit_progress(
                            &app_handle,
                            &download,
                            "paused",
                            downloaded_bytes,
                            total_bytes,
                            Some(0),
                        );

                        emit_state_for_download(&app_handle, &pool, &download.id).await;
                        log_worker_event(&download.id, "torrent", "pause finalized; worker exiting");
                        return;
                    }
                    Some(WorkerControl::Cancel) => {
                        log_worker_event(&download.id, "torrent", "received control=Cancel");
                        if let Err(err) = torrent_session.delete(torrent_key, true).await {
                            eprintln!("[download-manager] failed deleting integrated torrent data: {}", err);
                            log_worker_event(
                                &download.id,
                                "torrent",
                                &format!("session delete on cancel failed: {}", err),
                            );
                            if let Err(remove_err) = remove_path_if_exists(&output_path_hint).await {
                                eprintln!("[download-manager] failed removing cancelled torrent target: {}", remove_err);
                            }
                        }

                        if let Err(err) = update_download_status(
                            &pool,
                            &download.id,
                            "cancelled",
                            None,
                            None,
                            Some(0),
                            None,
                            true,
                        ).await {
                            eprintln!("[download-manager] failed cancelling torrent: {}", err);
                        }

                        if let Err(err) = persist_torrent_tracking(
                            &pool,
                            &download.id,
                            Some(info_hash.as_str()),
                            Some(0),
                            None,
                        ).await {
                            eprintln!("[download-manager] failed updating torrent tracking on cancel: {}", err);
                        }

                        emit_state_for_download(&app_handle, &pool, &download.id).await;
                        log_worker_event(&download.id, "torrent", "cancel finalized; worker exiting");
                        return;
                    }
                    None => {}
                }
            }
            _ = progress_tick.tick() => {
                let stats = handle.stats();

                if matches!(stats.state, TorrentStatsState::Error) {
                    let message = stats
                        .error
                        .unwrap_or_else(|| "integrated torrent entered error state".to_string());

                    let _ = torrent_session.delete(torrent_key, false).await;
                    log_worker_event(
                        &download.id,
                        "torrent",
                        &format!("stats entered error state: {}", message),
                    );
                    mark_download_failed(&pool, &app_handle, &download.id, message).await;
                    return;
                }

                let total_bytes = u64_to_i64_option(stats.total_bytes);
                let downloaded_bytes = u64_to_i64(stats.progress_bytes);
                let speed_bps = torrent_speed_bps_from_stats(&stats);
                let peers = torrent_peers_from_stats(&stats);

                if let Err(err) = persist_torrent_progress(
                    &pool,
                    &download.id,
                    total_bytes,
                    downloaded_bytes,
                    speed_bps,
                    peers,
                    Some(info_hash.as_str()),
                    Some(output_path_string.as_str()),
                ).await {
                    eprintln!("[download-manager] failed persisting torrent progress: {}", err);
                }

                emit_progress(
                    &app_handle,
                    &download,
                    "downloading",
                    downloaded_bytes,
                    total_bytes,
                    Some(speed_bps),
                );

                if stats.finished {
                    log_worker_event(
                        &download.id,
                        "torrent",
                        &format!(
                            "stats finished=true downloaded={} total={:?}",
                            downloaded_bytes, total_bytes
                        ),
                    );
                    let final_path = match resolve_completed_torrent_path(&output_path_hint).await {
                        Ok(path) => path,
                        Err(err) => {
                            eprintln!("[download-manager] failed resolving final torrent path: {}", err);
                            output_path_hint.clone()
                        }
                    };

                    let final_total = total_bytes.or(Some(downloaded_bytes));

                    if let Err(err) = update_download_status(
                        &pool,
                        &download.id,
                        "completed",
                        final_total,
                        Some(downloaded_bytes),
                        Some(0),
                        Some(final_path.to_string_lossy().to_string()),
                        true,
                    ).await {
                        eprintln!("[download-manager] failed completing torrent: {}", err);
                    }

                    if let Err(err) = persist_torrent_tracking(
                        &pool,
                        &download.id,
                        Some(info_hash.as_str()),
                        Some(0),
                        None,
                    ).await {
                        eprintln!("[download-manager] failed updating torrent tracking on completion: {}", err);
                    }

                    emit_progress(
                        &app_handle,
                        &download,
                        "completed",
                        downloaded_bytes,
                        final_total,
                        Some(0),
                    );

                    emit_state_for_download(&app_handle, &pool, &download.id).await;

                    if let Err(err) = torrent_session.delete(torrent_key, false).await {
                        eprintln!("[download-manager] failed forgetting completed torrent handle: {}", err);
                    }

                    log_worker_event(&download.id, "torrent", "completed finalized; worker exiting");

                    return;
                }
            }
        }
    }
}

struct MangaCbzUrl {
    plugin_id: String,
    chapter_id: String,
}

fn parse_manga_cbz_url(source_url: &str) -> Option<MangaCbzUrl> {
    let rest = source_url.strip_prefix("mangacbz://")?;
    let mut parts = rest.splitn(2, '/');
    let plugin_id = parts.next()?.trim();
    let chapter_id = parts.next()?.trim();

    if plugin_id.is_empty() || chapter_id.is_empty() {
        return None;
    }

    Some(MangaCbzUrl {
        plugin_id: plugin_id.to_string(),
        chapter_id: chapter_id.to_string(),
    })
}

async fn run_manga_cbz_download(
    download: DownloadRecord,
    pool: SqlitePool,
    app_handle: AppHandle,
    plugin_manager: Arc<Mutex<PluginManager>>,
    mut control_rx: mpsc::UnboundedReceiver<WorkerControl>,
) {
    let Some(parsed) = parse_manga_cbz_url(&download.source_url) else {
        mark_download_failed(
            &pool,
            &app_handle,
            &download.id,
            format!("invalid manga-cbz source url: {}", download.source_url),
        )
        .await;
        return;
    };

    let destination_path = match resolve_target_path(&app_handle, &download).await {
        Ok(path) => path,
        Err(err) => {
            mark_download_failed(&pool, &app_handle, &download.id, err).await;
            return;
        }
    };

    if let Err(err) = ensure_parent_dir(&destination_path).await {
        mark_download_failed(&pool, &app_handle, &download.id, err).await;
        return;
    }

    let resolved: Result<_, String> = (|| {
        let manager = plugin_manager
            .lock()
            .map_err(|_| "plugin manager lock poisoned".to_string())?;

        let plugin = manager
            .plugin_by_id(&parsed.plugin_id)
            .ok_or_else(|| format!("manga plugin '{}' not found", parsed.plugin_id))?;

        if plugin.role != AddonRole::MangaSource {
            return Err(format!(
                "plugin '{}' is not a manga source",
                parsed.plugin_id
            ));
        }
        if !plugin.enabled {
            return Err(format!("manga plugin '{}' is disabled", parsed.plugin_id));
        }

        let snapshot = manager.runtime_snapshot();
        Ok((snapshot.engine, snapshot.fuel_per_invocation, plugin))
    })();

    let (engine, fuel_per_invocation, plugin_descriptor) = match resolved {
        Ok(tuple) => tuple,
        Err(err) => {
            mark_download_failed(&pool, &app_handle, &download.id, err).await;
            return;
        }
    };

    let chapter_id = parsed.chapter_id.clone();
    let page_fetch = tokio::task::spawn_blocking({
        let engine = engine.clone();
        let plugin = plugin_descriptor.clone();
        move || {
            PluginManager::execute_manga_get_chapter_pages(
                &engine,
                fuel_per_invocation,
                &plugin,
                &chapter_id,
            )
        }
    })
    .await;

    let page_list = match page_fetch {
        Ok(Ok(list)) => list,
        Ok(Err(err)) => {
            mark_download_failed(
                &pool,
                &app_handle,
                &download.id,
                format!("failed to list pages: {}", err.message),
            )
            .await;
            return;
        }
        Err(err) => {
            mark_download_failed(
                &pool,
                &app_handle,
                &download.id,
                format!("manga page fetch join error: {}", err),
            )
            .await;
            return;
        }
    };

    let urls = page_list.page_urls;
    if urls.is_empty() {
        mark_download_failed(
            &pool,
            &app_handle,
            &download.id,
            "chapter has no pages".to_string(),
        )
        .await;
        return;
    }

    let total_pages = urls.len() as i64;

    if let Err(err) = update_download_status(
        &pool,
        &download.id,
        "downloading",
        Some(total_pages),
        Some(0),
        Some(0),
        Some(destination_path.to_string_lossy().to_string()),
        false,
    )
    .await
    {
        eprintln!("[download-manager] failed to init manga progress: {}", err);
    }

    emit_progress(
        &app_handle,
        &download,
        "downloading",
        0,
        Some(total_pages),
        Some(0),
    );

    let client = match reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(HTTP_CONNECT_TIMEOUT_SECONDS))
        .read_timeout(Duration::from_secs(HTTP_READ_TIMEOUT_SECONDS))
        .user_agent(DOWNLOAD_HTTP_USER_AGENT)
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            mark_download_failed(
                &pool,
                &app_handle,
                &download.id,
                format!("http client error: {}", err),
            )
            .await;
            return;
        }
    };

    let mut images: Vec<(String, Vec<u8>)> = Vec::with_capacity(urls.len());
    let mut last_emit = Instant::now();
    let mut failed_pages: Vec<usize> = Vec::new();

    for (index, url) in urls.iter().enumerate() {
        loop {
            match control_rx.try_recv() {
                Ok(WorkerControl::Cancel) => {
                    let _ = remove_path_if_exists(&destination_path).await;
                    if let Err(err) = update_download_status(
                        &pool,
                        &download.id,
                        "cancelled",
                        None,
                        None,
                        Some(0),
                        None,
                        true,
                    )
                    .await
                    {
                        eprintln!("[download-manager] cancel finalize failed: {}", err);
                    }
                    emit_state_for_download(&app_handle, &pool, &download.id).await;
                    return;
                }
                Ok(WorkerControl::Pause) => {
                    if let Err(err) = update_download_status(
                        &pool,
                        &download.id,
                        "paused",
                        Some(total_pages),
                        Some(index as i64),
                        Some(0),
                        Some(destination_path.to_string_lossy().to_string()),
                        false,
                    )
                    .await
                    {
                        eprintln!("[download-manager] pause finalize failed: {}", err);
                    }
                    emit_state_for_download(&app_handle, &pool, &download.id).await;
                    return;
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Empty)
                | Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break,
            }
        }

        let bytes = match fetch_image_with_retry(&client, url).await {
            Ok(bytes) => bytes,
            Err(err) => {
                eprintln!(
                    "[download-manager] page {} fetch failed, skipping: {}",
                    index + 1,
                    err
                );
                failed_pages.push(index + 1);
                continue;
            }
        };

        let extension = detect_image_extension(url, &bytes);
        let entry_name = format!("{:03}.{}", index + 1, extension);
        images.push((entry_name, bytes));

        let downloaded_pages = (index + 1) as i64;

        if last_emit.elapsed() >= Duration::from_millis(250) || downloaded_pages == total_pages {
            last_emit = Instant::now();
            if let Err(err) = persist_progress(
                &pool,
                &download.id,
                Some(total_pages),
                downloaded_pages,
                None,
            )
            .await
            {
                eprintln!("[download-manager] failed persisting manga progress: {}", err);
            }

            emit_progress(
                &app_handle,
                &download,
                "downloading",
                downloaded_pages,
                Some(total_pages),
                None,
            );
        }
    }

    if images.is_empty() {
        mark_download_failed(
            &pool,
            &app_handle,
            &download.id,
            format!(
                "all {} page(s) failed to download",
                failed_pages.len()
            ),
        )
        .await;
        return;
    }

    if !failed_pages.is_empty() {
        eprintln!(
            "[download-manager] manga cbz {}: skipped {} unavailable page(s): {:?}",
            download.id,
            failed_pages.len(),
            failed_pages
        );
    }

    let zip_destination = destination_path.clone();
    let zip_result = tokio::task::spawn_blocking(move || -> std::io::Result<()> {
        let file = std::fs::File::create(&zip_destination)?;
        let mut writer = zip::ZipWriter::new(file);
        let options: zip::write::SimpleFileOptions =
            zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

        for (name, bytes) in &images {
            writer
                .start_file(name, options)
                .map_err(|err| std::io::Error::new(ErrorKind::Other, err.to_string()))?;
            writer.write_all(bytes)?;
        }

        writer
            .finish()
            .map_err(|err| std::io::Error::new(ErrorKind::Other, err.to_string()))?;
        Ok(())
    })
    .await;

    match zip_result {
        Ok(Ok(())) => {}
        Ok(Err(err)) => {
            mark_download_failed(
                &pool,
                &app_handle,
                &download.id,
                format!("failed to write cbz: {}", err),
            )
            .await;
            return;
        }
        Err(err) => {
            mark_download_failed(
                &pool,
                &app_handle,
                &download.id,
                format!("cbz writer join error: {}", err),
            )
            .await;
            return;
        }
    }

    if let Err(err) = sqlx::query(
        "UPDATE downloads SET status = 'completed', downloaded_bytes = ?, total_bytes = ?, speed_bps = 0, error_message = NULL, file_path = ?, completed_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(total_pages)
    .bind(total_pages)
    .bind(destination_path.to_string_lossy().to_string())
    .bind(&download.id)
    .execute(&pool)
    .await
    {
        eprintln!("[download-manager] manga complete update failed: {}", err);
    }

    emit_progress(
        &app_handle,
        &download,
        "completed",
        total_pages,
        Some(total_pages),
        Some(0),
    );
    emit_state_for_download(&app_handle, &pool, &download.id).await;
}

async fn fetch_image_with_retry(
    client: &reqwest::Client,
    url: &str,
) -> Result<Vec<u8>, String> {
    let mut last_err: Option<String> = None;

    for attempt in 0..HTTP_RETRY_ATTEMPTS {
        match client.get(url).send().await {
            Ok(response) => {
                if !response.status().is_success() {
                    last_err = Some(format!("status {}", response.status()));
                } else {
                    match response.bytes().await {
                        Ok(bytes) => return Ok(bytes.to_vec()),
                        Err(err) => last_err = Some(format_http_error(&err)),
                    }
                }
            }
            Err(err) => last_err = Some(format_http_error(&err)),
        }

        if attempt + 1 < HTTP_RETRY_ATTEMPTS {
            tokio::time::sleep(retry_backoff_delay(attempt + 1)).await;
        }
    }

    Err(last_err.unwrap_or_else(|| "unknown error".to_string()))
}

fn detect_image_extension(url: &str, bytes: &[u8]) -> &'static str {
    if bytes.len() >= 8 && &bytes[0..8] == b"\x89PNG\r\n\x1a\n" {
        return "png";
    }

    if bytes.len() >= 3 && &bytes[0..3] == b"\xff\xd8\xff" {
        return "jpg";
    }

    if bytes.len() >= 6 && (&bytes[0..6] == b"GIF87a" || &bytes[0..6] == b"GIF89a") {
        return "gif";
    }

    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return "webp";
    }

    let lowered = url.split('?').next().unwrap_or(url).to_ascii_lowercase();
    for candidate in ["png", "jpg", "jpeg", "webp", "gif"] {
        if lowered.ends_with(&format!(".{}", candidate)) {
            return if candidate == "jpeg" { "jpg" } else { candidate };
        }
    }

    "jpg"
}

async fn finalize_paused(
    pool: &SqlitePool,
    app_handle: &AppHandle,
    download: &DownloadRecord,
    destination_path: &Path,
    total_bytes: Option<i64>,
    downloaded_bytes: i64,
) {
    if let Err(err) = update_download_status(
        pool,
        &download.id,
        "paused",
        total_bytes,
        Some(downloaded_bytes),
        Some(0),
        Some(destination_path.to_string_lossy().to_string()),
        false,
    )
    .await
    {
        eprintln!("[download-manager] failed finalizing pause: {}", err);
    }

    emit_progress(
        app_handle,
        download,
        "paused",
        downloaded_bytes,
        total_bytes,
        Some(0),
    );
    emit_state_for_download(app_handle, pool, &download.id).await;
}

async fn finalize_cancelled(
    pool: &SqlitePool,
    app_handle: &AppHandle,
    download: &DownloadRecord,
    destination_path: &Path,
) {
    if let Err(err) = remove_path_if_exists(destination_path).await {
        eprintln!("[download-manager] failed removing cancelled target: {}", err);
    }

    if let Err(err) = update_download_status(
        pool,
        &download.id,
        "cancelled",
        None,
        None,
        Some(0),
        None,
        true,
    )
    .await
    {
        eprintln!("[download-manager] failed finalizing cancel: {}", err);
    }

    emit_state_for_download(app_handle, pool, &download.id).await;
}

async fn mark_download_failed(
    pool: &SqlitePool,
    app_handle: &AppHandle,
    id: &str,
    message: String,
) {
    if let Err(err) = sqlx::query(
        "UPDATE downloads SET status = 'failed', error_message = ?, speed_bps = 0, completed_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(&message)
    .bind(id)
    .execute(pool)
    .await
    {
        eprintln!("[download-manager] failed to mark download failed: {}", err);
    }

    emit_state_for_download(app_handle, pool, id).await;
}

async fn update_download_status(
    pool: &SqlitePool,
    id: &str,
    status: &str,
    total_bytes: Option<i64>,
    downloaded_bytes: Option<i64>,
    speed_bps: Option<i64>,
    file_path: Option<String>,
    mark_completed: bool,
) -> Result<(), sqlx::Error> {
    let completed_sql = if mark_completed {
        ", completed_at = CURRENT_TIMESTAMP"
    } else {
        ""
    };

    let query = format!(
        "UPDATE downloads
         SET status = ?,
             total_bytes = COALESCE(?, total_bytes),
             downloaded_bytes = COALESCE(?, downloaded_bytes),
             speed_bps = ?,
             error_message = NULL,
             file_path = COALESCE(?, file_path),
             started_at = COALESCE(started_at, CURRENT_TIMESTAMP)
         {} WHERE id = ?",
        completed_sql
    );

    sqlx::query(&query)
        .bind(status)
        .bind(total_bytes)
        .bind(downloaded_bytes)
        .bind(speed_bps)
        .bind(file_path)
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

async fn persist_progress(
    pool: &SqlitePool,
    id: &str,
    total_bytes: Option<i64>,
    downloaded_bytes: i64,
    speed_bps: Option<i64>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE downloads
         SET status = 'downloading',
             total_bytes = COALESCE(?, total_bytes),
             downloaded_bytes = ?,
             speed_bps = ?,
             error_message = NULL,
             started_at = COALESCE(started_at, CURRENT_TIMESTAMP)
         WHERE id = ?",
    )
    .bind(total_bytes)
    .bind(downloaded_bytes)
    .bind(speed_bps)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}

async fn emit_state_for_download(app_handle: &AppHandle, pool: &SqlitePool, id: &str) {
    match fetch_download_by_id(pool, id).await {
        Ok(Some(record)) => {
            let _ = app_handle.emit(
                DOWNLOAD_STATE_EVENT,
                DownloadStateEvent::from_record(&record),
            );
        }
        Ok(None) => {}
        Err(err) => {
            eprintln!("[download-manager] failed emitting state event: {}", err);
        }
    }
}

fn emit_progress(
    app_handle: &AppHandle,
    download: &DownloadRecord,
    status: &str,
    downloaded_bytes: i64,
    total_bytes: Option<i64>,
    speed_bps: Option<i64>,
) {
    let progress_percent = match total_bytes {
        Some(total) if total > 0 => ((downloaded_bytes as f64 / total as f64) * 100.0) as f32,
        _ => 0.0,
    }
    .clamp(0.0, 100.0);

    let event = DownloadProgressEvent {
        id: download.id.clone(),
        file_name: download.file_name.clone(),
        status: status.to_string(),
        downloaded_bytes,
        total_bytes,
        speed_bps,
        progress_percent,
    };

    let _ = app_handle.emit(DOWNLOAD_PROGRESS_EVENT, event);
}

async fn list_downloads_internal(pool: &SqlitePool) -> Result<Vec<DownloadRecord>, sqlx::Error> {
    sqlx::query_as::<_, DownloadRecord>(
        "SELECT
            id,
            source_url,
            source_type,
            file_name,
            file_path,
            status,
            error_message,
            total_bytes,
            downloaded_bytes,
            speed_bps,
            torrent_info_hash,
            torrent_peers,
            torrent_seeds,
            CASE
                WHEN total_bytes IS NULL OR total_bytes <= 0 THEN 0.0
                ELSE (CAST(downloaded_bytes AS REAL) * 100.0 / CAST(total_bytes AS REAL))
            END AS progress_percent,
            created_at,
            started_at,
            completed_at
         FROM downloads
         ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await
}

async fn fetch_download_by_id(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<DownloadRecord>, sqlx::Error> {
    sqlx::query_as::<_, DownloadRecord>(
        "SELECT
            id,
            source_url,
            source_type,
            file_name,
            file_path,
            status,
            error_message,
            total_bytes,
            downloaded_bytes,
            speed_bps,
            torrent_info_hash,
            torrent_peers,
            torrent_seeds,
            CASE
                WHEN total_bytes IS NULL OR total_bytes <= 0 THEN 0.0
                ELSE (CAST(downloaded_bytes AS REAL) * 100.0 / CAST(total_bytes AS REAL))
            END AS progress_percent,
            created_at,
            started_at,
            completed_at
         FROM downloads
         WHERE id = ?
         LIMIT 1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

async fn download_exists(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
    let row = sqlx::query_as::<_, (i64,)>("SELECT COUNT(1) FROM downloads WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;

    Ok(row.0 > 0)
}

async fn resolve_target_path(
    app_handle: &AppHandle,
    download: &DownloadRecord,
) -> Result<PathBuf, String> {
    if let Some(existing_path) = download
        .file_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(PathBuf::from(existing_path));
    }

    let downloads_dir = resolve_downloads_dir(app_handle).await?;
    Ok(downloads_dir.join(sanitize_file_name(&download.file_name)))
}

async fn resolve_downloads_dir(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let path = crate::storage::resolve_lexicon_data_dir(app_handle)
        .map_err(|err| format!("failed to resolve lexicon data dir: {}", err))?;

    fs::create_dir_all(&path)
        .await
        .map_err(|err| format!("failed to create downloads directory: {}", err))?;

    Ok(path)
}

async fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|err| format!("failed to create target directory: {}", err))?;
    }

    Ok(())
}

fn remove_pending(pending: &mut VecDeque<String>, id: &str) -> bool {
    let before_len = pending.len();
    pending.retain(|pending_id| pending_id != id);
    pending.len() != before_len
}

fn should_retry_http_error(err: &reqwest::Error) -> bool {
    err.is_timeout() || err.is_connect() || err.is_body() || err.is_decode() || err.is_request()
}

fn retry_backoff_delay(attempt: usize) -> Duration {
    let multiplier = attempt.max(1).min(4) as u64;
    Duration::from_millis(HTTP_RETRY_BASE_DELAY_MS * multiplier)
}

fn format_http_error(err: &reqwest::Error) -> String {
    let mut message = err.to_string();

    if err.is_timeout() {
        message.push_str(" [timeout]");
    }
    if err.is_connect() {
        message.push_str(" [connect]");
    }
    if err.is_decode() {
        message.push_str(" [decode]");
    }

    message
}

fn is_supported_source_url(source_url: &str) -> bool {
    let normalized = source_url.to_ascii_lowercase();
    normalized.starts_with("http://")
        || normalized.starts_with("https://")
        || normalized.starts_with("magnet:")
        || normalized.ends_with(".torrent")
        || normalized.starts_with("mangacbz://")
}

fn is_http_or_magnet_url(source_url: &str) -> bool {
    let normalized = source_url.to_ascii_lowercase();
    normalized.starts_with("http://")
        || normalized.starts_with("https://")
        || normalized.starts_with("magnet:")
}

fn torrent_key_from_download(download: &DownloadRecord) -> Option<TorrentIdOrHash> {
    download
        .torrent_info_hash
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| TorrentIdOrHash::parse(value).ok())
}

async fn cancel_torrent_in_session(
    torrent_session: Option<&Arc<TorrentSession>>,
    download: &DownloadRecord,
    delete_files: bool,
    context: &str,
) {
    let Some(session) = torrent_session else {
        eprintln!(
            "[download-manager][session][{}][{}] skip delete: no torrent session",
            context, download.id
        );
        return;
    };

    let Some(torrent_key) = torrent_key_from_download(download) else {
        eprintln!(
            "[download-manager][session][{}][{}] skip delete: missing torrent_info_hash",
            context, download.id
        );
        return;
    };

    eprintln!(
        "[download-manager][session][{}][{}] deleting torrent delete_files={} key={}",
        context,
        download.id,
        delete_files,
        torrent_key
    );

    if let Err(err) = session.delete(torrent_key, delete_files).await {
        eprintln!(
            "[download-manager] failed session torrent {} for {}: {}",
            context,
            download.id,
            err
        );
    } else {
        eprintln!(
            "[download-manager][session][{}][{}] delete succeeded",
            context, download.id
        );
    }
}

fn detect_source_type(source_url: &str) -> &'static str {
    let normalized = source_url.to_ascii_lowercase();
    if normalized.starts_with("magnet:") || normalized.ends_with(".torrent") {
        return "torrent";
    }

    if normalized.starts_with("mangacbz://") {
        return "manga-cbz";
    }

    "http"
}

fn derive_file_name(source_url: &str, fallback_id: &str) -> String {
    let without_query = source_url
        .split('?')
        .next()
        .unwrap_or(source_url)
        .trim_end_matches('/');

    let candidate = without_query
        .rsplit('/')
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback_id);

    sanitize_file_name(candidate)
}

fn sanitize_file_name(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => ch,
        })
        .collect::<String>()
        .trim()
        .to_string();

    if sanitized.is_empty() {
        "download.bin".to_string()
    } else {
        sanitized
    }
}

async fn remove_download_artifact(download: &DownloadRecord) -> Result<(), String> {
    let Some(path) = download
        .file_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };

    remove_path_if_exists(Path::new(path)).await
}

async fn remove_path_if_exists(path: &Path) -> Result<(), String> {
    let metadata = match fs::metadata(path).await {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(()),
        Err(err) => {
            return Err(format!(
                "failed to inspect target '{}': {}",
                path.display(),
                err
            ));
        }
    };

    if metadata.is_dir() {
        fs::remove_dir_all(path).await.map_err(|err| {
            format!(
                "failed to remove directory '{}': {}",
                path.display(),
                err
            )
        })?;
    } else {
        fs::remove_file(path).await.map_err(|err| {
            format!("failed to remove file '{}': {}", path.display(), err)
        })?;
    }

    Ok(())
}

fn u64_to_i64(value: u64) -> i64 {
    value.min(i64::MAX as u64) as i64
}

fn u64_to_i64_option(value: u64) -> Option<i64> {
    if value == 0 {
        None
    } else {
        Some(u64_to_i64(value))
    }
}

fn torrent_speed_bps_from_stats(stats: &TorrentStats) -> i64 {
    let Some(live_stats) = stats.live.as_ref() else {
        return 0;
    };

    let bytes_per_second = live_stats.download_speed.mbps * 1024.0 * 1024.0;
    if !bytes_per_second.is_finite() || bytes_per_second <= 0.0 {
        return 0;
    }

    bytes_per_second.round().min(i64::MAX as f64) as i64
}

fn torrent_peers_from_stats(stats: &TorrentStats) -> Option<i64> {
    stats
        .live
        .as_ref()
        .map(|live_stats| live_stats.snapshot.peer_stats.live as i64)
}

async fn persist_torrent_progress(
    pool: &SqlitePool,
    id: &str,
    total_bytes: Option<i64>,
    downloaded_bytes: i64,
    speed_bps: i64,
    peers: Option<i64>,
    info_hash: Option<&str>,
    file_path: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE downloads
         SET status = 'downloading',
             total_bytes = COALESCE(?, total_bytes),
             downloaded_bytes = ?,
             speed_bps = ?,
             torrent_peers = ?,
             torrent_info_hash = COALESCE(?, torrent_info_hash),
             file_path = COALESCE(?, file_path),
             error_message = NULL,
             started_at = COALESCE(started_at, CURRENT_TIMESTAMP)
         WHERE id = ?",
    )
    .bind(total_bytes)
    .bind(downloaded_bytes)
    .bind(speed_bps)
    .bind(peers)
    .bind(info_hash)
    .bind(file_path)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}

async fn persist_torrent_tracking(
    pool: &SqlitePool,
    id: &str,
    info_hash: Option<&str>,
    peers: Option<i64>,
    seeds: Option<i64>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE downloads
         SET torrent_info_hash = COALESCE(?, torrent_info_hash),
             torrent_peers = ?,
             torrent_seeds = ?
         WHERE id = ?",
    )
    .bind(info_hash)
    .bind(peers)
    .bind(seeds)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}

async fn resolve_completed_torrent_path(output_path: &Path) -> Result<PathBuf, String> {
    let metadata = match fs::metadata(output_path).await {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(output_path.to_path_buf()),
        Err(err) => {
            return Err(format!(
                "failed to inspect completed torrent output '{}': {}",
                output_path.display(),
                err
            ));
        }
    };

    if !metadata.is_dir() {
        return Ok(output_path.to_path_buf());
    }

    let files = collect_files_recursively_limited(output_path, TORRENT_SCAN_FILE_LIMIT).await?;
    if files.is_empty() {
        return Ok(output_path.to_path_buf());
    }

    let epub_files: Vec<PathBuf> = files
        .iter()
        .filter(|path| {
            path.extension()
                .and_then(|value| value.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("epub"))
        })
        .cloned()
        .collect();

    if epub_files.len() == 1 {
        return Ok(epub_files[0].clone());
    }

    if files.len() == 1 {
        return Ok(files[0].clone());
    }

    Ok(output_path.to_path_buf())
}

async fn collect_files_recursively_limited(
    root: &Path,
    limit: usize,
) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let mut entries = fs::read_dir(&dir).await.map_err(|err| {
            format!(
                "failed reading torrent directory '{}': {}",
                dir.display(),
                err
            )
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|err| {
            format!(
                "failed iterating torrent directory '{}': {}",
                dir.display(),
                err
            )
        })? {
            let entry_path = entry.path();
            let file_type = entry.file_type().await.map_err(|err| {
                format!(
                    "failed reading file type for '{}': {}",
                    entry_path.display(),
                    err
                )
            })?;

            if file_type.is_dir() {
                stack.push(entry_path);
                continue;
            }

            if file_type.is_file() {
                files.push(entry_path);
                if files.len() >= limit {
                    return Ok(files);
                }
            }
        }
    }

    Ok(files)
}
