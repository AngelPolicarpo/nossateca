# 02 - Data Models

## Modelos de dominio principais

### Book
- id: i64
- title: String
- author: Option<String>
- format: String
- file_path: String
- file_hash: Option<String>
- status: String (unread | reading | finished)
- created_at: String

### Annotation
- id: String
- book_id: i64
- annotation_type: String
- position: String
- position_end: Option<String>
- selected_text: Option<String>
- note_text: Option<String>
- color: String
- created_at: String
- updated_at: String

### NewAnnotation
- annotation_type: String
- position: String
- position_end: Option<String>
- selected_text: Option<String>
- note_text: Option<String>
- color: Option<String>

### BookContent
- html: String
- current_chapter: usize
- total_chapters: usize
- chapter_title: String
- book_format: String
- book_file_path: Option<String>
- supports_annotations: bool

### PdfDocumentData
- bytes_base64: String
- total_pages: usize

### SearchBookResult
- id: String
- title: String
- author: Option<String>
- source: String
- format: Option<String>
- download_url: String
- score: f32

### AddonSettingEntry
- key: String
- value: String

### AddonDescriptor
- id: String
- file_name: String
- file_path: String
- role: AddonRole (discover | source | legacy_search)
- settings: AddonSettingEntry[]

### PluginTypedError
- kind: PluginErrorKind (network_failure | parsing_failure | rate_limit | not_found | unknown)
- message: String

### DiscoverCatalog
- plugin_id: String
- id: String
- name: String
- content_type: String
- genres: String[]
- supported_filters: String[]

### DiscoverCatalogItem
- plugin_id: String
- catalog_id: String
- id: String
- title: String
- author: String
- cover_url: String
- genres: String[]
- year: Option<u32>
- short_description: Option<String>
- format: Option<String>
- isbn: Option<String>

### DiscoverCatalogPageResponse
- plugin_id: String
- catalog_id: String
- items: DiscoverCatalogItem[]
- has_more: bool

### DiscoverItemDetails
- plugin_id: String
- id: String
- title: String
- author: String
- description: Option<String>
- cover_url: String
- genres: String[]
- year: Option<u32>
- format: Option<String>
- isbn: Option<String>
- origin_url: Option<String>

### SourceDownloadResult
- download_url: String
- format: String
- size: Option<String>
- language: Option<String>
- quality: Option<String>

### SourceSearchResultGroup
- plugin_id: String
- source_name: String
- source_id: String
- supported_formats: String[]
- results: SourceDownloadResult[]
- error: Option<PluginTypedError>

### DownloadRecord
- id: String
- source_url: String
- source_type: String
- file_name: String
- file_path: Option<String>
- status: String
- error_message: Option<String>
- total_bytes: Option<i64>
- downloaded_bytes: i64
- speed_bps: Option<i64>
- progress_percent: f32
- created_at: String
- started_at: Option<String>
- completed_at: Option<String>

### DownloadProgressEvent
- id: String
- file_name: String
- status: String
- downloaded_bytes: i64
- total_bytes: Option<i64>
- speed_bps: Option<i64>
- progress_percent: f32

### DownloadStateEvent
- id: String
- file_name: String
- status: String
- file_path: Option<String>
- error_message: Option<String>
- downloaded_bytes: i64
- total_bytes: Option<i64>
- speed_bps: Option<i64>
- progress_percent: f32

## ⚠️ Inconsistências encontradas
- Annotation.color e tipado como String no backend, enquanto a UI trabalha com uniao literal de cores; a validacao forte depende do comando update_annotation_color e CHECK no banco.
