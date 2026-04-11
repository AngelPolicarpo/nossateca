# 01 - Library Management

## Objetivo
Gerenciar entrada e consulta da biblioteca local de livros.

## Componentes principais
- Frontend: components/AddBookButton.tsx e components/LibraryView.tsx.
- Backend: commands/library.rs.
- Persistencia: db/repositories/book_repository.rs e tabela books.

## Fluxo principal
1. Usuário seleciona arquivo EPUB ou PDF.
2. Comando add_book valida caminho/extensão.
3. Backend extrai metadados e calcula SHA-256.
4. Livro é inserido se hash ainda não existir.
5. UI recarrega lista por list_books.
6. UI permite remover livro da biblioteca com confirmação e opção de excluir arquivo local.

## Regras de negocio
- Apenas .epub e .pdf são aceitos no fluxo atual de adição.
- Livro duplicado é bloqueado por hash.
- Título vazio recebe fallback pelo nome do arquivo na listagem.
- Remoção da biblioteca pode manter ou excluir o arquivo físico, conforme escolha do usuário.
- Metadados de PDF usam fallback do nome do arquivo quando não houver parser dedicado.

## Estados e filtros
- Status padrão de livro novo: unread.
- Status canônicos de leitura: unread, reading, finished.
- Busca textual por título e autor com debounce no frontend.
- Filtros de formato, status e autor aplicados na UI.
- Coleções de contexto locais na UI: todos, recentes e sem autor.
- Ordenação por data de adição, título e autor (asc/desc).
- Modos de visualização: grid, lista e tabela.
- Estados explícitos: loading, erro, biblioteca vazia e vazio por filtro.

## Decisoes de arquitetura
- Hash de arquivo evita duplicidade por caminho alternativo.
- Metadado mínimo de livro é resolvido no backend para manter consistência.
- Preferências de visualização e ordenação são persistidas localmente no frontend.

## ⚠️ Inconsistências encontradas
- Coleções da Biblioteca ainda são apenas contextos locais de UI e não existem como entidade persistida no backend.
