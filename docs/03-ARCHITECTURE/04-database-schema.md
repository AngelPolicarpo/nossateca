# 04 - Database Schema

## Fonte de verdade
- Banco SQLite inicializado em startup por src-tauri/src/db/connection.rs.
- Migracoes aplicadas de forma incremental a partir de src-tauri/migrations.

## Tabelas ativas
- books: cadastro de livros importados.
- reading_progress: progresso de leitura por livro.
- annotations: destaques, notas e marcadores.
- downloads: fila e estado de transferencias.
- user_settings: configuracoes chave/valor para plugins.

## Regras de dominio por tabela
- books.format aceita epub, pdf, mobi no schema historico.
- books.file_path e unico.
- books.file_hash e unico quando presente.
- books.status usa valores canonicos unread, reading e finished.
- reading_progress e 1:1 com books via book_id primary key.
- annotations valida tipo e cor por CHECK.
- downloads valida status e source_type por CHECK.

## Relacoes
- reading_progress.book_id referencia books.id com cascade delete.
- annotations.book_id referencia books.id com cascade delete.
- downloads.book_id referencia books.id quando associado.

## Indices relevantes
- books: title, author, status.
- reading_progress: progress_percent.
- annotations: book_id, type, created_at desc.
- downloads: status, created_at desc.

## Migracoes existentes
1. 001_initial_schema.sql: books e reading_progress.
2. 002_annotations.sql: annotations.
3. 003_embeddings.sql: estruturas legadas de IA e user_settings.
4. 004_rag_optimizations.sql: estruturas legadas de sumarios/progresso de indexacao.
5. 005_downloads.sql: downloads.
6. 006_remove_ai_schema.sql: remove tabelas legadas de IA.
7. 007_book_statuses.sql: normaliza status legados para unread/reading/finished.

## Convencoes de evolucao
- Nunca editar migracao antiga ja publicada.
- Toda mudanca estrutural exige nova migracao idempotente.
- Sempre documentar impacto em comandos e modelos.

