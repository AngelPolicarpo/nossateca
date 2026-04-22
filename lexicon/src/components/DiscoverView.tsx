import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import {
  type DiscoverCatalog,
  type DiscoverCatalogItem,
  type SourceDownloadResult,
  useDiscoverCatalogItems,
  useDiscoverCatalogs,
  useDiscoverItemDetails,
  useSourceSearchDownloads,
} from "../hooks/useDiscover";
import {
  type MangaChapter,
  buildMangaChapterDownloadUrl,
  buildMangaChapterFileName,
  buildMangaSubfolder,
  formatChapterLabel,
  useMangaChapters,
} from "../hooks/useMangaSource";
import {
  buildSubjectGroups,
  resolveSubjectSlug,
} from "../data/discoverFacets";
import { BookCover } from "./ui/BookCover";
import { Button } from "./ui/Button";
import { CustomSelect, type SelectOption } from "./ui/CustomSelect";
import { EmptyState } from "./ui/EmptyState";
import { Input } from "./ui/Input";
import { StateMessage } from "./ui/StateMessage";
import { ToggleChip } from "./ui/ToggleChip";
import { cn } from "../lib/cn";
import "./DiscoverView.css";

type DiscoverViewProps = {
  onQueueDownload: (sourceUrl: string, fileName: string, subfolder?: string) => Promise<void>;
  globalSearchInput: string;
  onGlobalSearchInputChange: (value: string) => void;
};

const PAGE_SIZE = 24;

const RESULT_GRID_COLUMNS_CLASS =
  "grid gap-[var(--space-14)] [grid-template-columns:repeat(auto-fill,minmax(148px,1fr))] min-[900px]:[grid-template-columns:repeat(auto-fill,minmax(162px,1fr))]";

const CATALOG_TYPE_LABELS: Record<string, string> = {
  subject: "Por tema",
  trending: "Em destaque",
  free: "Livros Gratuitos",
  manga: "Mangás",
};

function humanizeToken(value: string): string {
  const normalized = value.replace(/[:_-]+/g, " ").replace(/\s+/g, " ").trim();
  if (normalized.length === 0) {
    return "Sem nome";
  }

  return normalized.charAt(0).toUpperCase() + normalized.slice(1);
}

function getCatalogTypeLabel(contentType: string): string {
  const normalized = contentType.trim().toLowerCase();
  return CATALOG_TYPE_LABELS[normalized] ?? humanizeToken(normalized);
}

function getErrorMessage(err: unknown, fallback: string): string {
  if (err instanceof Error && err.message.trim().length > 0) {
    return err.message;
  }

  if (typeof err === "string" && err.trim().length > 0) {
    return err;
  }

  return fallback;
}

function catalogKey(catalog: DiscoverCatalog): string {
  return `${catalog.pluginId}::${catalog.id}`;
}

function decodeCatalogKey(value: string): { pluginId: string; catalogId: string } | null {
  const [pluginId, catalogId] = value.split("::");
  if (!pluginId || !catalogId) {
    return null;
  }

  return { pluginId, catalogId };
}

function itemIdentityKey(item: Pick<DiscoverCatalogItem, "pluginId" | "catalogId" | "id">): string {
  return `${item.pluginId}::${item.catalogId}::${item.id}`;
}

function buildDownloadFileName(title: string, download: SourceDownloadResult): string {
  const safeTitle = title.trim().replace(/[\\/:*?"<>|]+/g, "_").replace(/\s+/g, " ") || "book";
  const normalizedFormat = download.format.trim().toLowerCase();

  if (normalizedFormat.length > 0 && normalizedFormat.length <= 8 && !normalizedFormat.includes("/")) {
    return `${safeTitle}.${normalizedFormat}`;
  }

  return `${safeTitle}.bin`;
}

type ParsedSourceQualityMetadata = {
  pages: string | null;
  name: string | null;
  legacy: string | null;
};

function parseSourceQualityMetadata(quality: string | null | undefined): ParsedSourceQualityMetadata {
  const raw = quality?.trim() ?? "";
  if (raw.length === 0) {
    return { pages: null, name: null, legacy: null };
  }

  const parts = raw
    .split("|")
    .map((part) => part.trim())
    .filter((part) => part.length > 0);

  let pages: string | null = null;
  let name: string | null = null;
  let recognized = false;

  for (const part of parts) {
    const separatorIndex = part.indexOf(":");
    if (separatorIndex <= 0) {
      continue;
    }

    const key = part.slice(0, separatorIndex).trim().toLowerCase();
    const value = part.slice(separatorIndex + 1).trim();
    if (value.length === 0) {
      continue;
    }

    if (key === "pages" || key === "page") {
      pages = /^\d+$/.test(value) ? `${value} pág.` : value;
      recognized = true;
      continue;
    }

    if (key === "name" || key === "title") {
      name = value;
      recognized = true;
      continue;
    }
  }

  if (!recognized) {
    return { pages: null, name: null, legacy: raw };
  }

  return { pages, name, legacy: null };
}

function getFocusableElements(container: HTMLElement): HTMLElement[] {
  const selector = [
    "a[href]",
    "button:not([disabled])",
    "textarea:not([disabled])",
    "input:not([disabled])",
    "select:not([disabled])",
    '[tabindex]:not([tabindex="-1"])',
  ].join(",");

  return Array.from(container.querySelectorAll<HTMLElement>(selector)).filter(
    (element) =>
      element.getAttribute("aria-hidden") !== "true" &&
      !element.hasAttribute("disabled") &&
      element.tabIndex >= 0,
  );
}

type DiscoverCoverProps = {
  item: DiscoverCatalogItem;
};

type FacetChipProps = {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
  className?: string;
  disabled?: boolean;
};

function FacetChip({ active, onClick, children, className, disabled }: FacetChipProps) {
  return (
    <ToggleChip
      active={active}
      disabled={disabled}
      onClick={onClick}
      className={cn(
        "inline-flex self-start min-h-[var(--control-height-compact)] items-center gap-[var(--space-4)] rounded-[var(--radius-pill)] border border-black/15 bg-[var(--color-surface-primary)] px-[var(--space-8)] py-[var(--space-4)] text-[12px] font-semibold tracking-[0.0125em] text-[var(--color-text-secondary)] transition-[border-color,background,color] duration-150 hover:border-[rgba(216,75,42,0.28)] hover:bg-[var(--color-badge-bg)] hover:text-[var(--color-primary-active)] [&.active]:border-[rgba(216,75,42,0.36)] [&.active]:bg-[var(--color-badge-bg)] [&.active]:text-[var(--color-primary-active)]",
        className,
      )}
    >
      {children}
    </ToggleChip>
  );
}

function FacetsChevronIcon({ open }: { open: boolean }) {
  return (
    <svg
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.7"
      className={cn("dc-facets-chevron", open && "dc-facets-chevron--open")}
      aria-hidden="true"
      focusable="false"
    >
      <path d="m5.75 7.5 4.25 5 4.25-5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function ClearAllIcon() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      className="dc-icon-glyph dc-icon-glyph--action"
      aria-hidden="true"
      focusable="false"
    >
      <line x1="18" y1="6" x2="6" y2="18"/>
      <line x1="6" y1="6" x2="18" y2="18"/>
    </svg>
  );
}

function PreviousPageIcon() {
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
      className="dc-icon-glyph"
      aria-hidden="true"
      focusable="false"
    >
      <path d="M19 12H5M11 5l-7 7 7 7" />
    </svg>
  );
}

function NextPageIcon() {
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
      className="dc-icon-glyph"
      aria-hidden="true"
      focusable="false"
    >
      <path d="M5 12h14M13 5l7 7-7 7" />
    </svg>
  );
}

function PanelCloseIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      className="dc-icon-glyph dc-icon-glyph--panel-close"
      aria-hidden="true"
      focusable="false"
    >
      <path d="M18 6 6 18" />
      <path d="m6 6 12 12" />
    </svg>
  );
}

function DiscoverCover({ item }: DiscoverCoverProps) {
  const [hasImageError, setHasImageError] = useState(false);
  const coverUrl = item.coverUrl?.trim() ?? "";

  if (coverUrl.length === 0 || hasImageError) {
    return (
      <BookCover
        title={item.title ?? ""}
        author={item.author ?? undefined}
        format={item.format ?? undefined}
        className="h-full w-full"
      />
    );
  }

  return (
    <img
      src={coverUrl}
      alt=""
      loading="lazy"
      className="block h-full w-full min-h-full min-w-full object-cover"
      onError={() => {
        setHasImageError(true);
      }}
    />
  );
}

export function DiscoverView({
  onQueueDownload,
  globalSearchInput,
  onGlobalSearchInputChange,
}: DiscoverViewProps) {
  const [debouncedGlobalSearchInput, setDebouncedGlobalSearchInput] = useState("");
  const [selectedCatalogKey, setSelectedCatalogKey] = useState("");
  const [catalogTypeFilter, setCatalogTypeFilter] = useState("all");
  const [facetsExpanded, setFacetsExpanded] = useState(false);
  const [selectedGenre, setSelectedGenre] = useState<string | null>(null);
  const [subjectSearchInput, setSubjectSearchInput] = useState("");
  const [yearFilterInput, setYearFilterInput] = useState("");
  const [debouncedYearFilterInput, setDebouncedYearFilterInput] = useState("");
  const [skip, setSkip] = useState(0);
  const [selectedItem, setSelectedItem] = useState<DiscoverCatalogItem | null>(null);
  const [queueingUrl, setQueueingUrl] = useState<string | null>(null);
  const mainColumnRef = useRef<HTMLDivElement | null>(null);
  const facetsPanelRef = useRef<HTMLElement | null>(null);
  const sidePanelRef = useRef<HTMLElement | null>(null);
  const closePanelButtonRef = useRef<HTMLButtonElement | null>(null);
  const previousFocusedElementRef = useRef<HTMLElement | null>(null);

  const closePanel = useCallback(() => {
    setSelectedItem(null);
  }, []);

  const catalogsQuery = useDiscoverCatalogs(true);

  const catalogs = catalogsQuery.data ?? [];

  useEffect(() => {
    if (catalogs.length === 0) {
      setSelectedCatalogKey("");
      return;
    }

    setSelectedCatalogKey((previous) => {
      if (previous.length > 0 && catalogs.some((catalog) => catalogKey(catalog) === previous)) {
        return previous;
      }

      const preferred =
        catalogs.find((catalog) => catalog.id === "openlibrary:trending:daily") ?? catalogs[0];

      return catalogKey(preferred);
    });
  }, [catalogs]);

  const selectedCatalogInfo = useMemo(
    () => decodeCatalogKey(selectedCatalogKey),
    [selectedCatalogKey],
  );

  const selectedCatalog = useMemo(() => {
    if (!selectedCatalogInfo) {
      return null;
    }

    return (
      catalogs.find(
        (catalog) =>
          catalog.pluginId === selectedCatalogInfo.pluginId &&
          catalog.id === selectedCatalogInfo.catalogId,
      ) ?? null
    );
  }, [catalogs, selectedCatalogInfo]);

  const catalogTypeOptions = useMemo(() => {
    const catalogTypeCounts = new Map<string, number>();

    for (const catalog of catalogs) {
      const normalizedType = catalog.contentType.trim().toLowerCase() || "other";
      catalogTypeCounts.set(normalizedType, (catalogTypeCounts.get(normalizedType) ?? 0) + 1);
    }

    const dynamicOptions = Array.from(catalogTypeCounts.entries())
      .filter(([type]) => type !== "all")
      .sort(([left], [right]) =>
        getCatalogTypeLabel(left).localeCompare(getCatalogTypeLabel(right), "pt-BR"),
      )
      .map(([value, count]) => ({
        value,
        count,
        label: getCatalogTypeLabel(value),
      }));

    return [{ value: "all", label: "Todos", count: catalogs.length }, ...dynamicOptions];
  }, [catalogs]);

  const filteredCatalogs = useMemo(() => {
    return catalogs.filter((catalog) => {
      const normalizedType = catalog.contentType.trim().toLowerCase();

      if (catalogTypeFilter !== "all" && normalizedType !== catalogTypeFilter) {
        return false;
      }

      return true;
    });
  }, [catalogs, catalogTypeFilter]);

  const dropdownCatalogs = useMemo(() => {
    if (!selectedCatalog) {
      return filteredCatalogs;
    }

    const selectedKey = catalogKey(selectedCatalog);
    if (filteredCatalogs.some((catalog) => catalogKey(catalog) === selectedKey)) {
      return filteredCatalogs;
    }

    return [selectedCatalog, ...filteredCatalogs];
  }, [filteredCatalogs, selectedCatalog]);

  const catalogTypeSelectOptions = useMemo<SelectOption[]>(
    () =>
      catalogTypeOptions.map((option) => ({
        value: option.value,
        label: option.value === "all" ? option.label : `${option.label} (${option.count})`,
      })),
    [catalogTypeOptions],
  );

  const catalogSelectOptions = useMemo<SelectOption[]>(
    () =>
      dropdownCatalogs.map((catalog) => ({
        value: catalogKey(catalog),
        label: catalog.name,
      })),
    [dropdownCatalogs],
  );

  useEffect(() => {
    if (catalogTypeFilter === "all") {
      return;
    }

    if (!catalogTypeOptions.some((option) => option.value === catalogTypeFilter)) {
      setCatalogTypeFilter("all");
    }
  }, [catalogTypeFilter, catalogTypeOptions]);

  useEffect(() => {
    if (!selectedCatalog) {
      setSelectedGenre(null);
      setSubjectSearchInput("");
      return;
    }

    if (selectedCatalog.contentType === "subject") {
      setSelectedGenre((previous) => {
        const resolvedPrevious = previous ? resolveSubjectSlug(previous) : null;
        if (
          resolvedPrevious &&
          selectedCatalog.genres.some((genre) => resolveSubjectSlug(genre) === resolvedPrevious)
        ) {
          return resolvedPrevious;
        }

        const firstResolved = selectedCatalog.genres
          .map((genre) => resolveSubjectSlug(genre))
          .find((value): value is string => value !== null);

        return firstResolved ?? null;
      });
    } else {
      setSelectedGenre(null);
      setSubjectSearchInput("");
    }
  }, [selectedCatalog]);

  useEffect(() => {
    const timeout = window.setTimeout(() => {
      setDebouncedGlobalSearchInput(globalSearchInput);
    }, 400);

    return () => {
      window.clearTimeout(timeout);
    };
  }, [globalSearchInput]);

  useEffect(() => {
    const timeout = window.setTimeout(() => {
      setDebouncedYearFilterInput(yearFilterInput);
    }, 400);

    return () => {
      window.clearTimeout(timeout);
    };
  }, [yearFilterInput]);

  useEffect(() => {
    if (!facetsExpanded) {
      return;
    }

    const frameId = window.requestAnimationFrame(() => {
      const firstField = facetsPanelRef.current?.querySelector<HTMLElement>(
        "#discover-year-input, #discover-subject-search, input, select, button",
      );
      firstField?.focus();
    });

    return () => {
      window.cancelAnimationFrame(frameId);
    };
  }, [facetsExpanded, selectedCatalog?.contentType]);

  useEffect(() => {
    setSkip(0);
    closePanel();
  }, [debouncedGlobalSearchInput, debouncedYearFilterInput, closePanel]);

  const yearFilter = useMemo(() => {
    const parsed = Number.parseInt(debouncedYearFilterInput.trim(), 10);
    if (Number.isNaN(parsed) || parsed < 0) {
      return null;
    }

    return parsed;
  }, [debouncedYearFilterInput]);

  const isSearching = debouncedGlobalSearchInput.trim().length > 0;
  const selectedCatalogIsManga = selectedCatalog?.contentType === "manga";

  const mangaCatalog = useMemo(
    () => catalogs.find((catalog) => catalog.contentType === "manga") ?? null,
    [catalogs],
  );

  const primaryCatalog = useMemo(() => {
    if (!isSearching) {
      return selectedCatalog;
    }

    if (selectedCatalog && !selectedCatalogIsManga) {
      return selectedCatalog;
    }

    return (
      catalogs.find((catalog) => catalog.contentType !== "manga") ?? selectedCatalog
    );
  }, [catalogs, isSearching, selectedCatalog, selectedCatalogIsManga]);

  const primaryCatalogIsManga = primaryCatalog?.contentType === "manga";

  const itemsQuery = useDiscoverCatalogItems(
    primaryCatalog?.pluginId ?? "",
    primaryCatalog?.id ?? "",
    skip,
    PAGE_SIZE,
    !isSearching && primaryCatalog?.contentType === "subject" ? selectedGenre : null,
    isSearching ? null : yearFilter,
    isSearching ? debouncedGlobalSearchInput : null,
    Boolean(primaryCatalog),
  );

  const items = itemsQuery.data?.items ?? [];

  const showMangaSection =
    isSearching && mangaCatalog !== null && !primaryCatalogIsManga;

  const mangaItemsQuery = useDiscoverCatalogItems(
    mangaCatalog?.pluginId ?? "",
    mangaCatalog?.id ?? "",
    0,
    PAGE_SIZE,
    null,
    null,
    debouncedGlobalSearchInput,
    showMangaSection,
  );

  const mangaItems = mangaItemsQuery.data?.items ?? [];

  const availableSubjectSlugs = useMemo(
    () =>
      selectedCatalog?.contentType === "subject"
        ? selectedCatalog.genres
            .map((genre) => resolveSubjectSlug(genre))
            .filter((value): value is string => value !== null)
        : [],
    [selectedCatalog],
  );

  const subjectGroups = useMemo(
    () => buildSubjectGroups(availableSubjectSlugs, subjectSearchInput),
    [availableSubjectSlugs, subjectSearchInput],
  );

  const detailsQuery = useDiscoverItemDetails(
    selectedItem?.pluginId ?? "",
    selectedItem?.id ?? "",
    selectedItem !== null,
  );

  const detailsTitle = detailsQuery.data?.title ?? selectedItem?.title ?? "";
  const detailsAuthor = detailsQuery.data?.author ?? selectedItem?.author ?? "";
  const detailsIsbn = detailsQuery.data?.isbn ?? selectedItem?.isbn ?? null;
  const detailsYear = detailsQuery.data?.year ?? selectedItem?.year ?? null;
  const detailsPageCount = detailsQuery.data?.pageCount ?? selectedItem?.pageCount ?? null;

  const sourceTitle = detailsTitle;
  const sourceAuthor = detailsAuthor.trim().length > 0 ? detailsAuthor : null;
  const sourceIsbn = detailsIsbn;
  const isMangaItem = (selectedItem?.format ?? "").trim().toLowerCase() === "manga";
  const sourceQueryEnabled =
    selectedItem !== null && !isMangaItem && sourceTitle.trim().length > 0;

  const sourceQuery = useSourceSearchDownloads(
    sourceTitle,
    sourceAuthor,
    sourceIsbn,
    sourceQueryEnabled,
  );

  const mangaChaptersQuery = useMangaChapters(
    selectedItem?.id ?? "",
    selectedItem !== null && isMangaItem,
  );

  const handleCatalogChange = (nextCatalogKey: string) => {
    if (nextCatalogKey === selectedCatalogKey) {
      return;
    }

    setSelectedCatalogKey(nextCatalogKey);
    setSubjectSearchInput("");
    setSkip(0);
    closePanel();
  };

  const clearAllFilters = () => {
    onGlobalSearchInputChange("");
    setDebouncedGlobalSearchInput("");
    setCatalogTypeFilter("all");
    setSelectedGenre(null);
    setYearFilterInput("");
    setDebouncedYearFilterInput("");
    setSubjectSearchInput("");
    setSkip(0);
    closePanel();
  };

  const handleSubjectChange = (genre: string) => {
    const canonicalGenre = resolveSubjectSlug(genre);
    if (!canonicalGenre) {
      return;
    }

    setSelectedGenre(canonicalGenre);
    setSkip(0);
    closePanel();
  };

  const handleQueueDownload = async (download: SourceDownloadResult) => {
    if (!selectedItem) {
      return;
    }

    setQueueingUrl(download.downloadUrl);

    try {
      await onQueueDownload(
        download.downloadUrl,
        buildDownloadFileName(detailsQuery.data?.title ?? selectedItem.title, download),
      );
    } catch (err) {
      alert(getErrorMessage(err, "Falha ao enfileirar download"));
    } finally {
      setQueueingUrl(null);
    }
  };

  const handleQueueMangaChapter = async (pluginId: string, chapter: MangaChapter) => {
    if (!selectedItem) {
      return;
    }

    const sourceUrl = buildMangaChapterDownloadUrl(pluginId, chapter.id);
    setQueueingUrl(sourceUrl);

    try {
      const mangaTitle = detailsQuery.data?.title ?? selectedItem.title;
      await onQueueDownload(
        sourceUrl,
        buildMangaChapterFileName(mangaTitle, chapter),
        buildMangaSubfolder(mangaTitle),
      );
    } catch (err) {
      alert(getErrorMessage(err, "Falha ao enfileirar capítulo"));
    } finally {
      setQueueingUrl(null);
    }
  };

  const hasPrevious = skip > 0;
  const hasNext = itemsQuery.data?.hasMore ?? false;
  const currentPage = Math.floor(skip / PAGE_SIZE) + 1;
  const hasCatalogExplorerFilters = catalogTypeFilter !== "all";
  const panelOpen = selectedItem !== null;
  const selectedItemKey = useMemo(
    () => (selectedItem ? itemIdentityKey(selectedItem) : null),
    [selectedItem],
  );
  const activeFacetSummary = useMemo(() => {
    const chunks: string[] = [];

    if (yearFilter !== null) {
      chunks.push(`Ano: ${yearFilter}`);
    }

    return chunks.join(" • ");
  }, [yearFilter]);
  const hasRemoteFiltersApplied =
    yearFilter !== null ||
    (selectedCatalog?.contentType === "subject" && selectedGenre !== null);
  const hasAnyFilterApplied =
    globalSearchInput.trim().length > 0 ||
    hasCatalogExplorerFilters ||
    hasRemoteFiltersApplied;
  const itemTotalCount = items.length;

  useEffect(() => {
    const mainColumn = mainColumnRef.current;
    if (!mainColumn) {
      return;
    }

    if (panelOpen) {
      mainColumn.setAttribute("inert", "");
      return;
    }

    mainColumn.removeAttribute("inert");
  }, [panelOpen]);

  useEffect(() => {
    if (!panelOpen) {
      return;
    }

    const { body, documentElement } = document;
    const previousBodyOverflow = body.style.overflow;
    const previousHtmlOverflow = documentElement.style.overflow;

    body.style.overflow = "hidden";
    documentElement.style.overflow = "hidden";

    return () => {
      body.style.overflow = previousBodyOverflow;
      documentElement.style.overflow = previousHtmlOverflow;
    };
  }, [panelOpen]);

  useEffect(() => {
    if (!panelOpen) {
      if (previousFocusedElementRef.current && document.contains(previousFocusedElementRef.current)) {
        previousFocusedElementRef.current.focus();
      } else {
        mainColumnRef.current?.querySelector<HTMLElement>("[data-discover-card='true']")?.focus();
      }
      return;
    }

    const activeElement = document.activeElement;
    previousFocusedElementRef.current = activeElement instanceof HTMLElement ? activeElement : null;
    const frameId = window.requestAnimationFrame(() => closePanelButtonRef.current?.focus());

    return () => {
      window.cancelAnimationFrame(frameId);
    };
  }, [panelOpen]);

  useEffect(() => {
    setQueueingUrl(null);
  }, [selectedItem?.pluginId, selectedItem?.catalogId, selectedItem?.id]);

  const renderDetailsPanel = () => {
    if (!selectedItem) {
      return null;
    }

    return (
      <div className="flex h-full flex-col overflow-hidden bg-[var(--color-surface-primary)]">
        <div className="dc-panel-head">
          <span className="dc-panel-eyebrow">Livro selecionado</span>
          <button
            ref={closePanelButtonRef}
            type="button"
            className="dc-panel-close"
            aria-label="Fechar painel"
            title="Fechar"
            onClick={closePanel}
          >
            <PanelCloseIcon />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-[var(--space-16)]">
          <div className="grid gap-[var(--space-16)]">
            <div className="dc-panel-book-header">
              <div className="dc-panel-cover">
                <DiscoverCover item={selectedItem} />
              </div>
              <div className="dc-panel-book-info">
                <h2 id="discover-panel-title" className="dc-panel-title">
                  {detailsTitle}
                </h2>
                <p className="dc-panel-author">{detailsAuthor}</p>
                {(detailsYear !== null || detailsPageCount !== null || detailsIsbn) && (
                  <div className="flex flex-wrap gap-[var(--space-6)] mt-[var(--space-6)]">
                    {detailsYear !== null && (
                      <span className="rounded-[var(--radius-pill)] bg-[var(--color-surface-alt)] px-[var(--space-8)] py-[var(--space-2)] text-[12px] font-medium text-[var(--color-text-secondary)]">
                        {detailsYear}
                      </span>
                    )}
                    {detailsPageCount !== null && (
                      <span className="rounded-[var(--radius-pill)] bg-[var(--color-surface-alt)] px-[var(--space-8)] py-[var(--space-2)] text-[12px] font-medium text-[var(--color-text-secondary)]">
                        {detailsPageCount} pág.
                      </span>
                    )}
                  </div>
                )}
              </div>
            </div>

            {detailsQuery.isLoading && (
              <>
                <StateMessage>Carregando detalhes do item...</StateMessage>
                <div className="grid gap-[var(--space-8)]" aria-hidden>
                  <span className="h-[12px] w-[36%] animate-pulse rounded-[var(--radius-pill)] bg-[var(--color-surface-alt)]" />
                  <span className="h-[12px] animate-pulse rounded-[var(--radius-pill)] bg-[var(--color-surface-alt)]" />
                  <span className="h-[12px] w-[64%] animate-pulse rounded-[var(--radius-pill)] bg-[var(--color-surface-alt)]" />
                </div>
              </>
            )}

            {detailsQuery.isError && (
              <StateMessage tone="error">
                {getErrorMessage(detailsQuery.error, "Falha ao carregar detalhes do item")}
              </StateMessage>
            )}

            {!detailsQuery.isLoading && !detailsQuery.isError && detailsQuery.data?.description && (
              <p className="m-0 text-[14px] leading-[1.58] text-[var(--color-text-secondary)]">
                {detailsQuery.data.description}
              </p>
            )}

            {!detailsQuery.isLoading && !detailsQuery.isError && (
              <section className="grid gap-[var(--space-8)] rounded-[var(--radius-8)] border border-black/10 bg-[var(--color-surface-alt)] p-[var(--space-12)]">
                <h3 className="m-0 text-[12px] font-semibold uppercase tracking-[0.08em] text-[var(--color-text-muted)]">
                  Informações
                </h3>
                <div className="flex flex-wrap gap-x-[var(--space-16)] gap-y-[var(--space-11)]">
                  {detailsPageCount !== null && (
                    <div className="grid gap-[2px]">
                      <span className="text-[12px] font-semibold uppercase tracking-[0.08em] text-[var(--color-text-muted)]">Páginas</span>
                      <span className="text-[12px] text-[var(--color-text-secondary)]">{detailsPageCount}</span>
                    </div>
                  )}
                  {detailsYear !== null && (
                    <div className="grid gap-[2px]">
                      <span className="text-[12px] font-semibold uppercase tracking-[0.08em] text-[var(--color-text-muted)]">Ano</span>
                      <span className="text-[12px] text-[var(--color-text-secondary)]">{detailsYear}</span>
                    </div>
                  )}
                  {detailsIsbn && (
                    <div className="grid gap-[2px]">
                      <span className="text-[12px] font-semibold uppercase tracking-[0.08em] text-[var(--color-text-muted)]">ISBN</span>
                      <span className="text-[12px] text-[var(--color-text-secondary)]">{detailsIsbn}</span>
                    </div>
                  )}
                </div>
              </section>
            )}

            {isMangaItem && renderMangaChaptersSection()}

            {!isMangaItem && (
            <section className="grid gap-[var(--space-11)]">
              <header className="grid gap-[var(--space-4)] border-b border-black/10 pb-[var(--space-8)]">
                <h3 className="m-0 text-[1.1rem] font-semibold leading-[1.25] tracking-[-0.2px] text-[var(--color-text-primary)]">
                  Onde encontrar
                </h3>
                <p className="m-0 text-[12px] text-[var(--color-text-secondary)]">
                  Buscado por título, autor e ISBN.
                </p>
              </header>

              {!sourceQueryEnabled && (
                <StateMessage>Aguardando dados do livro para iniciar a busca...</StateMessage>
              )}

              {sourceQueryEnabled && sourceQuery.isLoading && (
                <>
                  <StateMessage>Buscando opções disponíveis...</StateMessage>
                  <div className="grid gap-[var(--space-8)]" aria-hidden>
                    {Array.from({ length: 3 }).map((_, index) => (
                      <div
                        key={`discover-source-skeleton-${index}`}
                        className="h-[84px] animate-pulse rounded-[var(--radius-8)] border border-black/10 bg-[var(--color-surface-alt)]"
                      />
                    ))}
                  </div>
                </>
              )}

              {sourceQueryEnabled && sourceQuery.isError && (
                <StateMessage tone="error">
                  {getErrorMessage(sourceQuery.error, "Falha ao consultar fontes")}
                </StateMessage>
              )}

              {sourceQueryEnabled &&
                !sourceQuery.isLoading &&
                !sourceQuery.isError &&
                (sourceQuery.data?.length ?? 0) === 0 && (
                  <StateMessage>Nenhuma fonte disponível no momento.</StateMessage>
                )}

              {sourceQueryEnabled &&
                !sourceQuery.isLoading &&
                !sourceQuery.isError &&
                (sourceQuery.data?.length ?? 0) > 0 && (
                  <div className="grid gap-[var(--space-11)]">
                    {sourceQuery.data?.map((group) => {
                      return (
                        <section
                          key={`${group.pluginId}-${group.sourceId}`}
                          className="overflow-hidden rounded-[var(--radius-8)] border border-black/10 bg-[var(--color-surface-primary)] shadow-[var(--shadow-card)]"
                        >
                          <header className="flex items-center justify-between gap-[var(--space-8)] border-b border-black/10 bg-[var(--color-surface-alt)] px-[var(--space-12)] py-[var(--space-8)]">
                            <div className="min-w-0">
                              <h4 className="m-0 text-[12px] font-semibold text-[var(--color-text-secondary)]">
                                {group.sourceName}
                              </h4>
                            </div>

                            {!group.error && (
                              <span className="text-[12px] text-[var(--color-text-muted)]">
                                {group.results.length} opção{group.results.length > 1 ? "ões" : ""}
                              </span>
                            )}
                          </header>

                          {group.error && (
                            <div className="px-[var(--space-12)] py-[var(--space-11)]">
                              <StateMessage tone="error">
                                [{group.error.kind}] {group.error.message}
                              </StateMessage>
                            </div>
                          )}

                          {!group.error && group.results.length === 0 && (
                            <div className="px-[var(--space-12)] py-[var(--space-11)]">
                              <StateMessage>Sem resultados nesta fonte.</StateMessage>
                            </div>
                          )}

                          {!group.error && group.results.length > 0 && (
                            <ul className="m-0 grid list-none p-0">
                              {group.results.map((download) => {
                                const formatKey = download.format.trim().toLowerCase();
                                const qualityMetadata = parseSourceQualityMetadata(download.quality);
                                const downloadSummary = [
                                  qualityMetadata.pages,
                                  download.size,
                                  qualityMetadata.name,
                                  qualityMetadata.legacy,
                                ]
                                  .filter((value): value is string => value !== null && value.trim().length > 0)
                                  .join(" · ");
                                return (
                                  <li
                                    key={`${group.pluginId}-${download.downloadUrl}`}
                                    className="flex items-center gap-[var(--space-11)] border-b border-black/10 px-[var(--space-12)] py-[var(--space-11)] last:border-b-0"
                                  >
                                    <span
                                      className={cn(
                                        "inline-flex min-w-[48px] justify-center rounded-[var(--radius-5)] px-[var(--space-8)] py-[var(--space-2)] text-[12px] font-semibold uppercase tracking-[0.06em] text-white",
                                        formatKey === "epub" && "bg-[var(--color-semantic-green)]",
                                        formatKey === "pdf" && "bg-[var(--color-danger)]",
                                        formatKey === "mobi" && "bg-[var(--color-brand-navy)]",
                                        formatKey === "azw3" && "bg-[var(--color-semantic-purple)]",
                                        !["epub", "pdf", "mobi", "azw3"].includes(formatKey) && "bg-[var(--color-text-secondary)]",
                                      )}
                                    >
                                      {download.format.toUpperCase()}
                                    </span>

                                    <div className="min-w-0 flex-1">
                                      <p className="m-0 text-[14px] leading-[1.43] text-[var(--color-text-secondary)]">
                                        {download.language ? `${download.language}` : "Não informado"}
                                      </p>
                                      {downloadSummary.length > 0 && (
                                        <p className="m-0 text-[12px] text-[var(--color-text-muted)]">
                                          {downloadSummary}
                                        </p>
                                      )}
                                    </div>

                                    <Button
                                      variant="primary"
                                      size="sm"
                                      disabled={queueingUrl !== null}
                                      onClick={() => void handleQueueDownload(download)}
                                    >
                                      {queueingUrl === download.downloadUrl ? "Enfileirando..." : "Baixar"}
                                    </Button>
                                  </li>
                                );
                              })}
                            </ul>
                          )}
                        </section>
                      );
                    })}
                  </div>
                )}
            </section>
            )}
          </div>
        </div>
      </div>
    );
  };

  const renderMangaChaptersSection = () => {
    if (!selectedItem) {
      return null;
    }

    const groups = mangaChaptersQuery.data ?? [];

    return (
      <section className="grid gap-[var(--space-11)]">
        <header className="grid gap-[var(--space-4)] border-b border-black/10 pb-[var(--space-8)]">
          <h3 className="m-0 text-[1.1rem] font-semibold leading-[1.25] tracking-[-0.2px] text-[var(--color-text-primary)]">
            Capítulos
          </h3>
          <p className="m-0 text-[12px] text-[var(--color-text-secondary)]">
            Baixados como CBZ e agrupados por pasta do mangá.
          </p>
        </header>

        {mangaChaptersQuery.isLoading && (
          <>
            <StateMessage>Carregando capítulos...</StateMessage>
            <div className="grid gap-[var(--space-8)]" aria-hidden>
              {Array.from({ length: 4 }).map((_, index) => (
                <div
                  key={`manga-chapter-skeleton-${index}`}
                  className="h-[60px] animate-pulse rounded-[var(--radius-8)] border border-black/10 bg-[var(--color-surface-alt)]"
                />
              ))}
            </div>
          </>
        )}

        {mangaChaptersQuery.isError && (
          <StateMessage tone="error">
            {getErrorMessage(mangaChaptersQuery.error, "Falha ao carregar capítulos")}
          </StateMessage>
        )}

        {!mangaChaptersQuery.isLoading && !mangaChaptersQuery.isError && groups.length === 0 && (
          <StateMessage>Nenhuma fonte de mangá habilitada.</StateMessage>
        )}

        {!mangaChaptersQuery.isLoading &&
          !mangaChaptersQuery.isError &&
          groups.length > 0 &&
          groups.map((group) => (
            <section
              key={`${group.pluginId}-${group.sourceId}`}
              className="overflow-hidden rounded-[var(--radius-8)] border border-black/10 bg-[var(--color-surface-primary)] shadow-[var(--shadow-card)]"
            >
              <header className="flex items-center justify-between gap-[var(--space-8)] border-b border-black/10 bg-[var(--color-surface-alt)] px-[var(--space-12)] py-[var(--space-8)]">
                <div className="min-w-0">
                  <h4 className="m-0 text-[12px] font-semibold text-[var(--color-text-secondary)]">
                    {group.sourceName}
                  </h4>
                </div>

                {!group.error && (
                  <span className="text-[12px] text-[var(--color-text-muted)]">
                    {group.chapters.length} {group.chapters.length === 1 ? "capítulo" : "capítulos"}
                  </span>
                )}
              </header>

              {group.error && (
                <div className="px-[var(--space-12)] py-[var(--space-11)]">
                  <StateMessage tone="error">
                    [{group.error.kind}] {group.error.message}
                  </StateMessage>
                </div>
              )}

              {!group.error && group.chapters.length === 0 && (
                <div className="px-[var(--space-12)] py-[var(--space-11)]">
                  <StateMessage>Sem capítulos disponíveis nesta fonte.</StateMessage>
                </div>
              )}

              {!group.error && group.chapters.length > 0 && (
                <ul className="m-0 grid max-h-[360px] list-none overflow-y-auto p-0">
                  {group.chapters.map((chapter) => {
                    const sourceUrl = buildMangaChapterDownloadUrl(group.pluginId, chapter.id);
                    const label = formatChapterLabel(chapter);
                    const hasPages = (chapter.pages ?? 0) > 0;
                    const metaParts: string[] = [];
                    if (chapter.title && chapter.title.trim().length > 0) {
                      metaParts.push(chapter.title.trim());
                    }
                    if (chapter.language && chapter.language.trim().length > 0) {
                      metaParts.push(chapter.language.trim().toUpperCase());
                    }
                    if (hasPages) {
                      metaParts.push(`${chapter.pages} pág.`);
                    }
                    if (chapter.scanlator && chapter.scanlator.trim().length > 0) {
                      metaParts.push(chapter.scanlator.trim());
                    }
                    if (!hasPages) {
                      metaParts.push("indisponível");
                    }
                    const meta = metaParts.join(" · ");
                    return (
                      <li
                        key={`${group.pluginId}-${chapter.id}`}
                        className="flex items-center gap-[var(--space-11)] border-b border-black/10 px-[var(--space-12)] py-[var(--space-11)] last:border-b-0"
                      >
                        <div className="min-w-0 flex-1">
                          <p className="m-0 text-[14px] font-semibold leading-[1.3] text-[var(--color-text-primary)]">
                            {label}
                          </p>
                          {meta.length > 0 && (
                            <p className="m-0 text-[12px] text-[var(--color-text-muted)]">{meta}</p>
                          )}
                        </div>

                        <Button
                          variant="primary"
                          size="sm"
                          disabled={queueingUrl !== null || !hasPages}
                          onClick={() => void handleQueueMangaChapter(group.pluginId, chapter)}
                        >
                          {queueingUrl === sourceUrl ? "Enfileirando..." : "Baixar"}
                        </Button>
                      </li>
                    );
                  })}
                </ul>
              )}
            </section>
          ))}
      </section>
    );
  };

  return (
    <div className="dc-wrap">
      <div ref={mainColumnRef} className="dc-main-column">
        <header className="">
          <h1 className="lx-page-title">Descubra</h1>
          <p className="lx-page-subtitle">
            Explore coleções, refine os filtros e encontre sua próxima leitura.
          </p>
        </header>

        {!isSearching && (
        <header className="dc-filters-strip">
          <div className="dc-filters-primary" role="group" aria-label="Filtros principais">
            <label htmlFor="discover-catalog-type" className="dc-filter-field">
              <CustomSelect
                id="discover-catalog-type"
                triggerClassName="dc-filter-select"
                menuClassName="dc-filter-menu"
                optionClassName="dc-filter-option"
                value={catalogTypeFilter}
                options={catalogTypeSelectOptions}
                onValueChange={(nextValue) => {
                  setCatalogTypeFilter(nextValue);
                  setSkip(0);
                  closePanel();
                }}
                disabled={catalogsQuery.isLoading || catalogs.length === 0}
              />
            </label>

            <label htmlFor="discover-catalog-select" className="dc-filter-field dc-filter-field--grow">
              <CustomSelect
                id="discover-catalog-select"
                triggerClassName="dc-filter-select"
                menuClassName="dc-filter-menu"
                optionClassName="dc-filter-option"
                value={selectedCatalogKey}
                options={catalogSelectOptions}
                onValueChange={handleCatalogChange}
                disabled={catalogsQuery.isLoading || catalogs.length === 0 || dropdownCatalogs.length === 0}
              />
            </label>

          </div>

          {!catalogsQuery.isLoading && catalogs.length > 0 && dropdownCatalogs.length === 0 && (
            <StateMessage>Nenhuma coleção encontrada para o filtro atual.</StateMessage>
          )}

          <section ref={facetsPanelRef} className="dc-filters-secondary" aria-label="Refinar resultados">
            <button
              type="button"
              className={cn(
                "dc-facets-trigger",
                facetsExpanded && "dc-facets-trigger--expanded",
              )}
              aria-expanded={facetsExpanded}
              aria-controls="discover-facets-body"
              onClick={() => setFacetsExpanded((previous) => !previous)}
            >
              <span className="dc-facets-label text-[14px] font-medium text-[var(--color-text-primary)]">
                Filtros avançados
              </span>
              {activeFacetSummary.length > 0 && (
                <span className="dc-facets-summary" title={activeFacetSummary}>
                  {activeFacetSummary}
                </span>
              )}
              <FacetsChevronIcon open={facetsExpanded} />
            </button>

            {facetsExpanded && (
              <div id="discover-facets-body" className="dc-facets-body">
                <div className="dc-facets-grid">
                  <div className="dc-facet-field">
                    <Input
                      id="discover-year-input"
                      type="text"
                      inputMode="numeric"
                      pattern="[0-9]{4}"
                      maxLength={4}
                      placeholder="Buscar por ano"
                      value={yearFilterInput}
                      onChange={(event) => {
                        setYearFilterInput(event.target.value);
                        closePanel();
                      }}
                      className="dc-facet-input"
                    />
                  </div>

                  {selectedCatalog?.contentType === "subject" && (
                    <div className="dc-facet-field">
                      <Input
                        id="discover-subject-search"
                        type="search"
                        placeholder="Buscar por gênero"
                        value={subjectSearchInput}
                        onChange={(event) => setSubjectSearchInput(event.target.value)}
                        className="dc-facet-input"
                      />
                    </div>
                  )}

                  <div className="dc-facet-field dc-facet-field--clear">
                    <Button
                      variant="secondary"
                      size="sm"
                      className="dc-facets-clear-btn"
                      aria-label="Limpar tudo"
                      title="Limpar tudo"
                      onClick={clearAllFilters}
                      disabled={!hasAnyFilterApplied}
                    >
                      <ClearAllIcon />
                    </Button>
                  </div>
                </div>

                {selectedCatalog?.contentType === "subject" && (
                  <div className="dc-subject-groups" aria-label="Assuntos organizados por grupo">
                    {subjectGroups.length === 0 && (
                      <StateMessage>Nenhum assunto encontrado para esta busca.</StateMessage>
                    )}

                    {subjectGroups.map((group) => (
                      <section key={group.id} className="dc-subject-group">
                        <header>
                          <h4 className="dc-subject-group-title">{group.labelPt}</h4>
                        </header>
                        <div
                          className="dc-subject-group-options"
                          aria-label={`Assuntos do grupo ${group.labelPt}`}
                        >
                          {group.options.map((subject) => (
                            <FacetChip
                              key={subject.slug}
                              active={selectedGenre === subject.slug}
                              onClick={() => handleSubjectChange(subject.slug)}
                              className="shrink-0"
                            >
                              {subject.label_pt}
                            </FacetChip>
                          ))}
                        </div>
                      </section>
                    ))}
                  </div>
                )}
              </div>
            )}
          </section>
        </header>
        )}

        <section className="dc-section">
          <header className="dc-section-head">
            <div>
              <h2 className="dc-section-title">
                {isSearching
                  ? primaryCatalogIsManga
                    ? "Mangás"
                    : "Livros"
                  : selectedCatalog?.name ?? "Livros"}
              </h2>
              {primaryCatalog && (
                <div className="dc-section-subtitle">
                  {itemTotalCount} {itemTotalCount === 1 ? "item" : "itens"} · página {currentPage}
                </div>
              )}
            </div>

            {!itemsQuery.isError && itemTotalCount > 0 && (
              <div className="flex items-center gap-[var(--space-6)]" aria-label="Navegação de páginas">
                <Button
                  variant="secondary"
                  size="sm"
                  className="dc-page-nav-btn"
                  aria-label="Página anterior"
                  title="Página anterior"
                  disabled={!hasPrevious}
                  onClick={() => setSkip((previous) => Math.max(0, previous - PAGE_SIZE))}
                >
                  <PreviousPageIcon />
                </Button>

                <Button
                  variant="secondary"
                  size="sm"
                  className="dc-page-nav-btn"
                  aria-label="Próxima página"
                  title="Próxima página"
                  disabled={!hasNext}
                  onClick={() => setSkip((previous) => previous + PAGE_SIZE)}
                >
                  <NextPageIcon />
                </Button>
              </div>
            )}
          </header>

          {catalogsQuery.isLoading && <StateMessage>Preparando coleções...</StateMessage>}

          {catalogsQuery.isError && (
            <StateMessage tone="error">
              {getErrorMessage(catalogsQuery.error, "Falha ao carregar coleções")}
            </StateMessage>
          )}

          {!catalogsQuery.isLoading && !catalogsQuery.isError && catalogs.length === 0 && (
            <EmptyState
              compact
              titleAs="h3"
              title="Nenhuma coleção disponível"
              description="Ative novas integrações na aba Addons para ampliar as opções de descoberta."
            />
          )}

          {primaryCatalog && (
            <>
              {itemsQuery.isFetching && !itemsQuery.isLoading && (
                <StateMessage>Atualizando sugestões...</StateMessage>
              )}

              <div
                aria-live="polite"
              >
                {itemsQuery.isLoading && (
                  <div className={RESULT_GRID_COLUMNS_CLASS} aria-hidden>
                    {Array.from({ length: 12 }).map((_, index) => (
                      <div
                        key={index}
                        className="h-[282px] animate-pulse rounded-[var(--radius-12)] border border-black/10 bg-[var(--color-surface-alt)]"
                      />
                    ))}
                  </div>
                )}

                {itemsQuery.isError && (
                  <StateMessage tone="error">
                    {getErrorMessage(itemsQuery.error, "Falha ao carregar livros")}
                  </StateMessage>
                )}

                {!itemsQuery.isLoading && !itemsQuery.isError && items.length === 0 && (
                  <EmptyState
                    compact
                    titleAs="h3"
                    title={
                      debouncedGlobalSearchInput.trim().length > 0
                        ? "Nenhum livro encontrado para esta busca"
                        : "Nenhum item encontrado"
                    }
                    description={
                      debouncedGlobalSearchInput.trim().length > 0
                        ? "Tente ajustar os termos da busca ou remover filtros adicionais."
                        : "Tente ajustar filtros ou selecionar outra coleção."
                    }
                  />
                )}

                {!itemsQuery.isError && items.length > 0 && (
                  <div className={RESULT_GRID_COLUMNS_CLASS} data-loading={itemsQuery.isFetching ? "true" : undefined}>
                    {items.map((item) => (
                      <button
                        type="button"
                        key={itemIdentityKey(item)}
                        data-discover-card="true"
                        className={cn(
                          "group flex w-full flex-col gap-[var(--space-8)] border-0 bg-transparent p-0 text-left text-inherit focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--color-focus)]",
                        )}
                        aria-label={`Abrir opções para ${item.title}`}
                        onClick={() => {
                          setQueueingUrl(null);
                          setSelectedItem(item);
                        }}
                      >
                        <div
                          className={cn(
                            "aspect-[3/5] w-full overflow-hidden rounded-[var(--radius-12)] border border-black/10 bg-transparent shadow-[var(--shadow-card)] transition-[border-color,box-shadow] duration-150 group-hover:shadow-[var(--shadow-soft)]",
                            selectedItemKey === itemIdentityKey(item) && "border-[rgba(9,127,232,0.38)] shadow-[0_0_0_2px_rgba(9,127,232,0.14)]",
                          )}
                          aria-hidden
                        >
                          <DiscoverCover item={item} />
                        </div>

                        <div className="grid gap-[var(--space-2)] px-[var(--space-2)] pb-[var(--space-2)]">
                          <h3
                            className="m-0 overflow-hidden [display:-webkit-box] [-webkit-box-orient:vertical] [-webkit-line-clamp:2] text-[15px] font-[var(--type-card-title-weight)] leading-[1.32] text-[var(--color-text-primary)]"
                            title={item.title}
                          >
                            {item.title}
                          </h3>

                          <p
                            className="m-0 overflow-hidden text-ellipsis whitespace-nowrap text-[13px] leading-[1.4] text-[var(--color-text-secondary)]"
                            title={item.author}
                          >
                            {item.author}
                          </p>

                          {item.year !== null && (
                            <p className="m-0 text-[12px] leading-[1.35] text-[var(--color-text-muted)]">
                              {item.year}
                            </p>
                          )}
                        </div>
                      </button>
                    ))}
                  </div>
                )}
              </div>
            </>
          )}
        </section>

        {showMangaSection && (
          <section className="dc-section">
            <header className="dc-section-head">
              <div>
                <h2 className="dc-section-title">Mangás</h2>
                <div className="dc-section-subtitle">
                  {mangaItems.length} {mangaItems.length === 1 ? "item" : "itens"}
                </div>
              </div>
            </header>

            <div aria-live="polite">
              {mangaItemsQuery.isLoading && (
                <div className={RESULT_GRID_COLUMNS_CLASS} aria-hidden>
                  {Array.from({ length: 6 }).map((_, index) => (
                    <div
                      key={`manga-skeleton-${index}`}
                      className="h-[282px] animate-pulse rounded-[var(--radius-12)] border border-black/10 bg-[var(--color-surface-alt)]"
                    />
                  ))}
                </div>
              )}

              {mangaItemsQuery.isError && (
                <StateMessage tone="error">
                  {getErrorMessage(mangaItemsQuery.error, "Falha ao carregar mangás")}
                </StateMessage>
              )}

              {!mangaItemsQuery.isLoading && !mangaItemsQuery.isError && mangaItems.length === 0 && (
                <EmptyState
                  compact
                  titleAs="h3"
                  title="Nenhum mangá encontrado para esta busca"
                  description="Tente ajustar os termos da busca."
                />
              )}

              {!mangaItemsQuery.isError && mangaItems.length > 0 && (
                <div
                  className={RESULT_GRID_COLUMNS_CLASS}
                  data-loading={mangaItemsQuery.isFetching ? "true" : undefined}
                >
                  {mangaItems.map((item) => (
                    <button
                      type="button"
                      key={itemIdentityKey(item)}
                      data-discover-card="true"
                      className={cn(
                        "group flex w-full flex-col gap-[var(--space-8)] border-0 bg-transparent p-0 text-left text-inherit focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--color-focus)]",
                      )}
                      aria-label={`Abrir opções para ${item.title}`}
                      onClick={() => {
                        setQueueingUrl(null);
                        setSelectedItem(item);
                      }}
                    >
                      <div
                        className={cn(
                          "aspect-[3/5] w-full overflow-hidden rounded-[var(--radius-12)] border border-black/10 bg-transparent shadow-[var(--shadow-card)] transition-[border-color,box-shadow] duration-150 group-hover:shadow-[var(--shadow-soft)]",
                          selectedItemKey === itemIdentityKey(item) && "border-[rgba(9,127,232,0.38)] shadow-[0_0_0_2px_rgba(9,127,232,0.14)]",
                        )}
                        aria-hidden
                      >
                        <DiscoverCover item={item} />
                      </div>

                      <div className="grid gap-[var(--space-2)] px-[var(--space-2)] pb-[var(--space-2)]">
                        <h3
                          className="m-0 overflow-hidden [display:-webkit-box] [-webkit-box-orient:vertical] [-webkit-line-clamp:2] text-[15px] font-[var(--type-card-title-weight)] leading-[1.32] text-[var(--color-text-primary)]"
                          title={item.title}
                        >
                          {item.title}
                        </h3>

                        <p
                          className="m-0 overflow-hidden text-ellipsis whitespace-nowrap text-[13px] leading-[1.4] text-[var(--color-text-secondary)]"
                          title={item.author}
                        >
                          {item.author}
                        </p>

                        {item.year !== null && (
                          <p className="m-0 text-[12px] leading-[1.35] text-[var(--color-text-muted)]">
                            {item.year}
                          </p>
                        )}
                      </div>
                    </button>
                  ))}
                </div>
              )}
            </div>
          </section>
        )}
      </div>

      {panelOpen && selectedItem && (
        <>
          <button
            type="button"
            className="fixed inset-0 z-[var(--z-overlay-backdrop)] border-0 bg-[rgba(0,0,0,0.22)] backdrop-blur-sm motion-safe:animate-[discoverBackdropIn_180ms_ease-out]"
            aria-label="Fechar painel de fontes"
            tabIndex={-1}
            onClick={closePanel}
          />

          <aside
            ref={sidePanelRef}
            className="fixed inset-y-0 right-0 z-[var(--z-overlay-panel)] w-[480px] max-w-[calc(100vw-var(--space-16))] overflow-hidden rounded-none border-l border-black/10 bg-[var(--color-surface-primary)] shadow-[var(--shadow-deep)] motion-safe:animate-[discoverDrawerIn_240ms_cubic-bezier(0.34,1.04,0.64,1)] max-[680px]:w-screen max-[680px]:max-w-none"
            role="dialog"
            aria-modal="true"
            aria-labelledby="discover-panel-title"
            onKeyDown={(event) => {
              if (event.key === "Escape") {
                closePanel();
                return;
              }

              if (event.key !== "Tab") {
                return;
              }

              const panel = sidePanelRef.current;
              if (!panel) {
                return;
              }

              const focusables = getFocusableElements(panel);
              if (focusables.length === 0) {
                event.preventDefault();
                closePanelButtonRef.current?.focus();
                return;
              }

              const first = focusables[0];
              const last = focusables[focusables.length - 1];
              const activeElement = document.activeElement as HTMLElement | null;

              if (event.shiftKey) {
                if (!activeElement || activeElement === first || !panel.contains(activeElement)) {
                  event.preventDefault();
                  last.focus();
                }
                return;
              }

              if (!activeElement || activeElement === last || !panel.contains(activeElement)) {
                event.preventDefault();
                first.focus();
              }
            }}
          >
            {renderDetailsPanel()}
          </aside>
        </>
      )}
    </div>
  );
}
