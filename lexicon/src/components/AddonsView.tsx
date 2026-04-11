import { useCallback, useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";

import {
  type AddonDescriptor,
  type AddonSettingEntry,
  getAddonSettings,
  installAddon,
  listAddons,
  reloadAddons,
  removeAddon,
  updateAddonSettings,
} from "../hooks/useAddons";

function createEmptySetting(): AddonSettingEntry {
  return { key: "", value: "" };
}

export function AddonsView() {
  const [addons, setAddons] = useState<AddonDescriptor[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedAddonId, setSelectedAddonId] = useState<string | null>(null);
  const [settingsDraft, setSettingsDraft] = useState<AddonSettingEntry[]>([]);
  const [settingsLoading, setSettingsLoading] = useState(false);
  const [settingsError, setSettingsError] = useState<string | null>(null);
  const [savingSettings, setSavingSettings] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [reloading, setReloading] = useState(false);
  const [removingAddonId, setRemovingAddonId] = useState<string | null>(null);

  const getErrorMessage = (err: unknown, fallback: string): string => {
    if (err instanceof Error && err.message.trim().length > 0) {
      return err.message;
    }

    if (typeof err === "string" && err.trim().length > 0) {
      return err;
    }

    return fallback;
  };

  const loadAddons = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const result = await listAddons();
      setAddons(result);

      if (result.length === 0) {
        setSelectedAddonId(null);
        return;
      }

      setSelectedAddonId((previous) => {
        if (previous && result.some((addon) => addon.id === previous)) {
          return previous;
        }

        return result[0].id;
      });
    } catch (err) {
      setError(getErrorMessage(err, "Falha ao listar addons"));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadAddons();
  }, [loadAddons]);

  const selectedAddon = useMemo(
    () => addons.find((addon) => addon.id === selectedAddonId) ?? null,
    [addons, selectedAddonId],
  );

  const loadSelectedAddonSettings = useCallback(async () => {
    if (!selectedAddon) {
      setSettingsDraft([]);
      setSettingsError(null);
      return;
    }

    setSettingsLoading(true);
    setSettingsError(null);

    try {
      const settings = await getAddonSettings(selectedAddon.id);
      setSettingsDraft(settings.length > 0 ? settings : selectedAddon.settings);
    } catch (err) {
      setSettingsError(getErrorMessage(err, "Falha ao carregar configurações do addon"));
      setSettingsDraft(selectedAddon.settings);
    } finally {
      setSettingsLoading(false);
    }
  }, [selectedAddon]);

  useEffect(() => {
    void loadSelectedAddonSettings();
  }, [loadSelectedAddonSettings]);

  const handleInstallAddon = async () => {
    setInstalling(true);
    setError(null);

    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [
          {
            name: "WASM Addon",
            extensions: ["wasm"],
          },
        ],
      });

      if (!selected || Array.isArray(selected)) {
        return;
      }

      const installed = await installAddon(selected);
      await loadAddons();
      setSelectedAddonId(installed.id);
    } catch (err) {
      setError(getErrorMessage(err, "Falha ao instalar addon"));
    } finally {
      setInstalling(false);
    }
  };

  const handleReloadAddons = async () => {
    setReloading(true);
    setError(null);

    try {
      const result = await reloadAddons();
      setAddons(result);

      if (!result.some((addon) => addon.id === selectedAddonId)) {
        setSelectedAddonId(result[0]?.id ?? null);
      }
    } catch (err) {
      setError(getErrorMessage(err, "Falha ao recarregar addons"));
    } finally {
      setReloading(false);
    }
  };

  const handleRemoveAddon = async (addon: AddonDescriptor) => {
    const shouldRemove = window.confirm(
      `Remover o addon '${addon.id}'? Esta ação também remove as configurações salvas.`,
    );

    if (!shouldRemove) {
      return;
    }

    setRemovingAddonId(addon.id);
    setError(null);

    try {
      await removeAddon(addon.id);
      await loadAddons();
    } catch (err) {
      setError(getErrorMessage(err, "Falha ao remover addon"));
    } finally {
      setRemovingAddonId(null);
    }
  };

  const handleSettingChange = (index: number, field: keyof AddonSettingEntry, value: string) => {
    setSettingsDraft((previous) =>
      previous.map((setting, settingIndex) =>
        settingIndex === index
          ? {
              ...setting,
              [field]: value,
            }
          : setting,
      ),
    );
  };

  const handleSaveSettings = async () => {
    if (!selectedAddon) {
      return;
    }

    setSavingSettings(true);
    setSettingsError(null);

    try {
      await updateAddonSettings(selectedAddon.id, settingsDraft);
      await loadAddons();
    } catch (err) {
      setSettingsError(getErrorMessage(err, "Falha ao salvar configurações do addon"));
    } finally {
      setSavingSettings(false);
    }
  };

  return (
    <section className="addons-screen">
      <header className="addons-hero panel">
        <div className="addons-hero-content">
          <p className="hero-label">Extensões WASM</p>
          <h1>Addons</h1>
          <p className="addons-hero-copy">
            Instale manualmente plugins WASM e configure papéis isolados de Discover e Source.
            Nenhum addon nativo vem integrado ao runtime.
          </p>
        </div>

        <div className="addons-header-actions">
          <button
            type="button"
            className="secondary-button"
            onClick={() => void handleReloadAddons()}
            disabled={reloading}
          >
            {reloading ? "Recarregando..." : "Recarregar"}
          </button>
          <button
            type="button"
            className="primary-button"
            onClick={() => void handleInstallAddon()}
            disabled={installing}
          >
            {installing ? "Instalando..." : "+ Instalar Addon"}
          </button>
        </div>
      </header>

      {error && <p className="state-message error">{error}</p>}

      {loading && <p className="state-message">Carregando addons...</p>}

      {!loading && addons.length === 0 && (
        <section className="empty-state addons-empty-state">
          <h2>Nenhum addon instalado</h2>
          <p>Use o botão "Instalar Addon" para selecionar um arquivo .wasm manualmente.</p>
          <button
            type="button"
            className="primary-button"
            onClick={() => void handleInstallAddon()}
            disabled={installing}
          >
            {installing ? "Instalando..." : "Selecionar arquivo .wasm"}
          </button>
        </section>
      )}

      {!loading && addons.length > 0 && (
        <div className="addons-layout">
          <aside className="addons-layout-sidebar">
            <section className="addons-list panel">
              <header className="addons-panel-head">
                <h2>Addons instalados</h2>
                <p>Selecione um addon para editar as configurações.</p>
              </header>

              <ul>
                {addons.map((addon) => (
                  <li
                    key={addon.id}
                    className={`addon-card ${selectedAddonId === addon.id ? "active" : ""}`}
                  >
                    <button
                      type="button"
                      className={`addon-select-button ${selectedAddonId === addon.id ? "active" : ""}`}
                      onClick={() => setSelectedAddonId(addon.id)}
                    >
                      <strong>{addon.id}</strong>
                      <span>{addon.fileName}</span>
                      <span>Papel: {addon.role}</span>
                    </button>

                    <div className="addon-card-actions">
                      <button
                        type="button"
                        className="secondary-button danger compact"
                        onClick={() => void handleRemoveAddon(addon)}
                        disabled={removingAddonId === addon.id}
                      >
                        {removingAddonId === addon.id ? "Removendo..." : "Remover"}
                      </button>
                    </div>
                  </li>
                ))}
              </ul>
            </section>
          </aside>

          <section className="addons-layout-main">
            <section className="addons-settings panel">
              {!selectedAddon && (
                <section className="empty-state slim">
                  <h2>Selecione um addon</h2>
                  <p>Escolha um addon na lista para editar suas configurações.</p>
                </section>
              )}

              {selectedAddon && (
                <>
                  <header className="addons-settings-head">
                    <div>
                      <h2>Configurações do addon</h2>
                      <p>Edite pares chave/valor usados no runtime deste plugin.</p>
                    </div>
                  </header>

                  <dl className="addon-meta-grid">
                    <div>
                      <dt>ID</dt>
                      <dd>{selectedAddon.id}</dd>
                    </div>
                    <div>
                      <dt>Arquivo</dt>
                      <dd>{selectedAddon.fileName}</dd>
                    </div>
                    <div>
                      <dt>Papel</dt>
                      <dd>{selectedAddon.role}</dd>
                    </div>
                    <div className="addon-meta-path">
                      <dt>Caminho</dt>
                      <dd>{selectedAddon.filePath}</dd>
                    </div>
                  </dl>

                  {settingsError && <p className="state-message error">{settingsError}</p>}
                  {settingsLoading && <p className="state-message">Carregando configurações...</p>}

                  {!settingsLoading && (
                    <>
                      {settingsDraft.length === 0 && (
                        <p className="state-message">
                          Este addon ainda não possui chaves configuradas. Clique em "Nova chave" para
                          adicionar a primeira configuração.
                        </p>
                      )}

                      {settingsDraft.length > 0 && (
                        <div className="addon-settings-list">
                          {settingsDraft.map((setting, index) => {
                            const keyInputId = `addon-${selectedAddon.id}-setting-key-${index}`;
                            const valueInputId = `addon-${selectedAddon.id}-setting-value-${index}`;

                            return (
                              <div className="addon-setting-row" key={`${selectedAddon.id}-${index}`}>
                                <div className="addon-setting-field">
                                  <label htmlFor={keyInputId}>Chave</label>
                                  <input
                                    id={keyInputId}
                                    value={setting.key}
                                    onChange={(event) =>
                                      handleSettingChange(index, "key", event.target.value)
                                    }
                                    placeholder="ex: rapidapi_key"
                                  />
                                </div>

                                <div className="addon-setting-field">
                                  <label htmlFor={valueInputId}>Valor</label>
                                  <input
                                    id={valueInputId}
                                    value={setting.value}
                                    onChange={(event) =>
                                      handleSettingChange(index, "value", event.target.value)
                                    }
                                    placeholder="ex: sua-chave-aqui"
                                  />
                                </div>

                                <button
                                  type="button"
                                  className="secondary-button compact"
                                  onClick={() =>
                                    setSettingsDraft((previous) =>
                                      previous.filter((_, settingIndex) => settingIndex !== index),
                                    )
                                  }
                                >
                                  Remover
                                </button>
                              </div>
                            );
                          })}
                        </div>
                      )}

                      <div className="addons-settings-actions">
                        <button
                          type="button"
                          className="secondary-button"
                          onClick={() =>
                            setSettingsDraft((previous) => [...previous, createEmptySetting()])
                          }
                        >
                          + Nova chave
                        </button>
                        <button
                          type="button"
                          className="primary-button"
                          onClick={() => void handleSaveSettings()}
                          disabled={savingSettings}
                        >
                          {savingSettings ? "Salvando..." : "Salvar configurações"}
                        </button>
                      </div>
                    </>
                  )}
                </>
              )}
            </section>
          </section>
        </div>
      )}
    </section>
  );
}
