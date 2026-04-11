import { useEffect, useMemo, useState } from "react";

import {
  type DiscoverCatalog,
  type DiscoverCatalogItem,
  type SourceDownloadResult,
  useDiscoverCatalogItems,
  useDiscoverCatalogs,
  useDiscoverItemDetails,
  useSourceSearchDownloads,
} from "../hooks/useDiscover";

type DiscoverViewProps = {
  onQueueDownload: (sourceUrl: string, fileName: string) => Promise<void>;
};

const PAGE_SIZE = 24;

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

function buildDownloadFileName(title: string, download: SourceDownloadResult): string {
  const safeTitle = title.trim().replace(/[\\/:*?"<>|]+/g, "_").replace(/\s+/g, " ") || "book";
  const normalizedFormat = download.format.trim().toLowerCase();

  if (normalizedFormat.length > 0 && normalizedFormat.length <= 8 && !normalizedFormat.includes("/")) {
    return `${safeTitle}.${normalizedFormat}`;
  }

  return `${safeTitle}.bin`;
}

function renderCover(item: DiscoverCatalogItem) {
  if (!item.coverUrl || item.coverUrl.trim().length === 0) {
    return <div className="discover-cover-placeholder">SEM CAPA</div>;
  }

  return (
    <img
      src={item.coverUrl}
      alt=""
      loading="lazy"
      onError={(event) => {
        event.currentTarget.style.display = "none";
      }}
    />
  );
}

export function DiscoverView({ onQueueDownload }: DiscoverViewProps) {
  const [selectedCatalogKey, setSelectedCatalogKey] = useState("");
  const [selectedGenre, setSelectedGenre] = useState<string | null>(null);
  const [yearFilterInput, setYearFilterInput] = useState("");
  const [skip, setSkip] = useState(0);
  const [selectedItem, setSelectedItem] = useState<DiscoverCatalogItem | null>(null);
  const [queueingUrl, setQueueingUrl] = useState<string | null>(null);

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

  useEffect(() => {
    if (!selectedCatalog) {
      setSelectedGenre(null);
      return;
    }

    if (selectedCatalog.contentType === "subject") {
      setSelectedGenre((previous) => {
        if (previous && selectedCatalog.genres.includes(previous)) {
          return previous;
        }

        return selectedCatalog.genres[0] ?? null;
      });
    } else {
      setSelectedGenre(null);
    }
  }, [selectedCatalog]);

  const yearFilter = useMemo(() => {
    const parsed = Number.parseInt(yearFilterInput.trim(), 10);
    if (Number.isNaN(parsed) || parsed < 0) {
      return null;
    }

    return parsed;
  }, [yearFilterInput]);

  const itemsQuery = useDiscoverCatalogItems(
    selectedCatalogInfo?.pluginId ?? "",
    selectedCatalogInfo?.catalogId ?? "",
    skip,
    PAGE_SIZE,
    selectedCatalog?.contentType === "subject" ? selectedGenre : null,
    yearFilter,
    Boolean(selectedCatalogInfo),
  );

  const items = itemsQuery.data?.items ?? [];

  const detailsQuery = useDiscoverItemDetails(
    selectedItem?.pluginId ?? "",
    selectedItem?.id ?? "",
    selectedItem !== null,
  );

  const sourceTitle = detailsQuery.data?.title ?? selectedItem?.title ?? "";
  const sourceAuthor = detailsQuery.data?.author ?? selectedItem?.author ?? null;
  const sourceIsbn = detailsQuery.data?.isbn ?? selectedItem?.isbn ?? null;

  const sourceQuery = useSourceSearchDownloads(
    sourceTitle,
    sourceAuthor,
    sourceIsbn,
    selectedItem !== null,
  );

  const handleCatalogChange = (nextCatalogKey: string) => {
    setSelectedCatalogKey(nextCatalogKey);
    setSkip(0);
    setSelectedItem(null);
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

  const hasPrevious = skip > 0;
  const hasNext = itemsQuery.data?.hasMore ?? false;

  return (
    <section className="discover-screen">
      <header className="discover-hero panel">
        <div className="discover-hero-main">
          <p className="hero-label">Discover Plugins</p>
          <h1>Discover</h1>
          <p className="discover-copy">
            Navegue por catálogos de plugins Discover e busque links em todos os Source plugins ao
            selecionar um livro.
          </p>
        </div>

        <div className="discover-catalog-controls">
          <label htmlFor="discover-catalog-select">Catálogo</label>
          <select
            id="discover-catalog-select"
            value={selectedCatalogKey}
            onChange={(event) => handleCatalogChange(event.target.value)}
            disabled={catalogsQuery.isLoading || catalogs.length === 0}
          >
            {catalogs.map((catalog) => (
              <option key={catalogKey(catalog)} value={catalogKey(catalog)}>
                {catalog.name} ({catalog.pluginId})
              </option>
            ))}
          </select>

          <label htmlFor="discover-year-input">Ano (opcional)</label>
          <input
            id="discover-year-input"
            type="number"
            min={0}
            placeholder="Ex: 2020"
            value={yearFilterInput}
            onChange={(event) => {
              setYearFilterInput(event.target.value);
              setSkip(0);
            }}
          />

          {selectedCatalog?.contentType === "subject" && (
            <div className="discover-subject-suggestions" aria-label="Gêneros disponíveis">
              {selectedCatalog.genres.map((genre) => (
                <button
                  key={genre}
                  type="button"
                  className={`discover-subject-pill ${selectedGenre === genre ? "active" : ""}`}
                  onClick={() => {
                    setSelectedGenre(genre);
                    setSkip(0);
                  }}
                >
                  {genre}
                </button>
              ))}
            </div>
          )}
        </div>
      </header>

      <section className="discover-feed panel">
        <header className="discover-feed-head">
          <h2>{selectedCatalog?.name ?? "Catálogo"}</h2>
          <p>
            {selectedCatalog
              ? `Plugin: ${selectedCatalog.pluginId}`
              : "Selecione um catálogo para começar"}
          </p>
        </header>

        {catalogsQuery.isLoading && <p className="state-message">Carregando catálogos...</p>}

        {catalogsQuery.isError && (
          <p className="state-message error">
            {getErrorMessage(catalogsQuery.error, "Falha ao carregar catálogos")}
          </p>
        )}

        {!catalogsQuery.isLoading && !catalogsQuery.isError && catalogs.length === 0 && (
          <section className="empty-state slim">
            <h3>Nenhum plugin Discover encontrado</h3>
            <p>Instale um addon Discover (.wasm) na aba Addons para exibir catálogos.</p>
          </section>
        )}

        {selectedCatalog && (
          <>
            {itemsQuery.isLoading && <p className="state-message">Carregando itens do catálogo...</p>}

            {itemsQuery.isError && (
              <p className="state-message error">
                {getErrorMessage(itemsQuery.error, "Falha ao carregar itens do catálogo")}
              </p>
            )}

            {!itemsQuery.isLoading && !itemsQuery.isError && items.length === 0 && (
              <section className="empty-state slim">
                <h3>Nenhum item encontrado</h3>
                <p>Ajuste filtros de gênero/ano ou mude de catálogo.</p>
              </section>
            )}

            {!itemsQuery.isLoading && !itemsQuery.isError && items.length > 0 && (
              <>
                <div className="discover-grid">
                  {items.map((item) => (
                    <article
                      key={`${item.pluginId}:${item.id}`}
                      className={`discover-card ${selectedItem?.id === item.id ? "active" : ""}`}
                    >
                      <div className="discover-cover-shell" aria-hidden>
                        {renderCover(item)}
                      </div>

                      <div className="discover-card-body">
                        <h3>{item.title}</h3>
                        <p className="discover-card-authors">{item.author}</p>
                        <p className="discover-card-meta">
                          {item.year ? `Ano: ${item.year}` : "Ano indisponível"}
                          {item.format ? ` | Formato: ${item.format}` : ""}
                        </p>

                        {item.shortDescription && (
                          <p className="discover-card-meta">{item.shortDescription}</p>
                        )}

                        <div className="discover-chip-row">
                          {item.genres.slice(0, 3).map((genre) => (
                            <span key={`${item.id}-${genre}`} className="discover-chip">
                              {genre}
                            </span>
                          ))}
                        </div>

                        <div className="discover-card-actions">
                          <button
                            type="button"
                            className="secondary-button compact"
                            onClick={() => setSelectedItem(item)}
                          >
                            Ver fontes
                          </button>
                        </div>
                      </div>
                    </article>
                  ))}
                </div>

                <div className="discover-pagination">
                  <button
                    type="button"
                    className="secondary-button"
                    disabled={!hasPrevious}
                    onClick={() => setSkip((previous) => Math.max(0, previous - PAGE_SIZE))}
                  >
                    Página anterior
                  </button>
                  <button
                    type="button"
                    className="secondary-button"
                    disabled={!hasNext}
                    onClick={() => setSkip((previous) => previous + PAGE_SIZE)}
                  >
                    Próxima página
                  </button>
                </div>
              </>
            )}
          </>
        )}
      </section>

      {selectedItem && (
        <section className="discover-details panel">
          <header className="discover-details-head">
            <div>
              <p className="hero-label">Livro selecionado</p>
              <h2>{detailsQuery.data?.title ?? selectedItem.title}</h2>
              <p className="discover-card-authors">{detailsQuery.data?.author ?? selectedItem.author}</p>
            </div>
            <button type="button" className="secondary-button" onClick={() => setSelectedItem(null)}>
              Fechar
            </button>
          </header>

          {detailsQuery.isLoading && <p className="state-message">Carregando detalhes do item...</p>}

          {detailsQuery.isError && (
            <p className="state-message error">
              {getErrorMessage(detailsQuery.error, "Falha ao carregar detalhes do item")}
            </p>
          )}

          {detailsQuery.data?.description && (
            <p className="discover-description">{detailsQuery.data.description}</p>
          )}

          {detailsQuery.data?.originUrl && (
            <a className="discover-openlibrary-link" href={detailsQuery.data.originUrl} target="_blank" rel="noreferrer">
              Abrir origem
            </a>
          )}

          <section className="discover-sources">
            <header className="discover-editions-head">
              <h3>Resultados por fonte</h3>
              <p>
                Consulta simultânea em todos os Source plugins instalados usando título, autor e ISBN.
              </p>
            </header>

            {sourceQuery.isLoading && <p className="state-message">Consultando fontes de download...</p>}

            {sourceQuery.isError && (
              <p className="state-message error">
                {getErrorMessage(sourceQuery.error, "Falha ao consultar fontes")}
              </p>
            )}

            {!sourceQuery.isLoading && !sourceQuery.isError && (sourceQuery.data?.length ?? 0) === 0 && (
              <p className="state-message">Nenhum Source plugin disponível.</p>
            )}

            {!sourceQuery.isLoading && !sourceQuery.isError && (sourceQuery.data?.length ?? 0) > 0 && (
              <div className="discover-source-groups">
                {sourceQuery.data?.map((group) => (
                  <section key={`${group.pluginId}-${group.sourceId}`} className="discover-source-group">
                    <header className="discover-source-group-head">
                      <h4>{group.sourceName}</h4>
                      <p>
                        Plugin: {group.pluginId}
                        {group.supportedFormats.length > 0
                          ? ` | Formatos: ${group.supportedFormats.join(", ")}`
                          : ""}
                      </p>
                    </header>

                    {group.error && (
                      <p className="state-message error">
                        [{group.error.kind}] {group.error.message}
                      </p>
                    )}

                    {!group.error && group.results.length === 0 && (
                      <p className="state-message">Sem resultados nesta fonte.</p>
                    )}

                    {!group.error && group.results.length > 0 && (
                      <ul className="discover-edition-list">
                        {group.results.map((download) => (
                          <li key={`${group.pluginId}-${download.downloadUrl}`} className="discover-edition-item">
                            <strong>{download.format.toUpperCase()}</strong>
                            <p className="discover-edition-meta">
                              {download.language ? `Idioma: ${download.language}` : "Idioma: n/d"}
                              {download.size ? ` | Tamanho: ${download.size}` : ""}
                              {download.quality ? ` | ${download.quality}` : ""}
                            </p>
                            <div className="discover-card-actions">
                              <a
                                className="secondary-button compact"
                                href={download.downloadUrl}
                                target="_blank"
                                rel="noreferrer"
                              >
                                Abrir link
                              </a>
                              <button
                                type="button"
                                className="primary-button compact"
                                disabled={queueingUrl !== null}
                                onClick={() => void handleQueueDownload(download)}
                              >
                                {queueingUrl === download.downloadUrl
                                  ? "Enfileirando..."
                                  : "Adicionar na fila"}
                              </button>
                            </div>
                          </li>
                        ))}
                      </ul>
                    )}
                  </section>
                ))}
              </div>
            )}
          </section>
        </section>
      )}
    </section>
  );
}
