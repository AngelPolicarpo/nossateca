# Graph Report - /home/god/Projetos/Book  (2026-04-21)

## Corpus Check
- 73 files · ~116,813 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 670 nodes · 1178 edges · 58 communities detected
- Extraction: 86% EXTRACTED · 14% INFERRED · 0% AMBIGUOUS · INFERRED: 161 edges (avg confidence: 0.78)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- [[_COMMUNITY_Community 0|Community 0]]
- [[_COMMUNITY_Community 1|Community 1]]
- [[_COMMUNITY_Community 2|Community 2]]
- [[_COMMUNITY_Community 3|Community 3]]
- [[_COMMUNITY_Community 4|Community 4]]
- [[_COMMUNITY_Community 5|Community 5]]
- [[_COMMUNITY_Community 6|Community 6]]
- [[_COMMUNITY_Community 7|Community 7]]
- [[_COMMUNITY_Community 8|Community 8]]
- [[_COMMUNITY_Community 9|Community 9]]
- [[_COMMUNITY_Community 10|Community 10]]
- [[_COMMUNITY_Community 11|Community 11]]
- [[_COMMUNITY_Community 12|Community 12]]
- [[_COMMUNITY_Community 13|Community 13]]
- [[_COMMUNITY_Community 14|Community 14]]
- [[_COMMUNITY_Community 15|Community 15]]
- [[_COMMUNITY_Community 16|Community 16]]
- [[_COMMUNITY_Community 17|Community 17]]
- [[_COMMUNITY_Community 18|Community 18]]
- [[_COMMUNITY_Community 19|Community 19]]
- [[_COMMUNITY_Community 20|Community 20]]
- [[_COMMUNITY_Community 21|Community 21]]
- [[_COMMUNITY_Community 22|Community 22]]
- [[_COMMUNITY_Community 23|Community 23]]
- [[_COMMUNITY_Community 24|Community 24]]
- [[_COMMUNITY_Community 25|Community 25]]
- [[_COMMUNITY_Community 26|Community 26]]
- [[_COMMUNITY_Community 27|Community 27]]
- [[_COMMUNITY_Community 28|Community 28]]
- [[_COMMUNITY_Community 29|Community 29]]
- [[_COMMUNITY_Community 30|Community 30]]
- [[_COMMUNITY_Community 31|Community 31]]
- [[_COMMUNITY_Community 32|Community 32]]
- [[_COMMUNITY_Community 33|Community 33]]
- [[_COMMUNITY_Community 34|Community 34]]
- [[_COMMUNITY_Community 35|Community 35]]
- [[_COMMUNITY_Community 36|Community 36]]
- [[_COMMUNITY_Community 37|Community 37]]
- [[_COMMUNITY_Community 38|Community 38]]
- [[_COMMUNITY_Community 39|Community 39]]
- [[_COMMUNITY_Community 40|Community 40]]
- [[_COMMUNITY_Community 41|Community 41]]
- [[_COMMUNITY_Community 42|Community 42]]
- [[_COMMUNITY_Community 43|Community 43]]
- [[_COMMUNITY_Community 44|Community 44]]
- [[_COMMUNITY_Community 45|Community 45]]
- [[_COMMUNITY_Community 46|Community 46]]
- [[_COMMUNITY_Community 47|Community 47]]
- [[_COMMUNITY_Community 48|Community 48]]
- [[_COMMUNITY_Community 49|Community 49]]
- [[_COMMUNITY_Community 50|Community 50]]
- [[_COMMUNITY_Community 51|Community 51]]
- [[_COMMUNITY_Community 52|Community 52]]
- [[_COMMUNITY_Community 53|Community 53]]
- [[_COMMUNITY_Community 54|Community 54]]
- [[_COMMUNITY_Community 55|Community 55]]
- [[_COMMUNITY_Community 56|Community 56]]
- [[_COMMUNITY_Community 57|Community 57]]

## God Nodes (most connected - your core abstractions)
1. `parse_search_results()` - 18 edges
2. `PluginManager` - 16 edges
3. `get_json()` - 15 edges
4. `parse_item()` - 14 edges
5. `run_torrent_download()` - 13 edges
6. `http_get()` - 11 edges
7. `run_http_download()` - 11 edges
8. `string_field()` - 10 edges
9. `emit_state_for_download()` - 10 edges
10. `DownloadActor` - 9 edges

