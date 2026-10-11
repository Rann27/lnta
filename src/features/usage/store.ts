import { create } from "zustand";
import { ipc } from "../../lib/ipc";
import type { ProjectUsageResponse } from "../../lib/bindings";
import type { UsageGrouping, UsageScope } from "./types";

interface UsageState {
  grouping: UsageGrouping;
  scope: UsageScope;
  usageData: ProjectUsageResponse | null;
  loading: boolean;
  error: string | null;

  setGrouping: (grouping: UsageGrouping) => void;
  setScope: (scope: UsageScope) => void;
  fetchUsage: (projectPath: string, runId?: string) => Promise<void>;
  reset: () => void;
}

export const useUsageStore = create<UsageState>((set) => ({
  grouping: "chapter",
  scope: "project",
  usageData: null,
  loading: false,
  error: null,

  setGrouping: (grouping) => set({ grouping }),
  setScope: (scope) => set({ scope }),

  fetchUsage: async (projectPath: string, runId?: string) => {
    if (!projectPath) return;
    set({ loading: true, error: null });
    try {
      const data = await ipc.getProjectUsage(projectPath, runId);
      set({ usageData: data, loading: false });
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
        loading: false,
      });
    }
  },

  reset: () => set({ usageData: null, loading: false, error: null }),
}));
