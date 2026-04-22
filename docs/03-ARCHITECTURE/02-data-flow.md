# 02 - Data Flow

## Fluxo de biblioteca
1. UI solicita add_book com caminho EPUB ou PDF.
2. Backend valida arquivo, calcula hash e extrai metadados.
3. Repository persiste em books com status unread.
4. UI atualiza lista via list_books.

## Fluxo de leitura
1. UI chama get_book_content por book_id e chapter_index.
2. Backend carrega livro e decide caminho por formato.
3. EPUB: parser resolve spine e retorna HTML com metadados do capítulo.
4. PDF: backend retorna metadados de leitura e UI chama get_pdf_document para bytes + total de páginas.
5. UI renderiza conteúdo e chama save_progress.
6. save_progress atualiza reading_progress e sincroniza books.status para reading/finished.

## Fluxo de anotacoes
1. UI cria/edita/exclui anotacao via comandos dedicados.
2. Backend valida livro/cor e persiste em annotations.
3. UI recarrega anotacoes e sincroniza destaque visual.

## Fluxo de discover (catalogo)
1. UI chama list_discover_catalogs.
2. Host agrega catalogos de plugins discover instalados.
3. UI chama list_discover_catalog_items com plugin_id, catalog_id, skip, page_size e filtros opcionais.
4. Plugin discover retorna pagina de itens sem resolver download.

## Fluxo de source (downloads por fonte)
1. UI seleciona item e chama get_discover_item_details.
2. Host extrai title, author e isbn dos detalhes.
3. Host dispara search_source_downloads em paralelo para todos os plugins source.
4. Cada fonte retorna lista propria de links diretos ou erro tipado.
5. UI exibe resultados agrupados por fonte na barra lateral.

## Fluxo de busca Discover-first
1. UI dispara busca global no Discover via list_discover_catalog_items com search_query.
2. Plugin Discover executa consulta remota e retorna itens paginados.
3. Usuario seleciona item e host dispara search_source_downloads em paralelo.
4. UI apresenta resultados agrupados por fonte para iniciar download.

## Fluxo de download
1. UI envia start_download com source_url.
2. DownloadManager cria registro queued em downloads.
3. Ator inicia worker respeitando max_concurrent.
4. Worker emite download:progress e download:state.
5. UI atualiza progresso e acoes de pausa/retomada/cancelamento.

## Persistencia de configuracao
- Configuracoes por addon sao salvas em user_settings com prefixo addon::<id>::<chave>.
- Na inicializacao, valores sao hidratados para o PluginManager e enviados para o contrato correspondente do addon.
- plugin_role pode ser forçado por configuracao (role ou plugin_role) quando necessario.

## Pontos de falha tratados
- Erro de plugin gera log e segue sem derrubar fluxo principal.
- Falha em um Source plugin nao interrompe resposta dos demais.
- Falha de download altera estado para failed/cancelled sem travar app.
- Erro de banco retorna mensagem para UI via Result<String>.

## ⚠️ Inconsistências encontradas
- user_settings nasce em migracao historica 003 ligada ao periodo de IA, mas continua sendo usada legitimamente pela configuracao de busca atual.
- O endpoint Tauri legado search_books foi removido; qualquer referencia a esse fluxo deve ser tratada como historica.
