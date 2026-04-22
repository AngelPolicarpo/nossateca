import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AddBookButton } from "./AddBookButton";
import { BookCover } from "./ui/BookCover";
import { Button } from "./ui/Button";
import { CustomSelect, type SelectOption } from "./ui/CustomSelect";
import { EmptyState } from "./ui/EmptyState";
import { Input } from "./ui/Input";
import { Panel } from "./ui/Panel";
import { StateMessage } from "./ui/StateMessage";
import { cn } from "../lib/cn";

export type Book = {
  id: number;
  title: string;
  author: string | null;
  format: string;
  file_path: string;
  file_hash: string | null;
  status: string;
  created_at: string;
};

type LibraryViewProps = {
  onOpenReader: (bookId: string) => void;
};

type LibraryViewMode = "grid" | "table";
type LibrarySortField = "title" | "author" | "created_at";
type SortDirection = "asc" | "desc";

type LibraryPreferences = {
  viewMode: LibraryViewMode;
  sortField: LibrarySortField;
  sortDirection: SortDirection;
};

const LIBRARY_PREFERENCES_KEY = "app.library.preferences";

const DEFAULT_LIBRARY_PREFERENCES: LibraryPreferences = {
  viewMode: "grid",
  sortField: "created_at",
  sortDirection: "desc",
};

const BOOK_STATUSES = ["unread", "reading", "finished"] as const;
type BookStatus = (typeof BOOK_STATUSES)[number];
type StatusFilter = "all" | BookStatus;

const STATUS_LABELS: Record<string, string> = {
  unread: "Não iniciado",
  reading: "Lendo",
  finished: "Concluído",
};

function normalizeBookStatus(status: string | null | undefined): string {
  const normalized = normalizeText(status);

  if (normalized === "discovered") {
    return "unread";
  }

  if (normalized === "completed") {
    return "finished";
  }

  if (normalized === "in-progress" || normalized === "in_progress") {
    return "reading";
  }

  return normalized;
}

function readLibraryPreferences(): LibraryPreferences {
  if (typeof window === "undefined") {
    return DEFAULT_LIBRARY_PREFERENCES;
  }

  try {
    const raw = window.localStorage.getItem(LIBRARY_PREFERENCES_KEY);
    if (!raw) {
      return DEFAULT_LIBRARY_PREFERENCES;
    }

    const parsed = JSON.parse(raw) as Partial<LibraryPreferences>;

    return {
      viewMode: parsed.viewMode === "table" ? "table" : "grid",
      sortField:
        parsed.sortField === "title" || parsed.sortField === "author" || parsed.sortField === "created_at"
          ? parsed.sortField
          : DEFAULT_LIBRARY_PREFERENCES.sortField,
      sortDirection:
        parsed.sortDirection === "asc" || parsed.sortDirection === "desc"
          ? parsed.sortDirection
          : DEFAULT_LIBRARY_PREFERENCES.sortDirection,
    };
  } catch {
    return DEFAULT_LIBRARY_PREFERENCES;
  }
}

function writeLibraryPreferences(preferences: LibraryPreferences): void {
  if (typeof window === "undefined") {
    return;
  }

  window.localStorage.setItem(LIBRARY_PREFERENCES_KEY, JSON.stringify(preferences));
}

function normalizeText(value: string | null | undefined): string {
  return (value ?? "").trim().toLowerCase();
}

function toComparableDate(value: string): number {
  const timestamp = Date.parse(value);
  return Number.isNaN(timestamp) ? 0 : timestamp;
}

function formatBookDate(value: string): string {
  const timestamp = toComparableDate(value);

  if (timestamp === 0) {
    return "Data indisponível";
  }

  return new Intl.DateTimeFormat("pt-BR", { dateStyle: "medium" }).format(new Date(timestamp));
}

