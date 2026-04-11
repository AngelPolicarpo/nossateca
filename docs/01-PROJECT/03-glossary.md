# 03 - Glossary

## Termos de produto
- Biblioteca: conjunto de livros cadastrados no SQLite.
- Leitura: abertura de capitulo EPUB e navegacao entre capitulos.
- Progresso: posicao persistida por livro em reading_progress.
- Anotacao: registro de destaque, nota ou marcador vinculado ao livro.
- Fonte externa: origem de resultados de busca entregue por plugin.
- Download: processo assinado por id com estado e progresso.

## Termos de arquitetura
- Comando Tauri: funcao Rust exposta para invocacao pelo frontend.
- Repositorio: camada de acesso a dados SQL por agregado.
- Orquestrador de busca: componente que consulta plugins em paralelo.
- Snapshot de plugin: estado imutavel para execucao concorrente segura.
- Evento de download: payload emitido para atualizar UI em tempo real.

## Termos de dados
- Book: entidade persistida na tabela books.
- Annotation: entidade persistida na tabela annotations.
- DownloadRecord: entidade persistida na tabela downloads.
- SearchBookResult: resultado normalizado de busca entre fontes.
- SearchPluginSettings: credenciais/host persistidos em user_settings.

## Termos de status
- unread: livro cadastrado e ainda nao iniciado.
- reading: livro em leitura ativa.
- finished: livro concluido.
- queued/downloading/paused/completed/failed/cancelled: estados de download.

## ⚠️ Inconsistências encontradas
- O status indexed ainda aparece em partes da UI como estado pronto, mas nao e mais gerado pelo backend atual.
