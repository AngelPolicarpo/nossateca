# 07 - Download Feature Review

## Escopo analisado
- Backend: [lexicon/src-tauri/src/download/manager.rs](../../../lexicon/src-tauri/src/download/manager.rs), [lexicon/src-tauri/src/commands/download.rs](../../../lexicon/src-tauri/src/commands/download.rs), [lexicon/src-tauri/src/models/download.rs](../../../lexicon/src-tauri/src/models/download.rs), [lexicon/src-tauri/migrations/005_downloads.sql](../../../lexicon/src-tauri/migrations/005_downloads.sql).
- Frontend: [lexicon/src/App.tsx](../../../lexicon/src/App.tsx).
- Paths e runtime data dir: [lexicon/src-tauri/src/storage.rs](../../../lexicon/src-tauri/src/storage.rs).

## Como a feature funciona hoje
### Metodos suportados
- HTTP/HTTPS.
- OPDS (tratado como origem do tipo opds, usando pipeline HTTP).
- Torrent por magnet link ou arquivo .torrent.

### Fluxo atual
1. UI chama start_download.
2. Backend valida URL e cria registro em downloads com status queued.
3. Download manager (modelo actor) controla fila + concorrencia maxima.
4. Worker executa HTTP (reqwest) ou torrent via sessao integrada em Rust (librqbit).
5. Eventos download:progress e download:state atualizam UI.
6. Usuario pode pausar, retomar, cancelar, remover da lista e excluir arquivo.
7. Em download concluido EPUB, UI permite adicionar a biblioteca.

### Capacidades que ja existem
- Retomada HTTP por Range e arquivo parcial existente.
- Retentativa HTTP com backoff em erros transitivos.
- Fila + downloads paralelos (max concorrencia configuravel por LEXICON_MAX_CONCURRENT_DOWNLOADS).
- Persistencia de estado em SQLite.
- Remocao de item da lista com opcao de excluir arquivo.
- Torrent nativo sem dependencia de cliente externo (rqbit/librqbit).
- Progresso torrent em bytes reais, velocidade real e peers ativos persistidos em banco.

## Mudanca implementada nesta iteracao
1. Substituicao do fluxo torrent baseado em CLI externa por sessao integrada com librqbit.
2. Pause/resume/cancel mapeados para APIs nativas de sessao (pause, unpause, delete).
3. Persistencia de telemetria de torrent no schema existente (torrent_info_hash, torrent_peers).
4. Encerramento de torrent concluido com limpeza de handle na sessao para evitar seeding acidental.
5. Resolucao de caminho final de payload para melhorar acao "Adicionar a biblioteca" quando houver unico EPUB.

## Pontos de falha, limitacoes e friccoes
1. Ainda falta reconciliacao robusta no startup para downloads em "downloading" apos crash/kill do app.
2. torrent_seeds ainda nao esta sendo preenchido (hoje gravamos peers ativos e info_hash).
3. Persistencia cross-restart de sessao torrent (fastresume completo) ainda nao esta habilitada.
4. Fluxo de adicionar a biblioteca em torrent multi-file ainda precisa seletor guiado de arquivo.
5. Falta observabilidade orientada a etapa (metadata, trackers, peer starvation, disco cheio) no nivel de UX.

## Torrent: respostas objetivas
### O sistema exige cliente torrent instalado?
- Nao. O backend agora usa engine integrada (librqbit) e nao depende de transmission-cli/aria2c.

### E possivel integrar engine torrent direto na aplicacao?
- Sim, e foi implementado nesta iteracao com librqbit.
- Decisao aplicada:
1. Engine integrada em Rust no proprio backend Tauri.
2. Sem parse de stdout/stderr e sem dependencia de processos externos.
3. Telemetria de progresso obtida direto do estado interno da torrent session.

### Tradeoffs: integrado vs delegar ao cliente do usuario
#### Integrado (estado atual)
- Ganhos observados:
1. UX previsivel e consistente em qualquer maquina.
2. Progresso em bytes reais e velocidade derivada de estatisticas internas.
3. Controle de ciclo de vida torrent sem dependencias externas.
- Custos assumidos:
1. Mais codigo de manutencao no backend.
2. Maior tempo de compilacao e superficie de dependencia.

## Melhorias por impacto e complexidade
Escala usada:
- Complexidade baixa: 1 a 3 dias.
- Complexidade media: 4 a 10 dias.
- Complexidade alta: 2+ semanas.

| Melhoria | Impacto | Complexidade | Depende de decisao de produto? | Status |
|---|---|---|---|---|
| Migrar torrent para engine integrada (sem parse de stdout) | Alto | Alta | Sim | Implementado |
| Reconciliacao no startup para downloads orfaos | Alto | Media | Nao | Pendente |
| Enriquecer telemetria (seeds + ETA + detalhes por etapa) | Alto | Media | Nao | Parcial |
| Retomada torrent entre reinicios com fastresume persistido | Alto | Alta | Nao | Pendente |
| Fluxo guiado para importar EPUB em torrent multi-file | Medio | Media | Sim | Pendente |
| Acoes em lote na UI (limpar concluidos/falhos/cancelados) | Medio | Baixa | Nao | Pendente |
| Limites de banda e politicas de seeding configuraveis | Medio | Media | Sim | Pendente |

## Decisoes de produto que devem vir antes de implementar
1. Politica de seeding: nunca seed, seed por tempo, seed por ratio?
2. Politica de retencao: manter historico de downloads por quanto tempo?
3. Auto-import para biblioteca: EPUB/PDF unicos ou fluxo guiado para torrents multi-file?
4. Nivel de observabilidade exposto ao usuario final vs. apenas logs tecnicos.

## Plano recomendado (ordem pratica)
1. Curto prazo: startup reconciliation + retry UX + erros detalhados por etapa.
2. Medio prazo: fastresume entre reinicios + seletor de importacao para multi-file.
3. Produto: politicas de seeding/banda e telemetria exposta em UI.

## ⚠️ Inconsistências encontradas
- `torrent_seeds` segue sem preenchimento efetivo no fluxo atual (somente peers ativos).
- A UX ainda nao diferencia claramente torrent single-file vs multi-file para acao de importacao na biblioteca.