## Surprising Connections (you probably didn't know these)
- `main()` --calls--> `hydrate_addon_settings_from_db()`  [INFERRED]
  /home/god/Projetos/Book/lexicon/src-tauri/src/main.rs → /home/god/Projetos/Book/lexicon/src-tauri/src/commands/addons.rs
- `resolve_lexicon_data_dir()` --calls--> `resolve_downloads_dir()`  [INFERRED]
  /home/god/Projetos/Book/lexicon/src-tauri/src/storage.rs → /home/god/Projetos/Book/lexicon/src-tauri/src/download/manager.rs
- `http_get()` --calls--> `default_headers()`  [EXTRACTED]
  /home/god/Projetos/Book/lexicon/src-tauri/plugins/libgen-source-plugin/src/lib.rs → /home/god/Projetos/Book/lexicon/src-tauri/plugins/libgen-li-plugin/src/lib.rs
- `parse_search_results()` --calls--> `compute_score()`  [EXTRACTED]
  /home/god/Projetos/Book/lexicon/src-tauri/plugins/libgen-source-plugin/src/lib.rs → /home/god/Projetos/Book/lexicon/src-tauri/plugins/libgen-li-plugin/src/lib.rs
- `extract_get_href_from_html()` --calls--> `extract_raw_link_by_prefix()`  [EXTRACTED]
  /home/god/Projetos/Book/lexicon/src-tauri/plugins/libgen-source-plugin/src/lib.rs → /home/god/Projetos/Book/lexicon/src-tauri/plugins/libgen-li-plugin/src/lib.rs

## Communities

### Community 0 - "Community 0"
Cohesion: 0.05
Nodes (66): AnnotationRepository<'a>, add_annotation(), delete_annotation(), get_annotations(), update_annotation_color(), update_annotation_note(), archive_file_download_url(), archive_legacy_download_url() (+58 more)

### Community 1 - "Community 1"
Cohesion: 0.06
Nodes (48): BookRepository<'a>, append_fragment(), EpubMetadata, EpubParser, extract_fragment(), fallback_title_from_path(), guess_mime_from_path(), insert_navigation_path_variants() (+40 more)

### Community 2 - "Community 2"
Cohesion: 0.07
Nodes (46): DownloadProgressEvent, DownloadRecord, DownloadStateEvent, StartDownloadRequest, ActiveDownload, cancel_torrent_in_session(), collect_files_recursively_limited(), derive_file_name() (+38 more)

### Community 3 - "Community 3"
Cohesion: 0.07
Nodes (48): absolutize_url(), AnnasArchiveSourcePlugin, build_query(), build_search_attempts(), CandidateDownload, clean_text_from_node(), collapse_whitespace(), compute_score() (+40 more)

### Community 4 - "Community 4"
Cohesion: 0.07
Nodes (36): build_storage_key(), delete_addon_settings(), derive_target_file_name(), get_addon_settings(), hydrate_addon_settings_from_db(), install_addon(), list_addons(), load_all_addon_settings() (+28 more)

### Community 5 - "Community 5"
Cohesion: 0.08
Nodes (22): buildSubjectGroups(), getFormatLabelPt(), getLanguageLabelPt(), getSubjectLabelPt(), hasKeywordMatch(), humanizeSlug(), matchesAudienceFacet(), matchesFormatFacet() (+14 more)

### Community 6 - "Community 6"
Cohesion: 0.07
Nodes (9): clampNumber(), decodeEpubFragment(), findEpubAnchorTarget(), getAdaptiveContentWidth(), getErrorMessage(), isPdfWorkerModuleScriptError(), readStoredNumber(), readStoredValue() (+1 more)

### Community 7 - "Community 7"
Cohesion: 0.08
Nodes (10): getErrorMessage(), openAddDownloadPrompt(), resolveInitialReaderShortcuts(), isShortcutEventMatch(), normalizeKeyName(), normalizeShortcutValue(), parseShortcutBinding(), parseShortcutBindings() (+2 more)

### Community 8 - "Community 8"
Cohesion: 0.13
Nodes (12): getAddonName(), getErrorMessage(), handleInstallAddon(), handleReloadAddons(), handleRemoveAddon(), handleSaveSettings(), removeExtension(), toStartCase() (+4 more)

### Community 9 - "Community 9"
Cohesion: 0.27
Nodes (8): aggregates_multiple_plugins_and_keeps_unique_results(), dedup_key(), deduplication_prefers_higher_ranked_result(), format_bonus(), normalize_for_dedup(), ranking_boosts_preferred_formats(), resolve_plugin_timeout(), SearchOrchestrator

