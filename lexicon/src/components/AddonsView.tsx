import { useCallback, useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useQueryClient } from "@tanstack/react-query";

import {
  type AddonDescriptor,
  type AddonRole,
  type AddonSettingEntry,
  getAddonSettings,
  installAddon,
  listAddons,
  reloadAddons,
  removeAddon,
  setAddonEnabled,
  updateAddonSettings,
} from "../hooks/useAddons";
import { Button } from "./ui/Button";
import { EmptyState } from "./ui/EmptyState";
import { Input } from "./ui/Input";
import { StateMessage } from "./ui/StateMessage";
import { cn } from "../lib/cn";

type AddonRoleMeta = {
  label: string;
  letter: string;
  toneClass: "discover" | "source" | "manga" | "legacy";
  description: string;
};

const ADDON_ROLE_META: Record<AddonRole, AddonRoleMeta> = {
  discover: {
    label: "discover",
    letter: "D",
    toneClass: "discover",
    description: "Catálogos editoriais e discovery de obras no feed principal.",
  },
  source: {
    label: "source",
    letter: "S",
    toneClass: "source",
    description: "Resolução de fontes e links de download para a camada Discover.",
  },
  manga_source: {
    label: "manga",
    letter: "M",
    toneClass: "manga",
    description: "Fontes de mangá: lista capítulos e fornece páginas para download em CBZ.",
  },
  legacy_search: {
    label: "legacy",
    letter: "L",
    toneClass: "legacy",
    description: "Compatibilidade com busca legada e rotinas de fallback.",
  },
};

const FALLBACK_ROLE_META: AddonRoleMeta = {
  label: "addon",
  letter: "?",
  toneClass: "legacy",
  description: "Tipo de addon desconhecido.",
};

function resolveRoleMeta(role: AddonRole | string): AddonRoleMeta {
  return (ADDON_ROLE_META as Record<string, AddonRoleMeta>)[role] ?? FALLBACK_ROLE_META;
}


function createEmptySetting(): AddonSettingEntry {
  return { key: "", value: "" };
}

function toStartCase(value: string): string {
  return value
    .replace(/[._-]+/g, " ")
    .replace(/\s+/g, " ")
    .trim()
    .replace(/\b\w/g, (char) => char.toUpperCase());
}

function removeExtension(fileName: string): string {
  return fileName.replace(/\.[a-z0-9]+$/i, "");
}

function getAddonName(addon: AddonDescriptor): string {
  const normalizedName = removeExtension(addon.fileName);
  if (normalizedName.trim().length > 0) {
    return toStartCase(normalizedName);
  }

  return toStartCase(addon.id);
}

function getSourceId(addonId: string): string {
  const chunks = addonId.split(/[-_]/).filter(Boolean);
  if (chunks.length === 0) {
    return addonId;
  }

  return chunks[0];
}

function normalizeSettingKey(key: string): string {
  return key.trim().toLowerCase().replace(/-/g, "_");
}

function isEnabledSettingKey(key: string): boolean {
  return normalizeSettingKey(key) === "enabled";
}

function withEnabledSetting(settings: AddonSettingEntry[], enabled: boolean): AddonSettingEntry[] {
  const next = settings.filter((setting) => !isEnabledSettingKey(setting.key));
  next.push({ key: "enabled", value: enabled ? "true" : "false" });
  return next;
}

function AddonKeyIcon() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="32"
      height="32"
      fill="currentColor"
      viewBox="0 0 16 16"
      className="lx-addon-action-icon"
      aria-hidden="true"
      focusable="false"
    >
      <path d="M0 8a4 4 0 0 1 7.465-2H14a.5.5 0 0 1 .354.146l1.5 1.5a.5.5 0 0 1 0 .708l-1.5 1.5a.5.5 0 0 1-.708 0L13 9.207l-.646.647a.5.5 0 0 1-.708 0L11 9.207l-.646.647a.5.5 0 0 1-.708 0L9 9.207l-.646.647A.5.5 0 0 1 8 10h-.535A4 4 0 0 1 0 8m4-3a3 3 0 1 0 2.712 4.285A.5.5 0 0 1 7.163 9h.63l.853-.854a.5.5 0 0 1 .708 0l.646.647.646-.647a.5.5 0 0 1 .708 0l.646.647.646-.647a.5.5 0 0 1 .708 0l.646.647.793-.793-1-1h-6.63a.5.5 0 0 1-.451-.285A3 3 0 0 0 4 5" />
      <path d="M4 8a1 1 0 1 1-2 0 1 1 0 0 1 2 0" />
    </svg>
  );
}

