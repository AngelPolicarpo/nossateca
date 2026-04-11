import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { AddonsView } from "./components/AddonsView";
import { DiscoverView } from "./components/DiscoverView";
import { LibraryView, type Book } from "./components/LibraryView";
import { ReaderView } from "./components/ReaderView";
import { useSearch, type SearchBookResult } from "./hooks/useSearch";
import "./App.css";

type AppTab = "home" | "discover" | "library" | "downloads" | "addons";

type DownloadStatus = "queued" | "downloading" | "paused" | "completed" | "failed" | "cancelled";

type DownloadItem = {
  id: string;
  sourceUrl: string;
  sourceType: "http" | "torrent" | "opds";
  fileName: string;
  filePath: string | null;
  status: DownloadStatus;
  errorMessage: string | null;
  totalBytes: number | null;
  downloadedBytes: number;
  speedBps: number | null;
  progressPercent: number;
  createdAt: string;
  startedAt: string | null;
  completedAt: string | null;
};

type DownloadProgressEvent = {
  id: string;
  fileName: string;
  status: DownloadStatus;
  downloadedBytes: number;
  totalBytes: number | null;
  speedBps: number | null;
  progressPercent: number;
};

type DownloadStateEvent = {
  id: string;
  fileName: string;
  status: DownloadStatus;
  filePath: string | null;
  errorMessage: string | null;
  downloadedBytes: number;
  totalBytes: number | null;
  speedBps: number | null;
  progressPercent: number;
};

const navItems: Array<{ key: AppTab; label: string }> = [
  { key: "home", label: "Inicio" },
  { key: "discover", label: "Discover" },
  { key: "library", label: "Biblioteca" },
  { key: "downloads", label: "Downloads" },
  { key: "addons", label: "Addons" },
];

const DOWNLOAD_STATUS_LABELS: Record<DownloadStatus, string> = {
  queued: "Na fila",
  downloading: "Baixando",
  paused: "Pausado",
  completed: "Concluído",
  failed: "Falhou",
  cancelled: "Cancelado",
};

const DOWNLOAD_STATUS_TONES: Record<DownloadStatus, string> = {
  queued: "queued",
  downloading: "running",
  paused: "paused",
  completed: "success",
  failed: "danger",
  cancelled: "danger",
};

const DOWNLOAD_SOURCE_LABELS: Record<DownloadItem["sourceType"], string> = {
  http: "HTTP",
  torrent: "Torrent",
  opds: "OPDS",
};

