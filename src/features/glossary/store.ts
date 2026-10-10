import { create } from "zustand";
import type { GlossaryEntry, GlossaryEntryInput } from "../../lib/bindings";
import { ipc } from "../../lib/ipc";

export type GlossarySortColumn =
  | "source"
  | "translation"
  | "note"
  | "tier"
  | "first_chapter_id"
  | "occurrences";

export interface GlossaryState {
  entries: GlossaryEntry[];
  isLoading: boolean;
  error: string | null;
  searchQuery: string;
  sortColumn: GlossarySortColumn;
  sortDirection: "asc" | "desc";
  editingId: string | null;

  // Actions
  loadGlossary: (projectPath: string) => Promise<void>;
  setSearchQuery: (query: string) => void;
  setSorting: (column: GlossarySortColumn) => void;
  setEditingId: (id: string | null) => void;
  addEntry: (projectPath: string, input: GlossaryEntryInput) => Promise<void>;
  updateEntry: (projectPath: string, input: GlossaryEntryInput) => Promise<void>;
  deleteEntry: (projectPath: string, source: string) => Promise<void>;
  acceptProposal: (
    projectPath: string,
    entry: GlossaryEntry,
    proposalText: string
  ) => Promise<void>;
  ignoreProposal: (projectPath: string, entry: GlossaryEntry) => Promise<void>;
}

export const useGlossaryStore = create<GlossaryState>((set, get) => ({
  entries: [],
  isLoading: false,
  error: null,
  searchQuery: "",
  sortColumn: "occurrences",
  sortDirection: "desc",
  editingId: null,

  loadGlossary: async (projectPath: string) => {
    if (!projectPath) return;
    set({ isLoading: true, error: null });
    try {
      const list = await ipc.listGlossary(projectPath);
      set({ entries: list, isLoading: false });
    } catch (err) {
      set({
        isLoading: false,
        error: err instanceof Error ? err.message : "Failed to load glossary",
      });
    }
  },

  setSearchQuery: (query: string) => {
    set({ searchQuery: query });
  },

  setSorting: (column: GlossarySortColumn) => {
    const { sortColumn, sortDirection } = get();
    if (sortColumn === column) {
      set({ sortDirection: sortDirection === "asc" ? "desc" : "asc" });
    } else {
      set({ sortColumn: column, sortDirection: "asc" });
    }
  },

  setEditingId: (id: string | null) => {
    set({ editingId: id });
  },

  addEntry: async (projectPath: string, input: GlossaryEntryInput) => {
    const added = await ipc.addGlossaryEntry(projectPath, input);
    set((state) => ({
      entries: [...state.entries.filter((e) => e.source !== added.source), added],
      editingId: null,
    }));
  },

  updateEntry: async (projectPath: string, input: GlossaryEntryInput) => {
    await ipc.updateGlossaryEntry(projectPath, input);
    set((state) => ({
      entries: state.entries.map((e) =>
        e.source === input.source
          ? {
              ...e,
              translation: input.translation,
              note: input.note,
              status: e.status === "conflict" ? "active" : e.status,
            }
          : e
      ),
      editingId: null,
    }));
  },

  deleteEntry: async (projectPath: string, source: string) => {
    await ipc.deleteGlossaryEntry(projectPath, source);
    set((state) => ({
      entries: state.entries.filter((e) => e.source !== source),
    }));
  },

  acceptProposal: async (
    projectPath: string,
    entry: GlossaryEntry,
    proposalText: string
  ) => {
    // Clean out proposal mention from note if present
    const cleanedNote = (entry.note ?? "")
      .replace(/\(usulan(?:\s+model)?:\s*[^)]+\)/gi, "")
      .trim();

    const input: GlossaryEntryInput = {
      source: entry.source,
      translation: proposalText,
      note: cleanedNote || null,
    };

    await ipc.updateGlossaryEntry(projectPath, input);
    set((state) => ({
      entries: state.entries.map((e) =>
        e.id === entry.id
          ? {
              ...e,
              translation: proposalText,
              note: cleanedNote || null,
              status: "active",
            }
          : e
      ),
    }));
  },

  ignoreProposal: async (projectPath: string, entry: GlossaryEntry) => {
    // Clean out proposal mention from note if present, keep current translation
    const cleanedNote = (entry.note ?? "")
      .replace(/\(usulan(?:\s+model)?:\s*[^)]+\)/gi, "")
      .trim();

    const input: GlossaryEntryInput = {
      source: entry.source,
      translation: entry.translation,
      note: cleanedNote || null,
    };

    await ipc.updateGlossaryEntry(projectPath, input);
    set((state) => ({
      entries: state.entries.map((e) =>
        e.id === entry.id
          ? {
              ...e,
              note: cleanedNote || null,
              status: "active",
            }
          : e
      ),
    }));
  },
}));