function AddonFileActionIcon() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="32"
      height="32"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      className="lx-addon-action-icon"
      aria-hidden="true"
      focusable="false"
    >
      <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l4 4v12a2 2 0 0 1-2 2z" />
      <polyline points="17 21 17 13 7 13 7 21" />
      <polyline points="7 3 7 8 15 8" />
    </svg>
  );
}

function AddonTrashIcon() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      width="16"
      height="16"
      className="lx-addon-action-icon"
      aria-hidden="true"
      focusable="false"
    >
      <path d="M3 6h18" />
      <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" />
      <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
      <line x1="10" y1="11" x2="10" y2="17" />
      <line x1="14" y1="11" x2="14" y2="17" />
    </svg>
  );
}

export function AddonsView() {
  const queryClient = useQueryClient();
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
  const [togglingAddonId, setTogglingAddonId] = useState<string | null>(null);

  const getErrorMessage = (err: unknown, fallback: string): string => {
    if (err instanceof Error && err.message.trim().length > 0) {
      return err.message;
    }

    if (typeof err === "string" && err.trim().length > 0) {
      return err;
    }

    return fallback;
  };

  const invalidateDiscoverQueries = useCallback(async () => {
    await queryClient.invalidateQueries({ queryKey: ["discover"] });
  }, [queryClient]);

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
      const resolvedSettings = settings.length > 0 ? settings : selectedAddon.settings;
      setSettingsDraft(resolvedSettings.filter((entry) => !isEnabledSettingKey(entry.key)));
    } catch (err) {
      setSettingsError(getErrorMessage(err, "Falha ao carregar configurações do addon"));
      setSettingsDraft(selectedAddon.settings.filter((entry) => !isEnabledSettingKey(entry.key)));
    } finally {
      setSettingsLoading(false);
    }
  }, [selectedAddon]);

  useEffect(() => {
    void loadSelectedAddonSettings();
  }, [loadSelectedAddonSettings]);

  const activeAddonsCount = useMemo(
    () => addons.filter((addon) => addon.enabled).length,
    [addons],
  );

  const selectedAddonEnabled = selectedAddon?.enabled ?? false;

  const selectedAddonRoleMeta = selectedAddon ? resolveRoleMeta(selectedAddon.role) : null;

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
      await invalidateDiscoverQueries();
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
      await invalidateDiscoverQueries();

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
      await invalidateDiscoverQueries();
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
      const payload = withEnabledSetting(settingsDraft, selectedAddon.enabled);
      await updateAddonSettings(selectedAddon.id, payload);
      await loadAddons();
      await invalidateDiscoverQueries();
    } catch (err) {
      setSettingsError(getErrorMessage(err, "Falha ao salvar configurações do addon"));
    } finally {
      setSavingSettings(false);
    }
  };

  const handleToggleAddonEnabled = async (addon: AddonDescriptor) => {
    setTogglingAddonId(addon.id);
    setError(null);

    try {
      const updated = await setAddonEnabled(addon.id, !addon.enabled);
      setAddons((previous) =>
        previous.map((entry) => (entry.id === updated.id ? updated : entry)),
      );
      await invalidateDiscoverQueries();
    } catch (err) {
      setError(getErrorMessage(err, "Falha ao atualizar status do addon"));
    } finally {
      setTogglingAddonId(null);
    }
  };

  return (
    <section className="lx-addons-shell">
      <header className="lx-page-header">
        <div className="lx-page-header-titles">
          <h1 className="lx-page-title">Addons</h1>
          <p className="lx-page-subtitle">{activeAddonsCount} ativos</p>
        </div>

        <div className="lx-page-header-actions">
          <Button
            variant="secondary"
            onClick={() => void handleReloadAddons()}
            disabled={reloading}
          >
            {reloading ? "Recarregando..." : "Recarregar"}
          </Button>
          <Button
            variant="primary"
            onClick={() => void handleInstallAddon()}
            disabled={installing}
          >
            {installing ? "Instalando..." : "Instalar .wasm"}
          </Button>
        </div>
      </header>

      {error && <StateMessage tone="error">{error}</StateMessage>}

      {loading && <StateMessage>Carregando addons...</StateMessage>}

      {!loading && addons.length === 0 && (
        <EmptyState
          title="Nenhum addon instalado"
          description="Use o botão Instalar Addon para selecionar um arquivo .wasm manualmente."
          className="justify-items-start text-left"
          action={
            <Button
              variant="primary"
              onClick={() => void handleInstallAddon()}
              disabled={installing}
            >
              {installing ? "Instalando..." : "Selecionar arquivo .wasm"}
            </Button>
          }
        />
      )}

      {!loading && addons.length > 0 && (
        <div className="lx-addons-layout">
          <div className="lx-addons-grid">
            {addons.map((addon) => {
              const roleMeta = resolveRoleMeta(addon.role);
              const isSelected = selectedAddonId === addon.id;
              const isEnabled = addon.enabled;

              return (
                <button
                  key={addon.id}
                  type="button"
                  className={cn("lx-addon-card", isSelected && "active")}
                  onClick={() => setSelectedAddonId(addon.id)}
                >
                  <div className="lx-addon-head">
                    <div className={cn("lx-addon-icon", roleMeta.toneClass)}>{roleMeta.letter}</div>

                    <div className="lx-addon-head-content">
                      <div className="lx-addon-name">{getAddonName(addon)}</div>
                      <div className="lx-addon-id">{addon.id}</div>
                    </div>

                    <span className={cn("lx-addon-role-pill", roleMeta.toneClass)}>{roleMeta.label}</span>
                  </div>

                  <p className="lx-addon-desc">{roleMeta.description}</p>

                  <div className="lx-addon-stats">
                    <div className="lx-addon-stat">
                      <strong>{addon.settings.length}</strong> chaves
                    </div>

                    {addon.role === "source" && (
                      <div className="lx-addon-stat">
                        source_id: <strong>{getSourceId(addon.id)}</strong>
                      </div>
                    )}

                    {addon.role !== "source" && (
                      <div className="lx-addon-stat" title={addon.fileName}>
                        <strong>{addon.fileName}</strong>
                      </div>
                    )}

                    <div className="lx-addon-status">
                      <span className={cn("lx-addon-status-dot", isEnabled && "active")} />
                      {isEnabled ? "Ativo" : "Desativado"}
                    </div>
                  </div>
                </button>
              );
            })}

            <button
              type="button"
              className="lx-addon-install-card"
              onClick={() => void handleInstallAddon()}
              disabled={installing}
            >
              <span className="lx-addon-install-card-icon" aria-hidden="true">
                +
              </span>
              <strong>{installing ? "Instalando..." : "Instalar novo addon"}</strong>
              <span>Selecione um arquivo .wasm</span>
            </button>
          </div>

          <aside className="lx-addon-detail-panel">
            {!selectedAddon && (
              <EmptyState
                compact
                title="Selecione um addon"
                description="Escolha um card para abrir configurações."
              />
            )}

            {selectedAddon && selectedAddonRoleMeta && (
              <>
                <header className="lx-addon-detail-head">
                  <div className={cn("lx-addon-icon", selectedAddonRoleMeta.toneClass, "large")}>
                    {selectedAddonRoleMeta.letter}
                  </div>

                  <div className="lx-addon-detail-title-wrap">
                    <h2 className="lx-addon-detail-title">{getAddonName(selectedAddon)}</h2>
                    <p className="lx-addon-id">{selectedAddon.id}</p>

                    <div className="lx-addon-detail-pills">
                      <span className={cn("lx-addon-role-pill", selectedAddonRoleMeta.toneClass)}>
                        {selectedAddonRoleMeta.label}
                      </span>
                    </div>
                  </div>
                </header>

                <p className="lx-addon-detail-description">{selectedAddonRoleMeta.description}</p>

                <div className="lx-addon-toggle-row">
                  <div>
                    <p className="lx-addon-toggle-title">Habilitar addon</p>
                    <p className="lx-addon-toggle-subtitle">
                      {selectedAddonEnabled
                        ? "Carregado no runtime local"
                        : "Ignorado no runtime local enquanto desativado"}
                    </p>
                  </div>

                  <button
                    type="button"
                    className={cn("lx-addon-switch", selectedAddonEnabled && "on")}
                    onClick={() => void handleToggleAddonEnabled(selectedAddon)}
                    disabled={togglingAddonId === selectedAddon.id}
                    aria-label={selectedAddonEnabled ? "Desativar addon" : "Ativar addon"}
                    aria-pressed={selectedAddonEnabled}
                    title={selectedAddonEnabled ? "Desativar addon" : "Ativar addon"}
                  >
                    <span className="lx-addon-switch-thumb" />
                  </button>
                </div>

                <h3 className="lx-addon-section-title">Configuração</h3>

                {settingsError && <StateMessage tone="error">{settingsError}</StateMessage>}
                {settingsLoading && <StateMessage>Carregando configurações...</StateMessage>}

                {!settingsLoading && settingsDraft.length === 0 && (
                  <StateMessage>
                    Este addon ainda não possui chaves configuradas. Clique em Nova chave para
                    adicionar a primeira configuração.
                  </StateMessage>
                )}

                {!settingsLoading && settingsDraft.length > 0 && (
                  <div className="lx-addon-config-list">
                    {settingsDraft.map((setting, index) => {
                      const keyInputId = `addon-${selectedAddon.id}-setting-key-${index}`;
                      const valueInputId = `addon-${selectedAddon.id}-setting-value-${index}`;

                      return (
                        <div className="lx-addon-config-row" key={`${selectedAddon.id}-${index}`}>
                          <div className="lx-addon-config-input-group">
                            <label htmlFor={keyInputId} className="lx-addon-config-label">
                              Chave
                            </label>
                            <Input
                              id={keyInputId}
                              value={setting.key}
                              onChange={(event) =>
                                handleSettingChange(index, "key", event.target.value)
                              }
                              placeholder="api_key"
                              className="lx-addon-config-input"
                            />
                          </div>

                          <div className="lx-addon-config-input-group">
                            <label htmlFor={valueInputId} className="lx-addon-config-label">
                              Valor
                            </label>
                            <Input
                              id={valueInputId}
                              value={setting.value}
                              onChange={(event) =>
                                handleSettingChange(index, "value", event.target.value)
                              }
                              placeholder="chave"
                              className="lx-addon-config-input"
                            />
                          </div>

                          <Button
                            variant="secondary"
                            size="sm"
                            className="size-[35px] !rounded-[var(--radius-4)] lx-addon-config-remove-btn"
                            onClick={() =>
                              setSettingsDraft((previous) =>
                                previous.filter((_, settingIndex) => settingIndex !== index),
                              )
                            }
                            aria-label="Remover chave"
                            title="Remover chave"
                          >
                            <AddonTrashIcon />
                          </Button>
                        </div>
                      );
                    })}
                  </div>
                )}

                <div className="lx-addon-detail-actions">
                  <Button
                    variant="secondary"
                    className="lx-addon-action-icon-btn"
                    onClick={() =>
                      setSettingsDraft((previous) => [...previous, createEmptySetting()])
                    }
                    aria-label="Adicionar nova chave"
                    title="Adicionar nova chave"
                  >
                    <AddonKeyIcon />
                  </Button>

                  <Button
                    variant="primary"
                    className="lx-addon-action-icon-btn"
                    onClick={() => void handleSaveSettings()}
                    disabled={savingSettings}
                    aria-label={
                      savingSettings ? "Salvando configurações do addon" : "Salvar configurações do addon"
                    }
                    title={
                      savingSettings ? "Salvando configurações do addon" : "Salvar configurações do addon"
                    }
                  >
                    <AddonFileActionIcon />
                  </Button>

                  <Button
                    variant="danger"
                    className="lx-addon-action-icon-btn"
                    onClick={() => void handleRemoveAddon(selectedAddon)}
                    disabled={removingAddonId === selectedAddon.id}
                    aria-label={
                      removingAddonId === selectedAddon.id
                        ? "Removendo addon"
                        : "Remover addon"
                    }
                    title={
                      removingAddonId === selectedAddon.id
                        ? "Removendo addon"
                        : "Remover addon"
                    }
                  >
                    <AddonTrashIcon />
                  </Button>
                </div>
              </>
            )}
          </aside>
        </div>
      )}
    </section>
  );
}
