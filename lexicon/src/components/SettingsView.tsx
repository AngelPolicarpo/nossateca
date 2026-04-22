import { useEffect, useMemo, useState } from "react";
import {
  DEFAULT_READER_SHORTCUTS,
  READER_SHORTCUT_DEFINITIONS,
  normalizeShortcutValue,
  parseShortcutBindings,
  type ReaderShortcutAction,
  type ReaderShortcutConfig,
} from "../lib/readerShortcuts";
import { Button } from "./ui/Button";
import { Input } from "./ui/Input";
import { Panel } from "./ui/Panel";
import { StateMessage } from "./ui/StateMessage";

type ThemeMode = "light" | "dark";

type SettingsViewProps = {
  themeMode: ThemeMode;
  onThemeModeChange: (nextThemeMode: ThemeMode) => void;
  readerShortcuts: ReaderShortcutConfig;
  onReaderShortcutsChange: (nextShortcuts: ReaderShortcutConfig) => void;
};

type FeedbackState = {
  tone: "default" | "error";
  message: string;
};

function buildNormalizedShortcutConfig(config: ReaderShortcutConfig): ReaderShortcutConfig {
  const normalized = { ...config };

  for (const definition of READER_SHORTCUT_DEFINITIONS) {
    normalized[definition.action] = normalizeShortcutValue(config[definition.action]);
  }

  return normalized;
}

function areShortcutConfigsEqual(
  left: ReaderShortcutConfig,
  right: ReaderShortcutConfig,
): boolean {
  return READER_SHORTCUT_DEFINITIONS.every(
    (definition) => left[definition.action] === right[definition.action],
  );
}

export function SettingsView({
  themeMode,
  onThemeModeChange,
  readerShortcuts,
  onReaderShortcutsChange,
}: SettingsViewProps) {
  const [shortcutDraft, setShortcutDraft] = useState<ReaderShortcutConfig>(readerShortcuts);
  const [feedback, setFeedback] = useState<FeedbackState | null>(null);

  useEffect(() => {
    setShortcutDraft(readerShortcuts);
  }, [readerShortcuts]);

  const hasPendingShortcutChanges = useMemo(
    () => !areShortcutConfigsEqual(shortcutDraft, readerShortcuts),
    [shortcutDraft, readerShortcuts],
  );

  const handleShortcutFieldChange = (action: ReaderShortcutAction, nextValue: string) => {
    setFeedback(null);
    setShortcutDraft((previous) => ({
      ...previous,
      [action]: nextValue,
    }));
  };

  const handleSaveShortcuts = () => {
    const normalizedDraft = buildNormalizedShortcutConfig(shortcutDraft);

    const invalidDefinitions = READER_SHORTCUT_DEFINITIONS.filter(
      (definition) => parseShortcutBindings(normalizedDraft[definition.action]).length === 0,
    );

    if (invalidDefinitions.length > 0) {
      setFeedback({
        tone: "error",
        message: `Formato de atalho inválido em: ${invalidDefinitions
          .map((definition) => definition.label)
          .join(", ")}. Use combinações como Ctrl+F, Cmd+F ou ArrowRight.`,
      });
      return;
    }

    onReaderShortcutsChange(normalizedDraft);
    setShortcutDraft(normalizedDraft);
    setFeedback({
      tone: "default",
      message: "Atalhos atualizados com sucesso.",
    });
  };

  const handleResetShortcuts = () => {
    const defaultConfig = { ...DEFAULT_READER_SHORTCUTS };
    setShortcutDraft(defaultConfig);
    setFeedback({
      tone: "default",
      message: "Atalhos restaurados para o padrão. Clique em salvar para aplicar.",
    });
  };

  const handleDiscardShortcutChanges = () => {
    setShortcutDraft(readerShortcuts);
    setFeedback(null);
  };

  return (
    <section className="lx-settings-shell">
      <header className="lx-page-header">
        <div className="lx-page-header-titles">
          <h1 className="lx-page-title">Configurações</h1>
          <p className="lx-page-subtitle">
            Personalize a aparência da aplicação e os atalhos do leitor.
          </p>
        </div>

        <div className="lx-page-header-actions">
          <Button
            variant={themeMode === "light" ? "primary" : "secondary"}
            onClick={() => onThemeModeChange("light")}
            aria-pressed={themeMode === "light"}
          >
            Modo claro
          </Button>
          <Button
            variant={themeMode === "dark" ? "primary" : "secondary"}
            onClick={() => onThemeModeChange("dark")}
            aria-pressed={themeMode === "dark"}
          >
            Modo escuro
          </Button>
        </div>
      </header>

      <Panel className="lx-settings-panel">
        <header className="lx-settings-panel-header">
          <h2 className="lx-settings-panel-title">
            Atalhos de teclado
          </h2>
          <p className="lx-settings-panel-description">
            Configure os atalhos já existentes no leitor. Você pode informar mais de uma opção
            separando por vírgula.
          </p>
        </header>

        <div className="lx-settings-shortcuts-list">
          {READER_SHORTCUT_DEFINITIONS.map((definition) => {
            const inputId = `reader-shortcut-${definition.action}`;

            return (
              <div
                key={definition.action}
                className="lx-settings-shortcut-card"
              >
                <label
                  htmlFor={inputId}
                  className="lx-settings-shortcut-label"
                >
                  {definition.label}
                </label>
                <p className="lx-settings-shortcut-description">
                  {definition.description}
                </p>
                <Input
                  id={inputId}
                  value={shortcutDraft[definition.action]}
                  onChange={(event) =>
                    handleShortcutFieldChange(definition.action, event.currentTarget.value)
                  }
                  placeholder={definition.placeholder}
                  autoComplete="off"
                  className="lx-settings-shortcut-input"
                />
              </div>
            );
          })}
        </div>

        <div className="lx-settings-actions">
          <Button
            variant="primary"
            onClick={handleSaveShortcuts}
            disabled={!hasPendingShortcutChanges}
            className="justify-start"
          >
            Salvar atalhos
          </Button>
          <Button variant="secondary" onClick={handleResetShortcuts} className="justify-start">
            Restaurar padrão
          </Button>
          <Button
            variant="secondary"
            onClick={handleDiscardShortcutChanges}
            disabled={!hasPendingShortcutChanges}
            className="justify-start"
          >
            Descartar alterações
          </Button>
        </div>

        <p className="lx-settings-shortcuts-help">
          Formatos aceitos: tecla simples (ex: F), combinação (ex: Ctrl+F) e múltiplas opções (ex:
          Ctrl+F, Cmd+F).
        </p>

        {feedback && <StateMessage tone={feedback.tone}>{feedback.message}</StateMessage>}
      </Panel>
    </section>
  );
}