function App() {
  const [activeTab, setActiveTab] = useState<AppTab>("home");
  const [mobileNavOpen, setMobileNavOpen] = useState(false);
  const [selectedBookId, setSelectedBookId] = useState<string | null>(null);
  const [books, setBooks] = useState<Book[]>([]);
  const [booksLoading, setBooksLoading] = useState(true);
  const [booksError, setBooksError] = useState<string | null>(null);
  const [semanticSearch, setSemanticSearch] = useState("");
  const [downloads, setDownloads] = useState<DownloadItem[]>([]);
  const [downloadsLoading, setDownloadsLoading] = useState(false);
  const [downloadsError, setDownloadsError] = useState<string | null>(null);

  const searchQuery = useSearch(semanticSearch);
  const hasSearchInput = semanticSearch.trim().length >= 2;

  const getErrorMessage = (err: unknown, fallback: string): string => {
    if (err instanceof Error && err.message.trim().length > 0) {
      return err.message;
    }

    if (typeof err === "string" && err.trim().length > 0) {
      return err;
    }

    return fallback;
  };

  const formatBytes = (value: number | null): string => {
    if (value === null || Number.isNaN(value)) {
      return "?";
    }

    if (value < 1024) {
      return `${Math.max(0, Math.floor(value))} B`;
    }

    const units = ["KB", "MB", "GB", "TB"];
    let size = value / 1024;
    let unitIndex = 0;

    while (size >= 1024 && unitIndex < units.length - 1) {
      size /= 1024;
      unitIndex += 1;
    }

    return `${size.toFixed(size >= 10 ? 1 : 2)} ${units[unitIndex]}`;
  };

  const loadBooks = useCallback(async () => {
    setBooksLoading(true);
    setBooksError(null);

    try {
      const result = await invoke<Book[]>("list_books");
      setBooks(result);
    } catch (err) {
      setBooksError(getErrorMessage(err, "Falha ao carregar biblioteca"));
    } finally {
      setBooksLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadBooks();
  }, [loadBooks]);

  useEffect(() => {
    if (activeTab === "home") {
      void loadBooks();
    }
  }, [activeTab, loadBooks]);

  useEffect(() => {
    setMobileNavOpen(false);
  }, [activeTab]);

  const loadDownloads = useCallback(async () => {
    setDownloadsLoading(true);
    setDownloadsError(null);

    try {
      const result = await invoke<DownloadItem[]>("list_downloads");
      setDownloads(result);
    } catch (err) {
      setDownloadsError(getErrorMessage(err, "Falha ao carregar downloads"));
    } finally {
      setDownloadsLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadDownloads();
  }, [loadDownloads]);

  useEffect(() => {
    let unlistenProgress: UnlistenFn | null = null;
    let unlistenState: UnlistenFn | null = null;

    const registerListeners = async () => {
      unlistenProgress = await listen<DownloadProgressEvent>("download:progress", (event) => {
        setDownloads((previous) =>
          previous.map((item) =>
            item.id === event.payload.id
              ? {
                  ...item,
                  status: event.payload.status,
                  downloadedBytes: event.payload.downloadedBytes,
                  totalBytes: event.payload.totalBytes,
                  speedBps: event.payload.speedBps,
                  progressPercent: event.payload.progressPercent,
                }
              : item,
          ),
        );
      });

      unlistenState = await listen<DownloadStateEvent>("download:state", (event) => {
        setDownloads((previous) =>
          previous.map((item) =>
            item.id === event.payload.id
              ? {
                  ...item,
                  status: event.payload.status,
                  filePath: event.payload.filePath,
                  errorMessage: event.payload.errorMessage,
                  downloadedBytes: event.payload.downloadedBytes,
                  totalBytes: event.payload.totalBytes,
                  speedBps: event.payload.speedBps,
                  progressPercent: event.payload.progressPercent,
                }
              : item,
          ),
        );
      });
    };

    void registerListeners();

    return () => {
      if (unlistenProgress) {
        void unlistenProgress();
      }
      if (unlistenState) {
        void unlistenState();
      }
    };
  }, []);

  const continueReading = books[0] ?? null;

  const startDownload = useCallback(
    async (sourceUrl: string, fileName?: string) => {
      await invoke<DownloadItem>("start_download", {
        sourceUrl,
        fileName: fileName ?? null,
      });
      await loadDownloads();
    },
    [loadDownloads],
  );

  const pauseDownload = useCallback(
    async (id: string) => {
      await invoke("pause_download", { id });
      await loadDownloads();
    },
    [loadDownloads],
  );

  const resumeDownload = useCallback(
    async (id: string) => {
      await invoke("resume_download", { id });
      await loadDownloads();
    },
    [loadDownloads],
  );

  const cancelDownload = useCallback(
    async (id: string) => {
      await invoke("cancel_download", { id });
      await loadDownloads();
    },
    [loadDownloads],
  );

  const removeDownload = useCallback(
    async (id: string, deleteFile: boolean) => {
      await invoke("remove_download", { id, deleteFile });
      await loadDownloads();
    },
    [loadDownloads],
  );

  const addDownloadedBookToLibrary = useCallback(
    async (item: DownloadItem) => {
      if (!item.filePath) {
        alert("Arquivo nao encontrado para adicionar a biblioteca.");
        return;
      }

      const normalizedPath = item.filePath.toLowerCase();
      if (!normalizedPath.endsWith(".epub") && !normalizedPath.endsWith(".pdf")) {
        alert("Somente arquivos EPUB e PDF podem ser adicionados automaticamente a biblioteca.");
        return;
      }

      try {
        await invoke<Book>("add_book", { filePath: item.filePath });
        await loadBooks();
        alert("Livro adicionado a biblioteca.");
      } catch (err) {
        alert(getErrorMessage(err, "Falha ao adicionar livro a biblioteca"));
      }
    },
    [loadBooks],
  );

  const queueSearchResultForDownload = async (result: SearchBookResult) => {
    const format = (result.format ?? "bin").toLowerCase();

    try {
      await startDownload(result.download_url, `${result.title}.${format}`);
      setActiveTab("downloads");
    } catch (err) {
      const message = getErrorMessage(err, "Falha ao enfileirar download");
      alert(message);
    }
  };

  const queueDiscoverResultForDownload = useCallback(
    async (sourceUrl: string, fileName: string) => {
      await startDownload(sourceUrl, fileName);
      setActiveTab("downloads");
    },
    [startDownload],
  );

  const openAddDownloadPrompt = async () => {
    const sourceUrl = window.prompt("Cole a URL HTTP/HTTPS ou Magnet link");
    if (!sourceUrl || sourceUrl.trim().length === 0) {
      return;
    }

    const suggestedName = window.prompt("Nome opcional do arquivo (pressione OK para automatico)") ?? "";

    try {
      await startDownload(sourceUrl.trim(), suggestedName.trim().length > 0 ? suggestedName.trim() : undefined);
      setActiveTab("downloads");
    } catch (err) {
      const message = getErrorMessage(err, "Falha ao iniciar download");
      alert(message);
    }
  };

  if (selectedBookId) {
    return (
      <ReaderView
        bookId={selectedBookId}
        onClose={() => {
          setSelectedBookId(null);
          void loadBooks();
        }}
      />
    );
  }

  return (
    <div className="app-shell">
      <header className="global-navbar">
        <div className="global-navbar-inner">
          <div className="brand-wrap">
            <div className="brand-mark" aria-hidden>
              LX
            </div>
            <strong>Lexicon</strong>
          </div>

          <button
            type="button"
            className="nav-mobile-toggle"
            aria-expanded={mobileNavOpen}
            aria-controls="global-nav"
            onClick={() => setMobileNavOpen((prev) => !prev)}
          >
            <span className="nav-mobile-icon" aria-hidden>
              <span />
              <span />
              <span />
            </span>
          </button>

          <nav
            id="global-nav"
            className={`global-nav-links ${mobileNavOpen ? "open" : ""}`}
            aria-label="Navegacao principal"
          >
            {navItems.map((item) => (
              <button
                key={item.key}
                type="button"
                className={activeTab === item.key ? "active" : ""}
                onClick={() => setActiveTab(item.key)}
              >
                {item.label}
              </button>
            ))}
          </nav>
        </div>
      </header>

      <main className="app-content">
        {activeTab === "home" && (
          <section className="home-screen">
            <header className="hero-search-card">
              <p className="hero-label">Busca semântica</p>
              <h1>O que você quer ler hoje?</h1>
              <p className="hero-subtitle">
                Continue de onde parou ou descubra novos livros.
              </p>
              <label className="hero-search-input">
                <span className="sr-only">Buscar livros e autores</span>
                <input
                  value={semanticSearch}
                  onChange={(event) => setSemanticSearch(event.target.value)}
                  placeholder="Buscar livros, autores ou temas..."
                />
              </label>

              {hasSearchInput && (
                <section className="hero-search-results" aria-live="polite">
                  {searchQuery.isLoading && (
                    <p className="state-message">Buscando fontes via plugins...</p>
                  )}

                  {searchQuery.isError && (
                    <p className="state-message error">
                      {getErrorMessage(searchQuery.error, "Falha ao buscar livros")}
                    </p>
                  )}

                  {!searchQuery.isLoading && !searchQuery.isError && (searchQuery.data?.length ?? 0) === 0 && (
                    <p className="state-message">Nenhum resultado encontrado para esta busca.</p>
                  )}

                  {!searchQuery.isLoading && !searchQuery.isError && (searchQuery.data?.length ?? 0) > 0 && (
                    <ul className="search-results-list">
                      {searchQuery.data?.map((result) => (
                        <li key={result.id} className="search-result-item">
                          <div>
                            <h3>{result.title}</h3>
                            <p>{result.author ?? "Autor desconhecido"}</p>
                            <p className="search-result-meta">
                              Fonte: {result.source} | Formato: {(result.format ?? "n/a").toUpperCase()} | Score: {result.score.toFixed(2)}
                            </p>
                          </div>
                          <div className="search-result-actions">
                            <button
                              type="button"
                              className="secondary-button compact"
                              onClick={() => void queueSearchResultForDownload(result)}
                            >
                              Adicionar na fila
                            </button>
                          </div>
                        </li>
                      ))}
                    </ul>
                  )}
                </section>
              )}
            </header>

            <section className="panel">
              <div className="section-head">
                <h2>Continue lendo</h2>
                <span className="pill-badge">Leitura ativa</span>
              </div>

              {booksLoading && <p className="state-message">Carregando progresso de leitura...</p>}
              {booksError && <p className="state-message error">{booksError}</p>}

              {!booksLoading && !booksError && continueReading && (
                <article className="continue-reading-card">
                  <div className="book-cover-placeholder large">{continueReading.format.toUpperCase()}</div>
                  <div>
                    <h3>{continueReading.title}</h3>
                    <p>{continueReading.author ?? "Autor desconhecido"}</p>
                    <p className="book-card-status">Ultimo acesso: Ontem</p>
                    <div className="progress-track" aria-label="Progresso de leitura">
                      <span style={{ width: "65%" }} />
                    </div>
                    <p className="progress-caption">65% concluido</p>
                    <button
                      type="button"
                      className="primary-button"
                      onClick={() => setSelectedBookId(String(continueReading.id))}
                    >
                      Retomar leitura
                    </button>
                  </div>
                </article>
              )}

              {!booksLoading && !booksError && !continueReading && (
                <section className="empty-state slim">
                  <h3>Nenhum livro em andamento</h3>
                  <p>Adicione um livro na biblioteca para comecar sua jornada.</p>
                  <button type="button" className="primary-button" onClick={() => setActiveTab("library")}>
                    Ir para Biblioteca
                  </button>
                </section>
              )}
            </section>
          </section>
        )}

        {activeTab === "discover" && <DiscoverView onQueueDownload={queueDiscoverResultForDownload} />}

        {activeTab === "library" && <LibraryView onOpenReader={setSelectedBookId} />}

        {activeTab === "downloads" && (
          <section className="downloads-screen">
            <header className="downloads-hero panel">
              <div>
                <p className="hero-label">Transferências locais</p>
                <h1>Downloads</h1>
                <p className="downloads-hero-copy">
                  Gerencie a fila HTTP e torrent com progresso em tempo real e controle total por
                  item.
                </p>
              </div>

              <div className="downloads-hero-actions">
                <button
                  type="button"
                  className="primary-button"
                  onClick={() => void openAddDownloadPrompt()}
                >
                  + Adicionar URL ou magnet
                </button>
              </div>
            </header>

            <section className="downloads-flow panel">
              <header className="downloads-flow-head">
                <h2>Fila de downloads</h2>
                <p>Limites de velocidade e agendamento estarão disponíveis em breve.</p>
              </header>

              {downloadsLoading && <p className="state-message">Carregando downloads...</p>}
              {downloadsError && <p className="state-message error">{downloadsError}</p>}

              {!downloadsLoading && downloads.length === 0 && (
                <section className="empty-state">
                  <h2>Sua fila está vazia</h2>
                  <p>Cole um link magnet ou uma URL direta para iniciar o primeiro download.</p>
                  <button
                    type="button"
                    className="primary-button empty-state-cta"
                    onClick={() => void openAddDownloadPrompt()}
                  >
                    Adicionar primeiro download
                  </button>
                </section>
              )}

              {!downloadsLoading && downloads.length > 0 && (
                <div className="download-list">
                  {downloads.map((item) => {
                    const progressText =
                      item.sourceType === "torrent"
                        ? `Progresso: ${item.progressPercent.toFixed(1)}%`
                        : `${formatBytes(item.downloadedBytes)} de ${item.totalBytes === null ? "?" : formatBytes(item.totalBytes)} (${item.progressPercent.toFixed(1)}%)`;

                    const speedText =
                      item.status === "downloading"
                        ? `Velocidade atual: ${item.speedBps === null ? "calculando..." : `${formatBytes(item.speedBps)}/s`}`
                        : null;

                    return (
                      <article key={item.id} className="download-item">
                        <div className="download-item-head">
                          <div>
                            <strong>{item.fileName}</strong>
                            <p className="download-meta">
                              Origem: {DOWNLOAD_SOURCE_LABELS[item.sourceType]}
                            </p>
                          </div>
                          <span className={`download-status ${DOWNLOAD_STATUS_TONES[item.status]}`}>
                            {DOWNLOAD_STATUS_LABELS[item.status]}
                          </span>
                        </div>

                        <div className="progress-track" aria-label={`Progresso do download de ${item.fileName}`}>
                          <span style={{ width: `${item.progressPercent}%` }} />
                        </div>

                        <p className="download-progress-caption">{progressText}</p>
                        {speedText && <p className="download-speed">{speedText}</p>}

                        {item.filePath && <p className="downloads-path">Arquivo salvo em: {item.filePath}</p>}
                        {item.errorMessage && <p className="state-message error">{item.errorMessage}</p>}

                        <div className="download-actions">
                          {(item.status === "queued" || item.status === "paused" || item.status === "failed") && (
                            <button
                              type="button"
                              className="secondary-button"
                              onClick={() => void resumeDownload(item.id)}
                            >
                              Retomar
                            </button>
                          )}

                          {item.status === "downloading" && (
                            <button
                              type="button"
                              className="secondary-button"
                              onClick={() => void pauseDownload(item.id)}
                            >
                              Pausar
                            </button>
                          )}

                          {item.status !== "completed" && item.status !== "cancelled" && (
                            <button
                              type="button"
                              className="secondary-button danger"
                              onClick={() => void cancelDownload(item.id)}
                            >
                              Cancelar
                            </button>
                          )}

                          {item.status === "completed" &&
                            item.filePath &&
                            (item.filePath.toLowerCase().endsWith(".epub") ||
                              item.filePath.toLowerCase().endsWith(".pdf")) && (
                            <button
                              type="button"
                              className="secondary-button"
                              onClick={() => void addDownloadedBookToLibrary(item)}
                            >
                              Adicionar a biblioteca
                            </button>
                            )}

                          {(item.status === "completed" || item.status === "failed" || item.status === "cancelled") && (
                            <button
                              type="button"
                              className="secondary-button"
                              onClick={() => void removeDownload(item.id, false)}
                            >
                              Remover da lista
                            </button>
                          )}

                          {item.filePath && (item.status === "completed" || item.status === "failed" || item.status === "cancelled") && (
                            <button
                              type="button"
                              className="secondary-button danger"
                              onClick={() => void removeDownload(item.id, true)}
                            >
                              Excluir arquivo
                            </button>
                          )}
                        </div>
                      </article>
                    );
                  })}
                </div>
              )}
            </section>
          </section>
        )}

        {activeTab === "addons" && <AddonsView />}
      </main>
    </div>
  );
}

export default App;
