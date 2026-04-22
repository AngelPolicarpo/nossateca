# 10 - Search Option Implementation Handoff

## Contexto
Este arquivo registra o estado atual da implementacao da feature Search Option (Busca Geral) com duas decisoes obrigatorias:
- Nao existe fallback sequencial entre fontes.
- Todos os plugins relevantes de fonte rodam em paralelo no fluxo Discover.

Tambem foi removido o Home legado da UI para que a busca ocorra apenas em Discover.

## Decisoes aplicadas neste ciclo
- Busca principal agora e Discover-first.
- Home legado removido da navegacao e da renderizacao.
- Open Library passou a ter plugin de Source proprio para participar do fan-out paralelo com LibGen e Anna's Archive.
- Busca global do Discover passa por consulta remota no backend/plugin (nao mais filtro local de pagina).

## Mudancas implementadas

### 1) Remocao do Home legado
Arquivo: `nossateca/src/App.tsx`

Mudancas principais:
- Removida aba `home` do tipo `AppTab` e do `navItems`.
- `activeTab` inicial alterado para `discover`.
- Removido fluxo de busca legado da home (`useSearch`, `semanticSearch`, lista de resultados, botao de fila da busca antiga).
- Removido bloco completo de renderizacao `activeTab === "home"`.
- Mantidos fluxos Discover, Library, Downloads e Addons.

Resultado:
- A interface nao apresenta mais Home legado.
- Entrada principal do app agora cai direto em Discover.

### 2) Novo plugin Source da Open Library
Novos arquivos:
- `nossateca/src-tauri/plugins/openlibrary-source-plugin/Cargo.toml`
- `nossateca/src-tauri/plugins/openlibrary-source-plugin/src/lib.rs`

Comportamento do plugin:
- Contrato: `source-plugin` (WIT `discover-source-plugin.wit`).
- `get_source_info` retorna `source_id=openlibrary`.
- `find_downloads` consulta `https://openlibrary.org/search.json` via `host_http::send_http_request`.
- Campos usados em `fields`:
  - `key,title,author_name,language,ia,public_scan_b,editions.ebook_access,has_fulltext,availability`
- Regra de filtro para retorno:
  - somente docs com `ia` presente
  - e `ebook_access` publico (top-level ou em `editions.docs`)
- Link de download montado:
  - Resolvido via `archive.org/metadata/{ia_id}` com selecao de arquivo real (pdf/epub/mobi).
  - Fallback legado para `https://archive.org/download/{ia_id}/{ia_id}.pdf` apenas se metadata nao trouxer candidato.
- Formato retornado atualmente: `pdf`.
- Resultado vazio retorna `PluginError::NotFound`.

Correcao posterior neste ciclo:
- Ajustado filtro/publicacao para evitar falso `not_found` no Source plugin:
  - `fields` agora inclui `ebook_access`.
  - Query inclui `ebook_access=public`.
  - Validacao de publico considera `ebook_access`, `public_scan_b` e `availability`.
- Ajustado resolvedor de URL para evitar 401/404 em fila de download:
  - URL de download passa a usar nome de arquivo retornado por metadata do Internet Archive, em vez de presumir sempre `{ia_id}.pdf`.
  - Parser de formato por metadata passou a usar tokenizacao exata para evitar falso positivo (ex.: `RePublisher ... log` nao e mais classificado como `epub`).
  - Candidatos criptografados/DRM foram excluidos da selecao (`.lcp*`, `_lcp`, `encrypted`, `acsm`, ou format hint com `lcp/encrypted/drm`) para evitar arquivos invalidos no Reader (`Invalid file header`).
  - O loop de selecao agora percorre todos os `ia[]` do doc (nao para no primeiro), permitindo expor multiplas opcoes validas por obra.
  - Cada URL candidata e validada com `HEAD`; links que retornam 401/4xx sao descartados antes de aparecer na UI/fila.

### 3) Plugin compilado para runtime
Artefato gerado:
- `nossateca/src-tauri/plugins/dist/openlibrary-source-plugin.wasm`

Com isso, o runtime passa a carregar esse plugin junto dos outros Source plugins, e a busca de fontes no Discover continua em paralelo pelo comando existente `search_source_downloads`.

