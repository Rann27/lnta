import { create } from "zustand";
import type {
  AppInfo,
  GlobalSettings,
  ModelEntry,
  ModelReference,
  ProviderConfigEntry,
  ProviderTestResult,
  RoleConfig,
  RolesConfig,
} from "../../lib/bindings";
import { ipc } from "../../lib/ipc";

export interface SettingsState {
  globalSettings: GlobalSettings | null;
  providers: ProviderConfigEntry[];
  apiKeysStatus: Record<string, boolean>;
  testResults: Record<string, ProviderTestResult | null>;
  isTesting: Record<string, boolean>;
  models: ModelEntry[];
  appInfo: AppInfo | null;
  isLoading: boolean;
  error: string | null;

  // Actions
  loadSettings: () => Promise<void>;
  saveProvider: (entry: ProviderConfigEntry) => Promise<void>;
  deleteProvider: (id: string) => Promise<void>;
  setApiKey: (providerId: string, key: string) => Promise<void>;
  clearApiKey: (providerId: string) => Promise<void>;
  testProvider: (providerId: string) => Promise<ProviderTestResult>;
  registerModel: (entry: ModelEntry) => Promise<void>;
  fetchProviderModels: (providerId: string) => Promise<ModelEntry[]>;
  updateRolePrimary: (roleName: keyof RolesConfig, primary: ModelReference) => Promise<void>;
  addRoleFallback: (roleName: keyof RolesConfig, fallback: ModelReference) => Promise<void>;
  removeRoleFallback: (roleName: keyof RolesConfig, index: number) => Promise<void>;
  reorderRoleFallback: (
    roleName: keyof RolesConfig,
    fromIndex: number,
    toIndex: number
  ) => Promise<void>;
  updateRoleConfig: (
    roleName: keyof RolesConfig,
    config: Partial<RoleConfig>
  ) => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  globalSettings: null,
  providers: [],
  apiKeysStatus: {},
  testResults: {},
  isTesting: {},
  models: [],
  appInfo: null,
  isLoading: false,
  error: null,

  loadSettings: async () => {
    set({ isLoading: true, error: null });
    try {
      const [settings, providerList, modelList, info] = await Promise.all([
        ipc.getGlobalSettings().catch(() => null),
        ipc.listProviders().catch(() => []),
        ipc.listModels().catch(() => []),
        ipc.getAppInfo().catch(() => ({
          version: "0.1.0",
          license: "MIT + Commons Clause",
          repo_url: "https://github.com/Rann27/lnta",
        })),
      ]);

      const keyStatusMap: Record<string, boolean> = {};
      await Promise.all(
        providerList.map(async (p) => {
          try {
            const hasKey = await ipc.hasApiKey(p.id);
            keyStatusMap[p.id] = hasKey;
          } catch {
            keyStatusMap[p.id] = false;
          }
        })
      );

      set({
        globalSettings: settings,
        providers: providerList,
        apiKeysStatus: keyStatusMap,
        models: modelList,
        appInfo: info,
        isLoading: false,
      });
    } catch (err) {
      set({
        isLoading: false,
        error: err instanceof Error ? err.message : "Failed to load settings",
      });
    }
  },

  saveProvider: async (entry: ProviderConfigEntry) => {
    await ipc.saveProvider(entry);
    const updated = await ipc.listProviders();
    const hasKey = await ipc.hasApiKey(entry.id).catch(() => false);
    set((state) => ({
      providers: updated,
      apiKeysStatus: { ...state.apiKeysStatus, [entry.id]: hasKey },
    }));
  },

  deleteProvider: async (id: string) => {
    await ipc.deleteProvider(id);
    const updated = await ipc.listProviders();
    set((state) => {
      const nextKeyStatus = { ...state.apiKeysStatus };
      delete nextKeyStatus[id];
      const nextTestResults = { ...state.testResults };
      delete nextTestResults[id];
      return {
        providers: updated,
        apiKeysStatus: nextKeyStatus,
        testResults: nextTestResults,
      };
    });
  },

  setApiKey: async (providerId: string, key: string) => {
    await ipc.setApiKey(providerId, key);
    set((state) => ({
      apiKeysStatus: { ...state.apiKeysStatus, [providerId]: Boolean(key.trim()) },
    }));
  },

  clearApiKey: async (providerId: string) => {
    await ipc.setApiKey(providerId, "");
    set((state) => ({
      apiKeysStatus: { ...state.apiKeysStatus, [providerId]: false },
      testResults: { ...state.testResults, [providerId]: null },
    }));
  },

