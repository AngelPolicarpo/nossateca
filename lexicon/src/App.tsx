import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { AddonsView } from "./components/AddonsView";
import { DiscoverView } from "./components/DiscoverView";
import { LibraryView } from "./components/LibraryView";
import { ReaderView } from "./components/ReaderView";
import { SettingsView } from "./components/SettingsView";
import { Button } from "./components/ui/Button";
import { EmptyState } from "./components/ui/EmptyState";
import { Input } from "./components/ui/Input";
import { Panel } from "./components/ui/Panel";
import { StateMessage } from "./components/ui/StateMessage";
import {
  DEFAULT_READER_SHORTCUTS,
  READER_SHORTCUT_STORAGE_KEY,
  sanitizeReaderShortcutConfig,
  type ReaderShortcutConfig,
} from "./lib/readerShortcuts";
import "./App.css";

type AppTab = "discover" | "library" | "downloads" | "addons" | "settings";
type ThemeMode = "light" | "dark";

type DownloadStatus = "queued" | "downloading" | "paused" | "completed" | "failed" | "cancelled";

type DownloadItem = {
  id: string;
  sourceUrl: string;
  sourceType: "http" | "torrent" | "opds" | "manga-cbz";
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

type SidebarIconName =
  | "compass"
  | "library"
  | "download"
  | "puzzle"
  | "settings"
  | "sun"
  | "moon";

type SidebarNavItem = {
  key: AppTab;
  label: string;
  icon: SidebarIconName;
  badge?: number | null;
};

const SIDEBAR_BASE_ITEMS: SidebarNavItem[] = [
  { key: "discover", label: "Descobrir", icon: "compass" },
  { key: "library", label: "Biblioteca", icon: "library" },
  { key: "downloads", label: "Downloads", icon: "download" },
  { key: "addons", label: "Addons", icon: "puzzle" },
  { key: "settings", label: "Configurações", icon: "settings" },
];

const DOWNLOAD_PROGRESS_TONES: Record<DownloadStatus, string> = {
  queued: "bg-[var(--color-text-muted)]",
  downloading: "bg-[var(--color-primary)]",
  paused: "bg-[var(--color-semantic-brown)]",
  completed: "bg-[var(--color-semantic-green)]",
  failed: "bg-[var(--color-semantic-orange)]",
  cancelled: "bg-[var(--color-semantic-orange)]",
};

function SidebarIcon({
  name,
  size = 18,
  stroke = 1.6,
}: {
  name: SidebarIconName;
  size?: number;
  stroke?: number;
}) {
  const iconProps = {
    width: size,
    height: size,
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: stroke,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    className: "lx-nav-icon",
    "aria-hidden": true,
    focusable: false,
  };

  switch (name) {
    case "compass":
      return (
        <svg {...iconProps}>
          <circle cx="12" cy="12" r="10" />
          <path d="m16.24 7.76-1.804 5.411a2 2 0 0 1-1.265 1.265L7.76 16.24l1.804-5.411a2 2 0 0 1 1.265-1.265z" />
        </svg>
      );
    case "library":
      return (
        <svg {...iconProps}>
          <path d="m16 6 4 14" />
          <path d="M12 6v14" />
          <path d="M8 8v12" />
          <path d="M4 4v16" />
        </svg>
      );
    case "download":
      return (
        <svg {...iconProps}>
          <path d="M12 15V3" />
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
          <path d="m7 10 5 5 5-5" />
        </svg>
      );
    case "puzzle":
      return (
        <svg {...iconProps}>
          <path d="M15.39 4.39a1 1 0 0 0 1.68-.474 2.5 2.5 0 1 1 3.014 3.015 1 1 0 0 0-.474 1.68l1.683 1.682a2.414 2.414 0 0 1 0 3.414L19.61 15.39a1 1 0 0 1-1.68-.474 2.5 2.5 0 1 0-3.014 3.015 1 1 0 0 1 .474 1.68l-1.683 1.682a2.414 2.414 0 0 1-3.414 0L8.61 19.61a1 1 0 0 0-1.68.474 2.5 2.5 0 1 1-3.014-3.015 1 1 0 0 0 .474-1.68l-1.683-1.682a2.414 2.414 0 0 1 0-3.414L4.39 8.61a1 1 0 0 1 1.68.474 2.5 2.5 0 1 0 3.014-3.015 1 1 0 0 1-.474-1.68l1.683-1.682a2.414 2.414 0 0 1 3.414 0z" />
        </svg>
      );
    case "settings":
      return (
        <svg {...iconProps}>
          <path d="M9.671 4.136a2.34 2.34 0 0 1 4.659 0 2.34 2.34 0 0 0 3.319 1.915 2.34 2.34 0 0 1 2.33 4.033 2.34 2.34 0 0 0 0 3.831 2.34 2.34 0 0 1-2.33 4.033 2.34 2.34 0 0 0-3.319 1.915 2.34 2.34 0 0 1-4.659 0 2.34 2.34 0 0 0-3.32-1.915 2.34 2.34 0 0 1-2.33-4.033 2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915" />
          <circle cx="12" cy="12" r="3" />
        </svg>
      );
    case "sun":
      return (
        <svg {...iconProps}>
          <circle cx="12" cy="12" r="4" />
          <path d="M12 2v2M12 20v2M2 12h2M20 12h2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" />
        </svg>
      );
    case "moon":
      return (
        <svg {...iconProps}>
          <path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z" />
        </svg>
      );
    default:
      return null;
  }
}

function DownloadSearchIcon() {
  return (
    <svg
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      width="16"
      height="16"
      className="pointer-events-none absolute left-[12px] top-1/2 h-[16px] w-[16px] -translate-y-1/2 text-[var(--color-text-muted)]"
      aria-hidden="true"
      focusable="false"
    >
      <circle cx="8.5" cy="8.5" r="5.5" />
      <path d="m14 14 3.5 3.5" />
    </svg>
  );
}

function DownloadClearFiltersIcon() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      width="18"
      height="18"
      className="block h-[18px] w-[18px] shrink-0"
      aria-hidden="true"
      focusable="false"
    >
      <line x1="18" y1="6" x2="6" y2="18" />
      <line x1="6" y1="6" x2="18" y2="18" />
    </svg>
  );
}