### 4) Busca global do Discover agora remota
Arquivos alterados:
- `nossateca/src/hooks/useDiscover.ts`
- `nossateca/src/components/DiscoverView.tsx`
- `nossateca/src-tauri/src/commands/discover.rs`
- `nossateca/src-tauri/src/plugins/manager.rs`
- `nossateca/src-tauri/wit/discover-source-plugin.wit`
- `nossateca/src-tauri/plugins/openlibrary-discover-plugin/src/lib.rs`

Mudancas principais:
- Adicionado parametro opcional `search_query` no contrato `list_discover_catalog_items`.
- Frontend passou a enviar `searchQuery` debounced para o comando Discover.
- O input global da tela Discover deixou de filtrar apenas `items` locais e passou a acionar busca remota.
- Plugin `openlibrary-discover-plugin` agora usa `/search.json` quando `search_query` e informado.
- Resultado da grade no Discover agora vem diretamente da resposta remota paginada.

### 5) Indicacao visual de origem publica vs externa no painel
Arquivo alterado:
- `nossateca/src/components/DiscoverView.tsx`

Mudancas principais:
- Cada grupo de resultados na secao "Onde encontrar" agora mostra badge de categoria da fonte.
- Fontes Open Library aparecem como `Publico` com hint de acesso aberto.
- Fontes externas aparecem como `Externa` com hint de fonte comunitaria.

Resultado:
- O usuario consegue distinguir rapidamente resultados de acesso publico versus fontes externas.

### 6) Remocao do caminho legado `search_books` no backend
Arquivos alterados:
- `nossateca/src-tauri/src/main.rs`
- `nossateca/src-tauri/src/commands/mod.rs`
- `nossateca/src-tauri/src/commands/search.rs` (removido)

Mudancas principais:
- Comando Tauri `commands::search::search_books` removido do `invoke_handler`.
- Modulo de comando `search` removido de `commands/mod.rs`.
- Arquivo de comando legado excluido para evitar reintroducao acidental da rota.

Resultado:
- O backend nao expoe mais rota de busca legado; fronteira de busca agora e Discover-first.

### 7) Nova opcao de catalogo `Gratuitos`
Arquivo alterado:
- `nossateca/src-tauri/plugins/openlibrary-discover-plugin/src/lib.rs`

Mudancas principais:
- Novo catalogo Discover `openlibrary:free` com nome de exibicao `Gratuitos`.
- O catalogo chama `search.json` com `ebook_access=public`.
- Filtro defensivo no parser garante retorno apenas de docs com `ebook_access` publico (top-level ou em `editions.docs`).
- Busca global dentro desse catalogo continua ativa, preservando o filtro de gratuidade.

### 8) Remocao da priorizacao de idioma no Open Library Discover
Arquivo alterado:
- `nossateca/src-tauri/plugins/openlibrary-discover-plugin/src/lib.rs`

Mudancas principais:
- A selecao de edicao preferida deixou de usar ranking de idioma.
- Foi removida a regra implicita que priorizava `por` e depois fazia fallback para `eng`.
- A escolha da edicao agora considera apenas completude de metadados (titulo, paginas, ISBN).
- O catalogo `openlibrary:subjects` deixou de declarar `language` em `supported_filters`.
- O campo `language` foi removido de `SEARCH_FIELDS` no plugin Discover da Open Library.
- Titulo, autor e descricao no Discover agora priorizam metadados do `work` da Open Library.
- Metadados de `edition` sao usados apenas como complemento, sem sobrescrever o titulo original selecionado.

Resultado:
- Nao ha mais tentativa implicita de "buscar traducao" por idioma no fluxo Discover.
- Nao ha mais fallback para ingles na escolha de edicoes do Open Library.

### 9) Normalizacao de metadados no painel de fontes (LibGen + Open Library)
Arquivos alterados:
- `nossateca/src-tauri/plugins/libgen-source-plugin/src/lib.rs`
- `nossateca/src-tauri/plugins/openlibrary-source-plugin/src/lib.rs`
- `nossateca/src/components/DiscoverView.tsx`

Mudancas principais:
- Corrigido mapeamento de colunas do LibGen (`Language`, `Pages`, `Size`) para evitar troca indevida entre idioma e tamanho.
- Payload de `quality` dos plugins Source passou a usar formato estruturado:
  - `pages:<n>|name:<titulo>` (quando houver paginas)
  - `name:<titulo>`
- Removidos textos tecnicos desnecessarios no Open Library (`Open Library (public)` e identificador `IA ...`) da exibicao para usuario.
- Frontend passa a interpretar o `quality` estruturado e exibir metadados no padrao:
  - idioma (linha principal)
  - paginas, tamanho e nome (linha secundaria)

