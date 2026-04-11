# Lexicon

Leitor desktop local para biblioteca pessoal, leitura EPUB, anotacoes e busca de fontes externas via plugins.

## Estado atual
- Runtime ativo sem modulo de IA dentro de lexicon.
- Todo conteudo antigo de IA foi movido para a pasta IA na raiz do workspace.
- Arquitetura principal: React + TypeScript no frontend, Tauri + Rust no backend, SQLite local.

## Inicio rapido
- Entrar em lexicon.
- Instalar dependencias do frontend com npm install.
- Rodar app com npm run tauri dev.
- Gerar build web com npm run build.

## Mapa de documentacao
- Entrada principal: docs/00-START-HERE.md.
- Contexto de produto: docs/01-PROJECT.
- Stack e estrutura: docs/02-STACK.
- Arquitetura e fluxo: docs/03-ARCHITECTURE.
- Funcionalidades: docs/04-FEATURES.
- Padroes de implementacao: docs/05-IMPLEMENTATION.
- Referencias rapidas: docs/06-REFERENCES.

## Escopo funcional
- Biblioteca: importar e listar livros EPUB.
- Leitura: abrir capitulos, navegar, salvar progresso.
- Anotacoes: criar destaque, editar nota/cor, excluir.
- Busca externa: plugins WASM e fallback controlado.
- Downloads: fila, pausa, retomada, cancelamento e eventos de progresso.

## Fora do escopo atual
- Chat contextual por livro.
- Indexacao e sumarios por IA.
- Embeddings e pipeline RAG em runtime.

## ⚠️ Inconsistências encontradas
- Mensagem de sucesso em src/components/AddBookButton.tsx ainda diz que a indexacao inicia em segundo plano, mas a indexacao de IA nao faz mais parte do runtime ativo.
- Filtro de status pronto em src/components/LibraryView.tsx depende de status indexed, mas o fluxo atual de importacao persiste discovered e nao promove para indexed no backend.
