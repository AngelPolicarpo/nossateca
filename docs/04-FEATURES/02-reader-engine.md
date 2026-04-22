# 02 - Reader Engine

## Objetivo
Exibir conteúdo de livro por capítulo (EPUB) ou visualização direta (PDF) e manter progresso de leitura.

## Componentes principais
- Frontend: components/ReaderView.tsx e components/AnnotationSidebar.tsx.
- Backend leitura: commands/reader.rs e reader/epub.rs.
- Backend anotacoes: commands/annotations.rs e AnnotationRepository.
- Preferencias locais: localStorage para tema, tipografia, largura de leitura, reducao de animacoes e zoom do PDF.

## Comandos de leitura (Tauri)
- get_book_content: retorna conteudo EPUB por capitulo ou metadados de leitura para PDF.
- get_pdf_document: retorna bytes do PDF em base64 e total de paginas.
- get_reading_progress: retorna progresso salvo (current_position e progress_percent) para retomada.
- search_epub_content: busca textual no livro EPUB inteiro e retorna capitulo, snippet e ocorrencias.
- resolve_epub_link_target: resolve href interno do EPUB para capitulo de destino e ancora opcional.
- save_progress: salva posicao atual e percentual, com sincronizacao de status do livro.

## Fluxo de leitura
1. Ao abrir o Reader, a UI tenta recuperar progresso salvo via get_reading_progress.
2. A UI interpreta current_position e define capitulo/pagina inicial.
3. UI solicita get_book_content com livro e capitulo inicial.
4. Backend valida livro e identifica formato.
5. Para EPUB, EpubParser retorna HTML, titulo e contagem de capitulos.
6. Para PDF, backend retorna metadados e total de paginas do documento.
7. UI renderiza EPUB com navegacao por capitulo.
8. UI renderiza PDF com navegacao real por pagina (anterior/proxima e salto direto).
9. Quando houver scroll salvo para EPUB, a UI restaura a posicao apos carregar o capitulo.

## Fluxo de progresso
1. Ao trocar capitulo/pagina, UI chama save_progress.
2. Durante leitura de EPUB, a UI persiste scroll com debounce para evitar excesso de escrita.
3. Backend grava current_position e percentual em reading_progress.
4. Para EPUB, posicao pode ser registrada como chapter:N;scroll:Y.
5. Para PDF, posicao e registrada como page:N e percentual usa total real de paginas.
6. Backend sincroniza books.status para reading ou finished conforme percentual.

## Formato de current_position
- EPUB sem scroll: chapter:N
- EPUB com scroll: chapter:N;scroll:Y
- PDF: page:N

## Fluxo de anotacoes
1. Usuario seleciona trecho e cria highlight.
2. UI chama add_annotation.
3. Sidebar permite editar nota, trocar cor e excluir.
4. UI recarrega anotacoes e destaca trechos no HTML renderizado.
5. Usuario pode salvar marcador de leitura (tipo bookmark) por capitulo/posicao de scroll.
6. Sidebar possui filtro por tipo (todas, destaques, marcadores).

## Fluxo de controles de leitura
1. Reader aplica preferencias salvas do usuario ao iniciar a tela.
2. Para EPUB, usuario ajusta fonte, espacamento e largura do conteudo em tempo real.
3. Para PDF, usuario pode usar zoom manual (+/-) e salto direto para pagina.
4. Usuario pode alternar tela cheia pelo botao na toolbar.
5. Opcao "Reduzir animacoes" simplifica transicoes durante leitura.

## Fluxo de busca EPUB
1. Usuario digita um termo no campo de busca do Reader.
2. Acoes de capitulo atual usam navegacao de ocorrencias (anterior/proxima) no conteudo renderizado.
3. Ao buscar no livro inteiro, a UI chama search_epub_content.
4. Backend percorre capitulos EPUB, extrai texto legivel e retorna resultados com snippet por capitulo.
5. Ao selecionar um resultado, a UI abre o capitulo correspondente e posiciona na ocorrencia.