### Community 10 - "Community 10"
Cohesion: 0.17
Nodes (8): executeBookRemoval(), formatBookDate(), getBookStatusLabel(), getBookStatusToneClass(), getErrorMessage(), normalizeBookStatus(), normalizeText(), toComparableDate()

### Community 11 - "Community 11"
Cohesion: 0.12
Nodes (3): Cover(), pal(), MangaReader()

### Community 12 - "Community 12"
Cohesion: 0.14
Nodes (14): 128x128.png, 128x128@2x.png (256x256), 32x32.png, icon.png (512x512), Square107x107Logo.png, Square142x142Logo.png, Square150x150Logo.png, Square284x284Logo.png (+6 more)

### Community 13 - "Community 13"
Cohesion: 0.24
Nodes (9): init_db(), AppState, main(), canonicalize_data_dir(), copy_dir_recursive(), migrate_legacy_data_dir(), move_path_with_fallback(), move_selected_legacy_entries() (+1 more)

### Community 14 - "Community 14"
Cohesion: 0.36
Nodes (8): closeMenu(), findFirstEnabledIndex(), findLastEnabledIndex(), handlePointerDown(), handleTriggerKeyDown(), handleWindowBlur(), openMenu(), selectIndex()

### Community 15 - "Community 15"
Cohesion: 0.2
Nodes (9): DiscoverCatalog, DiscoverCatalogItem, DiscoverCatalogPageResponse, DiscoverItemDetails, PluginErrorKind, PluginTypedError, SourceDownloadResult, SourcePluginInfo (+1 more)

### Community 16 - "Community 16"
Cohesion: 0.29
Nodes (6): BookContent, EpubLinkTarget, EpubSearchMatch, EpubSearchResponse, PdfDocumentData, ReadingProgressData

### Community 17 - "Community 17"
Cohesion: 0.29
Nodes (0): 

### Community 18 - "Community 18"
Cohesion: 0.29
Nodes (0): 

### Community 19 - "Community 19"
Cohesion: 0.33
Nodes (0): 

### Community 20 - "Community 20"
Cohesion: 0.4
Nodes (0): 

### Community 21 - "Community 21"
Cohesion: 0.5
Nodes (3): AddonDescriptor, AddonRole, AddonSettingEntry

### Community 22 - "Community 22"
Cohesion: 0.67
Nodes (2): Annotation, NewAnnotation

### Community 23 - "Community 23"
Cohesion: 1.0
Nodes (2): useDebouncedValue(), useSearch()

### Community 24 - "Community 24"
Cohesion: 0.67
Nodes (0): 

### Community 25 - "Community 25"
Cohesion: 0.67
Nodes (0): 

### Community 26 - "Community 26"
Cohesion: 1.0
Nodes (0): 

### Community 27 - "Community 27"
Cohesion: 1.0
Nodes (1): Book

### Community 28 - "Community 28"
Cohesion: 1.0
Nodes (1): BookRepository

### Community 29 - "Community 29"
Cohesion: 1.0
Nodes (1): AnnotationRepository

### Community 30 - "Community 30"
Cohesion: 1.0
Nodes (0): 

### Community 31 - "Community 31"
Cohesion: 1.0
Nodes (0): 

### Community 32 - "Community 32"
Cohesion: 1.0
Nodes (0): 

### Community 33 - "Community 33"
Cohesion: 1.0
Nodes (0): 

### Community 34 - "Community 34"
Cohesion: 1.0
Nodes (0): 

### Community 35 - "Community 35"
Cohesion: 1.0
Nodes (0): 

### Community 36 - "Community 36"
Cohesion: 1.0
Nodes (0): 

### Community 37 - "Community 37"
Cohesion: 1.0
Nodes (0): 

### Community 38 - "Community 38"
Cohesion: 1.0
Nodes (0): 

### Community 39 - "Community 39"
Cohesion: 1.0
Nodes (0): 

### Community 40 - "Community 40"
Cohesion: 1.0
Nodes (0): 

### Community 41 - "Community 41"
Cohesion: 1.0
Nodes (2): index.html, vite.svg

### Community 42 - "Community 42"
Cohesion: 1.0
Nodes (0): 

### Community 43 - "Community 43"
Cohesion: 1.0
Nodes (0): 

### Community 44 - "Community 44"
Cohesion: 1.0
Nodes (0): 

