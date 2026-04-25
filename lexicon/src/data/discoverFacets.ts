import facetRegistryJson from "./discoverFacets.json";

export type SubjectFacet = {
  slug: string;
  label_pt: string;
  group_id: string;
  group_label_pt: string;
  is_category: boolean;
  aliases: string[];
};

export type FormatFacet = {
  slug: string;
  label_pt: string;
  group_id: string;
  group_label_pt: string;
  is_category: boolean;
  aliases: string[];
};

export type AudienceFacet = {
  slug: string;
  label_pt: string;
  aliases: string[];
};

export type LanguageFacet = {
  slug: string;
  iso_code: string;
  label_pt: string;
  aliases: string[];
};

type DiscoverFacetRegistry = {
  version: number;
  subjects: SubjectFacet[];
  formats: FormatFacet[];
  audiences: AudienceFacet[];
  languages: LanguageFacet[];
};

export type SubjectGroup = {
  id: string;
  labelPt: string;
  options: SubjectFacet[];
};

export type DiscoverFacetItemLike = {
  title: string;
  format: string | null;
  genres: string[];
  shortDescription: string | null;
};

const registry = facetRegistryJson as DiscoverFacetRegistry;

const subjectBySlug = new Map(registry.subjects.map((subject) => [subject.slug, subject]));
const formatBySlug = new Map(registry.formats.map((format) => [format.slug, format]));
const languageBySlug = new Map(registry.languages.map((language) => [language.slug, language]));
const languageByCode = new Map(
  registry.languages.map((language) => [language.iso_code.toLowerCase(), language]),
);

const subjectAliasToSlug = new Map<string, string>();
for (const subject of registry.subjects) {
  subjectAliasToSlug.set(normalizeFacetToken(subject.slug), subject.slug);

  for (const alias of subject.aliases) {
    subjectAliasToSlug.set(normalizeFacetToken(alias), subject.slug);
  }
}

const formatAliasToSlug = new Map<string, string>();
for (const format of registry.formats) {
  formatAliasToSlug.set(normalizeFacetToken(format.slug), format.slug);

  for (const alias of format.aliases) {
    formatAliasToSlug.set(normalizeFacetToken(alias), format.slug);
  }
}

export const discoverFacetsVersion = registry.version;
export const subjectFacetRegistry = registry.subjects;
export const formatFacetRegistry = registry.formats;
export const audienceFacetRegistry = registry.audiences;
export const languageFacetRegistry = registry.languages;

const preferredSubjectQuickPicks = [
  "arts",
  "fiction",
  "science_fiction",
  "fantasy",
  "romance",
  "history",
  "business",
  "programming",
  "sciencemathematics",
  "juvenile_fiction",
  "textbooks",
  "biography",
  "social_science",
  "self_help",
  "music",
] as const;

const formatKeywordHints: Record<string, string[]> = {
  novel: ["novel", "romance", "fiction"],
  short_stories: ["short stories", "short_stories", "contos"],
  anthology: ["anthology", "coletanea", "coletanea"],
  poetry: ["poetry", "poesia"],
  plays: ["plays", "teatro", "drama"],
  picture_book: ["picture books", "picture book", "infancy"],
  graphic_novel: ["graphic novel", "graphic novels", "comics", "manga"],
  photography_book: ["photography", "fotografia"],
  textbook: ["textbook", "textbooks", "didatico", "didatico"],
  cookbook: ["cookbook", "cookbooks", "cooking", "receitas"],
  encyclopedia: ["encyclopedia", "enciclopedia"],
  workbook: ["workbook", "exercise", "exercicios"],
  autobiography: ["autobiography", "autobiografia"],
  memoir: ["memoir", "memorias"],
  diary_letters: ["diary", "letters", "cartas"],
};

const audienceKeywordHints: Record<string, string[]> = {
  baby_toddler_0_3: ["baby", "toddler", "infancy", "bedtime"],
  children_4_8: ["children", "kids", "juvenile", "picture books"],
  middle_grade_9_12: ["middle grade", "juvenile", "young readers"],
  young_adult_13_17: ["young adult", "ya"],
  adult: ["adult", "biography", "business", "politics", "history"],
};

const languageKeywordHints: Record<string, string[]> = {
  chinese: ["chinese", "chin", "mandarin"],
  english: ["english", "ingles"],
  french: ["french", "frances"],
  german: ["german", "alemao", "deutsch"],
  italian: ["italian", "italiano"],
  japanese: ["japanese", "japones"],
  portugues: ["portugues", "portuguese", "portugues"],
  russian: ["russian", "russo"],
  spanish: ["spanish", "espanhol"],
};

