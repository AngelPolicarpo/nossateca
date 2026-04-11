import { useState } from "react";

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

const colorOptions: Annotation["color"][] = [
  "yellow",
  "green",
  "blue",
  "pink",
  "purple",
];

export function AnnotationSidebar({
  annotations,
  loading,
  onAddNote,
  onDelete,
  onColorChange,
  onSelectAnnotation,
}: AnnotationSidebarProps) {
  const [draftNotes, setDraftNotes] = useState<Record<string, string>>({});

  return (
    <aside className="annotation-sidebar">
      <h3>Anotações</h3>
      {loading && <p>Carregando anotações...</p>}
      {!loading && annotations.length === 0 && <p>Nenhuma anotação neste livro.</p>}

      {!loading &&
        annotations.map((annotation) => {
          const draft = draftNotes[annotation.id] ?? annotation.note_text ?? "";

          return (
            <article
              key={annotation.id}
              className="annotation-item"
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
                <strong>{annotation.annotation_type}</strong>
              </header>

              <p className="annotation-text">{annotation.selected_text ?? "(sem texto selecionado)"}</p>

              <label>
                Cor
                <select
                  value={annotation.color}
                  onClick={(event) => event.stopPropagation()}
                  onKeyDown={(event) => event.stopPropagation()}
                  onChange={(event) => {
                    event.stopPropagation();
                    const selectedColor = event.currentTarget.value as Annotation["color"];
                    void onColorChange(annotation.id, selectedColor);
                  }}
                >
                  {colorOptions.map((option) => (
                    <option key={option} value={option}>
                      {option}
                    </option>
                  ))}
                </select>
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
                  Adicionar nota
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
