# 05 - Plugin System

## Objetivo
Permitir extensao por plugins com papeis isolados, sem acoplar catalogo e resolucao de download ao core principal.

## Componentes principais
- Runtime: plugins/manager.rs.
- Orquestracao Discover/Source: commands/discover.rs.
- Contratos: wit/discover-source-plugin.wit.
- Comandos de addon: commands/addons.rs.
- UI Discover: src/components/DiscoverView.tsx.
- UI de addon: src/components/AddonsView.tsx.

## Papeis de plugin
- Discover: list-catalogs, list-catalog-items, get-item-details.
- Source: get-source-info, find-downloads.
- Legacy Search: papel mantido apenas para compatibilidade interna, sem endpoint Tauri publico.

## Fluxo de execucao
1. Usuario abre Discover e host chama list_discover_catalogs.
2. Plugins Discover retornam catalogos e itens paginados por list_discover_catalog_items.
3. Usuario seleciona item e host chama get_discover_item_details.
4. Host dispara search_source_downloads em paralelo para todos os plugins Source instalados.
5. UI agrupa resultados por fonte e permite enfileirar download.
6. O endpoint Tauri legado `search_books` foi removido para consolidar o fluxo Discover-first.

## Configuracao
- Configuracoes sao persistidas em user_settings por addon.
- Chaves seguem prefixo addon::<addon_id>::<chave>.
- Valores sao enviados para o addon no contrato correspondente.
- role/plugin_role em settings pode forcar papel do addon quando necessario.

## Regras de resiliencia
- Falha de addon nao interrompe busca completa.
- Timeout por addon evita travamento global.
- Falha em um Source plugin nao afeta resposta dos outros.
- Fallback mock existe apenas em ambiente de desenvolvimento.

## Decisoes de arquitetura
- Wasm em sandbox reduz risco de acoplamento e impacto de falha.
- Contratos separados (Discover vs Source) evitam mistura de responsabilidades.
- Snapshot evita disputa de lock durante execucao concorrente.
- Core nao integra fonte externa de forma fixa; plugins sao intercambiaveis por contrato.

## ⚠️ Inconsistências encontradas
- Ainda existem estruturas internas de compatibilidade para Legacy Search no runtime, mesmo com a fronteira Tauri consolidada em Discover-first.
