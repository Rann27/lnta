import { create } from "zustand";
import { ipc } from "../../lib/ipc";
import type { QaFindingsSummary, QaFixLogItem } from "../../lib/bindings";

export interface QaState {
  findingsSummary: QaFindingsSummary | null;
  fixLog: QaFixLogItem[];
  selectedGroupCode: string | null;
  activeSubTab: "findings" | "fix_log";
  selectedFixIds: string[];
  loading: boolean;
  error: string | null;

  // Actions
  loadQaData: (path: string) => Promise<void>;
  setSelectedGroupCode: (code: string | null) => void;
  setActiveSubTab: (tab: "findings" | "fix_log") => void;
  toggleFixSelection: (fixId: string) => void;
  selectAllFixes: () => void;
  clearFixSelection: () => void;
  undoFix: (path: string, fixId: string) => Promise<void>;
  undoBatch: (path: string, fixIds: string[]) => Promise<void>;
  undoRun: (path: string, runId: string) => Promise<void>;
}

export const useQaStore = create<QaState>((set, get) => ({
  findingsSummary: null,
  fixLog: [],
  selectedGroupCode: null,
  activeSubTab: "findings",
  selectedFixIds: [],
  loading: false,
  error: null,

  loadQaData: async (path: string) => {
    if (!path) return;
    set({ loading: true, error: null });
    try {
      const [summary, log] = await Promise.all([
        ipc.getQaSummary(path),
        ipc.getQaFixLog(path),
      ]);
      set({
        findingsSummary: summary,
        fixLog: log,
        loading: false,
      });
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
        loading: false,
      });
    }
  },

  setSelectedGroupCode: (code) => set({ selectedGroupCode: code }),

  setActiveSubTab: (tab) => set({ activeSubTab: tab }),

  toggleFixSelection: (fixId) => {
    const current = get().selectedFixIds;
    if (current.includes(fixId)) {
      set({ selectedFixIds: current.filter((id) => id !== fixId) });
    } else {
      set({ selectedFixIds: [...current, fixId] });
    }
  },

  selectAllFixes: () => {
    const activeFixes = get().fixLog.filter((f) => !f.reverted_at).map((f) => f.id);
    set({ selectedFixIds: activeFixes });
  },

  clearFixSelection: () => set({ selectedFixIds: [] }),

  undoFix: async (path, fixId) => {
    try {
      await ipc.undoQaFix(path, fixId);
      await get().loadQaData(path);
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  undoBatch: async (path, fixIds) => {
    try {
      await ipc.undoQaBatch(path, fixIds);
      set({ selectedFixIds: [] });
      await get().loadQaData(path);
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  undoRun: async (path, runId) => {
    try {
      await ipc.undoQaRun(path, runId);
      await get().loadQaData(path);
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },
}));
