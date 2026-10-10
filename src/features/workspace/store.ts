import { create } from "zustand";
import { ipc } from "../../lib/ipc";
import { notifyOS } from "../../lib/notifications";
import i18n from "../../lib/i18n";
import type {
  Chapter,
  EngineEvent,
  PreflightReport,
  ProjectConfig,
  RunStateResponse,
} from "../../lib/bindings";
import type { SegmentItem } from "../../components/ui/SegmentRuler";

export interface LogEntry {
  id: string;
  timestamp: string;
  scope: string;
  level: "info" | "warning" | "error";
  message: string;
}

export type WorkspaceTab =
  | "run"
  | "chapters"
  | "glossary"
  | "qa"
  | "usage"
  | "project";

export interface WorkspaceState {
  projectPath: string;
  projectConfig: ProjectConfig | null;
  chapters: Chapter[];
  selectedChapterId: string | null;
  chapterSegments: Record<string, SegmentItem[]>;
  runState: RunStateResponse;
  preflight: PreflightReport | null;
  inputTokens: number;
  outputTokens: number;
  cachedTokens: number;
  activeTab: WorkspaceTab;
  railWidth: number;
  railCollapsed: boolean;
  logDrawerOpen: boolean;
  logDrawerHeight: number;
  logFilter: "all" | "warn_error" | "selected_chapter";
  logs: LogEntry[];
  docxStatus: { status: string; lastRenderedTime?: string };
  providerStatus: { name: string; model: string; connected: boolean };
  glossaryCount: number;
  qaFindingsCount: number;

  // Actions
  initWorkspace: (path: string) => Promise<void>;
  selectChapter: (id: string) => void;
  setActiveTab: (tab: WorkspaceTab) => void;
  setRailWidth: (width: number) => void;
  toggleRailCollapsed: () => void;
  setLogDrawerOpen: (open: boolean) => void;
  toggleLogDrawer: () => void;
  setLogDrawerHeight: (height: number) => void;
  setLogFilter: (filter: "all" | "warn_error" | "selected_chapter") => void;
  startRun: (chapterId?: string) => Promise<void>;
  pauseRun: () => Promise<void>;
  resumeRun: () => Promise<void>;
  stopRun: () => Promise<void>;
  addLog: (entry: Omit<LogEntry, "id">) => void;
  handleEngineEvent: (event: EngineEvent) => void;
}

function synthesizeSegmentsForChapter(ch: Chapter): SegmentItem[] {
  // If chapter has char_count === 0 and title_src is empty, it's an image-page chapter
  if (!ch.title_src && ch.char_count === 0) {
    return [
      { idx: 1, kind: "image", status: "done", isFixed: true },
      { idx: 2, kind: "image", status: "done", isFixed: true },
    ];
  }

  const segmentCount = Math.max(12, Math.min(80, Math.floor(ch.char_count / 150) || 24));
  const items: SegmentItem[] = [];

  for (let i = 1; i <= segmentCount; i++) {
    // Distribute scene markers or images occasionally
    const isFixed = i === Math.floor(segmentCount / 2);
    let status: SegmentItem["status"] = "pending";

    if (ch.status === "done") {
      status = "done";
    } else if (ch.status === "running") {
      if (i < segmentCount * 0.4) {
        status = "done";
      } else if (i < segmentCount * 0.6) {
        status = "live";
      } else {
        status = "pending";
      }
    } else if (ch.status === "paused") {
      if (i < segmentCount * 0.4) {
        status = "done";
      } else if (i < segmentCount * 0.6) {
        status = "paused";
      } else {
        status = "pending";
      }
    } else if (ch.status === "needs_attention") {
      if (i === Math.floor(segmentCount * 0.3)) {
        status = "flag";
      } else if (i < segmentCount * 0.5) {
        status = "done";
      }
    }

    items.push({
      idx: i,
      kind: isFixed ? "scene" : (i % 3 === 0 ? "dialogue" : "narration"),
      status,
      isFixed,
    });
  }

  return items;
}