Resultado:
- A secao "Onde encontrar" fica consistente entre fontes e sem ruido tecnico para o usuario final.

### 10) Mitigacao de trap WASM em `find_downloads` (Open Library + LibGen)
Arquivos alterados:
- `nossateca/src-tauri/src/plugins/manager.rs`
- `nossateca/src-tauri/plugins/openlibrary-source-plugin/src/lib.rs`
- `nossateca/src-tauri/plugins/libgen-source-plugin/src/lib.rs`

Mudancas principais:
- Runtime host agora aplica piso seguro para `nossateca_PLUGIN_FUEL`:
  - valor minimo forçado para `80_000_000` (com log quando o env vier abaixo disso).
- Open Library Source recebeu limites defensivos para reduzir custo por consulta:
  - limite de `ia[]` por doc (`MAX_IA_IDS_PER_DOC`)
  - limite de arquivos em metadata do Archive (`MAX_ARCHIVE_FILES_TO_SCAN`)
  - selecao de melhor candidato em passagem unica (sem coletar/sortear toda a lista)
  - logs em falhas de request/parsing/status para facilitar debug.
- LibGen Source recebeu limites de varredura e resolucao custosa:
  - limite de linhas processadas (`MAX_ROWS_TO_SCAN`)
  - limite de resolucoes de download que exigem paginas auxiliares (`MAX_EXPENSIVE_RESOLUTIONS=5`)
  - quando o budget de resolucao acaba, o plugin nao tenta mais resolver `ads.php` por caminhos alternativos.
  - fallback para candidatos inline diretos (`get.php`) quando o budget de resolucao acaba.
  - extracao de `href` em paginas auxiliares agora usa parser HTML (`a[href]`) em vez de slicing manual por bytes.
  - tentativa ISBN-first foi desativada quando titulo esta presente (ISBN segue sendo usado para priorizacao).

Resultado:
- Reducao do risco de `error while executing at wasm backtrace` por exaustao de fuel em consultas Source.
- Falhas agora tendem a degradar para `NotFound`/`NetworkFailure` com logs, em vez de trap opaco.
- Caso real reproduzido (`Rich Dad, Poor Dad`, `Robert T. Kiyosaki`, `9781533221827`) deixou de estourar em `OutOfFuel` com o piso atual de fuel.

### 11) Busca do Discover movida para topbar global
Arquivos alterados:
- `nossateca/src/App.tsx`
- `nossateca/src/components/DiscoverView.tsx`
- `nossateca/src/App.css`

Mudancas principais:
- O campo `Buscar por titulo, autor ou ISBN` saiu do bloco local de filtros do Discover e foi movido para o header global (`lx-topbar`).
- O estado de texto da busca passou a ser controlado no `App.tsx` e propagado para `DiscoverView` por props.
- A navegacao para `Descubra` em outras abas agora ocorre somente ao pressionar `Enter` no campo da topbar.
- Ao sair da aba `Descubra`, o texto de busca global e limpo.
- A consulta remota continua no mesmo fluxo Discover-first (`list_discover_catalog_items` com `search_query`), sem mudanca de backend.

Resultado:
- A busca fica acessivel no shell global da aplicacao, mantendo o comportamento remoto do Discover.
- O usuario pode iniciar a busca de qualquer tela e abrir o Discover sob demanda com `Enter`.

### 12) Redesign do header `dc-filters-strip` (compacto + secoes separadas)
Arquivos alterados:
- `nossateca/src/components/DiscoverView.tsx`
- `nossateca/src/components/DiscoverView.css`

Mudancas principais:
- A linha principal de filtros foi simplificada para tres blocos: `Tipo` (select), `Colecao` (select) e `Limpar tudo`.
- Os chips de tipo foram removidos da area principal para reduzir ruido visual e melhorar consistencia com toolbars da Biblioteca.
- O botao `Limpar tudo` passou a manter posicao estavel na UI (sempre visivel, desabilitado quando nao ha filtros ativos).
- Os filtros avancados foram separados em uma segunda faixa visual (`Filtros avancados`) com resumo de estado sempre visivel.
- O painel avancado continua expansivel, mas com hierarquia mais clara entre cabecalho e conteudo.
- Abertura de filtros avancados agora move foco para o primeiro campo editavel, melhorando navegacao por teclado.