### Community 45 - "Community 45"
Cohesion: 1.0
Nodes (0): 

### Community 46 - "Community 46"
Cohesion: 1.0
Nodes (0): 

### Community 47 - "Community 47"
Cohesion: 1.0
Nodes (0): 

### Community 48 - "Community 48"
Cohesion: 1.0
Nodes (0): 

### Community 49 - "Community 49"
Cohesion: 1.0
Nodes (0): 

### Community 50 - "Community 50"
Cohesion: 1.0
Nodes (0): 

### Community 51 - "Community 51"
Cohesion: 1.0
Nodes (0): 

### Community 52 - "Community 52"
Cohesion: 1.0
Nodes (0): 

### Community 53 - "Community 53"
Cohesion: 1.0
Nodes (1): Discover Facet Registry JSON

### Community 54 - "Community 54"
Cohesion: 1.0
Nodes (1): Discover Facets JSON Registry

### Community 55 - "Community 55"
Cohesion: 1.0
Nodes (1): README.md

### Community 56 - "Community 56"
Cohesion: 1.0
Nodes (1): tauri.svg

### Community 57 - "Community 57"
Cohesion: 1.0
Nodes (1): react.svg

## Knowledge Gaps
- **43 isolated node(s):** `CandidateDownload`, `ArchiveDownloadCandidate`, `RankedArchiveDownloadCandidate`, `DiscoverFacetRegistry`, `SubjectFacetEntry` (+38 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **Thin community `Community 26`** (2 nodes): `main()`, `build.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 27`** (2 nodes): `Book`, `book.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 28`** (2 nodes): `BookRepository`, `book_repository.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 29`** (2 nodes): `AnnotationRepository`, `annotation_repository.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 30`** (2 nodes): `cn()`, `cn.ts`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 31`** (2 nodes): `AnnotationSidebar()`, `AnnotationSidebar.tsx`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 32`** (2 nodes): `AddBookButton()`, `AddBookButton.tsx`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 33`** (2 nodes): `Button()`, `Button.tsx`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 34`** (2 nodes): `Input.tsx`, `Input()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 35`** (2 nodes): `ToggleChip.tsx`, `ToggleChip()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 36`** (2 nodes): `Panel.tsx`, `Panel()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 37`** (2 nodes): `StateMessage.tsx`, `StateMessage()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 38`** (2 nodes): `hashString()`, `BookCover.tsx`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 39`** (2 nodes): `Badge()`, `Badge.tsx`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 40`** (2 nodes): `EmptyState()`, `EmptyState.tsx`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 41`** (2 nodes): `index.html`, `vite.svg`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 42`** (1 nodes): `vite.config.ts`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 43`** (1 nodes): `mod.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 44`** (1 nodes): `mod.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 45`** (1 nodes): `mod.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 46`** (1 nodes): `mod.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 47`** (1 nodes): `mod.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 48`** (1 nodes): `mod.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 49`** (1 nodes): `mod.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 50`** (1 nodes): `mod.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 51`** (1 nodes): `main.tsx`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 52`** (1 nodes): `vite-env.d.ts`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 53`** (1 nodes): `Discover Facet Registry JSON`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 54`** (1 nodes): `Discover Facets JSON Registry`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 55`** (1 nodes): `README.md`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 56`** (1 nodes): `tauri.svg`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 57`** (1 nodes): `react.svg`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `main()` connect `Community 13` to `Community 0`, `Community 2`, `Community 3`, `Community 4`?**
  _High betweenness centrality (0.028) - this node is a cross-community bridge._
- **Why does `parse_search_results()` connect `Community 3` to `Community 0`?**
  _High betweenness centrality (0.018) - this node is a cross-community bridge._
- **Why does `PluginManager` connect `Community 4` to `Community 9`?**
  _High betweenness centrality (0.015) - this node is a cross-community bridge._
- **What connects `CandidateDownload`, `ArchiveDownloadCandidate`, `RankedArchiveDownloadCandidate` to the rest of the system?**
  _43 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Community 0` be split into smaller, more focused modules?**
  _Cohesion score 0.05 - nodes in this community are weakly interconnected._
- **Should `Community 1` be split into smaller, more focused modules?**
  _Cohesion score 0.06 - nodes in this community are weakly interconnected._
- **Should `Community 2` be split into smaller, more focused modules?**
  _Cohesion score 0.07 - nodes in this community are weakly interconnected._