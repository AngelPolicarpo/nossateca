# 02 - Reader Engine

## Objetivo
Exibir conteúdo de livro por capítulo (EPUB) ou visualização direta (PDF) e manter progresso de leitura.

## Componentes principais
- Frontend: components/ReaderView.tsx e components/AnnotationSidebar.tsx.
- Backend leitura: commands/reader.rs e reader/epub.rs.
- Backend anotacoes: commands/annotations.rs e AnnotationRepository.

## Fluxo de leitura
1. UI solicita get_book_content com livro e capitulo.
2. Backend valida livro e identifica formato.
3. Para EPUB, EpubParser retorna HTML, título e contagem de capítulos.
4. Para PDF, backend retorna metadados e total de páginas do documento.
5. UI renderiza EPUB com navegação por capítulo.
6. UI renderiza PDF com navegação real por página (anterior/próxima e salto direto).

## Fluxo de progresso
1. Ao trocar capitulo, UI chama save_progress.
2. Backend grava current_position e percentual em reading_progress.
3. Para PDF, posição é registrada como page:N e percentual usa total real de páginas.
4. Backend sincroniza books.status para reading ou finished conforme percentual.

## Fluxo de anotacoes
1. Usuario seleciona trecho e cria highlight.
2. UI chama add_annotation.
3. Sidebar permite editar nota, trocar cor e excluir.
4. UI recarrega anotacoes e destaca trechos no HTML renderizado.

## Regras de negocio
- Reader aceita EPUB e PDF.
- Cores permitidas: yellow, green, blue, pink, purple.
- Anotação depende de livro existente e está disponível apenas para EPUB.
- Nomenclatura de navegação: EPUB usa "capítulo"; PDF usa "página".

## Decisoes de arquitetura
- Parser remove links CSS do EPUB para evitar erro de MIME em WebView.
- Posicao de anotacao e serializada em string para manter flexibilidade.

## ⚠️ Inconsistências encontradas
- Schema de books aceita mobi no histórico, mas o reader atual suporta apenas EPUB e PDF.
