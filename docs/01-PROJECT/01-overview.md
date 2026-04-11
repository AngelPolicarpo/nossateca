# 01 - Overview

## Objetivo do produto
Lexicon e um app desktop para gerenciar livros digitais, ler EPUB e registrar anotacoes locais, com extensao de busca por plugins.

## Problema resolvido
- Centralizar biblioteca pessoal sem depender de servico cloud.
- Oferecer leitura com progresso persistente e anotacoes por trecho.
- Permitir descoberta de catalogos e busca de download por plugins externos isolados.

## Perfil de uso
- Usuario individual, ambiente local.
- Foco em estudo e leitura tecnica/literaria.
- Preferencia por controle de dados no proprio dispositivo.

## Escopo ativo
- Importacao de livro EPUB por seletor de arquivo.
- Listagem, filtro e abertura de livros.
- Renderizacao de capitulos EPUB com navegacao.
- CRUD de anotacoes e destaques.
- Discover por plugins de catalogo e Source plugins com fan-out paralelo por fonte.
- Busca federada legada por plugins e gerenciador de downloads.

## Escopo removido do runtime
- Camada de IA, chat e indexacao semantica dentro de lexicon.
- Conteudo historico foi movido para pasta IA na raiz do workspace.

## Metricas operacionais
- App deve iniciar sem servicos externos obrigatorios.
- Falha de plugin nao pode derrubar biblioteca/leitor.
- Banco local deve migrar em startup sem intervencao manual.

## ⚠️ Inconsistências encontradas
- O schema historico ainda possui migracoes legadas de IA (003 e 004), mas os objetos sao removidos por 006_remove_ai_schema.sql; isso e esperado, porem pode confundir leitura cronologica.
