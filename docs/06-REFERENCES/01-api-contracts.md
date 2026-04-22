# 01 - API Contracts

## Contratos de comandos Tauri

### Biblioteca
- add_book(file_path) -> Book
- list_books() -> Book[]
- remove_book(book_id, delete_file) -> void
- add_book aceita arquivos .epub e .pdf.

### Leitura
- get_book_content(book_id, chapter_index) -> BookContent
- get_pdf_document(book_id) -> PdfDocumentData
- resolve_epub_link_target(book_id, chapter_index, href) -> EpubLinkTarget
- save_progress(book_id, chapter_index, scroll_position?) -> void
- BookContent inclui metadados de formato: book_format, book_file_path?, supports_annotations.
- chapter_index representa capítulo (EPUB) ou página base-zero (PDF).
- EpubLinkTarget inclui chapter_index e anchor_id? para navegacao interna de links no EPUB.

### Anotacoes
- add_annotation(book_id, annotation) -> Annotation
- get_annotations(book_id) -> Annotation[]
- update_annotation_note(id, note_text) -> void
- update_annotation_color(id, color) -> void
- delete_annotation(id) -> void

### Discover
- list_discover_catalogs() -> DiscoverCatalog[]
- list_discover_catalog_items(plugin_id, catalog_id, skip?, page_size?, genre?, year?, search_query?) -> DiscoverCatalogPageResponse
- get_discover_item_details(plugin_id, item_id) -> DiscoverItemDetails
- search_source_downloads(title, author?, isbn?) -> SourceSearchResultGroup[]

### Addons
- list_addons() -> AddonDescriptor[]
- reload_addons() -> AddonDescriptor[]
- install_addon(file_path) -> AddonDescriptor
- remove_addon(addon_id) -> void
- get_addon_settings(addon_id) -> AddonSettingEntry[]
- update_addon_settings(addon_id, settings) -> void

### Downloads
- start_download(source_url, file_name?) -> DownloadRecord
- pause_download(id) -> void
- resume_download(id) -> void
- cancel_download(id) -> void
- remove_download(id, delete_file) -> void
- list_downloads() -> DownloadRecord[]

## Eventos emitidos
- download:progress
- download:state

## Convencoes
- Erros retornam String na maior parte dos comandos legados.
- Comandos Discover/Source retornam PluginTypedError (kind + message).
- IDs de dominio variam entre i64 (book) e String (annotation/download).
- O endpoint Tauri legado `search_books` foi removido; busca principal e Discover-first.

## ⚠️ Inconsistências encontradas
- O comando greet ainda esta registrado em main.rs como utilitario de exemplo e nao pertence ao dominio funcional principal.