## Fluxo de links internos EPUB
1. Clique em link interno (href com #ancora ou caminho relativo) e interceptado no conteudo renderizado.
2. Se for ancora local, UI localiza o alvo no capitulo atual e faz scroll suave.
3. Se apontar para outro capitulo, UI chama resolve_epub_link_target para obter chapter_index e anchor_id.
4. Reader carrega o capitulo resolvido e aplica scroll para a ancora quando presente.

## Renderizacao EPUB
- Tabelas recebem tratamento para overflow horizontal e bordas legiveis.
- Listas (ul/ol/li) preservam recuo e marcadores.
- Imagens e SVGs sao ajustados com max-width e object-fit para evitar quebra de layout.
- Conteudo de capitulo e montado em Shadow DOM para isolar CSS do ebook sem afetar o restante da aplicacao.
- Sanitizacao preserva estilos do proprio livro (inline e stylesheet data URL), removendo scripts, handlers inline e URLs inseguras.

## Atalhos de teclado atuais
- ArrowRight / PageDown: proximo capitulo/pagina.
- ArrowLeft / PageUp: capitulo/pagina anterior.
- Ctrl/Cmd + F: focar busca EPUB.
- T: alternar tema de leitura.
- F: alternar tela cheia.
- B: salvar marcador no ponto atual (EPUB).

## Regras de negocio
- Reader aceita EPUB e PDF.
- Cores permitidas: yellow, green, blue, pink, purple.
- Anotacao depende de livro existente e esta disponivel para fluxo EPUB neste ciclo.
- Nomenclatura de navegação: EPUB usa "capítulo"; PDF usa "página".

## Decisoes de arquitetura
- Parser EPUB migrado para rbook 0.7.5 para leitura mais robusta de manifest, spine e toc.
- Recursos internos do EPUB (imagens, srcset, poster e CSS) sao resolvidos e convertidos para data URL no backend.
- Posicao de anotacao e serializada em string para manter flexibilidade.

## ⚠️ Inconsistências encontradas
- Schema de books aceita mobi no histórico, mas o reader atual suporta apenas EPUB e PDF.

## Atualizacao 2026-04-16
- Implementado comando get_reading_progress no backend.
- Reader agora inicia a partir da ultima posicao salva quando houver progresso registrado.
- Leitura EPUB agora persiste scroll de forma periodica (debounce), melhorando retomada precisa.
- Adicionados controles de leitura para EPUB (fonte, espacamento e largura).
- Adicionados controles de PDF (zoom manual e salto de pagina).
- Adicionados fullscreen, reducao de animacoes e atalhos de teclado basicos.
- Adicionado fluxo de bookmark por capitulo com filtro dedicado na sidebar de anotacoes.
- Adicionada busca no conteudo EPUB (capitulo atual e livro inteiro).
- Corrigido carregamento inicial de PDF para renderizar sem exigir clique em proximo/anterior.
- Melhorada renderizacao EPUB para tabelas, imagens e listas.
- Migrado parser EPUB para rbook 0.7.5 com inline de recursos internos para corrigir imagens nao carregadas.
- Adicionada largura adaptativa para EPUB em fontes maiores, reduzindo medida efetiva para manter conforto de leitura.
- Fortalecido fallback de worker do PDF para evitar quebra quando o WebView falha ao importar module script.
- Refinada separacao de camadas de cor entre pagina, superficie principal e superficie auxiliar (light/dark).
- Reduzido uso de destaque azul em hovers genericos para preservar acento em estados primarios e ativos.
- Simplificado modelo de elevacao: toolbar e sidebars em plano baixo; container de leitura/PDF como plano card principal; menu flutuante mantido em nivel profundo.
- Suavizado efeito de pulso em highlights para reduzir distracao durante navegacao por anotacoes.

## Atualizacao 2026-04-17
- Reader EPUB passou a renderizar capitulos em Shadow DOM, impedindo vazamento de CSS para o restante da UI.
- Sanitizacao EPUB foi reforcada para manter qualidade visual do livro sem permitir script, handlers inline ou links de stylesheet inseguros.
- Navegacao de busca no capitulo atual foi adaptada para operar no conteudo isolado (selecao e scroll ate a ocorrencia).
- Navegacao por links internos do EPUB foi ajustada para funcionar entre capitulos e em ancoras locais.

## Checklist QA Reader (regressao rapida)
1. Tema e contraste (light/dark): abrir o mesmo capitulo e validar hierarquia clara entre texto principal, metadados e paineis auxiliares.
2. Conteudo longo EPUB: validar leitura continua com paragrafos extensos, listas e tabelas sem perda de contraste em cada tema.
3. Medida de leitura: ajustar largura/fonte/espacamento e confirmar conforto visual sem linhas longas demais ou espacamento excessivo, inclusive com reducao automatica da medida efetiva em fontes altas.
4. Prioridade de layout (desktop medio): com indice e notas ativos, validar recolhimento automatico de notas e reabertura manual via botao "Notas".
5. Touch targets mobile: em <=768px, validar botoes e campos principais com alvo efetivo minimo de 44px.
6. Persistencia de preferencias: reiniciar Reader e confirmar restauracao de tema, fonte, espacamento, largura, reduzir animacoes e zoom do PDF.
7. Foco acessivel: navegar por teclado e confirmar :focus-visible em controles de toolbar, navegacao, busca e sidebar.
8. Regressao PDF: validar zoom +/-, salto de pagina e legibilidade do hint final.
9. Hierarquia de elevacao: confirmar que a leitura permanece como plano principal (toolbar e sidebars discretos, elevacao forte apenas em elementos efemeros).
10. Highlights e motion: validar pulso de destaque mais suave e comportamento consistente com a opcao de reduzir animacoes.