export const useWorkspaceStore = create<WorkspaceState>((set, get) => ({
  projectPath: "",
  projectConfig: null,
  chapters: [],
  selectedChapterId: null,
  chapterSegments: {},
  runState: {
    state: "idle",
    active: false,
    current_chapter_id: null,
    latest_run: null,
  },
  preflight: null,
  inputTokens: 12480,
  outputTokens: 9120,
  cachedTokens: 2048,
  activeTab: "run",
  railWidth: 300,
  railCollapsed: false,
  logDrawerOpen: false,
  logDrawerHeight: 132,
  logFilter: "all",
  logs: [
    {
      id: "log_1",
      timestamp: "10:30",
      scope: "run",
      level: "info",
      message: "Proyek dibuka dan diverifikasi.",
    },
    {
      id: "log_2",
      timestamp: "10:31",
      scope: "ch01",
      level: "info",
      message: "Bab 01 berhasil diproses.",
    },
  ],
  docxStatus: { status: "rendered", lastRenderedTime: "10:32" },
  providerStatus: {
    name: "Anthropic",
    model: "claude-3-5-sonnet",
    connected: true,
  },
  glossaryCount: 42,
  qaFindingsCount: 3,

  initWorkspace: async (path: string) => {
    try {
      const openResult = await ipc.openProject(path);
      const runState = await ipc.getRunState(path);

      const segmentsMap: Record<string, SegmentItem[]> = {};
      for (const ch of openResult.chapters) {
        segmentsMap[ch.id] = synthesizeSegmentsForChapter(ch);
      }

      const activeChapId =
        runState.current_chapter_id ||
        (openResult.chapters.length > 0 ? openResult.chapters[0].id : null);

      set({
        projectPath: path,
        projectConfig: openResult.config,
        chapters: openResult.chapters,
        selectedChapterId: activeChapId,
        chapterSegments: segmentsMap,
        runState,
        preflight: openResult.preflight,
      });

      // Try loading detailed segments for the selected chapter if available
      if (activeChapId) {
        try {
          const detail = await ipc.getChapter(path, activeChapId);
          if (detail && detail.segments.length > 0) {
            const mapped: SegmentItem[] = detail.segments.map((s) => ({
              idx: s.idx,
              kind: s.kind,
              status: detail.chapter.status === "done" ? "done" : "pending",
              isFixed: s.kind === "scene" || s.kind === "image" || s.kind === "image_missing",
            }));
            set((state) => ({
              chapterSegments: {
                ...state.chapterSegments,
                [activeChapId]: mapped,
              },
            }));
          }
        } catch {
          // Keep synthesized segments
        }
      }

      // Listen to engine events
      ipc.onEngineEvent((event) => {
        get().handleEngineEvent(event);
      });
    } catch {
      // Error handling
    }
  },

  selectChapter: (id: string) => {
    set({ selectedChapterId: id });
  },

  setActiveTab: (tab: WorkspaceTab) => {
    set({ activeTab: tab });
  },

  setRailWidth: (width: number) => {
    const clamped = Math.max(240, Math.min(420, width));
    set({ railWidth: clamped, railCollapsed: false });
  },

  toggleRailCollapsed: () => {
    set((state) => ({ railCollapsed: !state.railCollapsed }));
  },

  setLogDrawerOpen: (open: boolean) => {
    set({ logDrawerOpen: open });
  },

  toggleLogDrawer: () => {
    set((state) => ({ logDrawerOpen: !state.logDrawerOpen }));
  },

  setLogDrawerHeight: (height: number) => {
    set({ logDrawerHeight: Math.max(90, Math.min(400, height)) });
  },

  setLogFilter: (filter: "all" | "warn_error" | "selected_chapter") => {
    set({ logFilter: filter });
  },

  startRun: async (chapterId?: string) => {
    const { projectPath, chapters } = get();
    if (!projectPath) return;

    set((state) => ({
      runState: {
        ...state.runState,
        state: "running",
        active: true,
        current_chapter_id: chapterId ?? state.selectedChapterId ?? (chapters[0]?.id ?? null),
      },
    }));

    get().addLog({
      timestamp: new Date().toLocaleTimeString("id", { hour: "2-digit", minute: "2-digit" }),
      scope: "run",
      level: "info",
      message: chapterId ? `Run dimulai untuk bab ${chapterId}` : "Run volume dimulai.",
    });

    try {
      await ipc.startRun(projectPath);
    } catch {
      // In mock/preview, keep updated state
    }
  },

  pauseRun: async () => {
    const { projectPath } = get();
    set((state) => ({
      runState: { ...state.runState, state: "pausing" },
    }));
    try {
      await ipc.pauseRun(projectPath);
      set((state) => ({
        runState: { ...state.runState, state: "paused", active: false },
      }));
    } catch {
      set((state) => ({
        runState: { ...state.runState, state: "paused", active: false },
      }));
    }
  },

  resumeRun: async () => {
    const { projectPath } = get();
    set((state) => ({
      runState: { ...state.runState, state: "running", active: true },
    }));
    try {
      await ipc.resumeRun(projectPath);
    } catch {
      // preview
    }
  },

  stopRun: async () => {
    const { projectPath } = get();
    try {
      await ipc.stopRun(projectPath);
    } catch {
      // preview
    }
    set((state) => ({
      runState: { ...state.runState, state: "idle", active: false },
    }));
  },

  addLog: (entry: Omit<LogEntry, "id">) => {
    const newEntry: LogEntry = {
      ...entry,
      id: `log_${Date.now()}_${Math.random().toString(36).substring(2, 6)}`,
    };
    set((state) => ({
      logs: [...state.logs, newEntry],
    }));
  },

  handleEngineEvent: (event: EngineEvent) => {
    if (event.event === "run:state") {
      const priorState = get().runState.state;
      set((state) => ({
        runState: {
          ...state.runState,
          state: event.data.state,
          active: event.data.state === "running" || event.data.state === "qa_running",
        },
      }));

      if (event.data.state === "completed" && priorState !== "completed") {
        void notifyOS({
          title: i18n.t("notifications.run_completed_title"),
          body: i18n.t("notifications.run_completed_body"),
        });
      }
    } else if (event.event === "chapter:status") {
      set((state) => ({
        chapters: state.chapters.map((ch) =>
          ch.id === event.data.chapter_id ? { ...ch, status: event.data.status } : ch
        ),
      }));

      if (event.data.status === "blocked" || event.data.status === "needs_attention") {
        void notifyOS({
          title: i18n.t("notifications.blocker_title"),
          body: i18n.t("notifications.blocker_body", { chapterId: event.data.chapter_id }),
        });
      }
    } else if (event.event === "log:line") {
      const level =
        event.data.level === "error"
          ? "error"
          : event.data.level === "warn" || event.data.level === "warning"
          ? "warning"
          : "info";
      get().addLog({
        timestamp: event.data.timestamp.substring(11, 16) || "12:00",
        scope: "run",
        level,
        message: event.data.message,
      });
    } else if (event.event === "docx:render") {
      set({
        docxStatus: {
          status: event.data.status,
          lastRenderedTime: new Date().toLocaleTimeString("id", {
            hour: "2-digit",
            minute: "2-digit",
          }),
        },
      });
    } else if (event.event === "qa:progress") {
      set({
        qaFindingsCount: event.data.findings_count,
      });

      if (event.data.phase === "done" || event.data.phase === "completed") {
        void notifyOS({
          title: i18n.t("notifications.qa_done_title"),
          body: i18n.t("notifications.qa_done_body", { count: event.data.findings_count }),
        });
      }
    }
  },
}));
