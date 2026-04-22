import { useState } from "react";
import { CustomSelect, type SelectOption } from "./ui/CustomSelect";

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

type AnnotationSidebarProps = {
  annotations: Annotation[];
  loading: boolean;
  onAddNote: (annotationId: string, noteText: string) => Promise<void>;
  onDelete: (annotationId: string) => Promise<void>;
  onColorChange: (annotationId: string, color: Annotation["color"]) => Promise<void>;
  onSelectAnnotation: (annotation: Annotation) => Promise<void>;
};

type AnnotationFilter = "all" | "highlight" | "bookmark";

const colorOptions: Annotation["color"][] = [
  "yellow",
  "green",
  "blue",
  "pink",
  "purple",
];

const colorOptionLabels: Record<Annotation["color"], string> = {
  yellow: "Amarelo",
  green: "Verde",
  blue: "Azul",
  pink: "Rosa",
  purple: "Roxo",
};

const colorSelectOptions: SelectOption[] = colorOptions.map((option) => ({
  value: option,
  label: colorOptionLabels[option],
}));

export function AnnotationSidebar({
  annotations,
  loading,
  onAddNote,
  onDelete,
  onColorChange,
  onSelectAnnotation,
}: AnnotationSidebarProps) {
  const [draftNotes, setDraftNotes] = useState<Record<string, string>>({});
  const [activeFilter, setActiveFilter] = useState<AnnotationFilter>("all");
  const filteredAnnotations = annotations.filter((annotation) => {
    if (activeFilter === "all") {
      return true;
    }

    return annotation.annotation_type === activeFilter;
  });

  const formatAnnotationType = (annotationType: string): string => {
    if (annotationType === "highlight") {
      return "Destaque";
    }

    if (annotationType === "bookmark") {
      return "Marcador";
    }

    return annotationType;
  };

  return (
    <aside className="annotation-sidebar">
      <h3>Anotações</h3>
      <div className="annotation-filters" role="group" aria-label="Filtrar anotações">
        <button
          type="button"
          className={activeFilter === "all" ? "active" : ""}
          onClick={() => setActiveFilter("all")}
        >
          Todas
        </button>
        <button
          type="button"
          className={activeFilter === "highlight" ? "active" : ""}
          onClick={() => setActiveFilter("highlight")}
        >
          Destaques
        </button>
        <button
          type="button"
          className={activeFilter === "bookmark" ? "active" : ""}
          onClick={() => setActiveFilter("bookmark")}
        >
          Marcadores
        </button>
      </div>

      {loading && <p>Carregando anotações...</p>}
      {!loading && filteredAnnotations.length === 0 && <p>Nenhuma anotação neste filtro.</p>}

      {!loading &&
        filteredAnnotations.map((annotation) => {
          const draft = draftNotes[annotation.id] ?? annotation.note_text ?? "";
          const previewText = annotation.selected_text?.trim()
            ? annotation.selected_text
            : annotation.annotation_type === "bookmark"
              ? "(marcador de posição)"
              : "(sem texto selecionado)";

          return (
            <article
              key={annotation.id}
              className={`annotation-item color-${annotation.color}`}
              onClick={() => void onSelectAnnotation(annotation)}
              role="button"
              tabIndex={0}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  void onSelectAnnotation(annotation);
                }
              }}
            >
              <header>
                <span className={`annotation-color-dot ${annotation.color}`} />
                <strong>{formatAnnotationType(annotation.annotation_type)}</strong>
              </header>

              <p className="annotation-text">{previewText}</p>

              <label>
                Cor
                <div
                  onClick={(event) => event.stopPropagation()}
                  onKeyDown={(event) => event.stopPropagation()}
                >
                  <CustomSelect
                    ariaLabel="Cor da anotação"
                    value={annotation.color}
                    options={colorSelectOptions}
                    onValueChange={(nextValue) => {
                      void onColorChange(annotation.id, nextValue as Annotation["color"]);
                    }}
                    triggerClassName="annotation-color-select-trigger"
                    menuClassName="annotation-color-select-menu"
                    optionClassName="annotation-color-select-option"
                  />
                </div>
              </label>

              <label>
                Nota
                <textarea
                  value={draft}
                  onClick={(event) => event.stopPropagation()}
                  onKeyDown={(event) => event.stopPropagation()}
                  onChange={(event) => {
                    const nextValue = event.currentTarget.value;
                    setDraftNotes((prev) => ({
                      ...prev,
                      [annotation.id]: nextValue,
                    }));
                  }}
                  rows={3}
                />
              </label>

              <div className="annotation-actions">
                <button
                  type="button"
                  onClick={(event) => {
                    event.stopPropagation();
                    void onAddNote(annotation.id, draft);
                  }}
                >
                  Salvar nota
                </button>
                <button
                  type="button"
                  onClick={(event) => {
                    event.stopPropagation();
                    void onDelete(annotation.id);
                  }}
                >
                  Excluir
                </button>
              </div>
            </article>
          );
        })}
    </aside>
  );
}
