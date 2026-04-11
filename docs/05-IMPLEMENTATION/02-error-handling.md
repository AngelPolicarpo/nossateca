# 02 - Error Handling

## Objetivo
Definir estrategia consistente de erro entre Rust, Tauri e React.

## Backend
- Validar cedo e retornar erro textual claro no comando.
- Converter erros internos para String apenas na fronteira do comando.
- Preservar contexto suficiente para diagnostico.
- Logar falhas de infraestrutura que nao devam interromper startup.

## Frontend
- Tratar erro como unknown e normalizar mensagens.
- Exibir feedback de erro por contexto da tela.
- Evitar suprimir erro silenciosamente em fluxos criticos.

## Eventos e assincronismo
- Falha de listener nao deve quebrar estado global.
- Operacoes longas devem manter estado intermediario observavel.

## Banco
- Falha de migracao deve impedir inicializacao, com erro explicito.
- Falha de query de dominio deve retornar mensagem de comando adequada.

## ⚠️ Inconsistências encontradas
- Em alguns pontos do frontend ainda existe uso de alert para erro, enquanto outros trechos usam estado visual inline; padrao de UX de erro ainda nao e unico.
