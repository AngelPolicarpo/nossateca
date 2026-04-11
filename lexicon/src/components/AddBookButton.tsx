import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

type AddBookButtonProps = {
  onBookAdded: () => void | Promise<void>;
  label?: string;
  className?: string;
};

export function AddBookButton({
  onBookAdded,
  label = "Adicionar livro",
  className = "primary-button",
}: AddBookButtonProps) {
  const handleAddBook = async () => {
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [
          {
            name: "Livros digitais",
            extensions: ["epub", "pdf"],
          },
        ],
      });

      if (!selected || Array.isArray(selected)) {
        return;
      }

      await invoke("add_book", { filePath: selected });
      await Promise.resolve(onBookAdded());
      alert("Livro adicionado com sucesso.");
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Falha ao adicionar livro.";
      alert(message);
    }
  };

  return (
    <button type="button" onClick={handleAddBook} className={className}>
      {label}
    </button>
  );
}
