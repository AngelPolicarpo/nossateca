# 03 - Testing Strategy

## Objetivo
Garantir estabilidade dos contratos centrais sem inflar custo de manutencao.

## Prioridades de teste
1. Comandos Tauri de dominio critico (library, reader, annotations, download, search).
2. Parsers e normalizadores (EPUB, ranking/dedup de busca).
3. Fluxos de UI com maior risco de regressao funcional.

## Escopo minimo por feature
- Caso feliz completo.
- Validacao de entrada invalida.
- Falha de dependencia externa sem quebra total.
- Persistencia correta de estado final.

## Backend
- Preferir testes unitarios para regras puras.
- Adicionar testes de integracao para operacoes com SQLite quando o risco justificar.

## Frontend
- Cobrir estados loading, erro e vazio em componentes de tela.
- Cobrir callbacks de acoes primarias (abrir livro, salvar anotacao, controlar download).

## Criterios de saida
- Build frontend sem erro de tipo.
- cargo check sem erro.
- Nenhuma quebra de contrato em docs de referencia.

## ⚠️ Inconsistências encontradas
- A base possui testes unitarios em modulos especificos (ex.: reader/search), mas nao ha cobertura automatizada equivalente para todos os comandos expostos.
