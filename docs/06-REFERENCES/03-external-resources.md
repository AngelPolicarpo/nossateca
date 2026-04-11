# 03 - External Resources

## Documentacao oficial
- Tauri v2: comandos, plugins e eventos desktop.
- React: padroes de composicao e estado.
- sqlx: SQLite, migracoes e query macros.
- Tokio: runtime assincorno.
- Wasmtime component model + WASI.
- WIT bindgen para contrato de plugin.

## Fontes externas de busca
- Fontes externas acessadas apenas por addons instalados manualmente pelo usuario.

## Recurso local do projeto
- Pasta IA na raiz contem historico removido do runtime ativo.

## Uso recomendado
- Verificar primeiro docs internos antes de abrir referencia externa.
- Validar sempre contra codigo atual quando houver conflito documental.

## ⚠️ Inconsistências encontradas
- Dependencia de fonte externa pode variar por disponibilidade da API e chaves de acesso; em ambiente sem chave, comportamento cai para falha de fonte ou fallback mock conforme orquestrador.
