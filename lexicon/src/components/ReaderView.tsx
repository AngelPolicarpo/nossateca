import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AnnotationSidebar } from "./AnnotationSidebar";
import "./ReaderView.css";

type ReaderViewProps = {
  bookId: string;
  onClose: () => void;
};

type BookContent = {
  html: string;
  current_chapter: number;
  total_chapters: number;
  chapter_title: string;
  book_format: string;
  book_file_path: string | null;
  supports_annotations: boolean;
};

type PdfDocumentData = {
  bytes_base64: string;
  total_pages: number;
};

type Annotation = {
  id: string;
  book_id: number;
  annotation_type: string;
  position: string;
  position_end: string | null;
  selected_text: string | null;
  note_text: string | null;
  color: "yellow" | "green" | "blue" | "pink" | "purple";
  created_at: string;
  updated_at: string;
};

type FloatingHighlightAction = {
  text: string;
  position: string;
  top: number;
  left: number;
};

type ReaderTheme = "light" | "dark" | "sepia";

const HIGHLIGHT_COLORS = ["yellow", "green", "blue", "pink", "purple"] as const;

function getErrorMessage(err: unknown, fallback: string): string {
  if (err instanceof Error && err.message) {
    return err.message;
  }

  if (typeof err === "string" && err.trim().length > 0) {
    return err;
  }

  if (
    typeof err === "object" &&
    err !== null &&
    "message" in err &&
    typeof (err as { message?: unknown }).message === "string"
  ) {
    return (err as { message: string }).message;
  }

  return fallback;
}

function decodeBase64ToBytes(base64Value: string): Uint8Array {
  const binary = window.atob(base64Value);
  const bytes = new Uint8Array(binary.length);

  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }

  return bytes;
}

