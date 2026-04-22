import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";

import type { PluginTypedError } from "./useDiscover";

export type MangaChapter = {
  id: string;
  chapter: string | null;
  volume: string | null;
  title: string | null;
  language: string | null;
  pages: number | null;
  publishedAt: string | null;
  scanlator: string | null;
};

export type MangaChapterGroup = {
  pluginId: string;
  sourceName: string;
  sourceId: string;
  chapters: MangaChapter[];
  error: PluginTypedError | null;
};

const STALE_TIME_MS = 60_000;

export function useMangaChapters(itemId: string, enabled = true) {
  const normalizedItemId = itemId.trim();

  return useQuery({
    queryKey: ["manga", "chapters", normalizedItemId],
    queryFn: () =>
      invoke<MangaChapterGroup[]>("list_manga_chapters", {
        itemId: normalizedItemId,
      }),
    staleTime: STALE_TIME_MS,
    retry: 1,
    enabled: enabled && normalizedItemId.length > 0,
  });
}

export function buildMangaChapterDownloadUrl(
  pluginId: string,
  chapterId: string,
): string {
  return `mangacbz://${pluginId}/${chapterId}`;
}

export function buildMangaChapterFileName(
  mangaTitle: string,
  chapter: MangaChapter,
): string {
  const safeTitle = sanitizeForFilesystem(mangaTitle) || "manga";
  const chapterLabel = formatChapterLabel(chapter);
  return `${safeTitle} - ${chapterLabel}.cbz`;
}

export function buildMangaSubfolder(mangaTitle: string): string {
  return sanitizeForFilesystem(mangaTitle) || "manga";
}

export function formatChapterLabel(chapter: MangaChapter): string {
  const parts: string[] = [];
  if (chapter.volume && chapter.volume.trim().length > 0) {
    parts.push(`Vol ${chapter.volume.trim()}`);
  }
  if (chapter.chapter && chapter.chapter.trim().length > 0) {
    parts.push(`Ch ${chapter.chapter.trim()}`);
  }
  if (parts.length === 0) {
    parts.push(chapter.id.slice(0, 8));
  }
  return parts.join(" ");
}

function sanitizeForFilesystem(value: string): string {
  return value
    .trim()
    .replace(/[\\/:*?"<>|]+/g, "_")
    .replace(/\s+/g, " ");
}
