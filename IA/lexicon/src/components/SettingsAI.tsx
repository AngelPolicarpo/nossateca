import { ChangeEvent, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openPath, openUrl } from "@tauri-apps/plugin-opener";

type AiSettings = {
  aiModelPath: string;
  aiEmbeddingPath: string;
};

type SearchPluginSettings = {
  annaArchiveRapidapiKey: string;
  annaArchiveRapidapiHost: string;
};

type AiSetupInfo = {
  aiModelPath: string;
  aiEmbeddingPath: string;
  llmModelType: string | null;
  embeddingModelType: string | null;
  defaultModelsDir: string;
  detectedGguf: string[];
  detectedOnnx: string[];
  llmConfigured: boolean;
  embeddingConfigured: boolean;
  llmFileSizeMb: number | null;
  embeddingFileSizeMb: number | null;
};

type SettingsAIProps = {
  onClose: () => void;
};

export function SettingsAI({ onClose }: SettingsAIProps) {
  const [settings, setSettings] = useState<AiSettings>({
    aiModelPath: "",
    aiEmbeddingPath: "",
  });
  const [pluginSettings, setPluginSettings] = useState<SearchPluginSettings>({
    annaArchiveRapidapiKey: "",
    annaArchiveRapidapiHost: "",
  });
  const [setupInfo, setSetupInfo] = useState<AiSetupInfo | null>(null);
  const [saving, setSaving] = useState(false);
  const [loading, setLoading] = useState(true);
  const [showDownloadHelp, setShowDownloadHelp] = useState(false);

  const refreshSetupInfo = async () => {
    const [info, plugin] = await Promise.all([
      invoke<AiSetupInfo>("get_ai_setup_info"),
      invoke<SearchPluginSettings>("get_search_plugin_settings"),
    ]);

    setSetupInfo(info);
    setSettings({
      aiModelPath: info.aiModelPath,
      aiEmbeddingPath: info.aiEmbeddingPath,
    });
    setPluginSettings(plugin);
  };

  useEffect(() => {
    const load = async () => {
      setLoading(true);
      try {
        await refreshSetupInfo();
      } catch {
        // keep defaults
      } finally {
        setLoading(false);
      }
    };

    void load();
  }, []);

  const save = async () => {
    setSaving(true);
    try {
      await invoke("update_search_plugin_settings", { settings: pluginSettings });

      const hasAnyAiPathConfigured =
        settings.aiModelPath.trim().length > 0 || settings.aiEmbeddingPath.trim().length > 0;

      if (hasAnyAiPathConfigured) {
        await invoke("update_ai_settings", { settings });
      }

      alert("Configurações salvas.");
      await refreshSetupInfo();
    } catch (err) {
      const message = err instanceof Error ? err.message : "Falha ao salvar configurações";
      alert(message);
    } finally {
      setSaving(false);
    }
  };

  const openModelsFolder = async () => {
    try {
      const directory = await invoke<string>("ensure_models_directory");
      await openPath(directory);
    } catch (err) {
      const message = err instanceof Error ? err.message : "Falha ao abrir pasta de modelos";
      alert(message);
    }
  };

  const detectedCount =
    (setupInfo?.detectedGguf.length ?? 0) + (setupInfo?.detectedOnnx.length ?? 0);

  const applySuggestion = (type: "gguf" | "onnx", value: string) => {
    if (type === "gguf") {
      setSettings((prev) => ({ ...prev, aiModelPath: value }));
      return;
    }

    setSettings((prev) => ({ ...prev, aiEmbeddingPath: value }));
  };

  const handleModelPathChange = (event: ChangeEvent<HTMLInputElement>) => {
    const value = event.target?.value ?? "";
    setSettings((prev) => ({ ...prev, aiModelPath: value }));
  };

  const handleEmbeddingPathChange = (event: ChangeEvent<HTMLInputElement>) => {
    const value = event.target?.value ?? "";
    setSettings((prev) => ({ ...prev, aiEmbeddingPath: value }));
  };

  const handleRapidApiKeyChange = (event: ChangeEvent<HTMLInputElement>) => {
    const value = event.target?.value ?? "";
    setPluginSettings((prev) => ({ ...prev, annaArchiveRapidapiKey: value }));
  };

  const handleRapidApiHostChange = (event: ChangeEvent<HTMLInputElement>) => {
    const value = event.target?.value ?? "";
    setPluginSettings((prev) => ({ ...prev, annaArchiveRapidapiHost: value }));
  };

  return (
    <section className="settings-ai-modal">
      <div className="settings-ai-header">
        <h3>Configurações de IA</h3>
        <button type="button" onClick={onClose}>Fechar</button>
      </div>

      {loading && <p className="settings-ai-hint">Carregando configuração...</p>}

      {!loading && setupInfo && (
        <>
          <p className="settings-ai-hint">
            Pasta padrão: <strong>{setupInfo.defaultModelsDir}</strong>
          </p>

          <p className="settings-ai-hint">
            {detectedCount > 0
              ? `Modelos detectados: ${detectedCount} encontrados`
              : "Nenhum modelo encontrado"}
          </p>
        </>
      )}

      <div className="settings-ai-status-list">
        <p>
          {setupInfo?.llmConfigured ? "✓ Configurado" : "✗ Não configurado"} LLM GGUF
          {setupInfo?.llmFileSizeMb ? ` (${setupInfo.llmFileSizeMb} MB)` : ""}
        </p>
        <p>
          {setupInfo?.embeddingConfigured ? "✓ Configurado" : "✗ Não configurado"} Embeddings
          {setupInfo?.embeddingModelType ? ` (${setupInfo.embeddingModelType.toUpperCase()})` : ""}
          {setupInfo?.embeddingFileSizeMb ? ` (${setupInfo.embeddingFileSizeMb} MB)` : ""}
        </p>
        <p>
          {pluginSettings.annaArchiveRapidapiKey.trim().length > 0
            ? "✓ Configurado"
            : "✗ Não configurado"}{" "}
          Plugin Anna Archive API
        </p>
      </div>

      <label>
        Caminho do modelo GGUF
        <input
          value={settings.aiModelPath}
          onChange={handleModelPathChange}
          placeholder="/caminho/modelo.gguf"
        />
      </label>

      {setupInfo && setupInfo.detectedGguf.length > 0 && (
        <div className="settings-ai-suggestions">
          {setupInfo.detectedGguf.map((path) => (
            <button
              key={path}
              type="button"
              className="secondary-button"
              onClick={() => applySuggestion("gguf", path)}
            >
              Usar detectado: {path.split("/").pop()}
            </button>
          ))}
        </div>
      )}

      <label>
        Caminho do modelo de embeddings (ONNX ou GGUF)
        <input
          value={settings.aiEmbeddingPath}
          onChange={handleEmbeddingPathChange}
          placeholder="/caminho/modelo.onnx ou /caminho/modelo.gguf"
        />
      </label>

      {setupInfo && setupInfo.detectedOnnx.length > 0 && (
        <div className="settings-ai-suggestions">
          {setupInfo.detectedOnnx.map((path) => (
            <button
              key={path}
              type="button"
              className="secondary-button"
              onClick={() => applySuggestion("onnx", path)}
            >
              Usar detectado: {path.split("/").pop()}
            </button>
          ))}
        </div>
      )}

      <label>
        Anna Archive RapidAPI Key
        <input
          type="password"
          value={pluginSettings.annaArchiveRapidapiKey}
          onChange={handleRapidApiKeyChange}
          placeholder="Cole sua chave RapidAPI"
          autoComplete="off"
        />
      </label>

      <label>
        Anna Archive RapidAPI Host (opcional)
        <input
          value={pluginSettings.annaArchiveRapidapiHost}
          onChange={handleRapidApiHostChange}
          placeholder="annas-archive-api.p.rapidapi.com"
        />
      </label>

      <p className="settings-ai-hint">
        A chave do plugin Anna fica salva localmente em preferências e é aplicada automaticamente
        no runtime.
      </p>

      <div className="settings-ai-actions-row">
        <button type="button" onClick={() => setShowDownloadHelp(true)}>
          Como baixar modelos?
        </button>
        <button type="button" onClick={() => void openModelsFolder()}>
          Abrir pasta de modelos
        </button>
      </div>

      <div className="settings-ai-actions">
        <button type="button" onClick={onClose}>Cancelar</button>
        <button type="button" onClick={() => void save()} disabled={saving}>
          {saving ? "Salvando..." : "Salvar"}
        </button>
      </div>

      {showDownloadHelp && (
        <section className="settings-ai-help-modal">
          <h4>Como baixar modelos</h4>
          <p>LLM recomendado: Llama-2-7B-Chat-GGUF (Q4_K_M)</p>
          <p>
            <button
              type="button"
              onClick={() =>
                void openUrl(
                  "https://huggingface.co/TheBloke/Llama-2-7B-Chat-GGUF/resolve/main/llama-2-7b-chat.Q4_K_M.gguf",
                )
              }
            >
              Abrir link GGUF
            </button>
          </p>
          <p>Embeddings: sentence-transformers/all-MiniLM-L6-v2</p>
          <p>
            <button
              type="button"
              onClick={() =>
                void openUrl(
                  "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/onnx/model.onnx",
                )
              }
            >
              Abrir link ONNX
            </button>
          </p>
          <p>Baixe os arquivos e salve na pasta models/.</p>
          <pre>
curl -L -o modelo.gguf [URL]
          </pre>
          <div className="settings-ai-actions">
            <button type="button" onClick={() => setShowDownloadHelp(false)}>
              Fechar
            </button>
          </div>
        </section>
      )}
    </section>
  );
}
