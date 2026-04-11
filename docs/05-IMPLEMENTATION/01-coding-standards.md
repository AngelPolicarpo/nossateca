# 01 - Coding Standards

## Objetivo
Padronizar mudancas para manter previsibilidade entre frontend e backend.

## Metodologia de organizacao
1. Definir escopo em uma frase antes de codar.
2. Mapear arquivos e contratos impactados.
3. Implementar em passos pequenos por camada (backend -> frontend -> docs).
4. Validar build/check e revisar inconsistencias.
5. Registrar decisao e impacto nos docs de referencia.

## Regras gerais
- Nomear por dominio, nao por tecnologia generica.
- Evitar funcoes longas com multiplas responsabilidades.
- Mensagem de erro deve ser especifica e acionavel.
- Evitar comentarios obvios; comentar apenas decisao nao trivial.

## Frontend
- Componente deve manter responsabilidade unica.
- Chamadas Tauri via invoke devem tratar erro unknown de forma robusta.
- Estados de loading e erro devem ser explicitos na UI.
- Evitar acoplamento entre tela e regras de persistencia.

## Backend
- Comandos devem validar entrada antes de acessar repositorio.
- Repositorio nao deve conter regra de UI.
- Tipos em models devem refletir contrato serializado estavel.
- Sempre preferir Result com contexto util para debug.

## Banco
- Mudanca estrutural somente por nova migracao.
- Nao alterar migracoes antigas em uso.

## Documentacao
- Atualizar arquivo de feature e referencia quando contrato mudar.
- Registrar divergencias conhecidas na seção de inconsistencias.

