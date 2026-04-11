import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";

export type SearchBookResult = {
  id: string;
  title: string;
  author: string | null;
  source: string;
  format: string | null;
  download_url: string;
  score: number;
};

function useDebouncedValue(value: string, delayMs: number) {
  const [debounced, setDebounced] = useState(value);

  useEffect(() => {
    const timerId = window.setTimeout(() => {
      setDebounced(value);
    }, delayMs);

    return () => window.clearTimeout(timerId);
  }, [value, delayMs]);

  return debounced;
}

export function useSearch(query: string) {
  const normalizedQuery = query.trim();
  const debouncedQuery = useDebouncedValue(normalizedQuery, 400);

  return useQuery({
    queryKey: ["search-books", debouncedQuery],
    queryFn: async () => {
      if (debouncedQuery.length === 0) {
        return [] as SearchBookResult[];
      }

      return invoke<SearchBookResult[]>("search_books", { query: debouncedQuery });
    },
    enabled: debouncedQuery.length >= 2,
    staleTime: 30_000,
    retry: 1,
  });
}
