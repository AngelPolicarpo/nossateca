# 02 - Core Principles

## Principios de arquitetura
- Offline-first: operacao principal sem dependencia de backend remoto.
- Limites claros: UI acessa core apenas via comandos Tauri.
- Estado local: SQLite como fonte de verdade de biblioteca, progresso e anotacoes.
- Extensibilidade segura: busca externa por plugins WASM com isolamento.
- Falha isolada: erro em plugin/download nao deve quebrar leitura local.

## Principios de evolucao
- Preservar contratos de comando e tipos serializaveis.
- Evitar acoplamento entre features (biblioteca, leitura, download, busca).
- Evoluir schema por migracao incremental, sem edicao retroativa.
- Documentar decisao arquitetural antes de expandir superficie publica.

## Principios de UX tecnica
- Respostas de erro devem refletir causa real retornada pelo backend.
- Fluxos lentos devem emitir estado observavel (loading, progresso, falha).
- Operacoes destrutivas devem ser explicitas (exclusao/cancelamento).

## Regras para contribuicao
- Mudancas de dominio primeiro no backend e depois na interface.
- Evitar texto/documentacao que implique features removidas de IA no runtime.
- Preferir naming consistente com arquivos de src-tauri/src/models.

## ⚠️ Inconsistências encontradas
- A UI usa abas com rotulo Estudo em src/App.tsx, mas o dominio principal documentado nao define esse modulo como feature de backend dedicada. [VERIFICAR]