export function normalizeFacetToken(value: string): string {
  if (value.trim().length === 0) {
    return "";
  }

  let decoded = value.trim();
  try {
    decoded = decodeURIComponent(decoded);
  } catch {
    decoded = value.trim();
  }

  return decoded
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/\s+/g, "_")
    .replace(/-/g, "_")
    .replace(/_+/g, "_")
    .replace(/^_+|_+$/g, "");
}

export function resolveSubjectSlug(value: string | null | undefined): string | null {
  if (!value) {
    return null;
  }

  const normalized = normalizeFacetToken(value);
  if (!normalized) {
    return null;
  }

  const byAlias = subjectAliasToSlug.get(normalized);
  if (byAlias) {
    return byAlias;
  }

  if (normalized.startsWith("place_") && subjectBySlug.has(normalized.replace("place_", "place:"))) {
    return normalized.replace("place_", "place:");
  }

  return null;
}

export function resolveFormatSlug(value: string | null | undefined): string | null {
  if (!value) {
    return null;
  }

  const normalized = normalizeFacetToken(value);
  if (!normalized) {
    return null;
  }

  return formatAliasToSlug.get(normalized) ?? null;
}

export function getSubjectLabelPt(slug: string): string {
  return subjectBySlug.get(slug)?.label_pt ?? humanizeSlug(slug);
}

export function getFormatLabelPt(slug: string): string {
  return formatBySlug.get(slug)?.label_pt ?? humanizeSlug(slug);
}

export function getLanguageLabelPt(slug: string): string {
  return languageBySlug.get(slug)?.label_pt ?? humanizeSlug(slug);
}

export function languageCodeBySlug(slug: string): string | null {
  return languageBySlug.get(slug)?.iso_code.toLowerCase() ?? null;
}

export function languageSlugByCode(code: string): string | null {
  return languageByCode.get(code.trim().toLowerCase())?.slug ?? null;
}

export function getLanguageLabelPtByCode(code: string): string | null {
  return languageByCode.get(code.trim().toLowerCase())?.label_pt ?? null;
}

export function buildSubjectGroups(availableSlugs: string[], searchTerm: string): SubjectGroup[] {
  const search = normalizeFacetToken(searchTerm);
  const canonicalAvailability = new Set(
    availableSlugs
      .map((slug) => resolveSubjectSlug(slug))
      .filter((slug): slug is string => typeof slug === "string"),
  );

  const groupOrder = new Map<string, number>();
  const groupLabelById = new Map<string, string>();

  registry.subjects.forEach((subject, index) => {
    if (!groupOrder.has(subject.group_id)) {
      groupOrder.set(subject.group_id, index);
      groupLabelById.set(subject.group_id, subject.group_label_pt);
    }
  });

  const grouped = new Map<string, SubjectFacet[]>();

  for (const subject of registry.subjects) {
    if (!canonicalAvailability.has(subject.slug)) {
      continue;
    }

    if (search.length > 0) {
      const searchable = [subject.slug, subject.label_pt, ...subject.aliases]
        .map((value) => normalizeFacetToken(value))
        .join(" ");

      if (!searchable.includes(search)) {
        continue;
      }
    }

    const list = grouped.get(subject.group_id) ?? [];
    list.push(subject);
    grouped.set(subject.group_id, list);
  }

  const groups = Array.from(grouped.entries())
    .sort((left, right) => (groupOrder.get(left[0]) ?? 0) - (groupOrder.get(right[0]) ?? 0))
    .map(([groupId, options]) => ({
      id: groupId,
      labelPt: groupLabelById.get(groupId) ?? humanizeSlug(groupId),
      options: options.sort((left, right) => {
        if (left.is_category !== right.is_category) {
          return left.is_category ? -1 : 1;
        }

        return left.label_pt.localeCompare(right.label_pt, "pt-BR");
      }),
    }));

  return groups;
}

