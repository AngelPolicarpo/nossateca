import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AddBookButton } from "./AddBookButton";

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

type LibraryCollection = "all" | "recent" | "without-author";
type LibraryViewMode = "grid" | "list" | "table";
type LibrarySortField = "title" | "author" | "created_at";
type SortDirection = "asc" | "desc";

type LibraryPreferences = {
  viewMode: LibraryViewMode;
  sortField: LibrarySortField;
  sortDirection: SortDirection;
};

const LIBRARY_PREFERENCES_KEY = "lexicon.library.preferences";

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
      viewMode:
        parsed.viewMode === "grid" || parsed.viewMode === "list" || parsed.viewMode === "table"
          ? parsed.viewMode
          : DEFAULT_LIBRARY_PREFERENCES.viewMode,
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

function getBookStatusTone(status: string): "neutral" | "info" | "success" {
  const normalized = normalizeBookStatus(status);

  if (normalized === "finished") {
    return "success";
  }

  if (normalized === "reading") {
    return "info";
  }

  return "neutral";
}

export function LibraryView({ onOpenReader }: LibraryViewProps) {
  const initialPreferences = useMemo(() => readLibraryPreferences(), []);

  const [books, setBooks] = useState<Book[]>([]);
  const [searchInput, setSearchInput] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [selectedFormat, setSelectedFormat] = useState("all");
  const [selectedStatus, setSelectedStatus] = useState<StatusFilter>("all");
  const [selectedAuthor, setSelectedAuthor] = useState("all");
  const [activeCollection, setActiveCollection] = useState<LibraryCollection>("all");
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

  const collectionOptions = useMemo(() => {
    const now = Date.now();
    const thirtyDaysMs = 30 * 24 * 60 * 60 * 1000;

    const recentCount = books.filter((book) => now - toComparableDate(book.created_at) <= thirtyDaysMs).length;
    const withoutAuthorCount = books.filter((book) => normalizeText(book.author).length === 0).length;

    return [
      {
        id: "all" as const,
        label: "Todos os livros",
        count: books.length,
      },
      {
        id: "recent" as const,
        label: "Adicionados recentemente",
        count: recentCount,
      },
      {
        id: "without-author" as const,
        label: "Sem autor",
        count: withoutAuthorCount,
      },
    ];
  }, [books]);

  const activeCollectionLabel =
    collectionOptions.find((collection) => collection.id === activeCollection)?.label ?? "Todos os livros";

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
    const now = Date.now();
    const thirtyDaysMs = 30 * 24 * 60 * 60 * 1000;

    const scopedBooks = books.filter((book) => {
      if (activeCollection === "recent") {
        return now - toComparableDate(book.created_at) <= thirtyDaysMs;
      }

      if (activeCollection === "without-author") {
        return normalizeText(book.author).length === 0;
      }

      return true;
    });

    const filtered = scopedBooks.filter((book) => {
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
    activeCollection,
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
    setActiveCollection("all");
  };

  const hasActiveFilters =
    normalizeText(searchInput).length > 0 ||
    selectedFormat !== "all" ||
    selectedStatus !== "all" ||
    selectedAuthor !== "all" ||
    activeCollection !== "all";

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

  return (
    <main className="library-screen">
      <aside className="collection-sidebar">
        <div>
          <h2>Coleções</h2>
          <ul className="collection-list">
            {collectionOptions.map((collection) => (
              <li key={collection.id}>
                <button
                  type="button"
                  className={collection.id === activeCollection ? "active" : ""}
                  onClick={() => setActiveCollection(collection.id)}
                >
                  <span>{collection.label}</span>
                  <small>{collection.count}</small>
                </button>
              </li>
            ))}
          </ul>
        </div>

        <div>
          <h3>Filtros</h3>
          <label>
            Formato
            <select value={selectedFormat} onChange={(event) => setSelectedFormat(event.currentTarget.value)}>
              <option value="all">Todos</option>
              {availableFormats.map((format) => (
                <option key={format} value={format}>
                  {format.toUpperCase()}
                </option>
              ))}
            </select>
          </label>

          <label>
            Status
            <select
              value={selectedStatus}
              onChange={(event) => setSelectedStatus(event.currentTarget.value as StatusFilter)}
            >
              <option value="all">Todos</option>
              {availableStatuses.map((status) => (
                <option key={status} value={status}>
                  {getBookStatusLabel(status)}
                </option>
              ))}
            </select>
          </label>

          <label>
            Autor
            <select value={selectedAuthor} onChange={(event) => setSelectedAuthor(event.currentTarget.value)}>
              <option value="all">Todos</option>
              {availableAuthors.map((author) => (
                <option key={author} value={author}>
                  {author}
                </option>
              ))}
            </select>
          </label>

          <button type="button" className="secondary-button compact" onClick={clearAllFilters}>
            Limpar filtros
          </button>
        </div>
      </aside>

      <section className="library-content-shell">
        <header className="library-content-header">
          <div className="library-title-row">
            <div>
              <p className="hero-label">Acervo local</p>
              <h1>Biblioteca</h1>
              <p>{activeCollectionLabel}</p>
            </div>

            <div className="library-header-actions">
              <button type="button" className="secondary-button" onClick={() => void loadBooks()}>
                Recarregar
              </button>
              <AddBookButton onBookAdded={loadBooks} label="+ Adicionar livro" className="primary-button" />
            </div>
          </div>

          <section className="library-topbar-actions panel">
            <label className="search-input-wrap">
              <span>Busca textual</span>
              <div className="library-search-input-shell">
                <input
                  value={searchInput}
                  onChange={(event) => setSearchInput(event.currentTarget.value)}
                  placeholder="Buscar por título ou autor..."
                />
                {normalizeText(searchInput).length > 0 && (
                  <button
                    type="button"
                    className="search-clear-button"
                    onClick={() => {
                      setSearchInput("");
                      setDebouncedSearch("");
                    }}
                  >
                    Limpar
                  </button>
                )}
              </div>
            </label>

            <div className="library-actions-row">
              <div className="library-sort-control" role="group" aria-label="Ordenação da biblioteca">
                <label htmlFor="library-sort-field" className="sr-only">
                  Ordenar por
                </label>
                <select
                  id="library-sort-field"
                  value={sortField}
                  onChange={(event) => setSortField(event.currentTarget.value as LibrarySortField)}
                >
                  <option value="created_at">Data de adição</option>
                  <option value="title">Título</option>
                  <option value="author">Autor</option>
                </select>
                <button
                  type="button"
                  className="library-sort-direction"
                  onClick={() => setSortDirection((previous) => (previous === "asc" ? "desc" : "asc"))}
                >
                  {sortDirection === "asc" ? "Crescente" : "Decrescente"}
                </button>
              </div>

              <div className="view-switch" role="group" aria-label="Alternar visualização">
                <button
                  type="button"
                  className={viewMode === "grid" ? "active" : ""}
                  aria-pressed={viewMode === "grid"}
                  onClick={() => setViewMode("grid")}
                >
                  Grid
                </button>
                <button
                  type="button"
                  className={viewMode === "list" ? "active" : ""}
                  aria-pressed={viewMode === "list"}
                  onClick={() => setViewMode("list")}
                >
                  Lista
                </button>
                <button
                  type="button"
                  className={viewMode === "table" ? "active" : ""}
                  aria-pressed={viewMode === "table"}
                  onClick={() => setViewMode("table")}
                >
                  Tabela
                </button>
              </div>

              {hasActiveFilters && (
                <button type="button" className="secondary-button" onClick={clearAllFilters}>
                  Limpar filtros
                </button>
              )}
            </div>
          </section>
        </header>

        {loading && <p className="state-message">Carregando livros...</p>}
        {error && <p className="state-message error">{error}</p>}

        {isEmptyLibrary && (
          <section className="empty-state">
            <h2>Sua biblioteca está vazia</h2>
            <p>Adicione seu primeiro EPUB ou PDF para iniciar seu acervo local.</p>
            <AddBookButton
              onBookAdded={loadBooks}
              label="Adicionar primeiro livro"
              className="primary-button empty-state-cta"
            />
          </section>
        )}

        {isEmptyFiltered && (
          <section className="empty-state slim">
            <h2>Nenhum resultado com os filtros atuais</h2>
            <p>Limpe os filtros para visualizar todos os livros.</p>
            <button type="button" className="secondary-button" onClick={clearAllFilters}>
              Limpar filtros
            </button>
          </section>
        )}

        {!loading && !error && filteredBooks.length > 0 && viewMode !== "table" && (
          <section className={`book-grid-layout ${viewMode === "list" ? "list" : ""}`}>
            {filteredBooks.map((book) => (
              <article
                key={book.id}
                className={`book-card-v2 ${viewMode === "list" ? "list" : ""}`}
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
                <div className="book-cover-placeholder">{book.format.toUpperCase()}</div>

                <div className="book-card-main">
                  <h3>{book.title}</h3>
                  <p className="book-card-author">{book.author ?? "Autor desconhecido"}</p>
                  <p className="book-card-status">
                    <span className={`book-status-badge ${getBookStatusTone(book.status)}`}>
                      {getBookStatusLabel(book.status)}
                    </span>
                    <span className="book-format-chip">{book.format.toUpperCase()}</span>
                  </p>

                  <p className="book-card-progress">Adicionado em {formatBookDate(book.created_at)}</p>

                  <div className="book-card-actions-row">
                    <button
                      type="button"
                      className={`${viewMode === "grid" ? "secondary-button" : "primary-button"} compact`}
                      onClick={(event) => {
                        event.stopPropagation();
                        onOpenReader(String(book.id));
                      }}
                    >
                      {normalizeBookStatus(book.status) === "unread" ? "Ler agora" : "Retomar leitura"}
                    </button>

                    <button
                      type="button"
                      className="secondary-button danger compact"
                      onClick={(event) => {
                        event.stopPropagation();
                        requestBookRemoval(book);
                      }}
                      disabled={removingBookId === book.id}
                    >
                      {removingBookId === book.id ? "Removendo..." : "Remover"}
                    </button>
                  </div>
                </div>
              </article>
            ))}
          </section>
        )}

        {!loading && !error && filteredBooks.length > 0 && viewMode === "table" && (
          <section className="book-table-shell panel">
            <table className="book-table">
              <thead>
                <tr>
                  <th>Título</th>
                  <th>Autor</th>
                  <th>Formato</th>
                  <th>Status</th>
                  <th>Adição</th>
                  <th aria-label="Ações" />
                </tr>
              </thead>
              <tbody>
                {filteredBooks.map((book) => (
                  <tr key={book.id}>
                    <td>{book.title}</td>
                    <td>{book.author ?? "Autor desconhecido"}</td>
                    <td>{book.format.toUpperCase()}</td>
                    <td>
                      <span className={`book-status-badge ${getBookStatusTone(book.status)}`}>
                        {getBookStatusLabel(book.status)}
                      </span>
                    </td>
                    <td>{formatBookDate(book.created_at)}</td>
                    <td>
                      <div className="book-table-actions">
                        <button
                          type="button"
                          className="secondary-button compact"
                          onClick={() => onOpenReader(String(book.id))}
                        >
                          Abrir
                        </button>
                        <button
                          type="button"
                          className="secondary-button danger compact"
                          onClick={() => requestBookRemoval(book)}
                          disabled={removingBookId === book.id}
                        >
                          {removingBookId === book.id ? "Removendo..." : "Remover"}
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </section>
        )}

        {pendingRemovalBook && (
          <div className="confirm-overlay" role="presentation">
            <section
              className="confirm-dialog panel"
              role="dialog"
              aria-modal="true"
              aria-labelledby="confirm-remove-book-title"
            >
              <h3 id="confirm-remove-book-title">Remover livro da biblioteca</h3>
              <p>
                Deseja remover "{pendingRemovalBook.title}" da biblioteca? Esta ação não pode ser
                desfeita.
              </p>
              <p>Você também pode excluir o arquivo do disco nesta etapa.</p>

              <div className="confirm-dialog-actions">
                <button
                  type="button"
                  className="secondary-button"
                  onClick={() => setPendingRemovalBook(null)}
                >
                  Cancelar
                </button>
                <button
                  type="button"
                  className="secondary-button"
                  onClick={() => void executeBookRemoval(false)}
                >
                  Apenas remover
                </button>
                <button
                  type="button"
                  className="secondary-button danger"
                  onClick={() => void executeBookRemoval(true)}
                >
                  Remover e excluir arquivo
                </button>
              </div>
            </section>
          </div>
        )}
      </section>
    </main>
  );
}
