# 09 - Discover and Source Orchestration

## Contexto
- O ciclo anterior misturava descoberta de catalogo e resolucao de download no host.
- O objetivo atual e separar papeis de plugin para manter extensibilidade e isolamento.

## Responsabilidades
- Frontend: lexicon/src/components/DiscoverView.tsx e lexicon/src/hooks/useDiscover.ts.
- Backend comandos: lexicon/src-tauri/src/commands/discover.rs.
- Runtime de plugins: lexicon/src-tauri/src/plugins/manager.rs.
- Contratos: lexicon/src-tauri/wit/discover-source-plugin.wit.
- Plugins ativos deste ciclo:
  - Discover: openlibrary-discover-plugin.
  - Source: libgen-source-plugin e annas-archive-source-plugin.

## Fluxo funcional
1. UI chama list_discover_catalogs e renderiza catalogos retornados por plugins Discover.
2. Usuario seleciona catalogo e a UI chama list_discover_catalog_items com paginacao e filtros opcionais.
3. Usuario seleciona item e a UI chama get_discover_item_details no plugin de origem.
4. Host recebe title, author e isbn do item e dispara search_source_downloads em paralelo para todos os plugins Source.
5. UI agrupa resultados por fonte na barra lateral, mantendo falhas por plugin isoladas.

## Regras de negocio
- Plugin Discover nao retorna URL de download.
- Plugin Source nao retorna catalogo.
- Item de Discover usa id no formato chave:valor preservando origem.
- Host nao usa item_id para buscar download; usa title, author e isbn.
- Erros sao tipados: network_failure, parsing_failure, rate_limit, not_found, unknown.
- URL de download em plugin Source deve ser link final direto para arquivo.

## Decisoes de arquitetura
- Contratos WIT separados por papel evitam acoplamento entre capacidades.
- Orquestracao concorrente no host garante degradacao parcial em falha de fonte.
- AddonRole (discover, source, legacy_search) permite convivencia com busca legada sem quebra.
- Bootstrap de plugins de src-tauri/plugins/dist para runtime facilita primeiro uso local.

## Testes minimos
- Carregar catalogos Discover sem plugins Source ativos.
- Selecionar item de catalogo e validar fan-out em multiplos Source plugins.
- Forcar falha em um Source plugin e validar retorno dos demais.
- Validar resultado por ISBN mais preciso que query apenas por titulo.
- Confirmar que plugin Discover retorna pagina sem resolver download.

## ⚠️ Inconsistências encontradas
- O tipo PluginTypedError e usado apenas no fluxo Discover/Source; comandos legados ainda retornam String na fronteira Tauri.