export function buildSubjectQuickPicks(availableSlugs: string[], limit = 10): SubjectFacet[] {
  const canonicalAvailability = new Set(
    availableSlugs
      .map((slug) => resolveSubjectSlug(slug))
      .filter((slug): slug is string => typeof slug === "string"),
  );

  const picks: SubjectFacet[] = [];

  for (const slug of preferredSubjectQuickPicks) {
    if (!canonicalAvailability.has(slug)) {
      continue;
    }

    const subject = subjectBySlug.get(slug);
    if (!subject) {
      continue;
    }

    picks.push(subject);

    if (picks.length >= limit) {
      return picks;
    }
  }

  if (picks.length >= limit) {
    return picks;
  }

  for (const subject of registry.subjects) {
    if (subject.is_category || !canonicalAvailability.has(subject.slug)) {
      continue;
    }

    if (picks.some((entry) => entry.slug === subject.slug)) {
      continue;
    }

    picks.push(subject);

    if (picks.length >= limit) {
      return picks;
    }
  }

  return picks;
}

function tokenizeItem(item: DiscoverFacetItemLike): string[] {
  const raw = [item.title, item.format ?? "", item.shortDescription ?? "", ...item.genres]
    .join(" ")
    .toLowerCase();

  return raw
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .split(/[^a-z0-9:]+/)
    .filter(Boolean);
}

function hasKeywordMatch(tokens: string[], keywords: string[]): boolean {
  if (keywords.length === 0) {
    return false;
  }

  const normalizedTokens = new Set(tokens.map((token) => normalizeFacetToken(token)));

  for (const keyword of keywords) {
    const normalizedKeyword = normalizeFacetToken(keyword);
    if (!normalizedKeyword) {
      continue;
    }

    if (normalizedTokens.has(normalizedKeyword)) {
      return true;
    }

    if (tokens.some((token) => token.includes(normalizedKeyword))) {
      return true;
    }
  }

  return false;
}

export function matchesFormatFacet(
  item: DiscoverFacetItemLike,
  selectedFormatSlug: string | null,
): boolean {
  if (!selectedFormatSlug) {
    return true;
  }

  const selected = formatBySlug.get(selectedFormatSlug);
  if (!selected) {
    return true;
  }

  const tokens = tokenizeItem(item);

  if (selected.is_category) {
    const siblingFormats = registry.formats.filter(
      (entry) => entry.group_id === selected.group_id && !entry.is_category,
    );

    return siblingFormats.some((entry) => {
      const keywords = [entry.slug, ...entry.aliases, ...(formatKeywordHints[entry.slug] ?? [])];
      return hasKeywordMatch(tokens, keywords);
    });
  }

  const keywords = [selected.slug, ...selected.aliases, ...(formatKeywordHints[selected.slug] ?? [])];
  return hasKeywordMatch(tokens, keywords);
}

export function matchesAudienceFacet(
  item: DiscoverFacetItemLike,
  selectedAudienceSlug: string | null,
): boolean {
  if (!selectedAudienceSlug) {
    return true;
  }

  const tokens = tokenizeItem(item);
  const hints = audienceKeywordHints[selectedAudienceSlug] ?? [];

  if (selectedAudienceSlug === "adult") {
    const isYouth = ["baby_toddler_0_3", "children_4_8", "middle_grade_9_12", "young_adult_13_17"]
      .flatMap((slug) => audienceKeywordHints[slug] ?? [])
      .some((hint) => hasKeywordMatch(tokens, [hint]));

    if (isYouth) {
      return false;
    }

    if (hints.length === 0) {
      return true;
    }
  }

  if (hints.length === 0) {
    return true;
  }

  return hasKeywordMatch(tokens, hints);
}

export function matchesLanguageFacet(
  item: DiscoverFacetItemLike,
  selectedLanguageSlug: string | null,
  selectedSubjectSlug: string | null,
): boolean {
  if (!selectedLanguageSlug) {
    return true;
  }

  const canonicalSelected = normalizeFacetToken(selectedLanguageSlug);

  const subjectSlug = selectedSubjectSlug ? normalizeFacetToken(selectedSubjectSlug) : "";
  if (subjectSlug === canonicalSelected) {
    return true;
  }

  const tokens = tokenizeItem(item);
  const hints = languageKeywordHints[canonicalSelected] ?? [];

  if (hints.length === 0) {
    return true;
  }

  const matched = hasKeywordMatch(tokens, hints);
  if (matched) {
    return true;
  }

  // OpenLibrary cards often omit explicit language metadata; keep non-destructive fallback.
  return tokens.length > 0;
}

export function humanizeSlug(slug: string): string {
  if (!slug) {
    return "";
  }

  return slug
    .replace(/^place:/i, "")
    .replace(/[_:]+/g, " ")
    .replace(/\s+/g, " ")
    .trim()
    .replace(/\b\w/g, (char) => char.toUpperCase());
}
