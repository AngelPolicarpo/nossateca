# 03 - Dependencies

## Dependencias de frontend
- react e react-dom: renderizacao da interface.
- @tauri-apps/api: invocacao de comandos e escuta de eventos.
- @tauri-apps/plugin-dialog: seletor de arquivo nativo.
- @tanstack/react-query: cache e ciclo de requisicoes de busca.

## Dependencias de backend
- tauri: runtime de comandos e eventos desktop.
- tauri-plugin-dialog e tauri-plugin-opener: integracoes nativas.
- tokio: async runtime.
- sqlx: acesso SQLite e migracoes.
- serde e serde_json: serializacao de contratos.
- anyhow: propagacao de erro contextual.
- epub: leitura de metadados e capitulos EPUB.
- reqwest: downloads HTTP e integracao com fonte externa.
- wasmtime e wasmtime-wasi: execucao de plugins WASM.
- wit-bindgen: binding do contrato WIT de plugin.
- sha2 e uuid: hash de arquivo e ids unicos.

## Dependencias por dominio
- Biblioteca: epub, sha2, sqlx.
- Leitor: epub.
- Anotacoes: sqlx, uuid.
- Busca: reqwest, wasmtime, wit-bindgen.
- Download: reqwest, tokio, sqlx.

## Politica de adicao
- Toda dependencia nova exige justificativa de dominio.
- Evitar duplicar bibliotecas que resolvem o mesmo problema.
- Priorizar crates maduras e ativamente mantidas.

## Politica de remocao
- Dependencia sem uso deve sair no mesmo ciclo de refatoracao.
- Ajustar docs/02-STACK/01-tech-stack.md apos remocao relevante.

## ⚠️ Inconsistências encontradas
- Texto de alerta em AddBookButton.tsx ainda menciona indexacao em background, mas as dependencias de IA ja nao existem no runtime.
