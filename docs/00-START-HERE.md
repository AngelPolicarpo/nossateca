# 00 - Start Here

Guia de entrada para qualquer agente que vai alterar o projeto.

## Ordem de leitura
1. docs/01-PROJECT/01-overview.md
2. docs/01-PROJECT/02-core-principles.md
3. docs/01-PROJECT/03-glossary.md
4. docs/05-IMPLEMENTATION/05-context-reset-playbook.md
5. docs/03-ARCHITECTURE/01-system-overview.md
6. docs/04-FEATURES referente ao dominio da tarefa
7. docs/06-REFERENCES/01-api-contracts.md (quando houver mudanca de contrato)
8. docs/05-IMPLEMENTATION/01-coding-standards.md metodologia de organizacao

## Reset de contexto em 10 minutos
1. Confirmar o dominio da tarefa e abrir o arquivo de feature correspondente em docs/04-FEATURES.
2. Ler docs/05-IMPLEMENTATION/05-context-reset-playbook.md e seguir o Resume checklist.
3. Se a tarefa envolver Discover/Source/Search, revisar docs/04-FEATURES/10-search-option-implementation-handoff.md.
4. Registrar escopo da sessao em uma frase antes de editar codigo.
5. Ao encerrar, atualizar os docs impactados e registrar riscos remanescentes.

## Regras de operacao
- Tratar src-tauri/src/main.rs como fronteira de comandos Tauri.
- Preservar fluxo offline-first e armazenamento local SQLite.
- Priorizar mudancas pequenas, sem quebrar contratos de comando.

## Atalhos de decisao
- Mudanca em dados: revisar docs/03-ARCHITECTURE/04-database-schema.md.
- Mudanca em comando: revisar docs/06-REFERENCES/01-api-contracts.md.
- Mudanca em UI de leitura: revisar docs/04-FEATURES/02-reader-engine.md.
- Mudanca em Discover/Source: revisar docs/04-FEATURES/09-discover-source-orchestration.md.
- Mudanca em busca/download: revisar docs/04-FEATURES/05-plugin-system.md e docs/04-FEATURES/04-download-engine.md.
- Retomada apos pausa longa: revisar docs/05-IMPLEMENTATION/05-context-reset-playbook.md.

## Checklist minimo antes de encerrar tarefa
- Confirmar que comandos Tauri continuam invocaveis.
- Confirmar que frontend compila.
- Confirmar que migracoes mantem banco inicializavel.
- Atualizar docs afetados pela mudanca.
- Atualizar estado de handoff quando houver decisao arquitetural relevante.

## ⚠️ Inconsistências encontradas
- Existe src-tauri/src/lib.rs com entrypoint minimo diferente do fluxo usado no desktop, que hoje passa por src-tauri/src/main.rs. [VERIFICAR]
