import { invoke } from "@tauri-apps/api/core";

export type AddonSettingEntry = {
  key: string;
  value: string;
};

export type AddonRole = "discover" | "source" | "manga_source" | "legacy_search";

export type AddonDescriptor = {
  id: string;
  fileName: string;
  filePath: string;
  role: AddonRole;
  enabled: boolean;
  settings: AddonSettingEntry[];
};

export async function listAddons() {
  return invoke<AddonDescriptor[]>("list_addons");
}

export async function reloadAddons() {
  return invoke<AddonDescriptor[]>("reload_addons");
}

export async function installAddon(filePath: string) {
  return invoke<AddonDescriptor>("install_addon", { filePath });
}

export async function removeAddon(addonId: string) {
  return invoke<void>("remove_addon", { addonId });
}

export async function getAddonSettings(addonId: string) {
  return invoke<AddonSettingEntry[]>("get_addon_settings", { addonId });
}

export async function updateAddonSettings(addonId: string, settings: AddonSettingEntry[]) {
  return invoke<void>("update_addon_settings", { addonId, settings });
}

export async function setAddonEnabled(addonId: string, enabled: boolean) {
  return invoke<AddonDescriptor>("set_addon_enabled", { addonId, enabled });
}