export function ReaderView({ bookId, onClose }: ReaderViewProps) {
  const contentRef = useRef<HTMLElement | null>(null);
  const pdfCanvasRef = useRef<HTMLCanvasElement | null>(null);
  const pdfDocumentRef = useRef<any | null>(null);
  const pdfGetDocumentRef = useRef<((source: unknown) => { promise: Promise<any> }) | null>(null);
  const pdfLoaderPromiseRef = useRef<Promise<((source: unknown) => { promise: Promise<any> })> | null>(null);

  const [content, setContent] = useState("");
  const [chapterTitle, setChapterTitle] = useState("");
  const [currentChapter, setCurrentChapter] = useState(0);
  const [totalChapters, setTotalChapters] = useState(0);
  const [bookFormat, setBookFormat] = useState("epub");
  const [supportsAnnotations, setSupportsAnnotations] = useState(true);
  const [annotations, setAnnotations] = useState<Annotation[]>([]);
  const [annotationsLoading, setAnnotationsLoading] = useState(false);
  const [tocOpen, setTocOpen] = useState(true);
  const [notesOpen, setNotesOpen] = useState(true);
  const [readerTheme, setReaderTheme] = useState<ReaderTheme>("light");
  const [floatingAction, setFloatingAction] = useState<FloatingHighlightAction | null>(null);
  const [pendingFocusAnnotationId, setPendingFocusAnnotationId] = useState<string | null>(null);
  const [pdfLoading, setPdfLoading] = useState(false);
  const [pdfRenderError, setPdfRenderError] = useState<string | null>(null);
  const [pdfPageRendering, setPdfPageRendering] = useState(false);
  const [pageInput, setPageInput] = useState("1");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const isPdfBook = bookFormat === "pdf";
  const navigationUnit = isPdfBook ? "Página" : "Capítulo";

  const ensurePdfEngineLoaded = useCallback(async () => {
    if (pdfGetDocumentRef.current) {
      return pdfGetDocumentRef.current;
    }

    if (!pdfLoaderPromiseRef.current) {
      pdfLoaderPromiseRef.current = Promise.all([
        import("pdfjs-dist"),
        import("pdfjs-dist/build/pdf.worker.min.mjs?url"),
      ])
        .then(([pdfjsModule, workerModule]) => {
          if (typeof workerModule.default !== "string" || workerModule.default.length === 0) {
            throw new Error("Worker PDF indisponível");
          }

          pdfjsModule.GlobalWorkerOptions.workerSrc = workerModule.default;
          const dynamicGetDocument = pdfjsModule.getDocument as (source: unknown) => {
            promise: Promise<any>;
          };
          pdfGetDocumentRef.current = dynamicGetDocument;
          return dynamicGetDocument;
        })
        .finally(() => {
          pdfLoaderPromiseRef.current = null;
        });
    }

    return pdfLoaderPromiseRef.current;
  }, []);

  const loadAnnotations = useCallback(async () => {
    setAnnotationsLoading(true);
    try {
      const result = await invoke<Annotation[]>("get_annotations", { bookId });
      setAnnotations(result);
    } finally {
      setAnnotationsLoading(false);
    }
  }, [bookId]);

  const loadChapter = useCallback(
    async (chapterIndex: number) => {
      setLoading(true);
      setError(null);
      let chapterLoaded = false;
      let supportsAnnotationsForBook = false;

      try {
        const result = await invoke<BookContent>("get_book_content", {
          bookId,
          chapterIndex,
        });

        setContent(result.html);
        setChapterTitle(result.chapter_title);
        setCurrentChapter(result.current_chapter);
        setTotalChapters(result.total_chapters);
        setBookFormat(result.book_format);
        setSupportsAnnotations(result.supports_annotations);
        setPdfRenderError(null);

        supportsAnnotationsForBook = result.supports_annotations;
        chapterLoaded = true;

        if (result.book_format === "pdf") {
          setTocOpen(false);
          setNotesOpen(false);
        }
      } catch (err) {
        const message = getErrorMessage(err, "Erro ao carregar capítulo");
        setError(message);
      } finally {
        setLoading(false);
      }

      if (!chapterLoaded) {
        return;
      }

      try {
        await invoke("save_progress", {
          bookId,
          chapterIndex,
          scrollPosition: null,
        });
      } catch (err) {
        console.warn("Falha ao salvar progresso:", getErrorMessage(err, "erro desconhecido"));
      }

      if (!supportsAnnotationsForBook) {
        setAnnotations([]);
        setFloatingAction(null);
        setPendingFocusAnnotationId(null);
        return;
      }

      try {
        await loadAnnotations();
      } catch (err) {
        console.warn("Falha ao carregar anotações:", getErrorMessage(err, "erro desconhecido"));
      }
    },
    [bookId, loadAnnotations],
  );

  useEffect(() => {
    void loadChapter(0);
  }, [loadChapter]);

  useEffect(() => {
    if (!isPdfBook) {
      const activeDocument = pdfDocumentRef.current;
      if (activeDocument && typeof activeDocument.destroy === "function") {
        void activeDocument.destroy();
      }
      pdfDocumentRef.current = null;
      setPdfLoading(false);
      setPdfPageRendering(false);
      setPdfRenderError(null);
      return;
    }

    let cancelled = false;

    const loadPdfDocument = async () => {
      setPdfLoading(true);
      setPdfRenderError(null);

      try {
        const getDocument = await ensurePdfEngineLoaded();
        const result = await invoke<PdfDocumentData>("get_pdf_document", { bookId });
        const sourceBytes = decodeBase64ToBytes(result.bytes_base64);
        const loadingTask = getDocument({ data: sourceBytes });
        const loadedPdf = await loadingTask.promise;

        if (cancelled) {
          await loadedPdf.destroy();
          return;
        }

        const previousPdf = pdfDocumentRef.current;
        if (previousPdf && typeof previousPdf.destroy === "function") {
          await previousPdf.destroy();
        }

        pdfDocumentRef.current = loadedPdf;

        const resolvedPages = Math.max(result.total_pages, loadedPdf.numPages, 1);
        setTotalChapters(resolvedPages);
        setCurrentChapter((previous) => Math.min(previous, resolvedPages - 1));
      } catch (err) {
        setPdfRenderError(getErrorMessage(err, "Falha ao carregar PDF"));
      } finally {
        if (!cancelled) {
          setPdfLoading(false);
        }
      }
    };

    void loadPdfDocument();

    return () => {
      cancelled = true;
    };
  }, [bookId, ensurePdfEngineLoaded, isPdfBook]);

  useEffect(() => {
    return () => {
      const activeDocument = pdfDocumentRef.current;
      if (activeDocument && typeof activeDocument.destroy === "function") {
        void activeDocument.destroy();
      }
      pdfDocumentRef.current = null;
    };
  }, []);

  useEffect(() => {
    if (!isPdfBook) {
      return;
    }

    setPageInput(String(currentChapter + 1));
  }, [currentChapter, isPdfBook]);

  useEffect(() => {
    if (!isPdfBook || !pdfDocumentRef.current || !pdfCanvasRef.current) {
      return;
    }

    let cancelled = false;

    const renderCurrentPdfPage = async () => {
      setPdfPageRendering(true);
      setPdfRenderError(null);

      try {
        const loadedPdf = pdfDocumentRef.current;
        const pageNumber = Math.min(Math.max(currentChapter + 1, 1), loadedPdf.numPages);
        const page = await loadedPdf.getPage(pageNumber);

        if (cancelled) {
          return;
        }

        const viewport = page.getViewport({ scale: 1.35 });
        const canvas = pdfCanvasRef.current;
        if (!canvas) {
          return;
        }

        const context = canvas.getContext("2d");
        if (!context) {
          throw new Error("Canvas de renderização indisponível");
        }

        const outputScale = window.devicePixelRatio || 1;

        canvas.width = Math.floor(viewport.width * outputScale);
        canvas.height = Math.floor(viewport.height * outputScale);
        canvas.style.width = `${Math.floor(viewport.width)}px`;
        canvas.style.height = `${Math.floor(viewport.height)}px`;

        context.setTransform(outputScale, 0, 0, outputScale, 0, 0);

        const renderTask = page.render({ canvasContext: context, viewport });
        await renderTask.promise;
      } catch (err) {
        if (!cancelled) {
          setPdfRenderError(getErrorMessage(err, "Falha ao renderizar página do PDF"));
        }
      } finally {
        if (!cancelled) {
          setPdfPageRendering(false);
        }
      }
    };

    void renderCurrentPdfPage();

    return () => {
      cancelled = true;
    };
  }, [currentChapter, isPdfBook]);

  const buildSelectionPosition = (text: string): string => {
    const hash = Array.from(text).reduce((acc, char) => (acc * 31 + char.charCodeAt(0)) >>> 0, 7);
    return `chapter:${currentChapter};hash:${hash};len:${text.length}`;
  };

  const getAnnotationChapter = (position: string): number | null => {
    const match = position.match(/chapter:(\d+)/);
    if (!match) {
      return null;
    }

    const chapter = Number.parseInt(match[1], 10);
    return Number.isNaN(chapter) ? null : chapter;
  };

  const escapeRegExp = (value: string): string => value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

  const injectHighlights = useCallback(
    (rawHtml: string): string => {
      const chapterAnnotations = annotations.filter((annotation) => {
        if (annotation.annotation_type !== "highlight") {
          return false;
        }
        return getAnnotationChapter(annotation.position) === currentChapter;
      });

      let html = rawHtml;

      for (const annotation of chapterAnnotations) {
        const text = annotation.selected_text?.trim();
        if (!text) {
          continue;
        }

        const color = HIGHLIGHT_COLORS.includes(annotation.color) ? annotation.color : "yellow";
        const replacement = `<span id="annotation-${annotation.id}" data-annotation-id="${annotation.id}" class="reader-highlight highlight-${color}">${text}</span>`;

        const pattern = new RegExp(escapeRegExp(text));
        html = html.replace(pattern, replacement);
      }

      return html;
    },
    [annotations, currentChapter],
  );

  const renderedContent = injectHighlights(content);

  const chapterItems = useMemo(
    () => Array.from({ length: totalChapters }, (_, index) => index),
    [totalChapters],
  );

  useEffect(() => {
    if (!supportsAnnotations) {
      return;
    }

    const contentEl = contentRef.current;
    if (!contentEl) {
      return;
    }

    const handleMouseUp = () => {
      const selection = window.getSelection();
      if (!selection || selection.rangeCount === 0) {
        setFloatingAction(null);
        return;
      }

      const selectedText = selection.toString().trim();
      if (!selectedText) {
        setFloatingAction(null);
        return;
      }

      const range = selection.getRangeAt(0);
      const rect = range.getBoundingClientRect();
      setFloatingAction({
        text: selectedText,
        position: buildSelectionPosition(selectedText),
        top: rect.top + window.scrollY - 40,
        left: rect.left + window.scrollX,
      });
    };

    contentEl.addEventListener("mouseup", handleMouseUp);
    return () => {
      contentEl.removeEventListener("mouseup", handleMouseUp);
    };
  }, [content, currentChapter, supportsAnnotations]);

  useEffect(() => {
    if (!supportsAnnotations || !pendingFocusAnnotationId) {
      return;
    }

    const target = document.getElementById(`annotation-${pendingFocusAnnotationId}`);
    if (!target) {
      return;
    }

    target.scrollIntoView({ behavior: "smooth", block: "center" });
    target.classList.add("highlight-pulse");

    const timeout = window.setTimeout(() => {
      target.classList.remove("highlight-pulse");
    }, 1400);

    setPendingFocusAnnotationId(null);

    return () => window.clearTimeout(timeout);
  }, [pendingFocusAnnotationId, renderedContent, supportsAnnotations]);

  const handleCreateHighlight = async () => {
    if (!supportsAnnotations || !floatingAction) {
      return;
    }

    try {
      await invoke("add_annotation", {
        bookId,
        annotation: {
          annotationType: "highlight",
          position: floatingAction.position,
          positionEnd: null,
          selectedText: floatingAction.text,
          noteText: null,
          color: "yellow",
        },
      });

      setFloatingAction(null);
      window.getSelection()?.removeAllRanges();
      await loadAnnotations();
      setNotesOpen(true);
    } catch (err) {
      const message = getErrorMessage(err, "Falha ao criar destaque");
      alert(message);
    }
  };

  const handleUpdateNote = async (annotationId: string, noteText: string) => {
    try {
      await invoke("update_annotation_note", { id: annotationId, noteText });
      await loadAnnotations();
    } catch (err) {
      alert(getErrorMessage(err, "Falha ao atualizar nota"));
    }
  };

  const handleUpdateColor = async (
    annotationId: string,
    color: "yellow" | "green" | "blue" | "pink" | "purple",
  ) => {
    try {
      await invoke("update_annotation_color", { id: annotationId, color });
      await loadAnnotations();
    } catch (err) {
      alert(getErrorMessage(err, "Falha ao atualizar cor"));
    }
  };

  const handleDeleteAnnotation = async (annotationId: string) => {
    try {
      await invoke("delete_annotation", { id: annotationId });
      await loadAnnotations();
    } catch (err) {
      alert(getErrorMessage(err, "Falha ao excluir anotação"));
    }
  };

  const handleSelectAnnotation = async (annotation: Annotation) => {
    if (!supportsAnnotations) {
      return;
    }

    const targetChapter = getAnnotationChapter(annotation.position);

    if (targetChapter === null) {
      return;
    }

    if (targetChapter !== currentChapter) {
      await loadChapter(targetChapter);
    }

    setPendingFocusAnnotationId(annotation.id);
  };

  const persistProgress = useCallback(
    async (index: number) => {
      try {
        await invoke("save_progress", {
          bookId,
          chapterIndex: index,
          scrollPosition: null,
        });
      } catch (err) {
        console.warn("Falha ao salvar progresso:", getErrorMessage(err, "erro desconhecido"));
      }
    },
    [bookId],
  );

  const goToPosition = useCallback(
    async (targetIndex: number) => {
      const maxIndex = Math.max(totalChapters - 1, 0);
      const clamped = Math.min(Math.max(targetIndex, 0), maxIndex);

      if (isPdfBook) {
        setCurrentChapter(clamped);
        await persistProgress(clamped);
        return;
      }

      if (clamped !== currentChapter) {
        await loadChapter(clamped);
      }
    },
    [currentChapter, isPdfBook, loadChapter, persistProgress, totalChapters],
  );

  const handlePageInputSubmit = () => {
    const parsedPage = Number.parseInt(pageInput, 10);
    if (Number.isNaN(parsedPage)) {
      setPageInput(String(currentChapter + 1));
      return;
    }

    void goToPosition(parsedPage - 1);
  };

  const canGoPrevious = currentChapter > 0;
  const canGoNext = totalChapters > 0 && currentChapter < totalChapters - 1;

  return (
    <main className={`reader-page theme-${readerTheme}`}>
      <header className="reader-toolbar">
        <div className="reader-toolbar-group">
          <button type="button" onClick={onClose}>
            Voltar
          </button>
          {!isPdfBook && (
            <button type="button" onClick={() => setTocOpen((prev) => !prev)}>
              {tocOpen ? "Ocultar índice" : "Índice"}
            </button>
          )}
        </div>

        <h2>{chapterTitle || "Leitor de documentos"}</h2>

        <div className="reader-toolbar-group reader-toolbar-controls">
          <div className="reader-theme-switch" role="group" aria-label="Tema de leitura">
            <button
              type="button"
              className={readerTheme === "light" ? "active" : ""}
              aria-pressed={readerTheme === "light"}
              onClick={() => setReaderTheme("light")}
            >
              Light
            </button>
            <button
              type="button"
              className={readerTheme === "dark" ? "active" : ""}
              aria-pressed={readerTheme === "dark"}
              onClick={() => setReaderTheme("dark")}
            >
              Dark
            </button>
            <button
              type="button"
              className={readerTheme === "sepia" ? "active" : ""}
              aria-pressed={readerTheme === "sepia"}
              onClick={() => setReaderTheme("sepia")}
            >
              Sepia
            </button>
          </div>

          {supportsAnnotations && (
            <button type="button" onClick={() => setNotesOpen((prev) => !prev)}>
              {notesOpen ? "Ocultar notas" : "Notas"}
            </button>
          )}
          {isPdfBook && <span className="reader-mode-pill">PDF</span>}
        </div>
      </header>

      <section className="reader-layout">
        {!isPdfBook && tocOpen && (
          <aside className="reader-left-sidebar">
            <h3>Índice</h3>
            {chapterItems.length === 0 && <p>Sem capítulos identificados.</p>}
            {chapterItems.length > 0 && (
              <ol>
                {chapterItems.map((index) => (
                  <li key={index}>
                    <button
                      type="button"
                      className={index === currentChapter ? "active" : ""}
                      onClick={() => void goToPosition(index)}
                    >
                      Capítulo {index + 1}
                    </button>
                  </li>
                ))}
              </ol>
            )}
          </aside>
        )}

        <section className="reader-main-column">
          <div className="reader-nav">
            <button type="button" onClick={() => void goToPosition(currentChapter - 1)} disabled={!canGoPrevious}>
              Anterior
            </button>
            <p>
              {navigationUnit} {currentChapter + 1} de {totalChapters || "-"}
            </p>
            <button type="button" onClick={() => void goToPosition(currentChapter + 1)} disabled={!canGoNext}>
              Próximo
            </button>

            {isPdfBook && (
              <label className="reader-page-jump">
                Ir para página
                <input
                  type="number"
                  min={1}
                  max={Math.max(totalChapters, 1)}
                  value={pageInput}
                  onChange={(event) => setPageInput(event.currentTarget.value)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") {
                      event.preventDefault();
                      handlePageInputSubmit();
                    }
                  }}
                  onBlur={handlePageInputSubmit}
                />
              </label>
            )}
          </div>

          {loading && <p className="reader-state">Carregando {navigationUnit.toLowerCase()}...</p>}
          {error && <p className="reader-state reader-error">{error}</p>}

          {!loading && !error && !isPdfBook && (
            <section
              ref={contentRef}
              className="reader-content"
              dangerouslySetInnerHTML={{ __html: renderedContent }}
            />
          )}

          {!loading && !error && isPdfBook && (
            <section className="reader-pdf-shell">
              {pdfLoading && <p className="reader-state">Carregando PDF...</p>}
              {pdfPageRendering && <p className="reader-state">Renderizando página...</p>}
              {pdfRenderError && <p className="reader-state reader-error">{pdfRenderError}</p>}

              {!pdfLoading && !pdfRenderError && (
                <div className="reader-pdf-canvas-wrap">
                  <canvas ref={pdfCanvasRef} className="reader-pdf-canvas" />
                </div>
              )}

              <p className="reader-pdf-hint">
                Leitura em PDF ativa. Anotações e destaques para PDF serão adicionados em uma próxima versão.
              </p>
            </section>
          )}
        </section>

        {supportsAnnotations && notesOpen && (
          <aside className="reader-right-sidebar">
            <div className="reader-right-header">
              <strong>Notas e destaques</strong>
              <button type="button" onClick={() => setNotesOpen(false)}>
                Fechar
              </button>
            </div>

            <div className="reader-right-content">
              <AnnotationSidebar
                annotations={annotations}
                loading={annotationsLoading}
                onAddNote={handleUpdateNote}
                onDelete={handleDeleteAnnotation}
                onColorChange={handleUpdateColor}
                onSelectAnnotation={handleSelectAnnotation}
              />
            </div>
          </aside>
        )}
      </section>

      {supportsAnnotations && floatingAction && (
        <div
          className="floating-action-menu"
          style={{ top: `${floatingAction.top}px`, left: `${floatingAction.left}px` }}
        >
          <button type="button" onClick={() => void handleCreateHighlight()}>
            Highlight
          </button>
        </div>
      )}
    </main>
  );
}