function getBookStatusLabel(status: string): string {
  const normalized = normalizeBookStatus(status);

  if (STATUS_LABELS[normalized]) {
    return STATUS_LABELS[normalized];
  }

  const sanitized = normalized.replace(/[_-]+/g, " ").trim();
  if (!sanitized) {
    return "Desconhecido";
  }

  return sanitized.charAt(0).toUpperCase() + sanitized.slice(1);
}

function getBookStatusToneClass(status: string): string {
  const normalized = normalizeBookStatus(status);

  if (normalized === "finished") {
    return "bg-[rgba(26,174,57,0.10)] text-[var(--color-semantic-green)]";
  }

  if (normalized === "reading") {
    return "bg-[var(--color-badge-bg)] text-[var(--color-badge-text)]";
  }

  return "text-[var(--color-text-secondary)]";
}

function SearchFieldIcon() {
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

function ClearFiltersIcon() {
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

function RemoveBookIcon() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      width="18"
      height="18"
      className="block h-[18px] w-[18px] shrink-0"
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

export function LibraryView({ onOpenReader }: LibraryViewProps) {
  const initialPreferences = useMemo(() => readLibraryPreferences(), []);

  const [books, setBooks] = useState<Book[]>([]);
  const [searchInput, setSearchInput] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [selectedFormat, setSelectedFormat] = useState("all");
  const [selectedStatus, setSelectedStatus] = useState<StatusFilter>("all");
  const [selectedAuthor, setSelectedAuthor] = useState("all");
  const [viewMode, setViewMode] = useState<LibraryViewMode>(initialPreferences.viewMode);
  const [sortField, setSortField] = useState<LibrarySortField>(initialPreferences.sortField);
  const [sortDirection, setSortDirection] = useState<SortDirection>(initialPreferences.sortDirection);
  const [pendingRemovalBook, setPendingRemovalBook] = useState<Book | null>(null);
  const [removingBookId, setRemovingBookId] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const getErrorMessage = (err: unknown, fallback: string): string => {
    if (err instanceof Error && err.message.trim().length > 0) {
      return err.message;
    }

    if (typeof err === "string" && err.trim().length > 0) {
      return err;
    }

    return fallback;
  };

  const loadBooks = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const result = await invoke<Book[]>("list_books");
      setBooks(result);
    } catch (err) {
      setError(getErrorMessage(err, "Erro ao listar livros"));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadBooks();
  }, [loadBooks]);

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setDebouncedSearch(searchInput);
    }, 260);

    return () => {
      window.clearTimeout(timer);
    };
  }, [searchInput]);

  useEffect(() => {
    writeLibraryPreferences({
      viewMode,
      sortField,
      sortDirection,
    });
  }, [sortDirection, sortField, viewMode]);

  useEffect(() => {
    if (!pendingRemovalBook) {
      return;
    }

    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setPendingRemovalBook(null);
      }
    };

    window.addEventListener("keydown", handleEscape);

    return () => {
      window.removeEventListener("keydown", handleEscape);
    };
  }, [pendingRemovalBook]);

  const availableFormats = useMemo(() => {
    const formats = new Set(books.map((book) => book.format.toLowerCase()));
    return Array.from(formats).sort();
  }, [books]);

  const availableStatuses = useMemo<BookStatus[]>(() => [...BOOK_STATUSES], []);

  const availableAuthors = useMemo(() => {
    const authors = new Set(
      books
        .map((book) => (book.author ?? "").trim())
        .filter((author) => author.length > 0),
    );

    return Array.from(authors).sort((left, right) =>
      left.localeCompare(right, "pt-BR", { sensitivity: "base" }),
    );
  }, [books]);

  const formatCounts = useMemo(() => {
    const counts = new Map<string, number>();
    for (const book of books) {
      const normalizedFormat = book.format.toLowerCase();
      counts.set(normalizedFormat, (counts.get(normalizedFormat) ?? 0) + 1);
    }
    return counts;
  }, [books]);

  const statusCounts = useMemo(() => {
    const counts = new Map<BookStatus, number>();
    for (const status of availableStatuses) {
      counts.set(status, 0);
    }

    for (const book of books) {
      const normalizedStatus = normalizeBookStatus(book.status) as BookStatus;
      if (!counts.has(normalizedStatus)) {
        continue;
      }

      counts.set(normalizedStatus, (counts.get(normalizedStatus) ?? 0) + 1);
    }

    return counts;
  }, [availableStatuses, books]);

  useEffect(() => {
    if (selectedFormat !== "all" && !availableFormats.includes(selectedFormat)) {
      setSelectedFormat("all");
    }
  }, [availableFormats, selectedFormat]);

  useEffect(() => {
    if (selectedStatus !== "all" && !availableStatuses.includes(selectedStatus)) {
      setSelectedStatus("all");
    }
  }, [availableStatuses, selectedStatus]);

  useEffect(() => {
    if (selectedAuthor !== "all" && !availableAuthors.includes(selectedAuthor)) {
      setSelectedAuthor("all");
    }
  }, [availableAuthors, selectedAuthor]);

  const filteredBooks = useMemo(() => {
    const normalizedSearch = normalizeText(debouncedSearch);
    const filtered = books.filter((book) => {
      if (
        normalizedSearch.length > 0 &&
        !normalizeText(book.title).includes(normalizedSearch) &&
        !normalizeText(book.author).includes(normalizedSearch)
      ) {
        return false;
      }

      if (selectedFormat !== "all" && book.format.toLowerCase() !== selectedFormat) {
        return false;
      }

      if (selectedStatus !== "all" && normalizeBookStatus(book.status) !== selectedStatus) {
        return false;
      }

      if (selectedAuthor !== "all" && !book.author) {
        return false;
      }

      if (selectedAuthor !== "all" && normalizeText(book.author) !== normalizeText(selectedAuthor)) {
        return false;
      }

      return true;
    });

    return filtered.sort((left, right) => {
      let comparison = 0;

      if (sortField === "title") {
        comparison = left.title.localeCompare(right.title, "pt-BR", { sensitivity: "base" });
      } else if (sortField === "author") {
        comparison = (left.author ?? "").localeCompare(right.author ?? "", "pt-BR", {
          sensitivity: "base",
        });
      } else {
        comparison = toComparableDate(left.created_at) - toComparableDate(right.created_at);
      }

      return sortDirection === "asc" ? comparison : -comparison;
    });
  }, [
    books,
    debouncedSearch,
    selectedAuthor,
    selectedFormat,
    selectedStatus,
    sortDirection,
    sortField,
  ]);

  const clearAllFilters = () => {
    setSearchInput("");
    setDebouncedSearch("");
    setSelectedFormat("all");
    setSelectedStatus("all");
    setSelectedAuthor("all");
  };

  const hasActiveFilters =
    normalizeText(searchInput).length > 0 ||
    selectedFormat !== "all" ||
    selectedStatus !== "all" ||
    selectedAuthor !== "all";

  const requestBookRemoval = (book: Book) => {
    setPendingRemovalBook(book);
  };

  const executeBookRemoval = async (deleteFile: boolean) => {
    if (!pendingRemovalBook) {
      return;
    }

    const bookToRemove = pendingRemovalBook;
    setPendingRemovalBook(null);
    setRemovingBookId(bookToRemove.id);

    try {
      setError(null);
      await invoke<void>("remove_book", {
        bookId: bookToRemove.id,
        deleteFile,
      });
      await loadBooks();
    } catch (err) {
      setError(getErrorMessage(err, "Falha ao remover livro"));
      await loadBooks();
    } finally {
      setRemovingBookId(null);
    }
  };

  const isEmptyLibrary = !loading && !error && books.length === 0;
  const isEmptyFiltered = !loading && !error && books.length > 0 && filteredBooks.length === 0;

  const controlFieldClass =
    "grid min-w-[164px] flex-1 basis-0 gap-[var(--space-6)] text-[12px] font-semibold uppercase tracking-[0.08em] text-[var(--color-text-muted)]";
  const controlSelectClass =
    "min-h-[var(--control-height)] rounded-[var(--radius-pill)] border border-black/10 px-[var(--space-12)] text-[14px] font-medium normal-case tracking-normal text-[var(--color-text-primary)] hover:border-black/20";
  const controlSelectMenuClass =
    "rounded-[var(--radius-12)] border border-black/10 p-[var(--space-6)]";
  const controlSelectOptionClass =
    "rounded-[var(--radius-8)] px-[var(--space-10)] py-[var(--space-8)] text-[13px]";

  const formatOptions: SelectOption[] = [
    { value: "all", label: `Todos (${books.length})` },
    ...availableFormats.map((format) => ({
      value: format,
      label: `${format.toUpperCase()} (${formatCounts.get(format) ?? 0})`,
    })),
  ];

  const statusOptions: SelectOption[] = [
    { value: "all", label: `Todos (${books.length})` },
    ...availableStatuses.map((status) => ({
      value: status,
      label: `${getBookStatusLabel(status)} (${statusCounts.get(status) ?? 0})`,
    })),
  ];

  const authorOptions: SelectOption[] = [
    { value: "all", label: "Todos" },
    ...availableAuthors.map((author) => ({
      value: author,
      label: author,
    })),
  ];

  const sortFieldOptions: SelectOption[] = [
    { value: "created_at", label: "Data de adição" },
    { value: "title", label: "Título" },
    { value: "author", label: "Autor" },
  ];

  const sortDirectionOptions: SelectOption[] = [
    { value: "desc", label: "Decrescente" },
    { value: "asc", label: "Crescente" },
  ];

  const viewModeOptions: SelectOption[] = [
    { value: "grid", label: "Grade" },
    { value: "table", label: "Tabela" },
  ];

  return (
    <main className="grid min-h-[calc(100vh-140px)] overflow-hidden rounded-[var(--radius-16)] shadow-[var(--shadow-card)]">
      <section className="grid content-start gap-[var(--space-24)] max-md:p-[var(--space-16)]">
        <header className="grid gap-[var(--space-16)]">
          <div className="lx-page-header">
            <div className="lx-page-header-titles">
              <h1 className="lx-page-title">Biblioteca</h1>
              <p className="lx-page-subtitle">
                {filteredBooks.length} de {books.length} obras · armazenadas localmente
              </p>
            </div>

            <div className="lx-page-header-actions">
              <Button variant="secondary" onClick={() => void loadBooks()}>
                Recarregar
              </Button>
              <AddBookButton onBookAdded={loadBooks} label="+ Adicionar" />
            </div>
          </div>

          <Panel
            className="grid gap-[var(--space-12)] !p-0"
          >
            <label className="m-0 grid gap-[var(--space-8)] text-[14px] leading-[1.43] text-[var(--color-text-secondary)]">
              <div className="flex items-center gap-[var(--space-8)]">
                <div className="relative min-w-0 flex-1">
                  <SearchFieldIcon />
                  <Input
                    type="search"
                    value={searchInput}
                    onChange={(event) => setSearchInput(event.currentTarget.value)}
                    placeholder="Buscar por título ou autor"
                    className="h-[40px] min-h-[40px] rounded-[var(--radius-pill)] border-black/15 pl-[34px]"
                  />
                </div>
                <Button
                  variant="secondary"
                  size="sm"
                  className="h-[40px] min-h-[40px] w-[40px] min-w-[40px] max-h-[40px] max-w-[40px] flex-none rounded-[var(--radius-pill)] p-0 [&_svg]:h-[18px] [&_svg]:w-[18px]"
                  aria-label="Limpar filtros"
                  title="Limpar filtros"
                  onClick={clearAllFilters}
                  disabled={!hasActiveFilters}
                >
                  <ClearFiltersIcon />
                </Button>
              </div>
            </label>

            <div className="m-0 flex flex-wrap items-end justify-start gap-[var(--space-12)] pl-0">
              <label className={controlFieldClass}>
                Formato
                <CustomSelect
                  triggerClassName={controlSelectClass}
                  menuClassName={controlSelectMenuClass}
                  optionClassName={controlSelectOptionClass}
                  value={selectedFormat}
                  options={formatOptions}
                  onValueChange={setSelectedFormat}
                />
              </label>

              <label className={controlFieldClass}>
                Status
                <CustomSelect
                  triggerClassName={controlSelectClass}
                  menuClassName={controlSelectMenuClass}
                  optionClassName={controlSelectOptionClass}
                  value={selectedStatus}
                  options={statusOptions}
                  onValueChange={(nextValue) => setSelectedStatus(nextValue as StatusFilter)}
                />
              </label>

              <label className={controlFieldClass}>
                Autor
                <CustomSelect
                  triggerClassName={controlSelectClass}
                  menuClassName={controlSelectMenuClass}
                  optionClassName={controlSelectOptionClass}
                  value={selectedAuthor}
                  options={authorOptions}
                  onValueChange={setSelectedAuthor}
                />
              </label>

              <label htmlFor="library-sort-field" className={controlFieldClass}>
                Ordenar por
                <CustomSelect
                  id="library-sort-field"
                  triggerClassName={controlSelectClass}
                  menuClassName={controlSelectMenuClass}
                  optionClassName={controlSelectOptionClass}
                  value={sortField}
                  options={sortFieldOptions}
                  onValueChange={(nextValue) => setSortField(nextValue as LibrarySortField)}
                />
              </label>

              <label className={controlFieldClass}>
                Direção
                <CustomSelect
                  triggerClassName={controlSelectClass}
                  menuClassName={controlSelectMenuClass}
                  optionClassName={controlSelectOptionClass}
                  value={sortDirection}
                  options={sortDirectionOptions}
                  onValueChange={(nextValue) => setSortDirection(nextValue as SortDirection)}
                />
              </label>

              <label className={controlFieldClass}>
                Visualização
                <CustomSelect
                  triggerClassName={controlSelectClass}
                  menuClassName={controlSelectMenuClass}
                  optionClassName={controlSelectOptionClass}
                  value={viewMode}
                  options={viewModeOptions}
                  onValueChange={(nextValue) => setViewMode(nextValue as LibraryViewMode)}
                />
              </label>
            </div>
          </Panel>
        </header>

        {loading && <StateMessage>Carregando livros...</StateMessage>}
        {error && <StateMessage tone="error">{error}</StateMessage>}

        {isEmptyLibrary && (
          <EmptyState
            title="Sua biblioteca está vazia"
            description="Adicione seu primeiro EPUB ou PDF para iniciar seu acervo local."
            action={
              <AddBookButton
                onBookAdded={loadBooks}
                label="Adicionar primeiro livro"
                className="min-h-14"
              />
            }
          />
        )}

        {isEmptyFiltered && (
          <EmptyState
            compact
            title="Nenhum resultado com os filtros atuais"
            description="Limpe os filtros para visualizar todos os livros."
            action={
              <Button variant="secondary" onClick={clearAllFilters}>
                Limpar filtros
              </Button>
            }
          />
        )}

        {!loading && !error && filteredBooks.length > 0 && viewMode === "grid" && (
          <section className="grid gap-x-[var(--space-20)] gap-y-[var(--space-28)] [grid-template-columns:repeat(auto-fill,minmax(164px,1fr))]">
            {filteredBooks.map((book) => (
              <article
                key={book.id}
                className="group grid cursor-pointer gap-[var(--space-12)] rounded-[var(--radius-12)] p-[var(--space-4)] transition-transform duration-150 hover:-translate-y-[2px]"
                onClick={() => onOpenReader(String(book.id))}
                role="button"
                tabIndex={0}
                aria-label={`Abrir ${book.title}`}
                onKeyDown={(event) => {
                  if (event.key === "Enter" || event.key === " ") {
                    event.preventDefault();
                    onOpenReader(String(book.id));
                  }
                }}
              >
                <BookCover
                  title={book.title}
                  author={book.author}
                  format={book.format}
                  size="md"
                  className="rounded-[var(--radius-12)]"
                />

                <div className="grid gap-[var(--space-4)] px-[var(--space-2)]">
                  <h3 className="m-0 overflow-hidden text-[16px] font-semibold leading-[1.3] tracking-[-0.01em] text-[var(--color-text-primary)] [display:-webkit-box] [-webkit-box-orient:vertical] [-webkit-line-clamp:2]">
                    {book.title}
                  </h3>
                  <p className="m-0 overflow-hidden text-[13px] leading-[1.4] text-[var(--color-text-muted)] [display:-webkit-box] [-webkit-box-orient:vertical] [-webkit-line-clamp:1]">
                    {book.author ?? "Autor desconhecido"}
                  </p>
                </div>
              </article>
            ))}
          </section>
        )}

        {!loading && !error && filteredBooks.length > 0 && viewMode === "table" && (
          <Panel className="overflow-x-auto p-0">
            <table className="w-full min-w-[760px] border-collapse max-md:min-w-[680px] max-[600px]:min-w-[620px]">
              <thead>
                <tr>
                  <th className="border-b border-black/10 px-[var(--space-16)] py-[var(--space-12)] text-left text-[12px] font-semibold uppercase leading-[1.33] tracking-[0.125px] text-[var(--color-text-muted)]">
                    Título
                  </th>
                  <th className="border-b border-black/10 px-[var(--space-16)] py-[var(--space-12)] text-left text-[12px] font-semibold uppercase leading-[1.33] tracking-[0.125px] text-[var(--color-text-muted)]">
                    Autor
                  </th>
                  <th className="border-b border-black/10 px-[var(--space-16)] py-[var(--space-12)] text-left text-[12px] font-semibold uppercase leading-[1.33] tracking-[0.125px] text-[var(--color-text-muted)]">
                    Formato
                  </th>
                  <th className="border-b border-black/10 px-[var(--space-16)] py-[var(--space-12)] text-left text-[12px] font-semibold uppercase leading-[1.33] tracking-[0.125px] text-[var(--color-text-muted)]">
                    Status
                  </th>
                  <th className="border-b border-black/10 px-[var(--space-16)] py-[var(--space-12)] text-left text-[12px] font-semibold uppercase leading-[1.33] tracking-[0.125px] text-[var(--color-text-muted)]">
                    Adição
                  </th>
                  <th
                    aria-label="Ações"
                    className="min-w-[200px] border-b border-black/10 px-[var(--space-16)] py-[var(--space-12)] text-right text-[12px] font-semibold uppercase leading-[1.33] tracking-[0.125px] text-[var(--color-text-muted)]"
                  />
                </tr>
              </thead>
              <tbody>
                {filteredBooks.map((book) => (
                  <tr
                    key={book.id}
                    className="cursor-pointer transition-colors hover:bg-[var(--color-control-secondary-bg)] focus-within:bg-[var(--color-control-secondary-bg)]"
                    tabIndex={0}
                    aria-label={`Abrir ${book.title}`}
                    onClick={(event) => {
                      const target = event.target as HTMLElement;
                      if (target.closest("button, a, input, select, textarea")) {
                        return;
                      }

                      onOpenReader(String(book.id));
                    }}
                    onKeyDown={(event) => {
                      if (event.key === "Enter" || event.key === " ") {
                        event.preventDefault();
                        onOpenReader(String(book.id));
                      }
                    }}
                  >
                    <td className="border-b border-black/10 px-[var(--space-16)] py-[var(--space-12)] text-[14px] font-semibold leading-[1.43] text-[var(--color-text-primary)]">
                      {book.title}
                    </td>
                    <td className="border-b border-black/10 px-[var(--space-16)] py-[var(--space-12)] text-[14px] leading-[1.43] text-[var(--color-text-secondary)]">
                      {book.author ?? "Autor desconhecido"}
                    </td>
                    <td className="border-b border-black/10 px-[var(--space-16)] py-[var(--space-12)] text-[14px] leading-[1.43] text-[var(--color-text-secondary)]">
                      {book.format.toUpperCase()}
                    </td>
                    <td className="border-b border-black/10 px-[var(--space-16)] py-[var(--space-12)] text-[14px] leading-[1.43] text-[var(--color-text-secondary)]">
                      <span
                        className={cn(
                          "inline-flex items-center rounded-[var(--radius-pill)] border border-black/10 px-[var(--space-8)] py-[var(--space-2)] text-[12px] font-semibold leading-[1.33] tracking-[0.125px]",
                          getBookStatusToneClass(book.status),
                        )}
                      >
                        {getBookStatusLabel(book.status)}
                      </span>
                    </td>
                    <td className="border-b border-black/10 px-[var(--space-16)] py-[var(--space-12)] text-[14px] leading-[1.43] text-[var(--color-text-secondary)]">
                      {formatBookDate(book.created_at)}
                    </td>
                    <td className="min-w-[200px] border-b border-black/10 px-[var(--space-16)] py-[var(--space-12)] text-right text-[14px] leading-[1.43] text-[var(--color-text-secondary)]">
                      <div className="flex flex-wrap justify-end gap-[var(--space-8)] max-[600px]:flex-col max-[600px]:items-stretch">
                        <Button
                          variant="danger"
                          size="sm"
                          className="h-[32px] min-h-[32px] w-[32px] min-w-[32px] rounded-[var(--radius-pill)] p-0 [&_svg]:h-[18px] [&_svg]:w-[18px]"
                          onClick={(event) => {
                            event.stopPropagation();
                            requestBookRemoval(book);
                          }}
                          aria-label={`Remover ${book.title}`}
                          title={`Remover ${book.title}`}
                          disabled={removingBookId === book.id}
                        >
                          {removingBookId === book.id ? (
                            <span
                              className="h-[14px] w-[14px] animate-spin rounded-full border-2 border-current border-t-transparent"
                              aria-hidden="true"
                            />
                          ) : (
                            <RemoveBookIcon />
                          )}
                        </Button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </Panel>
        )}

        {pendingRemovalBook && (
          <div
            className="fixed inset-0 z-40 grid place-items-center bg-[rgba(12,20,28,0.45)] p-[var(--space-16)]"
            role="presentation"
          >
            <Panel
              as="section"
              className="z-10 grid w-full max-w-[520px] gap-[var(--space-12)] rounded-[var(--radius-16)] border border-black/10  p-[var(--space-24)] shadow-[var(--shadow-card)]"
              role="dialog"
              aria-modal="true"
              aria-labelledby="confirm-remove-book-title"
            >
              <h3
                id="confirm-remove-book-title"
                className="m-0 text-[22px] font-[var(--type-sub-weight)] leading-[1.25] tracking-[var(--type-sub-track)]"
              >
                Remover livro da biblioteca
              </h3>
              <p className="m-0 text-[14px] leading-[1.43] text-[var(--color-text-secondary)]">
                Deseja remover "{pendingRemovalBook.title}" da biblioteca? Esta ação não pode ser
                desfeita.
              </p>
              <p className="m-0 text-[14px] leading-[1.43] text-[var(--color-text-secondary)]">
                Você também pode excluir o arquivo do disco nesta etapa.
              </p>

              <div className="mt-[var(--space-8)] flex flex-wrap justify-end gap-[var(--space-8)] max-[560px]:flex-col max-[560px]:items-stretch">
                <Button variant="secondary" onClick={() => setPendingRemovalBook(null)}>
                  Cancelar
                </Button>
                <Button variant="secondary" onClick={() => void executeBookRemoval(false)}>
                  Apenas remover
                </Button>
                <Button variant="danger" onClick={() => void executeBookRemoval(true)}>
                  Remover e excluir arquivo
                </Button>
              </div>
            </Panel>
          </div>
        )}
      </section>
    </main>
  );
}
