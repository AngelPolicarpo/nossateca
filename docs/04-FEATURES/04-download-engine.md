# 04 - Download Engine

## Objetivo
Executar downloads com fila, controle de estado e telemetria de progresso para a UI.

## Componentes principais
- Comandos: commands/download.rs.
- Core: download/manager.rs.
- Modelo: models/download.rs.
- Persistencia: tabela downloads.

## Fluxo principal
1. UI chama start_download com URL e nome opcional.
2. Manager valida origem e cria registro queued.
3. Ator agenda worker respeitando concorrencia maxima.
4. Worker publica eventos download:progress e download:state.
5. UI permite pausar, retomar, cancelar e listar.

## Regras de negocio
- source_url aceita http(s), magnet e .torrent.
- Fonte deve apontar para recurso final de download quando possivel.
- status valida transicoes esperadas entre queued/downloading/paused/completed/failed/cancelled.

## Confiabilidade
- Retentativas HTTP com backoff.
- Suporte a retomada por range em cenarios compativeis.
- Persistencia de bytes baixados e velocidade.

## Decisoes de arquitetura
- Ator central evita condicao de corrida em controle de fila.
- Eventos Tauri desacoplam atualizacao de UI do ciclo interno do worker.

## ⚠️ Inconsistências encontradas
- source_type aceita opds no schema, mas a deteccao efetiva de origem pode cair em http dependendo do link final; revisar semantica de classificacao. [VERIFICAR]
