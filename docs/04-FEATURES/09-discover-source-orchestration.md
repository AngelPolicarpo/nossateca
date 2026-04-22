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
  - Source: openlibrary-source-plugin, libgen-source-plugin e annas-archive-source-plugin.

## Fluxo funcional
1. UI chama list_discover_catalogs e renderiza catalogos retornados por plugins Discover.
2. Catalogos incluem opcoes como Trending, Subjects e `Gratuitos` (Open Library com `ebook_access=public`).
3. Usuario seleciona catalogo e a UI chama list_discover_catalog_items com paginacao e filtros opcionais.
4. Quando o catalogo `Gratuitos` esta ativo, o plugin Discover aplica filtro de acesso publico em toda consulta.
5. Quando search_query e informado, o plugin Discover pode responder por busca remota global sem depender de filtro local em memoria.
6. Usuario seleciona item e a UI chama get_discover_item_details no plugin de origem.
7. Host recebe title, author e isbn do item e dispara search_source_downloads em paralelo para todos os plugins Source.
8. UI agrupa resultados por fonte na barra lateral, mantendo falhas por plugin isoladas.

## Comportamento de interface (Discover)
- Desktop (largura acima de 1080px): painel de detalhes em sidebar fixa ao lado da grade de livros.
- Tablet e mobile (largura ate 1080px): painel em drawer lateral com backdrop.
- Com painel aberto no desktop, a grade permanece totalmente interativa e o clique em outro livro troca o contexto do painel.
- Em modo drawer, o painel usa semantica modal com fechamento por Escape, clique no backdrop e botao de fechar.
- Mudanca de filtros (tipo de colecao, colecao, tema, ano e limpar filtros) fecha o painel ativo para evitar contexto obsoleto.
- Quando nenhum livro esta selecionado no desktop, a sidebar exibe estado orientativo para guiar a proxima acao do usuario.

## Regras de UX e acessibilidade do painel
- Contexto do item selecionado deve manter metadados principais visiveis no topo (titulo, autor, ano/paginas/isbn quando disponivel).
- Estados de loading, erro e vazio continuam explicitos tanto para detalhes quanto para fontes de download.
- Em layout nao modal (desktop), o foco nao e aprisionado na sidebar.
- Em layout modal (drawer), o foco permanece dentro do painel enquanto aberto.

## Regras de negocio
- Plugin Discover nao retorna URL de download.
- Plugin Source nao retorna catalogo.
- Item de Discover usa id no formato chave:valor preservando origem.
- Host nao usa item_id para buscar download; usa title, author e isbn.
- Nao existe fallback sequencial de fonte: Open Library e fontes externas rodam em paralelo no fan-out Source.
- Erros sao tipados: network_failure, parsing_failure, rate_limit, not_found, unknown.
- URL de download em plugin Source deve ser link final direto para arquivo.

## Decisoes de arquitetura
- Contratos WIT separados por papel evitam acoplamento entre capacidades.
- Orquestracao concorrente no host garante degradacao parcial em falha de fonte.
- AddonRole (discover, source, legacy_search) continua suportado para compatibilidade interna, mas o endpoint Tauri `search_books` foi removido do backend.
- Bootstrap de plugins de src-tauri/plugins/dist para runtime facilita primeiro uso local.

## Testes minimos
- Carregar catalogos Discover sem plugins Source ativos.
- Selecionar item de catalogo e validar fan-out em multiplos Source plugins.
- Forcar falha em um Source plugin e validar retorno dos demais.
- Validar resultado por ISBN mais preciso que query apenas por titulo.
- Confirmar que plugin Discover retorna pagina sem resolver download.

## ⚠️ Inconsistências encontradas
- O tipo PluginTypedError e usado apenas no fluxo Discover/Source; comandos legados ainda retornam String na fronteira Tauri.