function TorrentSourceIcon() {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      <path d="M5 5v8a7 7 0 0 0 14 0V5M5 5h4v8a3 3 0 1 0 6 0V5h4" />
    </svg>
  );
}

function HttpSourceIcon() {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      <path d="M10 14a5 5 0 0 0 7 0l3-3a5 5 0 0 0-7-7l-1 1" />
      <path d="M14 10a5 5 0 0 0-7 0l-3 3a5 5 0 0 0 7 7l1-1" />
    </svg>
  );
}

function DownloadPauseIcon() {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      <rect x="6" y="4" width="4" height="16" fill="currentColor" stroke="none" />
      <rect x="14" y="4" width="4" height="16" fill="currentColor" stroke="none" />
    </svg>
  );
}

function DownloadCancelIcon() {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      <circle cx="12" cy="12" r="9" />
      <path d="M5.6 5.6l12.8 12.8" />
    </svg>
  );
}

function DownloadResumeIcon() {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      <path d="M7 4l13 8-13 8z" fill="currentColor" stroke="none" />
    </svg>
  );
}

function DownloadAddToLibraryIcon() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="16"
      height="16"
      fill="currentColor"
      viewBox="0 0 16 16"
      aria-hidden="true"
      focusable="false"
    >
      <path
        fillRule="evenodd"
        d="M8 4a.5.5 0 0 1 .5.5V6h1.5a.5.5 0 0 1 0 1H8.5v1.5a.5.5 0 0 1-1 0V7H6a.5.5 0 0 1 0-1h1.5V4.5A.5.5 0 0 1 8 4z"
      />
      <path d="M2 2a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v13.5a.5.5 0 0 1-.777.416L8 13.101l-5.223 2.815A.5.5 0 0 1 2 15.5zm2-1a1 1 0 0 0-1 1v12.566l4.723-2.482a.5.5 0 0 1 .554 0L13 14.566V2a1 1 0 0 0-1-1z" />
    </svg>
  );
}

function DownloadRemoveIcon() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      width="16"
      height="16"
      aria-hidden="true"
      focusable="false"
    >
      <path d="M3 6h18" />
      <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" />
      <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
      <line x1="10" y1="11" x2="10" y2="17" />
      <line x1="14" y1="11" x2="14" y2="17" />
    </svg>
  );
}

const THEME_STORAGE_KEY = "app.theme";

function resolveInitialTheme(): ThemeMode {
  if (typeof window === "undefined") {
    return "light";
  }

  const storedTheme = window.localStorage.getItem(THEME_STORAGE_KEY);
  if (storedTheme === "light" || storedTheme === "dark") {
    return storedTheme;
  }

  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function resolveInitialReaderShortcuts(): ReaderShortcutConfig {
  if (typeof window === "undefined") {
    return { ...DEFAULT_READER_SHORTCUTS };
  }

  const storedShortcuts = window.localStorage.getItem(READER_SHORTCUT_STORAGE_KEY);
  if (!storedShortcuts) {
    return { ...DEFAULT_READER_SHORTCUTS };
  }

  try {
    const parsedShortcuts = JSON.parse(storedShortcuts) as unknown;
    return sanitizeReaderShortcutConfig(parsedShortcuts);
  } catch {
    return { ...DEFAULT_READER_SHORTCUTS };
  }
}

function App() {
  const [activeTab, setActiveTab] = useState<AppTab>("discover");
  const [mobileNavOpen, setMobileNavOpen] = useState(false);
  const [globalSearchInput, setGlobalSearchInput] = useState("");
  const [themeMode, setThemeMode] = useState<ThemeMode>(resolveInitialTheme);
  const [readerShortcuts, setReaderShortcuts] = useState<ReaderShortcutConfig>(
    resolveInitialReaderShortcuts,
  );
  const [selectedBookId, setSelectedBookId] = useState<string | null>(null);
  const [downloads, setDownloads] = useState<DownloadItem[]>([]);
  const [downloadsLoading, setDownloadsLoading] = useState(false);
  const [downloadsError, setDownloadsError] = useState<string | null>(null);
  const [downloadSearchInput, setDownloadSearchInput] = useState("");
  const [libraryCount, setLibraryCount] = useState(0);
  const previousActiveTabRef = useRef<AppTab>(activeTab);

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

  useEffect(() => {
    document.documentElement.dataset.theme = themeMode;
    window.localStorage.setItem(THEME_STORAGE_KEY, themeMode);
  }, [themeMode]);

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }

    window.localStorage.setItem(READER_SHORTCUT_STORAGE_KEY, JSON.stringify(readerShortcuts));
  }, [readerShortcuts]);

  useEffect(() => {
    if (previousActiveTabRef.current === "discover" && activeTab !== "discover") {
      setGlobalSearchInput("");
    }

    previousActiveTabRef.current = activeTab;
  }, [activeTab]);

  useEffect(() => {
    setMobileNavOpen(false);
  }, [activeTab]);

  useEffect(() => {
    if (!mobileNavOpen) {
      return;
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setMobileNavOpen(false);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [mobileNavOpen]);

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

  const loadLibraryCount = useCallback(async () => {
    try {
      const books = await invoke<unknown[]>("list_books");
      setLibraryCount(Array.isArray(books) ? books.length : 0);
    } catch {
      setLibraryCount(0);
    }
  }, []);

  useEffect(() => {
    void loadDownloads();
  }, [loadDownloads]);

  useEffect(() => {
    void loadLibraryCount();
  }, [loadLibraryCount]);

  useEffect(() => {
    if (activeTab === "library") {
      void loadLibraryCount();
    }
  }, [activeTab, loadLibraryCount]);

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

  const startDownload = useCallback(
    async (sourceUrl: string, fileName?: string, subfolder?: string) => {
      await invoke<DownloadItem>("start_download", {
        sourceUrl,
        fileName: fileName ?? null,
        subfolder: subfolder ?? null,
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

  const addDownloadedBookToLibrary = useCallback(async (item: DownloadItem) => {
    if (!item.filePath) {
      alert("Arquivo nao encontrado para adicionar a biblioteca.");
      return;
    }

    const normalizedPath = item.filePath.toLowerCase();
    if (
      !normalizedPath.endsWith(".epub") &&
      !normalizedPath.endsWith(".pdf") &&
      !normalizedPath.endsWith(".cbz")
    ) {
      alert("Somente arquivos EPUB, PDF e CBZ podem ser adicionados automaticamente a biblioteca.");
      return;
    }

    try {
      await invoke("add_book", { filePath: item.filePath });
      alert("Livro adicionado a biblioteca.");
      void loadLibraryCount();
    } catch (err) {
      alert(getErrorMessage(err, "Falha ao adicionar livro a biblioteca"));
    }
  }, [loadLibraryCount]);

  const queueDiscoverResultForDownload = useCallback(
    async (sourceUrl: string, fileName: string, subfolder?: string) => {
      await startDownload(sourceUrl, fileName, subfolder);
      setActiveTab("downloads");
    },
    [startDownload],
  );

  const openAddDownloadPrompt = async () => {
    const sourceUrl = window.prompt("Cole a URL HTTP/HTTPS ou Magnet link");
    if (!sourceUrl || sourceUrl.trim().length === 0) {
      return;
    }

    const suggestedName =
      window.prompt("Nome opcional do arquivo (pressione OK para automatico)") ?? "";

    try {
      await startDownload(
        sourceUrl.trim(),
        suggestedName.trim().length > 0 ? suggestedName.trim() : undefined,
      );
      setActiveTab("downloads");
    } catch (err) {
      const message = getErrorMessage(err, "Falha ao iniciar download");
      alert(message);
    }
  };

  const filteredDownloads = useMemo(() => {
    const normalizedSearch = downloadSearchInput.trim().toLowerCase();

    return downloads.filter((item) => {
      if (
        normalizedSearch.length > 0 &&
        !item.fileName.toLowerCase().includes(normalizedSearch) &&
        !item.sourceUrl.toLowerCase().includes(normalizedSearch)
      ) {
        return false;
      }

      return true;
    });
  }, [downloadSearchInput, downloads]);

  const activeDownloadsCount = useMemo(
    () =>
      downloads.filter(
        (item) =>
          item.status === "queued" || item.status === "downloading" || item.status === "paused",
      ).length,
    [downloads],
  );

  const downloadingSidebarCount = useMemo(
    () => downloads.filter((item) => item.status === "queued" || item.status === "downloading").length,
    [downloads],
  );

  const sidebarItems = useMemo<SidebarNavItem[]>(
    () =>
      SIDEBAR_BASE_ITEMS.map((item): SidebarNavItem => {
        if (item.key === "library") {
          return { ...item, badge: libraryCount };
        }

        if (item.key === "downloads") {
          return {
            ...item,
            badge: downloadingSidebarCount > 0 ? downloadingSidebarCount : null,
          };
        }

        return item;
      }),
    [downloadingSidebarCount, libraryCount],
  );

  const completedDownloadsCount = useMemo(
    () => downloads.filter((item) => item.status === "completed").length,
    [downloads],
  );

  const hasActiveDownloadFilters = downloadSearchInput.trim().length > 0;

  const clearDownloadFilters = () => {
    setDownloadSearchInput("");
  };

  const handleRemoveDownloadAction = useCallback(
    async (item: DownloadItem) => {
      const decision = window.prompt(
        item.filePath
          ? "Digite 1 para remover da lista, 2 para remover e excluir arquivo, ou deixe vazio para cancelar."
          : "Digite 1 para remover da lista, ou deixe vazio para cancelar.",
        "1",
      );

      if (decision === null) {
        return;
      }

      const normalizedDecision = decision.trim();
      if (normalizedDecision !== "1" && normalizedDecision !== "2") {
        return;
      }

      if (normalizedDecision === "2" && !item.filePath) {
        alert("Nao ha arquivo local para excluir neste download.");
        return;
      }

      const shouldDeleteFile = normalizedDecision === "2" && Boolean(item.filePath);
      await removeDownload(item.id, shouldDeleteFile);
    },
    [removeDownload],
  );

  if (selectedBookId) {
    return (
      <ReaderView
        bookId={selectedBookId}
        shortcuts={readerShortcuts}
        themeMode={themeMode}
        onThemeModeChange={setThemeMode}
        onClose={() => {
          setSelectedBookId(null);
        }}
      />
    );
  }

  return (
    <div className={`lx-app${mobileNavOpen ? " nav-open" : ""}`}>
      {mobileNavOpen && (
        <button
          type="button"
          className="lx-nav-backdrop"
          aria-label="Fechar navegação"
          onClick={() => setMobileNavOpen(false)}
        />
      )}

      {/* Sidebar */}
      <aside className={`lx-sidebar${mobileNavOpen ? " open" : ""}`}>
        <div className="lx-brand">
          <div className="lx-brand-mark">N</div>
          <span className="lx-brand-name">Nossateca</span>
        </div>

        <nav className="lx-nav" aria-label="Navegação principal">
          <div className="lx-nav-list">
            {sidebarItems.map((item) => (
              <button
                key={item.key}
                type="button"
                className={`lx-nav-item${activeTab === item.key ? " active" : ""}`}
                onClick={() => setActiveTab(item.key)}
              >
                <SidebarIcon name={item.icon} />
                <span>{item.label}</span>
                {item.badge !== null && item.badge !== undefined && (
                  <span className="lx-nav-badge">{item.badge}</span>
                )}
              </button>
            ))}
          </div>
        </nav>

        <div className="lx-sidebar-foot">
          <div className="lx-theme-toggle" role="group" aria-label="Alternar tema">
            <button
              type="button"
              className={themeMode === "light" ? "on" : ""}
              onClick={() => setThemeMode("light")}
              aria-label="Tema claro"
            >
              <SidebarIcon name="sun" size={13} />
            </button>
            <button
              type="button"
              className={themeMode === "dark" ? "on" : ""}
              onClick={() => setThemeMode("dark")}
              aria-label="Tema escuro"
            >
              <SidebarIcon name="moon" size={13} />
            </button>
          </div>
        </div>
      </aside>

      {/* Main */}
      <div className="lx-main">
        <header className="lx-topbar">
          <button
            type="button"
            className="lx-nav-toggle"
            aria-label="Abrir navegação"
            aria-expanded={mobileNavOpen}
            onClick={() => setMobileNavOpen((open) => !open)}
          >
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden="true"
              focusable="false"
            >
              <line x1="3" y1="6" x2="21" y2="6" />
              <line x1="3" y1="12" x2="21" y2="12" />
              <line x1="3" y1="18" x2="21" y2="18" />
            </svg>
          </button>
          <div className="lx-topbar-search">
            <svg
              viewBox="0 0 20 20"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              className="lx-topbar-search-icon"
              aria-hidden="true"
            >
              <circle cx="8.5" cy="8.5" r="5.5" />
              <path d="m14 14 3.5 3.5" />
            </svg>
            <Input
              id="topbar-discover-search"
              type="search"
              placeholder="Buscar por título, autor ou ISBN"
              value={globalSearchInput}
              onChange={(event) => setGlobalSearchInput(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter" && activeTab !== "discover") {
                  event.preventDefault();
                  setActiveTab("discover");
                }
              }}
              className="min-h-[36px] rounded-[var(--radius-pill)] border-black/15 bg-[var(--color-surface-alt)] pl-[34px]"
              aria-label="Buscar por título, autor ou ISBN no Descubra"
            />
          </div>
        </header>

        <main className="lx-content">
          {activeTab === "discover" && (
            <DiscoverView
              onQueueDownload={queueDiscoverResultForDownload}
              globalSearchInput={globalSearchInput}
              onGlobalSearchInputChange={setGlobalSearchInput}
            />
          )}

          {activeTab === "library" && <LibraryView onOpenReader={setSelectedBookId} />}

          {activeTab === "downloads" && (
            <section className="lx-downloads-shell">
              <div className="lx-downloads-content">
                <header className="grid gap-[var(--space-16)]">
                  <div className="lx-page-header">
                    <div className="lx-page-header-titles">
                      <h1 className="lx-page-title">Downloads</h1>
                      <p className="lx-page-subtitle">
                        {activeDownloadsCount} ativos de {downloads.length} itens · {completedDownloadsCount} concluídos
                      </p>
                    </div>

                    <div className="lx-page-header-actions">
                      <Button
                        variant="secondary"
                        onClick={() => void loadDownloads()}
                        disabled={downloadsLoading}
                      >
                        {downloadsLoading ? "Recarregando..." : "Recarregar"}
                      </Button>
                      <Button variant="primary" onClick={() => void openAddDownloadPrompt()}>
                        + Adicionar
                      </Button>
                    </div>
                  </div>

                  <Panel className="grid gap-[var(--space-12)] !p-0">
                    <label className="m-0 grid gap-[var(--space-8)] text-[14px] leading-[1.43] text-[var(--color-text-secondary)]">
                      <div className="flex items-center gap-[var(--space-8)]">
                        <div className="relative min-w-0 flex-1">
                          <DownloadSearchIcon />
                          <Input
                            type="search"
                            value={downloadSearchInput}
                            onChange={(event) => setDownloadSearchInput(event.target.value)}
                            placeholder="Buscar por nome do arquivo ou URL"
                            className="h-[40px] min-h-[40px] rounded-[var(--radius-pill)] border-black/15 pl-[34px]"
                          />
                        </div>
                        <Button
                          variant="secondary"
                          size="sm"
                          className="h-[40px] min-h-[40px] w-[40px] min-w-[40px] max-h-[40px] max-w-[40px] flex-none rounded-[var(--radius-pill)] p-0 [&_svg]:h-[18px] [&_svg]:w-[18px]"
                          aria-label="Limpar filtros"
                          title="Limpar filtros"
                          onClick={clearDownloadFilters}
                          disabled={!hasActiveDownloadFilters}
                        >
                          <DownloadClearFiltersIcon />
                        </Button>
                      </div>
                    </label>

                  </Panel>
                </header>

                {downloadsLoading && <StateMessage>Carregando downloads...</StateMessage>}
                {downloadsError && <StateMessage tone="error">{downloadsError}</StateMessage>}

                {!downloadsLoading && downloads.length === 0 && (
                  <EmptyState
                    title="Sua fila está vazia"
                    description="Cole um link magnet ou uma URL direta para iniciar o primeiro download."
                    action={
                      <Button
                        variant="primary"
                        onClick={() => void openAddDownloadPrompt()}
                        className="min-h-14"
                      >
                        Adicionar primeiro download
                      </Button>
                    }
                  />
                )}

                {!downloadsLoading && downloads.length > 0 && filteredDownloads.length === 0 && (
                  <EmptyState
                    compact
                    title="Nenhum download com os filtros atuais"
                    description="Ajuste ou limpe a busca para voltar a ver toda a fila."
                    action={
                      <Button variant="secondary" onClick={clearDownloadFilters}>
                        Limpar filtros
                      </Button>
                    }
                  />
                )}

                {!downloadsLoading && filteredDownloads.length > 0 && (
                  <div className="lx-downloads-list">
                    {filteredDownloads.map((item) => {
                      const progressText =
                        item.sourceType === "torrent"
                          ? `Progresso: ${item.progressPercent.toFixed(1)}%`
                          : `${formatBytes(item.downloadedBytes)} de ${item.totalBytes === null ? "?" : formatBytes(item.totalBytes)} (${item.progressPercent.toFixed(1)}%)`;

                      const speedText =
                        item.status === "downloading"
                          ? item.speedBps === null
                            ? "Velocidade: calculando..."
                            : `Velocidade: ${formatBytes(item.speedBps)}/s`
                          : null;

                      const supportsQuickAddToLibrary =
                        item.status === "completed" &&
                        item.filePath &&
                        (item.filePath.toLowerCase().endsWith(".epub") ||
                          item.filePath.toLowerCase().endsWith(".pdf") ||
                          item.filePath.toLowerCase().endsWith(".cbz"));

                      const canRemoveEntry =
                        item.status === "completed" ||
                        item.status === "failed" ||
                        item.status === "cancelled";

                      return (
                        <article key={item.id} className="lx-download-row">
                          <div
                            className={`dl-icon${item.sourceType === "torrent" ? " torrent" : ""}`}
                            aria-hidden="true"
                          >
                            {item.sourceType === "torrent" ? <TorrentSourceIcon /> : <HttpSourceIcon />}
                          </div>

                          <div className="lx-download-main">
                            <strong className="lx-download-name">{item.fileName}</strong>
                            <p className="lx-download-url" title={item.sourceUrl}>
                              {item.sourceUrl}
                            </p>

                            <div className="lx-download-progress">
                              <div
                                className="lx-download-progress-track"
                                aria-label={`Progresso do download de ${item.fileName}`}
                              >
                                <span
                                  className={`lx-download-progress-fill ${DOWNLOAD_PROGRESS_TONES[item.status]}`}
                                  style={{ width: `${item.progressPercent}%` }}
                                />
                              </div>
                              <div className="lx-download-progress-meta">
                                <span>{progressText}</span>
                                {speedText && <span>{speedText}</span>}
                              </div>
                            </div>

                            {item.filePath && (
                              <p className="lx-download-file-path">Arquivo salvo em: {item.filePath}</p>
                            )}
                            {item.errorMessage && <StateMessage tone="error">{item.errorMessage}</StateMessage>}
                          </div>

                          <div className="lx-download-actions">
                            {(item.status === "queued" ||
                              item.status === "paused" ||
                              item.status === "failed") && (
                              <Button
                                variant="secondary"
                                size="sm"
                                className="lx-download-icon-btn"
                                onClick={() => void resumeDownload(item.id)}
                                aria-label={item.status === "paused" ? "Retomar download" : "Iniciar download"}
                                title={item.status === "paused" ? "Retomar download" : "Iniciar download"}
                              >
                                <DownloadResumeIcon />
                              </Button>
                            )}

                            {item.status === "downloading" && (
                              <Button
                                variant="secondary"
                                size="sm"
                                className="lx-download-icon-btn"
                                onClick={() => void pauseDownload(item.id)}
                                aria-label="Pausar download"
                                title="Pausar download"
                              >
                                <DownloadPauseIcon />
                              </Button>
                            )}

                            {item.status !== "completed" && item.status !== "cancelled" && (
                              <Button
                                variant="danger"
                                size="sm"
                                className="lx-download-icon-btn"
                                onClick={() => void cancelDownload(item.id)}
                                aria-label="Cancelar download"
                                title="Cancelar download"
                              >
                                <DownloadCancelIcon />
                              </Button>
                            )}

                            {supportsQuickAddToLibrary && (
                              <Button
                                variant="secondary"
                                size="sm"
                                className="lx-download-icon-btn"
                                onClick={() => void addDownloadedBookToLibrary(item)}
                                aria-label="Adicionar à biblioteca"
                                title="Adicionar à biblioteca"
                              >
                                <DownloadAddToLibraryIcon />
                              </Button>
                            )}

                            {canRemoveEntry && (
                              <Button
                                variant="danger"
                                size="sm"
                                className="lx-download-icon-btn"
                                onClick={() => void handleRemoveDownloadAction(item)}
                                aria-label="Remover download"
                                title="Remover download"
                              >
                                <DownloadRemoveIcon />
                              </Button>
                            )}
                          </div>
                        </article>
                      );
                    })}
                  </div>
                )}
              </div>
            </section>
          )}

          {activeTab === "addons" && <AddonsView />}

          {activeTab === "settings" && (
            <SettingsView
              themeMode={themeMode}
              onThemeModeChange={setThemeMode}
              readerShortcuts={readerShortcuts}
              onReaderShortcutsChange={setReaderShortcuts}
            />
          )}
        </main>
      </div>
    </div>
  );
}

export default App;
