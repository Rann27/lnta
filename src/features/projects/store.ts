import { create } from "zustand";
import { ipc } from "../../lib/ipc";
import type { ProjectOpenResult, RecentProjectEntry } from "../../lib/bindings";

export interface ProjectsState {
  recentProjects: RecentProjectEntry[];
  isLoading: boolean;
  error: string | null;
  activeProject: ProjectOpenResult | null;
  loadRecentProjects: () => Promise<void>;
  openProject: (path: string) => Promise<ProjectOpenResult>;
  resetActiveProject: () => void;
}

export const useProjectsStore = create<ProjectsState>((set) => ({
  recentProjects: [],
  isLoading: false,
  error: null,
  activeProject: null,

  loadRecentProjects: async () => {
    set({ isLoading: true, error: null });
    try {
      const list = await ipc.listRecentProjects();
      set({ recentProjects: list, isLoading: false });
    } catch (err: unknown) {
      set({ error: String(err), isLoading: false });
    }
  },

  openProject: async (path: string) => {
    set({ isLoading: true, error: null });
    try {
      const res = await ipc.openProject(path);
      set({ activeProject: res, isLoading: false });
      return res;
    } catch (err: unknown) {
      set({ error: String(err), isLoading: false });
      throw err;
    }
  },

  resetActiveProject: () => set({ activeProject: null }),
}));
