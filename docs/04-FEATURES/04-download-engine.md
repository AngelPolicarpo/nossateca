# 04 - Download Engine

## Contexto
- Esta feature cobre o ciclo completo de downloads locais com suporte a HTTP(S) e torrent.
- O escopo atual inclui fila, concorrencia, pause/resume/cancel/remove, persistencia em SQLite e eventos de progresso para a UI.
- A feature tambem integra o fluxo de descoberta (Discover) com a fila de downloads e permite adicionar o arquivo concluido na biblioteca quando compativel.

## Responsabilidades
- Frontend principal:
	- lexicon/src/App.tsx (tab Downloads, listeners de eventos, acoes por item).
	- lexicon/src/components/DiscoverView.tsx (acao de enfileirar download a partir de fontes).
- Backend principal:
	- lexicon/src-tauri/src/commands/download.rs.
	- lexicon/src-tauri/src/download/manager.rs.
- Modelos e eventos:
	- lexicon/src-tauri/src/models/download.rs.
- Persistencia:
	- lexicon/src-tauri/migrations/005_downloads.sql.
- Paths de runtime:
	- lexicon/src-tauri/src/storage.rs (resolve_lexicon_data_dir).

## Fluxo funcional
1. Usuario inicia download pela tab Downloads (+ Adicionar URL ou magnet) ou via Discover.
2. Frontend invoca start_download com source_url e file_name opcional.
3. DownloadManager valida origem e nome, cria registro queued na tabela downloads e enfileira o id.
4. Ator de downloads dispara workers conforme maximo de concorrencia (LEXICON_MAX_CONCURRENT_DOWNLOADS; padrao 2).
5. Worker escolhe pipeline por source_type:
	 - http/opds: reqwest com retries, range resume e escrita em arquivo.
	 - torrent: sessao integrada librqbit (sem CLI externo).
6. Backend emite:
	 - download:progress (telemetria frequente de progresso).
	 - download:state (transicoes de estado e erros).
7. Frontend atualiza lista local e permite acoes de pause, resume, cancel e remove.
8. Em completed com arquivo EPUB/PDF, a UI expoe Adicionar a biblioteca e chama add_book.

## Regras de negocio
- source_url aceita apenas http://, https://, magnet: ou sufixo .torrent.
- detect_source_type classifica como torrent para magnet/.torrent; demais casos entram como http.
- file_name e sanitizado para evitar caracteres invalidos de path.
- Estados canonicos:
	- queued, downloading, paused, completed, failed, cancelled.
- Restricoes de transicao:
	- resume rejeita completed e cancelled.
	- remove rejeita download ativo (exige pause/cancel antes).
- Cancelamento remove artefato local quando aplicavel e finaliza como cancelled.
- Add to library automatico (na UI) so aparece para completed com extensao .epub ou .pdf.

## Decisoes de arquitetura
- Modelo actor + channel para serializar comandos de controle e evitar corrida entre acoes de fila.
- Torrent integrado com librqbit para remover dependencia de cliente externo e evitar parse de stdout/stderr.
- Persistencia de estado em SQLite para permitir recuperar historico e telemetria por item.
- Eventos Tauri (download:progress e download:state) desacoplam worker de renderizacao de UI.
- Artefatos sao resolvidos no diretorio de dados da aplicacao via storage.rs com suporte a migracao de legado.

## Estado atual da implementacao
- Implementado:
	- Fila com concorrencia configuravel por variavel de ambiente.
	- HTTP com retry/backoff (3 tentativas) e timeout de conexao/leitura.
	- Retomada HTTP por Range quando servidor suporta partial content.
	- Torrent nativo com pause/resume/cancel via APIs da sessao integrada.
	- Persistencia de bytes baixados, velocidade e progresso percentual.
	- Emissao de estado/progresso para UI em tempo real.
	- Remocao da lista com opcao de excluir arquivo.
	- Resolucao de path final em torrent concluido com heuristica para arquivo unico EPUB.
- Ja consolidado nesta iteracao:
	- Substituicao do fluxo torrent baseado em CLI externa por librqbit integrado.
	- Controle de ciclo de vida torrent no proprio backend.

## Limitacoes conhecidas
- Nao existe reconciliacao robusta no startup para itens que ficaram em downloading apos crash/kill.
- Fastresume persistido de torrent entre reinicios ainda nao esta habilitado.
- Campo torrent_seeds existe no schema, mas nao e preenchido no fluxo atual.
- Para torrent multi-file, ainda falta fluxo guiado para escolher arquivo ao importar para biblioteca.
- UI de downloads ainda nao oferece acoes em lote nem filtros por estado.
- Limites de banda/agendamento ainda aparecem apenas como expectativa na UI.

## Testes minimos
- Caso feliz HTTP:
	- Iniciar download HTTP valido, observar transicao queued -> downloading -> completed.
	- Verificar progress/eventos e arquivo salvo no diretorio de dados.
- Caso feliz torrent:
	- Iniciar magnet/.torrent valido, confirmar telemetria de progresso e conclusao.
- Pause/resume:
	- Pausar download ativo e retomar sem perder consistencia de bytes/estado.
- Cancel/remove:
	- Cancelar em andamento e verificar status cancelled.
	- Remover item concluido/falho/cancelado com e sem delete_file.
- Erro principal:
	- URL invalida ou resposta HTML no lugar de binario deve resultar em failed com mensagem.
- Regressao de contrato:
	- Comandos Tauri start_download/pause_download/resume_download/cancel_download/remove_download/list_downloads continuam compativeis com a UI.

## ⚠️ Inconsistencias encontradas
- Schema permite source_type opds, mas detect_source_type atualmente retorna torrent ou http; opds fica semanticamente como alias de http.
- Campo torrent_seeds esta no schema e no modelo, mas nao recebe update no ciclo de progresso atual.
- A UX de importacao para biblioteca nao distingue explicitamente o caso torrent multi-file.
