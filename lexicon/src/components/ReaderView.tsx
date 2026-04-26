import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AnnotationSidebar } from "./AnnotationSidebar";
import {
  isShortcutEventMatch,
  parseShortcutBindings,
  type ReaderShortcutConfig,
} from "../lib/readerShortcuts";
import "./ReaderView.css";

type ReaderViewProps = {
  bookId: string;
  shortcuts: ReaderShortcutConfig;
  themeMode: ReaderTheme;
  onThemeModeChange: (nextThemeMode: ReaderTheme) => void;
  onClose: () => void;
};

type BookContent = {
  html: string;
  current_chapter: number;
  total_chapters: number;
  chapter_title: string;
  book_format: string;
  book_file_path: string | null;
  supports_annotations: boolean;
};

type PdfDocumentData = {
  bytes_base64: string;
  total_pages: number;
};

type CbzPageData = {
  bytes_base64: string;
  mime_type: string;
  page_index: number;
  total_pages: number;
};

type CbzViewMode = "single" | "strip";

type ReadingProgressData = {
  current_position: string;
  progress_percent: number;
};

type EpubSearchMatchData = {
  chapter_index: number;
  chapter_title: string;
  snippet: string;
  occurrences: number;
};

type EpubSearchResponse = {
  query: string;
  total_matches: number;
  results: EpubSearchMatchData[];
};

type EpubLinkTarget = {
  chapter_index: number;
  anchor_id: string | null;
};

type Annotation = {
  id: string;
  book_id: number;
  annotation_type: string;
  position: string;
  position_end: string | null;
  selected_text: string | null;
  note_text: string | null;
  color: "yellow" | "green" | "blue" | "pink" | "purple";
  created_at: string;
  updated_at: string;
};

type FloatingHighlightAction = {
  text: string;
  position: string;
  top: number;
  left: number;
};

type ReaderTheme = "light" | "dark";

type SidePanel = "toc" | "annot" | "search" | null;

type LoadChapterOptions = {
  persistProgress?: boolean;
  restoreScrollPosition?: number | null;
};

const HIGHLIGHT_COLORS = ["yellow", "green", "blue", "pink", "purple"] as const;
const READER_STORAGE_KEYS = {
  fontSize: "reader.fontSize",
  lineHeight: "reader.lineHeight",
  contentWidth: "reader.contentWidth",
  pdfZoom: "reader.pdfZoom",
  cbzZoom: "reader.cbzZoom",
  cbzMode: "reader.cbzMode",
} as const;

const READER_FONT_SIZE_DEFAULT = 18;
const READER_FONT_SIZE_MIN = 14;
const READER_FONT_SIZE_MAX = 28;

const READER_LINE_HEIGHT_DEFAULT = 1.7;
const READER_LINE_HEIGHT_MIN = 1.3;
const READER_LINE_HEIGHT_MAX = 2.0;

const READER_CONTENT_WIDTH_DEFAULT = 880;
const READER_CONTENT_WIDTH_MIN = 520;
const READER_CONTENT_WIDTH_MAX = 1800;

type PdfJsWorkerGlobal = typeof globalThis & {
  pdfjsWorker?: {
    WorkerMessageHandler?: unknown;
  };
};

function clampNumber(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

function getAdaptiveContentWidth(contentWidth: number, fontSize: number): number {
  const dynamicUpperBound = clampNumber(
    READER_CONTENT_WIDTH_MAX - (fontSize - READER_FONT_SIZE_DEFAULT) * 16,
    640,
    READER_CONTENT_WIDTH_MAX,
  );
  return Math.min(contentWidth, dynamicUpperBound);
}

function readStoredValue(key: string): string | null {
  if (typeof window === "undefined") return null;
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

function persistStoredValue(key: string, value: string): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // ignore
  }
}

function readStoredNumber(key: string, fallback: number, min: number, max: number): number {
  const rawValue = readStoredValue(key);
  if (!rawValue) return fallback;
  const parsed = Number.parseFloat(rawValue);
  if (Number.isNaN(parsed)) return fallback;
  return clampNumber(parsed, min, max);
}

function readStoredCbzMode(): CbzViewMode {
  const raw = readStoredValue(READER_STORAGE_KEYS.cbzMode);
  return raw === "strip" ? "strip" : "single";
}

function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  const tagName = target.tagName.toLowerCase();
  return tagName === "input" || tagName === "textarea" || tagName === "select";
}

function getErrorMessage(err: unknown, fallback: string): string {
  if (err instanceof Error && err.message) return err.message;
  if (typeof err === "string" && err.trim().length > 0) return err;
  if (
    typeof err === "object" &&
    err !== null &&
    "message" in err &&
    typeof (err as { message?: unknown }).message === "string"
  ) {
    return (err as { message: string }).message;
  }
  return fallback;
}

