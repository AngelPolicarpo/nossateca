import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";

export type PluginErrorKind =
  | "network_failure"
  | "parsing_failure"
  | "rate_limit"
  | "not_found"
  | "unknown";

export type PluginTypedError = {
  kind: PluginErrorKind;
  message: string;
};

export type DiscoverCatalog = {
  pluginId: string;
  id: string;
  name: string;
  contentType: string;
  genres: string[];
  supportedFilters: string[];
};

export type DiscoverCatalogItem = {
  pluginId: string;
  catalogId: string;
  id: string;
  title: string;
  author: string;
  coverUrl: string;
  genres: string[];
  year: number | null;
  pageCount: number | null;
  shortDescription: string | null;
  format: string | null;
  isbn: string | null;
};

export type DiscoverCatalogPageResponse = {
  pluginId: string;
  catalogId: string;
  items: DiscoverCatalogItem[];
  hasMore: boolean;
};

export type DiscoverItemDetails = {
  pluginId: string;
  id: string;
  title: string;
  author: string;
  description: string | null;
  coverUrl: string;
  genres: string[];
  year: number | null;
  pageCount: number | null;
  format: string | null;
  isbn: string | null;
  originUrl: string | null;
};

export type SourceDownloadResult = {
  downloadUrl: string;
  format: string;
  size: string | null;
  language: string | null;
  quality: string | null;
};

export type SourceSearchResultGroup = {
  pluginId: string;
  sourceName: string;
  sourceId: string;
  supportedFormats: string[];
  results: SourceDownloadResult[];
  error: PluginTypedError | null;
};

const STALE_TIME_MS = 60_000;

export function useDiscoverCatalogs(enabled = true) {
  return useQuery({
    queryKey: ["discover", "catalogs"],
    queryFn: () => invoke<DiscoverCatalog[]>("list_discover_catalogs"),
    staleTime: STALE_TIME_MS,
    retry: 1,
    enabled,
  });
}

export function useDiscoverCatalogItems(
  pluginId: string,
  catalogId: string,
  skip: number,
  pageSize: number,
  genre: string | null,
  year: number | null,
  searchQuery: string | null,
  enabled = true,
) {
  const normalizedPluginId = pluginId.trim();
  const normalizedCatalogId = catalogId.trim();
  const normalizedGenre = genre?.trim() ?? "";
  const normalizedSearchQuery = searchQuery?.trim() ?? "";

  return useQuery({
    queryKey: [
      "discover",
      "catalog-items",
      normalizedPluginId,
      normalizedCatalogId,
      skip,
      pageSize,
      normalizedGenre,
      year,
      normalizedSearchQuery,
    ],
    queryFn: () =>
      invoke<DiscoverCatalogPageResponse>("list_discover_catalog_items", {
        pluginId: normalizedPluginId,
        catalogId: normalizedCatalogId,
        skip,
        pageSize,
        genre: normalizedGenre.length > 0 ? normalizedGenre : null,
        year,
        searchQuery: normalizedSearchQuery.length > 0 ? normalizedSearchQuery : null,
      }),
    staleTime: STALE_TIME_MS,
    retry: 1,
    enabled: enabled && normalizedPluginId.length > 0 && normalizedCatalogId.length > 0,
  });
}

export function useDiscoverItemDetails(pluginId: string, itemId: string, enabled = true) {
  const normalizedPluginId = pluginId.trim();
  const normalizedItemId = itemId.trim();

  return useQuery({
    queryKey: ["discover", "item-details", normalizedPluginId, normalizedItemId],
    queryFn: () =>
      invoke<DiscoverItemDetails>("get_discover_item_details", {
        pluginId: normalizedPluginId,
        itemId: normalizedItemId,
      }),
    staleTime: STALE_TIME_MS,
    retry: 1,
    enabled: enabled && normalizedPluginId.length > 0 && normalizedItemId.length > 0,
  });
}

export function useSourceSearchDownloads(
  title: string,
  author: string | null,
  isbn: string | null,
  enabled = true,
) {
  const normalizedTitle = title.trim();
  const normalizedAuthor = author?.trim() ?? "";
  const normalizedIsbn = isbn?.trim() ?? "";

  return useQuery({
    queryKey: ["discover", "source-downloads", normalizedTitle, normalizedAuthor, normalizedIsbn],
    queryFn: () =>
      invoke<SourceSearchResultGroup[]>("search_source_downloads", {
        title: normalizedTitle,
        author: normalizedAuthor.length > 0 ? normalizedAuthor : null,
        isbn: normalizedIsbn.length > 0 ? normalizedIsbn : null,
      }),
    staleTime: 15_000,
    retry: 1,
    enabled: enabled && normalizedTitle.length > 0,
  });
}