  testProvider: async (providerId: string) => {
    set((state) => ({
      isTesting: { ...state.isTesting, [providerId]: true },
    }));
    try {
      const result = await ipc.testProvider(providerId);
      set((state) => ({
        testResults: { ...state.testResults, [providerId]: result },
        isTesting: { ...state.isTesting, [providerId]: false },
      }));
      return result;
    } catch (err) {
      const fallbackResult: ProviderTestResult = {
        success: false,
        latency_ms: 0n,
        error: err instanceof Error ? err.message : "Connection failed",
      };
      set((state) => ({
        testResults: { ...state.testResults, [providerId]: fallbackResult },
        isTesting: { ...state.isTesting, [providerId]: false },
      }));
      return fallbackResult;
    }
  },

  registerModel: async (entry: ModelEntry) => {
    await ipc.registerModel(entry);
    const updatedModels = await ipc.listModels();
    set({ models: updatedModels });
  },

  fetchProviderModels: async (providerId: string) => {
    const fetched = await ipc.fetchProviderModels(providerId);
    const current = await ipc.listModels();
    set({ models: current });
    return fetched;
  },

  updateRolePrimary: async (roleName: keyof RolesConfig, primary: ModelReference) => {
    const { globalSettings } = get();
    if (!globalSettings) return;

    const currentRole = globalSettings.roles[roleName];
    const updatedRole: RoleConfig = {
      ...currentRole,
      primary,
    };
    const nextSettings: GlobalSettings = {
      ...globalSettings,
      roles: {
        ...globalSettings.roles,
        [roleName]: updatedRole,
      },
    };

    await ipc.saveGlobalSettings(nextSettings);
    set({ globalSettings: nextSettings });
  },

  addRoleFallback: async (roleName: keyof RolesConfig, fallback: ModelReference) => {
    const { globalSettings } = get();
    if (!globalSettings) return;

    const currentRole = globalSettings.roles[roleName];
    const exists = currentRole.fallbacks.some(
      (f) => f.provider === fallback.provider && f.model_id === fallback.model_id
    );
    if (exists) return;

    const updatedRole: RoleConfig = {
      ...currentRole,
      fallbacks: [...currentRole.fallbacks, fallback],
    };
    const nextSettings: GlobalSettings = {
      ...globalSettings,
      roles: {
        ...globalSettings.roles,
        [roleName]: updatedRole,
      },
    };

    await ipc.saveGlobalSettings(nextSettings);
    set({ globalSettings: nextSettings });
  },

  removeRoleFallback: async (roleName: keyof RolesConfig, index: number) => {
    const { globalSettings } = get();
    if (!globalSettings) return;

    const currentRole = globalSettings.roles[roleName];
    const updatedFallbacks = currentRole.fallbacks.filter((_, i) => i !== index);
    const updatedRole: RoleConfig = {
      ...currentRole,
      fallbacks: updatedFallbacks,
    };
    const nextSettings: GlobalSettings = {
      ...globalSettings,
      roles: {
        ...globalSettings.roles,
        [roleName]: updatedRole,
      },
    };

    await ipc.saveGlobalSettings(nextSettings);
    set({ globalSettings: nextSettings });
  },

  reorderRoleFallback: async (
    roleName: keyof RolesConfig,
    fromIndex: number,
    toIndex: number
  ) => {
    const { globalSettings } = get();
    if (!globalSettings) return;

    const currentRole = globalSettings.roles[roleName];
    const fallbacks = [...currentRole.fallbacks];
    if (toIndex < 0 || toIndex >= fallbacks.length) return;

    const [moved] = fallbacks.splice(fromIndex, 1);
    fallbacks.splice(toIndex, 0, moved);

    const updatedRole: RoleConfig = {
      ...currentRole,
      fallbacks,
    };
    const nextSettings: GlobalSettings = {
      ...globalSettings,
      roles: {
        ...globalSettings.roles,
        [roleName]: updatedRole,
      },
    };

    await ipc.saveGlobalSettings(nextSettings);
    set({ globalSettings: nextSettings });
  },

  updateRoleConfig: async (
    roleName: keyof RolesConfig,
    config: Partial<RoleConfig>
  ) => {
    const { globalSettings } = get();
    if (!globalSettings) return;

    const currentRole = globalSettings.roles[roleName];
    const updatedRole: RoleConfig = {
      ...currentRole,
      ...config,
    };
    const nextSettings: GlobalSettings = {
      ...globalSettings,
      roles: {
        ...globalSettings.roles,
        [roleName]: updatedRole,
      },
    };

    await ipc.saveGlobalSettings(nextSettings);
    set({ globalSettings: nextSettings });
  },
}));
