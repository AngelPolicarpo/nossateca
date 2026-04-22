import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "./ui/Button";

type AddBookButtonVariant = "primary" | "secondary" | "ghost" | "danger";
type AddBookButtonSize = "md" | "sm";

type AddBookButtonProps = {
  onBookAdded: () => void | Promise<void>;
  label?: string;
  className?: string;
  variant?: AddBookButtonVariant;
  size?: AddBookButtonSize;
};

export function AddBookButton({
  onBookAdded,
  label = "Adicionar livro",
  className,
  variant = "primary",
  size = "md",
}: AddBookButtonProps) {
  const handleAddBook = async () => {
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [
          {
            name: "Livros digitais",
            extensions: ["epub", "pdf", "cbz"],
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
    <Button
      type="button"
      variant={variant}
      size={size}
      onClick={() => void handleAddBook()}
      className={className}
    >
      {label}
    </Button>
  );
}