Resultado:
- Header do Discover mais limpo e previsivel, com menor densidade de botoes na area primaria.
- Melhor alinhamento com o design system existente (altura de controles, bordas, espacamentos e padrao de grupos).
- Mantida compatibilidade funcional com o fluxo remoto existente de Discover (sem alteracao de backend).

JSON esperado (exemplo)
```json
{
  "author_name": ["Machado de Assis"],
  "ebook_access": "public",
  "has_fulltext": true,
  "ia": ["bwb_P9-BIH-494"],
  "title": "Dom Casmurro"
}
```

Link de download derivado do JSON (`ia[0]`):
- `https://archive.org/download/bwb_P9-BIH-494/bwb_P9-BIH-494.pdf`

## Validacao executada
- Build frontend (`npm run build`) executado com sucesso.
- `cargo check` no backend executado com sucesso.
- Build do plugin `openlibrary-discover-plugin` (`wasm32-wasip2 --release`) executado com sucesso.
- Artefato copiado para `nossateca/src-tauri/plugins/dist/openlibrary-discover-plugin.wasm`.
- Build do plugin `openlibrary-source-plugin` (`wasm32-wasip2 --release`) executado com sucesso apos fix de URL por metadata.
- Artefato copiado para `nossateca/src-tauri/plugins/dist/openlibrary-source-plugin.wasm`.
- Build do plugin `libgen-source-plugin` (`wasm32-wasip2 --release`) executado com sucesso.
- Reproducao controlada com `nossateca_PLUGIN_FUEL=1000000`:
  - antes: `source find_downloads failed: error while executing at wasm backtrace`
  - depois: runtime ajusta fuel para `80_000_000` e os plugins Source executam sem trap.
- Reproducao dirigida com payload real de Discover (`Rich Dad, Poor Dad` + ISBN):
  - antes da mitigacao final: `OutOfFuel` ate `120_000_000`.
  - depois da mitigacao final no LibGen: sucesso com `80_000_000` (`ok: 5`) e sucesso com env `50_000_000` (clamp para `80_000_000`).
- Validacao de erros de editor sem erros nos arquivos alterados.

## O que ja garante paralelo sem fallback sequencial
- O comando backend `search_source_downloads` (em `nossateca/src-tauri/src/commands/discover.rs`) ja executa todos os Source plugins em paralelo com `JoinSet`.
- Ao adicionar `openlibrary-source-plugin` ao conjunto de plugins Source, Open Library participa do mesmo fan-out paralelo de LibGen/Anna.

## Pendencias recomendadas para o proximo ciclo
1. Revisar se a busca remota deve ignorar filtros de catalogo em todos os plugins Discover (hoje implementado no Open Library).
2. Cobrir com testes automatizados os cenarios de busca remota, vazio, erro e paginacao no Discover.

## Comandos uteis para retomar
- Build plugin novo:
  - `cd nossateca/src-tauri && cargo build --manifest-path plugins/openlibrary-source-plugin/Cargo.toml --target wasm32-wasip2 --release`
- Copiar para dist:
  - `cp nossateca/src-tauri/plugins/openlibrary-source-plugin/target/wasm32-wasip2/release/openlibrary_source_plugin.wasm nossateca/src-tauri/plugins/dist/openlibrary-source-plugin.wasm`
- Build geral de plugins:
  - `cd nossateca/src-tauri/plugins && ./build-mock-plugin.sh`

## Arquivos alterados neste ciclo
- `nossateca/src/App.tsx`
- `nossateca/src-tauri/src/main.rs`
- `nossateca/src-tauri/src/commands/mod.rs`
- `nossateca/src-tauri/src/commands/search.rs` (removido)
- `nossateca/src-tauri/plugins/openlibrary-source-plugin/Cargo.toml`
- `nossateca/src-tauri/plugins/openlibrary-source-plugin/src/lib.rs`
- `nossateca/src-tauri/plugins/dist/openlibrary-source-plugin.wasm`
- `nossateca/src-tauri/plugins/dist/openlibrary-discover-plugin.wasm`
- `nossateca/src/hooks/useDiscover.ts`
- `nossateca/src/components/DiscoverView.tsx`
- `nossateca/src-tauri/src/commands/discover.rs`
- `nossateca/src-tauri/src/plugins/manager.rs`
- `nossateca/src-tauri/wit/discover-source-plugin.wit`
- `nossateca/src-tauri/plugins/openlibrary-discover-plugin/src/lib.rs`
- `docs/04-FEATURES/10-search-option-implementation-handoff.md`
