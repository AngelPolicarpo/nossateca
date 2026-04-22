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

## Termos de continuidade operacional
- Reset de contexto: processo padrao de retomada de sessao usando leitura canonica e checklist.
- Pacote de handoff: resumo curto com objetivo, decisoes, riscos e proximas acoes.
- Fonte canonica: documento oficial a ser priorizado quando houver conflito de informacao.
- Risco aberto: ponto conhecido que ainda nao foi resolvido e exige rastreabilidade.
- Validation checklist: lista objetiva para confirmar que a sessao foi retomada corretamente.

## ⚠️ Inconsistências encontradas
- Existe artefato de busca legada no frontend (`hooks/useSearch.ts`) apontando para `search_books`, enquanto o backend atual nao expoe mais esse comando.
