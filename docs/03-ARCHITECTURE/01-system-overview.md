# 01 - System Overview

## Visao geral
Nossateca e um app desktop local com fronteira clara entre UI React e core Rust via comandos Tauri.

## Componentes principais
- UI (React): navega entre biblioteca, leitura, estudo e downloads.
- Comandos (Tauri): API interna usada pelo frontend.
- Dominio Rust: biblioteca, leitor EPUB, anotacoes, busca e downloads.
- Persistencia: SQLite local com migracoes incrementais.
- Plugins WASM: fontes externas de busca desacopladas do core.

## Fronteiras de responsabilidade
- Frontend nao executa regra de persistencia diretamente.
- Backend nao conhece estado visual de componentes.
- Plugin nao acessa banco local do app diretamente.

## Estado global do backend
- Pool SQLite compartilhado.
- PluginManager compartilhado com lock.
- DownloadManager compartilhado com ator interno e fila.

## Caminho critico de inicializacao
1. App inicia e resolve app_data_dir.
2. Banco e migrado em startup.
3. Config de addons e hidratada de user_settings para o PluginManager.
4. Plugins sao carregados do diretorio de runtime do usuario.
5. Comandos Tauri sao registrados e app fica pronto.

## Objetivos nao funcionais
- Baixa dependencia externa.
- Recuperacao de falha local sem reinstalar app.
- Evolucao por modulo sem reescrever arquitetura.

