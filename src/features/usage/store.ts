import { create } from "zustand";
import { ipc } from "../../lib/ipc";
import type { ProjectUsageResponse } from "../../lib/bindings";
import type { UsageGrouping } from "./types";

interface UsageState {
  grouping: UsageGrouping;
  usageData: ProjectUsageResponse | null;
  loading: boolean;
  error: string | null;

  setGrouping: (grouping: UsageGrouping) => void;
  fetchUsage: (projectPath: string, runId?: string) => Promise<void>;
  reset: () => void;
}

export const useUsageStore = create<UsageState>((set) => ({
  grouping: "chapter",
  usageData: null,
  loading: false,
  error: null,

  setGrouping: (grouping) => set({ grouping }),

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
