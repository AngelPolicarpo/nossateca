# 03 - Modules

## Mapa de modulos backend
- commands: fronteira publica para chamadas da UI.
- db: conexao e repositorios SQL.
- models: DTOs compartilhados entre comandos e UI.
- reader: leitura EPUB e saneamento de HTML.
- plugins: descoberta e execucao de componentes WASM por papel (discover, source, legacy_search).
- download: ator, fila e workers de transferencia.

## Commands
- library: add_book, list_books.
- reader: get_book_content, save_progress.
- annotations: add/get/update/delete annotation.
- discover: list_discover_catalogs, list_discover_catalog_items, get_discover_item_details, search_source_downloads.
- download: start/pause/resume/cancel/list downloads.

## Repositorios
- BookRepository: insert, find_by_hash, find_by_id, list_all.
- AnnotationRepository: insert, list_by_book, update_note, update_color, delete.

## Reader
- EpubParser extrai metadados, spine, TOC e conteudo de capitulo.
- Saneamento remove links de stylesheet que quebram no WebView.

## Discover e plugins
- PluginManager carrega wasm, identifica papel do addon e executa contrato especifico por world WIT.
- Fluxo Discover chama plugins discover para catalogo e plugins source para links de download.
- Addons sao instalados manualmente no diretorio de runtime do usuario.
- O endpoint Tauri `search_books` foi removido para consolidar o fluxo Discover-first.

## Download
- DownloadManager expoe API assincorna e delega para ator.
- Ator controla fila pending, mapa active e limites de concorrencia.
- Eventos de estado/progresso sao emitidos para a UI.

## Fronteiras de mudanca recomendadas
- Alterar contrato de comando exige atualizar docs/06-REFERENCES/01-api-contracts.md.
- Alterar tipo de modelo exige atualizar docs/06-REFERENCES/02-data-models.md.
- Alterar schema exige migracao nova e revisao de docs/03-ARCHITECTURE/04-database-schema.md.


