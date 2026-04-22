# 05 - Context Reset Playbook

## Objetivo
Padronizar retomada de sessoes longas no Agent Mode sem perda de decisoes, riscos e proximas acoes.

## Quando usar
- Ao iniciar uma sessao nova depois de pausa longa.
- Ao trocar de agente/pessoa responsavel pela tarefa.
- Ao encerrar um ciclo com mudancas em multiplos modulos.

## Fontes canonicas (ordem recomendada)
1. docs/00-START-HERE.md
2. docs/01-PROJECT/01-overview.md
3. docs/01-PROJECT/02-core-principles.md
4. docs/01-PROJECT/03-glossary.md
5. docs/03-ARCHITECTURE/01-system-overview.md
6. docs/04-FEATURES/<feature-da-tarefa>.md
7. docs/04-FEATURES/10-search-option-implementation-handoff.md (quando a tarefa tocar Discover/Source/Search)
8. docs/06-REFERENCES/01-api-contracts.md

## Snapshot atual (2026-04-18)
- Produto desktop offline-first com SQLite local como fonte de verdade.
- Busca principal e Discover-first; Home legado de busca foi removido.
- Fan-out de busca de fontes roda em paralelo via plugins Source.
- Reader suporta EPUB e PDF com progresso persistente; anotacoes ficam no fluxo EPUB.
- Escopo de IA foi removido do runtime principal.

## Decisoes criticas e racional
1. Discover-first para busca geral:
- Reduz caminho paralelo de busca duplicada na UI e no backend.

2. Fan-out paralelo de Source plugins:
- Melhora tempo de resposta agregado e evita dependencia de fonte unica.

3. Fronteira de comandos em src-tauri/src/main.rs:
- Mantem contrato Tauri visivel e rastreavel em um ponto principal.

4. Evolucao de schema via migracoes incrementais:
- Preserva previsibilidade de bootstrap e evita regressao historica.

## Riscos e perguntas abertas
- Existe codigo residual de busca legada (ex.: hooks/useSearch.ts) chamando `search_books`, comando que nao esta mais exposto no runtime atual.
- Existe diferenca de entrypoint documentado entre src-tauri/src/main.rs e src-tauri/src/lib.rs.
- Nao ha estrategia de throttling documentada para eventos de download em alta concorrencia.
- Cobertura automatizada ainda nao e uniforme para todos os comandos expostos.

## Resume checklist (primeiras 8 acoes)
1. Ler docs/00-START-HERE.md e validar dominio da tarefa.
2. Ler docs/01-PROJECT/01-overview.md para confirmar escopo ativo.
3. Ler docs/01-PROJECT/03-glossary.md para alinhar nomenclatura.
4. Ler documento de feature alvo em docs/04-FEATURES.
5. Se envolver Discover/Source, ler docs/04-FEATURES/10-search-option-implementation-handoff.md.
6. Mapear inconsistencias marcadas com [VERIFICAR] no dominio impactado.
7. Definir escopo da mudanca em uma frase e listar contratos afetados.
8. Executar mudanca pequena e atualizar docs afetados no mesmo ciclo.

## Validation checklist
- O arquivo de feature impactado foi atualizado.
- Referencias de contrato (docs/06-REFERENCES) foram revisadas quando necessario.
- Mudancas de arquitetura relevantes aparecem em docs/03-ARCHITECTURE.
- Novas decisoes e trade-offs ficaram explicitos em texto curto.
- Riscos abertos continuam registrados (nao removidos silenciosamente).
- Passos de retomada estao claros para uma nova sessao sem contexto previo.

## Template de fechamento de sessao
Preencher ao fim de um ciclo relevante:

- Data:
- Objetivo do ciclo:
- Decisoes tomadas:
- Arquivos de docs atualizados:
- Riscos em aberto:
- Primeiras 3 acoes para a proxima sessao:
