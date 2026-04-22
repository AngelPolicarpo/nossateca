# 02 - Project Structure

## Raiz do workspace
- lexicon: projeto ativo da aplicacao.
- docs: base de conhecimento operacional.
- IA: arquivo historico de codigo e documentos removidos do runtime.

## Frontend em lexicon/src
- App.tsx: composicao de abas, estados globais de tela e downloads.
- components/DiscoverView.tsx: catalogos Discover e painel lateral de resultados por Source plugin.
- components/LibraryView.tsx: biblioteca, filtros e abertura do leitor.
- components/ReaderView.tsx: leitura por capitulo e integracao com anotacoes.
- components/AnnotationSidebar.tsx: CRUD de nota/cor por anotacao.
- components/AddBookButton.tsx: entrada de importacao EPUB.
- components/AddonsView.tsx: instalacao e configuracao manual de addons WASM.
- hooks/useSearch.ts: hook legado (nao utilizado no fluxo Discover-first atual).
- hooks/useAddons.ts: comandos de listagem, instalacao e configuracao de addons.
- hooks/useDiscover.ts: contratos Discover/Source para catalogos, detalhes e busca de downloads por fonte.

## Backend em lexicon/src-tauri/src
- main.rs: bootstrap, estado global e registro de comandos.
- commands: fronteira publica para UI.
- commands/discover.rs: comandos de catalogo Discover e fan-out em Source plugins.
- commands/addons.rs: gerenciamento de addons instalados e configuracoes.
- db: conexao SQLite e repositorios.
- models: contratos serializaveis de entrada/saida.
- reader: parser EPUB e extracao de conteudo.
- plugins: carregamento e execucao de componentes WASM.
- download: ator de gerenciamento de fila e progresso.

## Plugins WASM em lexicon/src-tauri/plugins
- openlibrary-discover-plugin: catalogos Discover (daily, weekly, subjects, gratuitos).
- libgen-source-plugin: busca de links diretos por title/author/isbn em LibGen.
- annas-archive-source-plugin: busca de links diretos por title/author/isbn em Anna's Archive.
- libgen-li-plugin, mock-search-plugin, opds-public-plugin: plugins legados de search (compatibilidade interna).
- build-mock-plugin.sh: build de todos os plugins em plugins/dist.

## Dados e schema
- migrations: evolucao incremental de schema.
- lexicon.db: criado no app_data_dir do usuario em runtime.

## Regras de organizacao
- Feature nova deve declarar: comando, modelo e doc correspondente.
- Evitar modulo utilitario generico sem dono de dominio.
- Manter simetria entre nomenclatura de comando e tipo retornado.

