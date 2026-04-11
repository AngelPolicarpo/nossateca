# 01 - Tech Stack

## Camadas tecnicas
- Frontend: React 19 + TypeScript + Vite.
- Integracao desktop: Tauri v2.
- Backend: Rust assincorno com Tokio.
- Banco local: SQLite via sqlx.
- Leitura EPUB: crate epub.
- Downloads HTTP: reqwest.
- Plugins: Wasmtime component model + WASI.

## Por que essa stack
- Tauri reduz custo de distribuicao para desktop.
- Rust centraliza regras criticas com tipagem forte e performance previsivel.
- SQLite atende uso local sem operador externo.
- Wasm permite extensao de fontes de busca sem acoplamento forte no core.

## Contratos entre camadas
- Frontend conversa com backend apenas por comandos Tauri.
- Backend emite eventos para atualizar progresso de download em tempo real.
- Plugins de busca retornam SearchBookResult normalizado.

## Restricoes de plataforma
- Runtime principal e desktop local.
- Fluxo nominal nao depende de servidor remoto proprietario.
- Busca externa pode depender de API key em fontes especificas.

## Decisoes de manutencao
- Evitar adicionar dependencia se a mesma regra pode ficar no dominio atual.
- Dependencia nova deve ter dono claro (frontend, backend ou plugin).
- Mudanca de stack exige atualizacao dos docs de arquitetura e referencias.

## ⚠️ Inconsistências encontradas
- package.json nao declara Tailwind, embora alguns textos historicos de docs antigos citassem Tailwind como parte da stack.
