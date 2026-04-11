# 04 - Performance

## Objetivo
Manter responsividade da interface e estabilidade do runtime local.

## Leitura
- Carregar capitulo por demanda, sem pre-carregar livro inteiro.
- Evitar reprocessar HTML alem do necessario para anotacoes visiveis.

## Busca
- Executar plugins em paralelo com timeout.
- Deduplicar e ordenar no backend para reduzir trabalho de UI.

## Download
- Limitar concorrencia por configuracao do manager.
- Reaproveitar progresso persistido para retomada quando possivel.

## Banco
- Usar indices existentes para listagens e filtros frequentes.
- Evitar queries nao paginadas em contextos de crescimento alto.

## UI
- Atualizar listas por diff de estado, nao por reload completo desnecessario.
- Evitar rerender custoso em eventos de progresso de alta frequencia.

## ⚠️ Inconsistências encontradas
- A aba de downloads atualiza estado por eventos em tempo real, mas nao ha estrategia documentada de throttling em cenarios de muitos downloads simultaneos. [VERIFICAR]