function decodeBase64ToBytes(base64Value: string): Uint8Array {
  const binary = window.atob(base64Value);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

const EPUB_ALLOWED_LINK_PREFIXES = ["data:", "blob:"] as const;

const EPUB_SHADOW_BASELINE_CSS = `
:host {
  display: block;
  color: var(--reader-text);
  font-size: var(--reader-font-size);
  line-height: var(--reader-line-height);
  font-family: var(--font-primary);
  text-rendering: optimizeLegibility;
  font-feature-settings: "kern", "liga";
  -webkit-font-smoothing: antialiased;
}

:host *,
:host *::before,
:host *::after {
  box-sizing: border-box;
}

:where(:host) img,
:where(:host) svg,
:where(:host) video,
:where(:host) iframe,
:where(:host) canvas {
  max-width: 100%;
  height: auto;
}

:where(:host) figure { margin: 1em 0; }

:where(:host) img,
:where(:host) svg {
  display: block;
  margin: 0.8em auto;
  object-fit: contain;
}

:where(:host) ul,
:where(:host) ol {
  margin: 0.9em 0;
  padding-left: 1.4em;
}
:where(:host) ul { list-style: disc; }
:where(:host) ol { list-style: decimal; }
:where(:host) li + li { margin-top: 0.3em; }

:where(:host) table {
  width: 100%;
  display: block;
  overflow-x: auto;
  border-collapse: collapse;
  margin: 1em 0;
  border: 1px solid var(--reader-border-soft);
}
:where(:host) thead { background: var(--reader-surface-alt); }
:where(:host) th,
:where(:host) td {
  border: 1px solid var(--reader-border-soft);
  padding: 8px 10px;
  text-align: left;
  vertical-align: top;
}

:where(:host) a { color: var(--reader-accent); }

:where(:host) pre,
:where(:host) code {
  font-family: ui-monospace, "JetBrains Mono", monospace;
}
:where(:host) pre {
  overflow-x: auto;
  padding: 0.75em 1em;
  background: var(--reader-surface-alt);
  border-radius: 6px;
}

:host .reader-highlight {
  border-radius: 2px;
  padding: 0 2px;
  cursor: pointer;
}

:host .highlight-yellow { background-color: rgba(232, 184, 36, 0.30); }
:host .highlight-green  { background-color: rgba(26, 174, 57, 0.24); }
:host .highlight-blue   { background-color: rgba(80, 138, 220, 0.25); }
:host .highlight-pink   { background-color: rgba(216, 75, 110, 0.25); }
:host .highlight-purple { background-color: rgba(120, 80, 200, 0.24); }

:host .highlight-pulse {
  animation: highlightPulse 1.1s ease;
}

@keyframes highlightPulse {
  0%   { box-shadow: 0 0 0 0   rgba(216, 75, 42, 0.32); }
  50%  { box-shadow: 0 0 0 6px rgba(216, 75, 42, 0.08); }
  100% { box-shadow: 0 0 0 0   rgba(216, 75, 42, 0); }
}
`;

function cleanEpubInlineStyle(styleValue: string): string {
  return styleValue
    .split(";")
    .map((d) => d.trim())
    .filter((d) => d.length > 0)
    .filter((d) => {
      const prop = d.split(":", 1)[0]?.trim().toLowerCase() ?? "";
      return prop !== "font-family" && prop !== "font";
    })
    .join("; ");
}

function sanitizeEpubCss(css: string): string {
  return css
    .replace(/@import\s+url\(\s*(["']?)(?:https?:|\/\/|javascript:)[\s\S]*?\)\s*;?/gi, "")
    .replace(/@import\s+(["'])(?:https?:|\/\/|javascript:)[\s\S]*?\1\s*;?/gi, "");
}

function isAllowedEpubStylesheetHref(href: string): boolean {
  const h = href.trim().toLowerCase();
  return EPUB_ALLOWED_LINK_PREFIXES.some((p) => h.startsWith(p));
}

function isUnsafeEpubUrl(value: string): boolean {
  return value.trim().toLowerCase().startsWith("javascript:");
}

function isExternalEpubHref(href: string): boolean {
  const lower = href.trim().toLowerCase();
  return (
    lower.startsWith("http://") ||
    lower.startsWith("https://") ||
    lower.startsWith("mailto:") ||
    lower.startsWith("tel:") ||
    lower.startsWith("javascript:") ||
    lower.startsWith("data:") ||
    lower.startsWith("blob:") ||
    lower.startsWith("file:") ||
    lower.startsWith("//")
  );
}

function extractEpubHrefFragment(href: string): string | null {
  const hashIndex = href.indexOf("#");
  if (hashIndex === -1) return null;
  const fragment = href.slice(hashIndex + 1).trim();
  return fragment || null;
}

function decodeEpubFragment(fragment: string): string {
  try {
    return decodeURIComponent(fragment);
  } catch {
    return fragment;
  }
}

function findEpubAnchorTarget(root: ParentNode, fragment: string): HTMLElement | null {
  const decoded = decodeEpubFragment(fragment);
  const candidates = decoded === fragment ? [fragment] : [fragment, decoded];
  const anchors = Array.from(root.querySelectorAll<HTMLAnchorElement>("a[name]"));

  for (const candidate of candidates) {
    const escaped =
      typeof CSS !== "undefined" && typeof CSS.escape === "function"
        ? CSS.escape(candidate)
        : candidate.replace(/(["'\\#.:\[\],>+~*()=\s])/g, "\\$1");

    const direct = root.querySelector(`#${escaped}`);
    if (direct instanceof HTMLElement) return direct;

    for (const a of anchors) {
      if ((a.getAttribute("name") ?? "").trim() === candidate) return a;
    }
  }
  return null;
}

function scrollToEpubAnchor(
  root: ParentNode,
  fragment: string,
  container: HTMLElement,
): boolean {
  const target = findEpubAnchorTarget(root, fragment);
  if (!target) return false;
  const containerRect = container.getBoundingClientRect();
  const targetRect = target.getBoundingClientRect();
  const targetTop = Math.max(container.scrollTop + targetRect.top - containerRect.top - 64, 0);
  container.scrollTo({ top: targetTop, behavior: "smooth" });
  return true;
}

function createReaderSearchRanges(root: ParentNode, query: string): Range[] {
  const normalized = query.trim().toLowerCase();
  if (!normalized) return [];

  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
    acceptNode: (node) => {
      const parent = node.parentElement;
      if (!parent) return NodeFilter.FILTER_REJECT;
      const tag = parent.tagName.toLowerCase();
      if (tag === "style" || tag === "script") return NodeFilter.FILTER_REJECT;
      if (!(node.textContent ?? "").trim()) return NodeFilter.FILTER_REJECT;
      return NodeFilter.FILTER_ACCEPT;
    },
  });

  const ranges: Range[] = [];
  while (walker.nextNode()) {
    const node = walker.currentNode as Text;
    const content = (node.textContent ?? "").toLowerCase();
    let offset = 0;
    while (offset < content.length) {
      const idx = content.indexOf(normalized, offset);
      if (idx === -1) break;
      const range = document.createRange();
      range.setStart(node, idx);
      range.setEnd(node, idx + normalized.length);
      ranges.push(range);
      offset = idx + normalized.length;
    }
  }
  return ranges;
}

function focusReaderSearchRange(range: Range): void {
  const selection = window.getSelection();
  if (!selection) return;
  selection.removeAllRanges();
  selection.addRange(range);
  range.startContainer.parentElement?.scrollIntoView({
    behavior: "smooth",
    block: "center",
    inline: "nearest",
  });
}

function buildEpubShadowMarkup(chapterHtml: string): string {
  return `<style>${EPUB_SHADOW_BASELINE_CSS}</style>${chapterHtml}`;
}

function sanitizeEpubChapterHtml(rawHtml: string): string {
  if (typeof DOMParser === "undefined") {
    return rawHtml
      .replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, "")
      .replace(/<\/?(?:html|head|body|meta|title|base)\b[^>]*>/gi, "");
  }

  const parser = new DOMParser();
  const parsed = parser.parseFromString(rawHtml, "text/html");

  parsed.querySelectorAll("script").forEach((n) => n.remove());
  parsed.querySelectorAll("meta, title, base").forEach((n) => n.remove());

  const headStyles = Array.from(parsed.head.querySelectorAll("style, link[rel]"))
    .map((node) => {
      const tag = node.tagName.toLowerCase();
      if (tag === "style") {
        const s = node as HTMLStyleElement;
        s.textContent = sanitizeEpubCss(s.textContent ?? "");
        return s.outerHTML;
      }
      const l = node as HTMLLinkElement;
      const rel = (l.getAttribute("rel") ?? "").toLowerCase();
      const href = (l.getAttribute("href") ?? "").trim();
      if (!rel.includes("stylesheet") || !href || !isAllowedEpubStylesheetHref(href)) return "";
      return l.outerHTML;
    })
    .filter((c) => c.length > 0);

  parsed.body.querySelectorAll("style").forEach((s) => {
    (s as HTMLStyleElement).textContent = sanitizeEpubCss((s as HTMLStyleElement).textContent ?? "");
  });

  parsed.body.querySelectorAll("link").forEach((l) => {
    const link = l as HTMLLinkElement;
    const rel = (link.getAttribute("rel") ?? "").toLowerCase();
    const href = (link.getAttribute("href") ?? "").trim();
    if (!rel.includes("stylesheet") || !href || !isAllowedEpubStylesheetHref(href)) link.remove();
  });

  parsed.body.querySelectorAll("*").forEach((el) => {
    for (const attr of Array.from(el.attributes)) {
      const name = attr.name.toLowerCase();
      const value = attr.value;
      if (name.startsWith("on")) { el.removeAttribute(attr.name); continue; }
      if (["href", "src", "poster", "xlink:href"].includes(name) && isUnsafeEpubUrl(value)) {
        el.removeAttribute(attr.name); continue;
      }
      if (name === "style") {
        const cleaned = cleanEpubInlineStyle(value);
        if (!cleaned) el.removeAttribute(attr.name);
        else el.setAttribute(attr.name, cleaned);
      }
    }
  });

  return `${headStyles.join("")}${parsed.body.innerHTML}`;
}

function findInDocument(query: string, backward: boolean): boolean {
  const maybeFind = (
    window as Window & {
      find?: (
        text: string,
        caseSensitive?: boolean,
        backwards?: boolean,
        wrapAround?: boolean,
        wholeWord?: boolean,
        searchInFrames?: boolean,
        showDialog?: boolean,
      ) => boolean;
    }
  ).find;
  if (typeof maybeFind !== "function") return false;
  return maybeFind(query, false, backward, true, false, false, false);
}

function parseReadingPosition(position: string): {
  chapterIndex: number | null;
  pageIndex: number | null;
  scrollPosition: number | null;
} {
  const chapterMatch = position.match(/chapter:(\d+)/i);
  const pageMatch = position.match(/page:(\d+)/i);
  const scrollMatch = position.match(/scroll:([-+]?\d*\.?\d+)/i);

  const chapterIndex = chapterMatch ? Number.parseInt(chapterMatch[1], 10) : Number.NaN;
  const pageNumber = pageMatch ? Number.parseInt(pageMatch[1], 10) : Number.NaN;
  const scrollPosition = scrollMatch ? Number.parseFloat(scrollMatch[1]) : Number.NaN;

  return {
    chapterIndex: Number.isNaN(chapterIndex) ? null : chapterIndex,
    pageIndex: Number.isNaN(pageNumber) ? null : Math.max(pageNumber - 1, 0),
    scrollPosition: Number.isNaN(scrollPosition) ? null : Math.max(scrollPosition, 0),
  };
}

function isPdfWorkerModuleScriptError(err: unknown): boolean {
  const message = getErrorMessage(err, "").toLowerCase();
  return (
    message.includes("importing a module script failed") ||
    message.includes("setting up fake worker failed") ||
    message.includes("failed to fetch dynamically imported module")
  );
}

async function registerPdfWorkerMainThreadFallback(): Promise<void> {
  try {
    const workerModule = await import("pdfjs-dist/build/pdf.worker.min.mjs");
    if ("WorkerMessageHandler" in workerModule) {
      (globalThis as PdfJsWorkerGlobal).pdfjsWorker = workerModule;
    }
  } catch {
    // ignore
  }
}

// ---------- SVG icons ----------
const IconBack = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <polyline points="15 18 9 12 15 6" />
  </svg>
);

const IconNext = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <polyline points="9 18 15 12 9 6" />
  </svg>
);

const IconSearch = () => (
  <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
    <circle cx="11" cy="11" r="8" />
    <line x1="21" y1="21" x2="16.65" y2="16.65" />
  </svg>
);

const IconToc = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
    <line x1="3" y1="6" x2="21" y2="6" />
    <line x1="3" y1="12" x2="15" y2="12" />
    <line x1="3" y1="18" x2="18" y2="18" />
  </svg>
);

const IconAnnot = () => (
  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
  </svg>
);

const IconBookmark = () => (
  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M19 21l-7-5-7 5V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z" />
  </svg>
);

const IconExpand = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
    <polyline points="15 3 21 3 21 9" />
    <polyline points="9 21 3 21 3 15" />
    <line x1="21" y1="3" x2="14" y2="10" />
    <line x1="3" y1="21" x2="10" y2="14" />
  </svg>
);

const IconCompress = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
    <polyline points="4 14 10 14 10 20" />
    <polyline points="20 10 14 10 14 4" />
    <line x1="10" y1="14" x2="3" y2="21" />
    <line x1="21" y1="3" x2="14" y2="10" />
  </svg>
);

// ---------- Component ----------
export function ReaderView({
  bookId,
  shortcuts,
  themeMode,
  onThemeModeChange,
  onClose,
}: ReaderViewProps) {
  const contentRef = useRef<HTMLDivElement | null>(null);
  const stageRef = useRef<HTMLDivElement | null>(null);
  const epubShadowRootRef = useRef<ShadowRoot | null>(null);
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const pdfCanvasRef = useRef<HTMLCanvasElement | null>(null);
  const pdfCanvasWrapRef = useRef<HTMLDivElement | null>(null);
  const pdfDocumentRef = useRef<any | null>(null);
  const pdfGetDocumentRef = useRef<((source: unknown) => { promise: Promise<any> }) | null>(null);
  const pdfLoaderPromiseRef = useRef<Promise<((source: unknown) => { promise: Promise<any> })> | null>(null);
  const skipScrollPersistUntilRef = useRef(0);
  const chapterSearchKeyRef = useRef("");
  const chapterSearchIndexRef = useRef(-1);

  const [content, setContent] = useState("");
  const [chapterTitle, setChapterTitle] = useState("");
  const [currentChapter, setCurrentChapter] = useState(0);
  const [totalChapters, setTotalChapters] = useState(0);
  const [bookFormat, setBookFormat] = useState("epub");
  const [supportsAnnotations, setSupportsAnnotations] = useState(true);
  const [annotations, setAnnotations] = useState<Annotation[]>([]);
  const [annotationsLoading, setAnnotationsLoading] = useState(false);
  const [sidePanel, setSidePanel] = useState<SidePanel>(null);
  const [preferencesOpen, setPreferencesOpen] = useState(false);
  const [fontSize, setFontSize] = useState(() =>
    readStoredNumber(READER_STORAGE_KEYS.fontSize, READER_FONT_SIZE_DEFAULT, READER_FONT_SIZE_MIN, READER_FONT_SIZE_MAX),
  );
  const [lineHeight, setLineHeight] = useState(() =>
    readStoredNumber(READER_STORAGE_KEYS.lineHeight, READER_LINE_HEIGHT_DEFAULT, READER_LINE_HEIGHT_MIN, READER_LINE_HEIGHT_MAX),
  );
  const [contentWidth, setContentWidth] = useState(() =>
    readStoredNumber(READER_STORAGE_KEYS.contentWidth, READER_CONTENT_WIDTH_DEFAULT, READER_CONTENT_WIDTH_MIN, READER_CONTENT_WIDTH_MAX),
  );
  const [pdfZoom, setPdfZoom] = useState(() =>
    readStoredNumber(READER_STORAGE_KEYS.pdfZoom, 1.35, 0.4, 4),
  );
  const [cbzZoom, setCbzZoom] = useState(() =>
    readStoredNumber(READER_STORAGE_KEYS.cbzZoom, 1.0, 0.4, 3),
  );
  const [cbzMode, setCbzMode] = useState<CbzViewMode>(() => readStoredCbzMode());
  const [cbzCurrentPage, setCbzCurrentPage] = useState<CbzPageData | null>(null);
  const [cbzStripPages, setCbzStripPages] = useState<CbzPageData[]>([]);
  const [cbzPageLoading, setCbzPageLoading] = useState(false);
  const [cbzStripLoading, setCbzStripLoading] = useState(false);
  const [cbzError, setCbzError] = useState<string | null>(null);
  const [isFullscreen, setIsFullscreen] = useState(
    () => typeof document !== "undefined" && Boolean(document.fullscreenElement),
  );
  const [floatingAction, setFloatingAction] = useState<FloatingHighlightAction | null>(null);
  const [pendingFocusAnnotationId, setPendingFocusAnnotationId] = useState<string | null>(null);
  const [pdfLoading, setPdfLoading] = useState(false);
  const [pdfRenderError, setPdfRenderError] = useState<string | null>(null);
  const [pdfPageRendering, setPdfPageRendering] = useState(false);
  const [pdfRenderedWidth, setPdfRenderedWidth] = useState<number | null>(null);
  const [pageInput, setPageInput] = useState("1");
  const [searchQuery, setSearchQuery] = useState("");
  const [chapterMatchCount, setChapterMatchCount] = useState(0);
  const [bookSearchResults, setBookSearchResults] = useState<EpubSearchMatchData[]>([]);
  const [bookSearchTotalMatches, setBookSearchTotalMatches] = useState(0);
  const [bookSearchLoading, setBookSearchLoading] = useState(false);
  const [bookSearchError, setBookSearchError] = useState<string | null>(null);
  const [hasExecutedBookSearch, setHasExecutedBookSearch] = useState(false);
  const [pendingScrollRestore, setPendingScrollRestore] = useState<number | null>(null);
  const [pendingChapterAnchorId, setPendingChapterAnchorId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const isPdfBook = bookFormat === "pdf";
  const isCbzBook = bookFormat === "cbz";
  const isImageBook = isPdfBook || isCbzBook;
  const readerTheme = themeMode;
  const normalizedSearchQuery = searchQuery.trim();
  const navigationUnit = isImageBook ? "Página" : "Capítulo";
  const displayedPdfZoom = pdfZoom;
  const displayedCbzZoom = cbzZoom;
  const hasOpenOverlay = sidePanel !== null || preferencesOpen;

  const shortcutBindings = useMemo(
    () => ({
      nextPosition: parseShortcutBindings(shortcuts.nextPosition),
      previousPosition: parseShortcutBindings(shortcuts.previousPosition),
      openSearch: parseShortcutBindings(shortcuts.openSearch),
      toggleReaderTheme: parseShortcutBindings(shortcuts.toggleReaderTheme),
      toggleFullscreen: parseShortcutBindings(shortcuts.toggleFullscreen),
      createBookmark: parseShortcutBindings(shortcuts.createBookmark),
    }),
    [shortcuts],
  );

  const effectiveContentWidth = useMemo(
    () => getAdaptiveContentWidth(contentWidth, fontSize),
    [contentWidth, fontSize],
  );

  const effectivePdfWidth = useMemo(() => {
    if (!isPdfBook) return effectiveContentWidth;
    const renderedWidth = pdfRenderedWidth ? Math.ceil(pdfRenderedWidth + 24) : 0;
    return Math.max(effectiveContentWidth, renderedWidth);
  }, [effectiveContentWidth, isPdfBook, pdfRenderedWidth]);

  const hasEpubContent = !isPdfBook && content.trim().length > 0;

  const readerStyle = useMemo(
    () =>
      ({
        "--reader-font-size": `${fontSize}px`,
        "--reader-line-height": `${lineHeight}`,
        "--reader-content-width": `${effectiveContentWidth}px`,
        "--reader-pdf-width": `${effectivePdfWidth}px`,
      }) as CSSProperties,
    [effectiveContentWidth, effectivePdfWidth, fontSize, lineHeight],
  );

  const getEpubContentRoot = useCallback((): ParentNode | null => {
    return epubShadowRootRef.current ?? contentRef.current;
  }, []);

  const getStageScrollY = useCallback((): number => {
    return stageRef.current?.scrollTop ?? 0;
  }, []);

  // Persist display preferences
  useEffect(() => { persistStoredValue(READER_STORAGE_KEYS.fontSize, fontSize.toFixed(2)); }, [fontSize]);
  useEffect(() => { persistStoredValue(READER_STORAGE_KEYS.lineHeight, lineHeight.toFixed(2)); }, [lineHeight]);
  useEffect(() => { persistStoredValue(READER_STORAGE_KEYS.contentWidth, String(Math.round(contentWidth))); }, [contentWidth]);
  useEffect(() => { persistStoredValue(READER_STORAGE_KEYS.pdfZoom, pdfZoom.toFixed(2)); }, [pdfZoom]);
  useEffect(() => { persistStoredValue(READER_STORAGE_KEYS.cbzZoom, cbzZoom.toFixed(2)); }, [cbzZoom]);
  useEffect(() => { persistStoredValue(READER_STORAGE_KEYS.cbzMode, cbzMode); }, [cbzMode]);

  // Fullscreen tracking
  useEffect(() => {
    if (typeof document === "undefined") return;
    const handler = () => setIsFullscreen(Boolean(document.fullscreenElement));
    document.addEventListener("fullscreenchange", handler);
    return () => document.removeEventListener("fullscreenchange", handler);
  }, []);

  // Escape key to close panels
  useEffect(() => {
    if (!hasOpenOverlay) return;
    const handler = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      if (preferencesOpen) setPreferencesOpen(false);
      else setSidePanel(null);
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [hasOpenOverlay, preferencesOpen]);

  const ensurePdfEngineLoaded = useCallback(async () => {
    if (pdfGetDocumentRef.current) return pdfGetDocumentRef.current;
    if (!pdfLoaderPromiseRef.current) {
      pdfLoaderPromiseRef.current = Promise.all([
        import("pdfjs-dist"),
        import("pdfjs-dist/build/pdf.worker.min.mjs?url"),
        registerPdfWorkerMainThreadFallback(),
      ])
        .then(([pdfjsModule, workerModule]) => {
          if (typeof workerModule.default === "string" && workerModule.default.length > 0) {
            pdfjsModule.GlobalWorkerOptions.workerSrc = workerModule.default;
          }
          const dynamicGetDocument = pdfjsModule.getDocument as (source: unknown) => { promise: Promise<any> };
          pdfGetDocumentRef.current = dynamicGetDocument;
          return dynamicGetDocument;
        })
        .finally(() => { pdfLoaderPromiseRef.current = null; });
    }
    return pdfLoaderPromiseRef.current;
  }, []);

  const cycleReaderTheme = useCallback(() => {
    onThemeModeChange(readerTheme === "light" ? "dark" : "light");
  }, [onThemeModeChange, readerTheme]);

  const handleToggleFullscreen = useCallback(async () => {
    if (typeof document === "undefined") return;
    try {
      if (document.fullscreenElement) await document.exitFullscreen();
      else await document.documentElement.requestFullscreen();
    } catch (err) {
      console.warn("Falha ao alternar tela cheia:", getErrorMessage(err, "erro desconhecido"));
    }
  }, []);

  const loadAnnotations = useCallback(async () => {
    setAnnotationsLoading(true);
    try {
      const result = await invoke<Annotation[]>("get_annotations", { bookId });
      setAnnotations(result);
    } finally {
      setAnnotationsLoading(false);
    }
  }, [bookId]);

  const loadChapter = useCallback(
    async (
      chapterIndex: number,
      { persistProgress = true, restoreScrollPosition = null }: LoadChapterOptions = {},
    ) => {
      setLoading(true);
      setError(null);
      let chapterLoaded = false;
      let supportsAnnotationsForBook = false;
      let isPdfFormat = false;

      try {
        const result = await invoke<BookContent>("get_book_content", { bookId, chapterIndex });
        const chapterHtml = result.book_format === "epub" ? sanitizeEpubChapterHtml(result.html) : result.html;

        setContent(chapterHtml);
        setChapterTitle(result.chapter_title);
        setCurrentChapter(result.current_chapter);
        setTotalChapters(result.total_chapters);
        setBookFormat(result.book_format);
        setSupportsAnnotations(result.supports_annotations);
        setPdfRenderError(null);

        supportsAnnotationsForBook = result.supports_annotations;
        isPdfFormat = result.book_format === "pdf";
        chapterLoaded = true;

        if (isPdfFormat) {
          setSidePanel(null);
          setPreferencesOpen(false);
        }
      } catch (err) {
        setError(getErrorMessage(err, "Erro ao carregar capítulo"));
      } finally {
        setLoading(false);
      }

      if (!chapterLoaded) return false;

      if (!isPdfFormat && restoreScrollPosition !== null) {
        setPendingScrollRestore(restoreScrollPosition);
      }

      if (persistProgress) {
        try {
          await invoke("save_progress", { bookId, chapterIndex, scrollPosition: null });
        } catch (err) {
          console.warn("Falha ao salvar progresso:", getErrorMessage(err, "erro desconhecido"));
        }
      }

      if (!supportsAnnotationsForBook) {
        setAnnotations([]);
        setFloatingAction(null);
        setPendingFocusAnnotationId(null);
        return true;
      }

      try {
        await loadAnnotations();
      } catch (err) {
        console.warn("Falha ao carregar anotações:", getErrorMessage(err, "erro desconhecido"));
      }

      return true;
    },
    [bookId, loadAnnotations],
  );

  // Initialize reader
  useEffect(() => {
    let cancelled = false;

    const init = async () => {
      let initialIndex = 0;
      let initialScrollPosition: number | null = null;

      try {
        const progress = await invoke<ReadingProgressData | null>("get_reading_progress", { bookId });
        if (progress?.current_position) {
          const parsed = parseReadingPosition(progress.current_position);
          if (parsed.pageIndex !== null) initialIndex = parsed.pageIndex;
          else if (parsed.chapterIndex !== null) {
            initialIndex = parsed.chapterIndex;
            initialScrollPosition = parsed.scrollPosition;
          }
        }
      } catch (err) {
        console.warn("Falha ao recuperar progresso:", getErrorMessage(err, "erro desconhecido"));
      }

      if (cancelled) return;

      const loaded = await loadChapter(initialIndex, { persistProgress: false, restoreScrollPosition: initialScrollPosition });
      if (!loaded && initialIndex !== 0 && !cancelled) {
        await loadChapter(0, { persistProgress: false, restoreScrollPosition: null });
      }
    };

    void init();
    return () => { cancelled = true; };
  }, [bookId, loadChapter]);

  // Restore scroll position in stage
  useEffect(() => {
    if (pendingScrollRestore === null || isImageBook || loading || !!error) return;
    skipScrollPersistUntilRef.current = Date.now() + 1200;
    const stageEl = stageRef.current;
    if (!stageEl) return;
    const frameId = window.requestAnimationFrame(() => {
      stageEl.scrollTo({ top: pendingScrollRestore, behavior: "auto" });
      setPendingScrollRestore(null);
    });
    return () => window.cancelAnimationFrame(frameId);
  }, [error, isImageBook, loading, pendingScrollRestore]);

  // Scroll to chapter anchor
  useEffect(() => {
    if (!pendingChapterAnchorId || isImageBook || loading || !!error) return;
    const contentRoot = getEpubContentRoot();
    const stageEl = stageRef.current;
    if (!contentRoot || !stageEl) return;

    let frameId = 0;
    let attempts = 0;

    const tryScroll = () => {
      attempts += 1;
      const didScroll = scrollToEpubAnchor(contentRoot, pendingChapterAnchorId, stageEl);
      if (didScroll || attempts >= 6) {
        if (didScroll) skipScrollPersistUntilRef.current = Date.now() + 1200;
        setPendingChapterAnchorId(null);
        return;
      }
      frameId = window.requestAnimationFrame(tryScroll);
    };

    frameId = window.requestAnimationFrame(tryScroll);
    return () => window.cancelAnimationFrame(frameId);
  }, [error, getEpubContentRoot, isImageBook, loading, pendingChapterAnchorId]);

  // Persist scroll progress via stage element
  useEffect(() => {
    if (isImageBook || loading || !!error) return;
    const stageEl = stageRef.current;
    if (!stageEl) return;

    let timeoutId: number | null = null;

    const handleScroll = () => {
      if (Date.now() < skipScrollPersistUntilRef.current) return;
      if (timeoutId !== null) window.clearTimeout(timeoutId);
      timeoutId = window.setTimeout(() => {
        void invoke("save_progress", {
          bookId,
          chapterIndex: currentChapter,
          scrollPosition: stageEl.scrollTop,
        }).catch((err) => {
          console.warn("Falha ao salvar progresso:", getErrorMessage(err, "erro desconhecido"));
        });
      }, 500);
    };

    stageEl.addEventListener("scroll", handleScroll, { passive: true });
    return () => {
      stageEl.removeEventListener("scroll", handleScroll);
      if (timeoutId !== null) window.clearTimeout(timeoutId);
    };
  }, [bookId, currentChapter, error, isImageBook, loading]);

  // Reset search index on chapter/query change
  useEffect(() => {
    chapterSearchKeyRef.current = "";
    chapterSearchIndexRef.current = -1;
  }, [content, currentChapter, normalizedSearchQuery]);

  // Count matches in current chapter
  useEffect(() => {
    if (isImageBook) { setChapterMatchCount(0); return; }
    if (!normalizedSearchQuery || loading || !!error) { setChapterMatchCount(0); return; }
    const frameId = window.requestAnimationFrame(() => {
      const root = getEpubContentRoot();
      if (!root) { setChapterMatchCount(0); return; }
      setChapterMatchCount(createReaderSearchRanges(root, normalizedSearchQuery).length);
    });
    return () => window.cancelAnimationFrame(frameId);
  }, [content, currentChapter, error, getEpubContentRoot, isImageBook, loading, normalizedSearchQuery]);

  // Clear book search when query is empty
  useEffect(() => {
    if (normalizedSearchQuery.length > 0) return;
    setBookSearchResults([]);
    setBookSearchTotalMatches(0);
    setBookSearchError(null);
    setHasExecutedBookSearch(false);
  }, [normalizedSearchQuery]);

  // PDF document loading
  useEffect(() => {
    if (!isPdfBook) {
      const doc = pdfDocumentRef.current;
      if (doc && typeof doc.destroy === "function") void doc.destroy();
      pdfDocumentRef.current = null;
      setPdfLoading(false);
      setPdfPageRendering(false);
      setPdfRenderError(null);
      return;
    }

    let cancelled = false;

    const loadPdf = async () => {
      setPdfLoading(true);
      setPdfRenderError(null);
      try {
        const getDocument = await ensurePdfEngineLoaded();
        const result = await invoke<PdfDocumentData>("get_pdf_document", { bookId });
        const sourceBytes = decodeBase64ToBytes(result.bytes_base64);

        let loadedPdf: any;
        try {
          loadedPdf = await getDocument({ data: sourceBytes }).promise;
        } catch (workerErr) {
          if (!isPdfWorkerModuleScriptError(workerErr)) throw workerErr;
          await registerPdfWorkerMainThreadFallback();
          loadedPdf = await getDocument({ data: sourceBytes }).promise;
        }

        if (cancelled) { await loadedPdf.destroy(); return; }

        const prev = pdfDocumentRef.current;
        if (prev && typeof prev.destroy === "function") await prev.destroy();
        pdfDocumentRef.current = loadedPdf;

        const resolvedPages = Math.max(result.total_pages, loadedPdf.numPages, 1);
        setTotalChapters(resolvedPages);
        setCurrentChapter((prev) => Math.min(prev, resolvedPages - 1));
      } catch (err) {
        setPdfRenderError(getErrorMessage(err, "Falha ao carregar PDF"));
      } finally {
        if (!cancelled) setPdfLoading(false);
      }
    };

    void loadPdf();
    return () => { cancelled = true; };
  }, [bookId, ensurePdfEngineLoaded, isPdfBook]);

  // Cleanup PDF on unmount
  useEffect(() => {
    return () => {
      const doc = pdfDocumentRef.current;
      if (doc && typeof doc.destroy === "function") void doc.destroy();
      pdfDocumentRef.current = null;
    };
  }, []);

  // Sync page input with current chapter
  useEffect(() => {
    if (!isImageBook) return;
    setPageInput(String(currentChapter + 1));
  }, [currentChapter, isImageBook]);

  // CBZ: load current page (single mode)
  useEffect(() => {
    if (!isCbzBook || cbzMode !== "single") return;
    let cancelled = false;
    setCbzPageLoading(true);
    setCbzError(null);
    invoke<CbzPageData>("get_cbz_page", { bookId, pageIndex: currentChapter })
      .then((page) => {
        if (cancelled) return;
        setCbzCurrentPage(page);
        if (page.total_pages > 0 && page.total_pages !== totalChapters) {
          setTotalChapters(page.total_pages);
        }
      })
      .catch((err) => {
        if (cancelled) return;
        setCbzError(getErrorMessage(err, "Falha ao carregar página"));
      })
      .finally(() => {
        if (!cancelled) setCbzPageLoading(false);
      });
    return () => { cancelled = true; };
  }, [bookId, cbzMode, currentChapter, isCbzBook, totalChapters]);

  // CBZ: preload all pages (strip mode)
  useEffect(() => {
    if (!isCbzBook || cbzMode !== "strip" || totalChapters <= 0) return;
    let cancelled = false;
    setCbzStripLoading(true);
    setCbzError(null);
    setCbzStripPages([]);

    const loadAll = async () => {
      const collected: CbzPageData[] = [];
      for (let index = 0; index < totalChapters; index += 1) {
        if (cancelled) return;
        try {
          const page = await invoke<CbzPageData>("get_cbz_page", { bookId, pageIndex: index });
          collected.push(page);
          if (!cancelled) setCbzStripPages([...collected]);
        } catch (err) {
          if (!cancelled) setCbzError(getErrorMessage(err, "Falha ao carregar página"));
          return;
        }
      }
      if (!cancelled) setCbzStripLoading(false);
    };

    void loadAll();
    return () => { cancelled = true; setCbzStripLoading(false); };
  }, [bookId, cbzMode, isCbzBook, totalChapters]);

  // CBZ: clear state when leaving cbz
  useEffect(() => {
    if (isCbzBook) return;
    setCbzCurrentPage(null);
    setCbzStripPages([]);
    setCbzError(null);
    setCbzPageLoading(false);
    setCbzStripLoading(false);
  }, [isCbzBook]);

  const handleCbzZoomDelta = useCallback((delta: number) => {
    setCbzZoom((prev) => clampNumber(+(prev + delta).toFixed(2), 0.4, 3));
  }, []);

  // PDF page rendering
  useEffect(() => {
    if (pdfLoading || !isPdfBook || !pdfDocumentRef.current || !pdfCanvasRef.current) return;

    let cancelled = false;
    let activeRenderTask: { promise: Promise<unknown>; cancel: () => void } | null = null;

    const render = async () => {
      setPdfPageRendering(true);
      setPdfRenderError(null);
      try {
        const pdf = pdfDocumentRef.current;
        const pageNum = Math.min(Math.max(currentChapter + 1, 1), pdf.numPages);
        const page = await pdf.getPage(pageNum);
        if (cancelled) return;

        const viewport = page.getViewport({ scale: clampNumber(pdfZoom, 0.4, 4) });
        if (!cancelled) setPdfRenderedWidth(viewport.width);

        const canvas = pdfCanvasRef.current;
        if (!canvas) return;
        const ctx = canvas.getContext("2d");
        if (!ctx) throw new Error("Canvas indisponível");

        const dpr = window.devicePixelRatio || 1;
        canvas.width = Math.ceil(viewport.width * dpr);
        canvas.height = Math.ceil(viewport.height * dpr);
        canvas.style.width = `${Math.ceil(viewport.width)}px`;
        canvas.style.height = `${Math.ceil(viewport.height)}px`;
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

        const task = page.render({ canvasContext: ctx, viewport });
        activeRenderTask = task;
        await task.promise;
      } catch (err) {
        const msg = getErrorMessage(err, "Falha ao renderizar página");
        const isCancel =
          cancelled ||
          (err instanceof Error && err.name === "RenderingCancelledException") ||
          msg.toLowerCase().includes("rendering cancelled");
        if (!isCancel) setPdfRenderError(msg);
      } finally {
        if (!cancelled) setPdfPageRendering(false);
      }
    };

    void render();
    return () => {
      cancelled = true;
      activeRenderTask?.cancel();
    };
  }, [currentChapter, isPdfBook, pdfLoading, pdfZoom]);

  // Clear shadow DOM content when switching to PDF/CBZ
  useEffect(() => {
    if (!isImageBook) return;
    if (contentRef.current) contentRef.current.innerHTML = "";
    if (epubShadowRootRef.current) epubShadowRootRef.current.innerHTML = "";
  }, [isImageBook]);

  const chapterItems = useMemo(
    () => Array.from({ length: totalChapters }, (_, i) => i),
    [totalChapters],
  );

  // Annotation helpers
  const buildSelectionPosition = (text: string): string => {
    const hash = Array.from(text).reduce((acc, char) => (acc * 31 + char.charCodeAt(0)) >>> 0, 7);
    return `chapter:${currentChapter};hash:${hash};len:${text.length}`;
  };

  const getAnnotationChapter = (position: string): number | null => {
    const match = position.match(/chapter:(\d+)/);
    if (!match) return null;
    const ch = Number.parseInt(match[1], 10);
    return Number.isNaN(ch) ? null : ch;
  };

  const escapeRegExp = (value: string): string => value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

  const injectHighlights = useCallback(
    (rawHtml: string): string => {
      const chapterAnnotations = annotations.filter(
        (a) => a.annotation_type === "highlight" && getAnnotationChapter(a.position) === currentChapter,
      );
      let html = rawHtml;
      for (const annotation of chapterAnnotations) {
        const text = annotation.selected_text?.trim();
        if (!text) continue;
        const color = HIGHLIGHT_COLORS.includes(annotation.color) ? annotation.color : "yellow";
        const replacement = `<span id="annotation-${annotation.id}" data-annotation-id="${annotation.id}" class="reader-highlight highlight-${color}">${text}</span>`;
        html = html.replace(new RegExp(escapeRegExp(text)), replacement);
      }
      return html;
    },
    [annotations, currentChapter],
  );

  const renderedContent = injectHighlights(content);

  // Inject EPUB content into shadow DOM (depends on renderedContent)
  useEffect(() => {
    if (isImageBook) return;
    const host = contentRef.current;
    if (!host) return;

    let shadowRoot = epubShadowRootRef.current ?? host.shadowRoot;
    if (!shadowRoot) {
      try { shadowRoot = host.attachShadow({ mode: "open" }); }
      catch { shadowRoot = null; }
    }

    if (!shadowRoot) {
      epubShadowRootRef.current = null;
      host.innerHTML = renderedContent;
      return;
    }

    epubShadowRootRef.current = shadowRoot;
    shadowRoot.innerHTML = buildEpubShadowMarkup(renderedContent);
    if (host.innerHTML.length > 0) host.innerHTML = "";
  }, [isImageBook, renderedContent]);

  // Mouse selection → floating highlight action
  useEffect(() => {
    if (!supportsAnnotations || isImageBook) return;
    const contentRoot = getEpubContentRoot();
    const contentHost = contentRef.current;
    if (!contentRoot || !contentHost) return;

    const handleMouseUp = () => {
      const selection = window.getSelection();
      if (!selection || selection.rangeCount === 0) { setFloatingAction(null); return; }
      const selectedText = selection.toString().trim();
      if (!selectedText) { setFloatingAction(null); return; }

      const range = selection.getRangeAt(0);
      const rootNode = range.commonAncestorContainer.getRootNode();
      const isInsideShadow = epubShadowRootRef.current && rootNode === epubShadowRootRef.current;
      const isInsideHost = !epubShadowRootRef.current && contentHost.contains(range.commonAncestorContainer);

      if (!isInsideShadow && !isInsideHost) { setFloatingAction(null); return; }

      const rect = range.getBoundingClientRect();
      setFloatingAction({
        text: selectedText,
        position: buildSelectionPosition(selectedText),
        top: rect.top - 44,
        left: rect.left,
      });
    };

    const target = contentRoot as EventTarget;
    target.addEventListener("mouseup", handleMouseUp);
    return () => target.removeEventListener("mouseup", handleMouseUp);
  }, [content, currentChapter, getEpubContentRoot, isImageBook, supportsAnnotations]);

  // Focus annotation highlight in content
  useEffect(() => {
    if (!supportsAnnotations || !pendingFocusAnnotationId) return;
    const selector = `#annotation-${pendingFocusAnnotationId}`;
    const target = (epubShadowRootRef.current?.querySelector(selector) as HTMLElement | null)
      ?? document.getElementById(`annotation-${pendingFocusAnnotationId}`);
    if (!target) return;

    target.scrollIntoView({ behavior: "smooth", block: "center" });
    target.classList.add("highlight-pulse");
    const timeout = window.setTimeout(() => target.classList.remove("highlight-pulse"), 1400);
    setPendingFocusAnnotationId(null);
    return () => window.clearTimeout(timeout);
  }, [pendingFocusAnnotationId, renderedContent, supportsAnnotations]);

  // EPUB internal link clicks
  useEffect(() => {
    if (isImageBook) return;
    const contentRoot = getEpubContentRoot();
    if (!contentRoot) return;

    const handleClick = (event: Event) => {
      if (!(event instanceof MouseEvent) || event.defaultPrevented) return;
      if (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;

      const path = typeof event.composedPath === "function" ? event.composedPath() : [];
      let anchor = path.find((e): e is HTMLAnchorElement => e instanceof HTMLAnchorElement) ?? null;
      if (!anchor && event.target instanceof Element) {
        const closest = event.target.closest("a");
        if (closest instanceof HTMLAnchorElement) anchor = closest;
      }
      if (!anchor) return;

      const href = anchor.getAttribute("href")?.trim();
      if (!href || isExternalEpubHref(href)) return;

      event.preventDefault();
      void handleNavigateEpubInternalLink(href);
    };

    const target = contentRoot as EventTarget;
    target.addEventListener("click", handleClick);
    return () => target.removeEventListener("click", handleClick);
  }, [content, currentChapter, getEpubContentRoot, isImageBook]);

  // Annotation CRUD
  const handleCreateHighlight = async () => {
    if (!supportsAnnotations || !floatingAction) return;
    try {
      await invoke("add_annotation", {
        bookId,
        annotation: {
          annotationType: "highlight",
          position: floatingAction.position,
          positionEnd: null,
          selectedText: floatingAction.text,
          noteText: null,
          color: "yellow",
        },
      });
      setFloatingAction(null);
      window.getSelection()?.removeAllRanges();
      await loadAnnotations();
      setSidePanel("annot");
    } catch (err) {
      alert(getErrorMessage(err, "Falha ao criar destaque"));
    }
  };

  const handleCreateBookmark = useCallback(async () => {
    if (!supportsAnnotations) return;
    const scrollY = getStageScrollY();
    const bookmarkPosition = `chapter:${currentChapter};scroll:${scrollY.toFixed(2)}`;
    const alreadyExists = annotations.some(
      (a) => a.annotation_type === "bookmark" && a.position === bookmarkPosition,
    );
    if (alreadyExists) return;

    try {
      await invoke("add_annotation", {
        bookId,
        annotation: {
          annotationType: "bookmark",
          position: bookmarkPosition,
          positionEnd: null,
          selectedText: null,
          noteText: `Marcador no capítulo ${currentChapter + 1}`,
          color: "blue",
        },
      });
      await loadAnnotations();
      setSidePanel("annot");
    } catch (err) {
      alert(getErrorMessage(err, "Falha ao criar marcador"));
    }
  }, [annotations, bookId, currentChapter, getStageScrollY, loadAnnotations, supportsAnnotations]);

  const handleUpdateNote = async (annotationId: string, noteText: string) => {
    try {
      await invoke("update_annotation_note", { id: annotationId, noteText });
      await loadAnnotations();
    } catch (err) {
      alert(getErrorMessage(err, "Falha ao atualizar nota"));
    }
  };

  const handleUpdateColor = async (
    annotationId: string,
    color: "yellow" | "green" | "blue" | "pink" | "purple",
  ) => {
    try {
      await invoke("update_annotation_color", { id: annotationId, color });
      await loadAnnotations();
    } catch (err) {
      alert(getErrorMessage(err, "Falha ao atualizar cor"));
    }
  };

  const handleDeleteAnnotation = async (annotationId: string) => {
    try {
      await invoke("delete_annotation", { id: annotationId });
      await loadAnnotations();
    } catch (err) {
      alert(getErrorMessage(err, "Falha ao excluir anotação"));
    }
  };

  const handleSelectAnnotation = async (annotation: Annotation) => {
    if (!supportsAnnotations) return;
    const parsed = parseReadingPosition(annotation.position);
    const targetIndex = parsed.chapterIndex ?? parsed.pageIndex;
    if (targetIndex === null) return;

    if (targetIndex !== currentChapter) {
      await loadChapter(targetIndex, { restoreScrollPosition: parsed.scrollPosition });
    } else if (!isImageBook && parsed.scrollPosition !== null) {
      setPendingScrollRestore(parsed.scrollPosition);
    }

    if (annotation.annotation_type === "highlight") {
      setPendingFocusAnnotationId(annotation.id);
    }
  };

  // Navigation
  const persistProgress = useCallback(async (index: number) => {
    try {
      await invoke("save_progress", { bookId, chapterIndex: index, scrollPosition: null });
    } catch (err) {
      console.warn("Falha ao salvar progresso:", getErrorMessage(err, "erro desconhecido"));
    }
  }, [bookId]);

  const goToPosition = useCallback(
    async (targetIndex: number) => {
      const maxIndex = Math.max(totalChapters - 1, 0);
      const clamped = Math.min(Math.max(targetIndex, 0), maxIndex);
      if (isImageBook) {
        setCurrentChapter(clamped);
        await persistProgress(clamped);
        return;
      }
      if (clamped !== currentChapter) await loadChapter(clamped);
    },
    [currentChapter, isImageBook, loadChapter, persistProgress, totalChapters],
  );

  const handleNavigateEpubInternalLink = useCallback(
    async (rawHref: string) => {
      if (isImageBook) return;
      const href = rawHref.trim();
      if (!href || isExternalEpubHref(href)) return;
      const fallbackAnchorId = extractEpubHrefFragment(href);

      if (href.startsWith("#")) {
        if (fallbackAnchorId) setPendingChapterAnchorId(fallbackAnchorId);
        return;
      }

      try {
        const target = await invoke<EpubLinkTarget>("resolve_epub_link_target", {
          bookId,
          chapterIndex: currentChapter,
          href,
        });
        const targetIndex = Math.max(target.chapter_index, 0);
        if (targetIndex !== currentChapter) {
          const loaded = await loadChapter(targetIndex);
          if (!loaded) return;
        }
        const anchorId = target.anchor_id ?? fallbackAnchorId;
        if (anchorId) setPendingChapterAnchorId(anchorId);
      } catch (err) {
        console.warn("Falha ao navegar link interno:", getErrorMessage(err, "erro desconhecido"));
      }
    },
    [bookId, currentChapter, isImageBook, loadChapter],
  );

  // Search
  const handleFindInCurrentChapter = useCallback(
    (direction: "next" | "previous") => {
      if (isImageBook) return;
      const term = normalizedSearchQuery;
      if (!term) return;
      const contentRoot = getEpubContentRoot();
      if (!contentRoot) return;

      const ranges = createReaderSearchRanges(contentRoot, term);
      if (ranges.length === 0) return;

      const searchKey = `${currentChapter}:${term.toLowerCase()}`;
      if (chapterSearchKeyRef.current !== searchKey) {
        chapterSearchKeyRef.current = searchKey;
        chapterSearchIndexRef.current = direction === "previous" ? ranges.length : -1;
      }

      const nextIndex =
        direction === "previous"
          ? (chapterSearchIndexRef.current - 1 + ranges.length) % ranges.length
          : (chapterSearchIndexRef.current + 1) % ranges.length;

      chapterSearchIndexRef.current = nextIndex;
      focusReaderSearchRange(ranges[nextIndex]);
      findInDocument(term, direction === "previous");
    },
    [currentChapter, getEpubContentRoot, isImageBook, normalizedSearchQuery],
  );

  const handleSearchWholeBook = useCallback(async () => {
    if (isImageBook) return;
    const term = normalizedSearchQuery;
    if (!term) {
      setBookSearchResults([]);
      setBookSearchTotalMatches(0);
      setBookSearchError(null);
      setHasExecutedBookSearch(false);
      return;
    }
    setBookSearchLoading(true);
    setBookSearchError(null);
    setHasExecutedBookSearch(true);
    try {
      const response = await invoke<EpubSearchResponse>("search_epub_content", { bookId, query: term });
      setBookSearchResults(response.results);
      setBookSearchTotalMatches(response.total_matches);
    } catch (err) {
      setBookSearchResults([]);
      setBookSearchTotalMatches(0);
      setBookSearchError(getErrorMessage(err, "Falha ao buscar no livro"));
    } finally {
      setBookSearchLoading(false);
    }
  }, [bookId, isImageBook, normalizedSearchQuery]);

  const handleSelectBookSearchResult = useCallback(
    async (result: EpubSearchMatchData) => {
      await loadChapter(result.chapter_index);
      setSidePanel(null);
      const term = normalizedSearchQuery;
      if (!term) return;
      window.requestAnimationFrame(() => {
        window.requestAnimationFrame(() => handleFindInCurrentChapter("next"));
      });
    },
    [handleFindInCurrentChapter, loadChapter, normalizedSearchQuery],
  );

  const handlePdfZoomDelta = useCallback((delta: number) => {
    setPdfZoom((prev) => clampNumber(prev + delta, 0.4, 4));
  }, []);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (isEditableTarget(event.target)) return;

      if (isShortcutEventMatch(event, shortcutBindings.nextPosition)) {
        event.preventDefault();
        void goToPosition(currentChapter + 1);
        return;
      }
      if (isShortcutEventMatch(event, shortcutBindings.previousPosition)) {
        event.preventDefault();
        void goToPosition(currentChapter - 1);
        return;
      }
      if (isShortcutEventMatch(event, shortcutBindings.openSearch)) {
        if (!isImageBook) {
          event.preventDefault();
          setSidePanel((prev) => {
            if (prev !== "search") {
              window.requestAnimationFrame(() => {
                searchInputRef.current?.focus();
                searchInputRef.current?.select();
              });
              return "search";
            }
            searchInputRef.current?.focus();
            return prev;
          });
        }
        return;
      }
      if (isShortcutEventMatch(event, shortcutBindings.toggleReaderTheme)) {
        event.preventDefault();
        cycleReaderTheme();
        return;
      }
      if (isShortcutEventMatch(event, shortcutBindings.toggleFullscreen)) {
        event.preventDefault();
        void handleToggleFullscreen();
        return;
      }
      if (isShortcutEventMatch(event, shortcutBindings.createBookmark)) {
        event.preventDefault();
        void handleCreateBookmark();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [
    currentChapter,
    cycleReaderTheme,
    goToPosition,
    handleCreateBookmark,
    handleToggleFullscreen,
    isImageBook,
    shortcutBindings,
  ]);

  const handlePageInputSubmit = () => {
    const parsed = Number.parseInt(pageInput, 10);
    if (Number.isNaN(parsed)) { setPageInput(String(currentChapter + 1)); return; }
    void goToPosition(parsed - 1);
  };

  const canGoPrevious = currentChapter > 0;
  const canGoNext = totalChapters > 0 && currentChapter < totalChapters - 1;
  const progressPct = totalChapters > 0 ? ((currentChapter + 1) / totalChapters) * 100 : 0;
  const readingStatusLabel = `${navigationUnit} ${currentChapter + 1} / ${totalChapters || "—"}`;

  return (
    <div className={`reader-view theme-${readerTheme}`} style={readerStyle}>

      {/* ── TOP CHROME ── */}
      <div className="reader-chrome-top">
        <button
          type="button"
          className="reader-icon-btn"
          onClick={onClose}
          aria-label="Fechar leitor"
          title="Fechar"
        >
          <IconBack />
        </button>

        <div className="reader-title-block">
          <div className="reader-book-title">{chapterTitle || "Leitor"}</div>
          <div className="reader-book-chapter">{readingStatusLabel}</div>
        </div>

        <div className="reader-spacer" />

        {/* Inline search bar */}
        {!isImageBook && (
          <div className="reader-search-bar">
            <IconSearch />
            <input
              placeholder="Buscar no livro…"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.currentTarget.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  setSidePanel("search");
                  void handleSearchWholeBook();
                }
              }}
              aria-label="Busca no livro"
            />
            {normalizedSearchQuery && chapterMatchCount > 0 && (
              <span className="reader-search-count-badge">{chapterMatchCount}</span>
            )}
          </div>
        )}

        <div className="reader-spacer" />

        {/* TOC toggle */}
        {!isImageBook && (
          <button
            type="button"
            className={`reader-icon-btn${sidePanel === "toc" ? " active" : ""}`}
            onClick={() => setSidePanel((p) => (p === "toc" ? null : "toc"))}
            aria-label="Índice"
            title="Índice"
          >
            <IconToc />
          </button>
        )}

        {/* Annotations toggle */}
        {supportsAnnotations && !isImageBook && (
          <button
            type="button"
            className={`reader-icon-btn${sidePanel === "annot" ? " active" : ""}`}
            onClick={() => setSidePanel((p) => (p === "annot" ? null : "annot"))}
            aria-label="Anotações"
            title="Anotações"
          >
            <IconAnnot />
          </button>
        )}

        <div className="reader-chrome-divider" />

        {/* Typography popover */}
        <div className="reader-popover-wrap">
          <button
            type="button"
            className={`reader-icon-btn${preferencesOpen ? " active" : ""}`}
            onClick={() => setPreferencesOpen((p) => !p)}
            aria-label="Ajustes de leitura"
            title="Tipografia"
          >
            Aa
          </button>

          {preferencesOpen && (
            <div className="reader-popover">
              {/* Theme */}
              <div className="reader-popover-row">
                <span>Tema</span>
                <div className="reader-theme-swatches">
                  <div
                    className={`reader-theme-swatch${readerTheme === "light" ? " active" : ""}`}
                    style={{ background: "#f6f5f4" }}
                    onClick={() => onThemeModeChange("light")}
                    title="Claro"
                  />
                  <div
                    className={`reader-theme-swatch${readerTheme === "dark" ? " active" : ""}`}
                    style={{ background: "#0c0c0e" }}
                    onClick={() => onThemeModeChange("dark")}
                    title="Escuro"
                  />
                </div>
              </div>

              {!isImageBook && (
                <>
                  {/* Font size */}
                  <div className="reader-popover-row">
                    <span>Fonte</span>
                    <div className="reader-stepper">
                      <button type="button" onClick={() => setFontSize((s) => clampNumber(s - 1, READER_FONT_SIZE_MIN, READER_FONT_SIZE_MAX))}>−</button>
                      <div className="reader-stepper-val">{Math.round(fontSize)}px</div>
                      <button type="button" onClick={() => setFontSize((s) => clampNumber(s + 1, READER_FONT_SIZE_MIN, READER_FONT_SIZE_MAX))}>+</button>
                    </div>
                  </div>

                  {/* Line height */}
                  <div className="reader-popover-row">
                    <span>Entrelinha</span>
                    <div className="reader-stepper">
                      <button type="button" onClick={() => setLineHeight((l) => clampNumber(+(l - 0.05).toFixed(2), READER_LINE_HEIGHT_MIN, READER_LINE_HEIGHT_MAX))}>−</button>
                      <div className="reader-stepper-val">{lineHeight.toFixed(2)}</div>
                      <button type="button" onClick={() => setLineHeight((l) => clampNumber(+(l + 0.05).toFixed(2), READER_LINE_HEIGHT_MIN, READER_LINE_HEIGHT_MAX))}>+</button>
                    </div>
                  </div>

                  {/* Content width */}
                  <div className="reader-popover-row">
                    <span>Largura</span>
                    <div className="reader-stepper">
                      <button type="button" onClick={() => setContentWidth((w) => clampNumber(w - 40, READER_CONTENT_WIDTH_MIN, READER_CONTENT_WIDTH_MAX))}>−</button>
                      <div className="reader-stepper-val" style={{ fontSize: 10 }}>{Math.round(effectiveContentWidth)}px</div>
                      <button type="button" onClick={() => setContentWidth((w) => clampNumber(w + 40, READER_CONTENT_WIDTH_MIN, READER_CONTENT_WIDTH_MAX))}>+</button>
                    </div>
                  </div>
                </>
              )}

              {isPdfBook && (
                <div className="reader-popover-row">
                  <span>Zoom</span>
                  <div className="reader-stepper">
                    <button type="button" onClick={() => handlePdfZoomDelta(-0.1)}>−</button>
                    <div className="reader-stepper-val">{Math.round(displayedPdfZoom * 100)}%</div>
                    <button type="button" onClick={() => handlePdfZoomDelta(0.1)}>+</button>
                  </div>
                </div>
              )}

              {isCbzBook && (
                <>
                  <div className="reader-popover-row reader-popover-row-stack">
                    <span>Modo de leitura</span>
                    <div className="reader-mode-toggle" role="group" aria-label="Modo de leitura">
                      <button
                        type="button"
                        className={`reader-mode-option${cbzMode === "single" ? " active" : ""}`}
                        onClick={() => setCbzMode("single")}
                        aria-pressed={cbzMode === "single"}
                        title="Uma página por vez"
                      >
                        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                          <rect x="6" y="3.5" width="12" height="17" rx="1.8" />
                          <line x1="9" y1="8" x2="15" y2="8" />
                          <line x1="9" y1="12" x2="15" y2="12" />
                          <line x1="9" y1="16" x2="13" y2="16" />
                        </svg>
                        <span>Página</span>
                      </button>
                      <button
                        type="button"
                        className={`reader-mode-option${cbzMode === "strip" ? " active" : ""}`}
                        onClick={() => setCbzMode("strip")}
                        aria-pressed={cbzMode === "strip"}
                        title="Rolagem contínua"
                      >
                        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                          <rect x="7" y="2.5" width="10" height="6" rx="1.4" />
                          <rect x="7" y="10" width="10" height="6" rx="1.4" />
                          <rect x="7" y="17.5" width="10" height="4" rx="1.4" />
                        </svg>
                        <span>Tira</span>
                      </button>
                    </div>
                  </div>
                  <div className="reader-popover-row">
                    <span>Zoom</span>
                    <div className="reader-stepper">
                      <button type="button" onClick={() => handleCbzZoomDelta(-0.1)}>−</button>
                      <div className="reader-stepper-val">{Math.round(displayedCbzZoom * 100)}%</div>
                      <button type="button" onClick={() => handleCbzZoomDelta(0.1)}>+</button>
                    </div>
                  </div>
                </>
              )}
            </div>
          )}
        </div>

        {/* Bookmark */}
        {supportsAnnotations && !isImageBook && (
          <button
            type="button"
            className="reader-icon-btn"
            onClick={() => void handleCreateBookmark()}
            aria-label="Adicionar marcador"
            title="Marcador"
          >
            <IconBookmark />
          </button>
        )}

        {/* Fullscreen */}
        <button
          type="button"
          className="reader-icon-btn"
          onClick={() => void handleToggleFullscreen()}
          aria-label={isFullscreen ? "Sair da tela cheia" : "Tela cheia"}
          title={isFullscreen ? "Sair da tela cheia" : "Tela cheia"}
        >
          {isFullscreen ? <IconCompress /> : <IconExpand />}
        </button>
      </div>

      {/* ── MIDDLE ── */}
      <div className="reader-middle">

        {/* Stage */}
        <div className="reader-stage" ref={stageRef}>
          <div className="reader-content-col">

            {loading && !hasEpubContent && (
              <p className="reader-state">Carregando {navigationUnit.toLowerCase()}…</p>
            )}
            {error && <p className="reader-state reader-error">{error}</p>}

            {!error && !isImageBook && hasEpubContent && (
              <article className={`reader-epub-article${loading ? " is-transitioning" : ""}`}>
                <div ref={contentRef} className="reader-content-shadow-host" />
              </article>
            )}

            {!loading && !error && isPdfBook && (
              <div className="reader-pdf-shell">
                {pdfLoading && <p className="reader-state">Carregando PDF…</p>}
                {pdfPageRendering && <p className="reader-state">Renderizando página…</p>}
                {pdfRenderError && <p className="reader-state reader-error">{pdfRenderError}</p>}
                {!pdfLoading && !pdfRenderError && (
                  <div ref={pdfCanvasWrapRef} className="reader-pdf-canvas-wrap">
                    <div className="reader-pdf-canvas-stage">
                      <canvas ref={pdfCanvasRef} className="reader-pdf-canvas" />
                    </div>
                  </div>
                )}
              </div>
            )}

            {!loading && !error && isCbzBook && (
              <div className="reader-cbz-shell">
                {cbzError && <p className="reader-state reader-error">{cbzError}</p>}

                {!cbzError && cbzMode === "single" && (
                  <div className="reader-cbz-single">
                    {cbzPageLoading && !cbzCurrentPage && (
                      <p className="reader-state">Carregando página…</p>
                    )}
                    {cbzCurrentPage && (
                      <img
                        className="reader-cbz-image"
                        style={{ maxWidth: `${Math.round(cbzZoom * 100)}%` }}
                        src={`data:${cbzCurrentPage.mime_type};base64,${cbzCurrentPage.bytes_base64}`}
                        alt={`Página ${cbzCurrentPage.page_index + 1}`}
                      />
                    )}
                  </div>
                )}

                {!cbzError && cbzMode === "strip" && (
                  <div className="reader-cbz-strip">
                    {cbzStripPages.map((page) => (
                      <img
                        key={page.page_index}
                        className="reader-cbz-image"
                        style={{ maxWidth: `${Math.round(cbzZoom * 100)}%` }}
                        src={`data:${page.mime_type};base64,${page.bytes_base64}`}
                        alt={`Página ${page.page_index + 1}`}
                        loading="lazy"
                      />
                    ))}
                    {cbzStripLoading && (
                      <p className="reader-state">Carregando páginas… ({cbzStripPages.length}/{totalChapters})</p>
                    )}
                  </div>
                )}
              </div>
            )}

          </div>
        </div>

        {/* Side panel */}
        {sidePanel !== null && !isImageBook && (
          <aside className="reader-side" aria-label="Painel lateral">
            <div className="reader-side-tabs">
              <button
                type="button"
                className={`reader-side-tab${sidePanel === "toc" ? " active" : ""}`}
                onClick={() => setSidePanel("toc")}
              >
                Índice
              </button>
              {supportsAnnotations && (
                <button
                  type="button"
                  className={`reader-side-tab${sidePanel === "annot" ? " active" : ""}`}
                  onClick={() => setSidePanel("annot")}
                >
                  Anotações
                </button>
              )}
              <button
                type="button"
                className={`reader-side-tab${sidePanel === "search" ? " active" : ""}`}
                onClick={() => setSidePanel("search")}
              >
                Busca
              </button>
            </div>

            <div className="reader-side-body">

              {/* TOC */}
              {sidePanel === "toc" && (
                <>
                  {chapterItems.length === 0 && (
                    <p className="reader-state">Sem capítulos identificados.</p>
                  )}
                  {chapterItems.map((index) => (
                    <button
                      key={index}
                      type="button"
                      className={`reader-toc-item${index === currentChapter ? " active" : ""}`}
                      onClick={() => void goToPosition(index)}
                    >
                      <span className="reader-toc-num">{index + 1}</span>
                      <span>Capítulo {index + 1}</span>
                    </button>
                  ))}
                </>
              )}

              {/* Annotations */}
              {sidePanel === "annot" && supportsAnnotations && (
                <AnnotationSidebar
                  annotations={annotations}
                  loading={annotationsLoading}
                  onAddNote={handleUpdateNote}
                  onDelete={handleDeleteAnnotation}
                  onColorChange={handleUpdateColor}
                  onSelectAnnotation={handleSelectAnnotation}
                />
              )}

              {/* Search */}
              {sidePanel === "search" && (
                <div>
                  <div className="reader-side-search-input">
                    <IconSearch />
                    <input
                      ref={searchInputRef}
                      type="search"
                      value={searchQuery}
                      placeholder="Buscar no livro…"
                      onChange={(e) => setSearchQuery(e.currentTarget.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") {
                          e.preventDefault();
                          handleFindInCurrentChapter(e.shiftKey ? "previous" : "next");
                        }
                      }}
                      aria-label="Busca"
                    />
                  </div>

                  <div className="reader-side-search-actions">
                    <button
                      type="button"
                      className="reader-side-btn"
                      onClick={() => handleFindInCurrentChapter("next")}
                    >
                      Próxima
                    </button>
                    <button
                      type="button"
                      className="reader-side-btn"
                      onClick={() => void handleSearchWholeBook()}
                    >
                      Buscar livro
                    </button>
                  </div>

                  {normalizedSearchQuery && (
                    <p className="reader-state" style={{ textAlign: "left", padding: "0 0 8px" }}>
                      {chapterMatchCount} ocorrência(s) neste capítulo
                    </p>
                  )}

                  {bookSearchLoading && <p className="reader-state">Buscando em todos os capítulos…</p>}
                  {bookSearchError && <p className="reader-state reader-error">{bookSearchError}</p>}

                  {!bookSearchLoading && hasExecutedBookSearch && !bookSearchError && (
                    <>
                      <p className="reader-state" style={{ textAlign: "left", padding: "0 0 8px" }}>
                        {bookSearchTotalMatches} ocorrência(s) em {bookSearchResults.length} capítulo(s)
                      </p>
                      {bookSearchResults.map((result, i) => (
                        <button
                          key={`${result.chapter_index}-${i}`}
                          type="button"
                          className="reader-search-result-item"
                          onClick={() => void handleSelectBookSearchResult(result)}
                        >
                          <strong>{result.chapter_title} · {result.occurrences} ocor.</strong>
                          <small>{result.snippet}</small>
                        </button>
                      ))}
                    </>
                  )}
                </div>
              )}

            </div>
          </aside>
        )}
      </div>

      {/* ── BOTTOM CHROME ── */}
      <div className="reader-chrome-bottom">
        <button
          type="button"
          className="reader-icon-btn"
          onClick={() => void goToPosition(currentChapter - 1)}
          disabled={!canGoPrevious}
          aria-label="Capítulo anterior"
        >
          <IconBack />
        </button>

        {isImageBook && (
          <input
            className="reader-page-input"
            type="number"
            min={1}
            max={Math.max(totalChapters, 1)}
            value={pageInput}
            onChange={(e) => setPageInput(e.currentTarget.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") { e.preventDefault(); handlePageInputSubmit(); }
            }}
            onBlur={handlePageInputSubmit}
            aria-label="Página atual"
          />
        )}

        <span className="reader-progress-label">
          {isImageBook ? `/ ${totalChapters}` : `Cap. ${currentChapter + 1} / ${totalChapters || "—"}`}
        </span>

        <div className="reader-progress-track" role="progressbar" aria-valuenow={Math.round(progressPct)} aria-valuemin={0} aria-valuemax={100}>
          <div className="reader-progress-bar" style={{ width: `${progressPct}%` }} />
        </div>

        <span className="reader-progress-label">{Math.round(progressPct)}%</span>

        <button
          type="button"
          className="reader-icon-btn"
          onClick={() => void goToPosition(currentChapter + 1)}
          disabled={!canGoNext}
          aria-label="Próximo capítulo"
        >
          <IconNext />
        </button>
      </div>

      {/* ── FLOATING HIGHLIGHT ACTION ── */}
      {supportsAnnotations && floatingAction && (
        <div
          className="floating-action-menu"
          style={{ top: `${floatingAction.top}px`, left: `${floatingAction.left}px` }}
        >
          <button type="button" onClick={() => void handleCreateHighlight()}>
            Destacar
          </button>
        </div>
      )}

    </div>
  );
}
