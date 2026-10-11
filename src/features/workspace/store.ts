import { create } from "zustand";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { ipc } from "../../lib/ipc";
import { notifyOS } from "../../lib/notifications";
import { errorMessage } from "../../lib/errors";
import i18n from "../../lib/i18n";
import { formatDateTime } from "../../lib/format";
import type {
  Chapter,
  ChapterDetail,
  ChunkPlan,
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

/** The part the engine is working on, as reported by `chapter:status` events. */
export interface ActivePart {
  chapterId: string;
  current: number;
  total: number;
}

export interface WorkspaceState {
  projectPath: string;
  projectConfig: ProjectConfig | null;
  chapters: Chapter[];
  /** Raw engine status per chapter (`translating(part 1/2)`, `validating`, ...). */
  chapterPhase: Record<string, string>;
  selectedChapterId: string | null;
  chapterSegments: Record<string, SegmentItem[]>;
  chapterParts: Record<string, ChunkPlan[]>;
  imagePageIds: Record<string, boolean>;
  runState: RunStateResponse;
  activePart: ActivePart | null;
  /** Characters streamed by the current model call; reset when a new part starts. */
  streamChars: number;
  callStartedAt: number | null;
  runError: string | null;
  preflight: PreflightReport | null;
  inputTokens: number | null;
  outputTokens: number | null;
  cachedTokens: number | null;
  activeTab: WorkspaceTab;
  railWidth: number;
  railCollapsed: boolean;
  logDrawerOpen: boolean;
  logDrawerHeight: number;
  logFilter: "all" | "warn_error" | "selected_chapter";
  logs: LogEntry[];
  docxStatus: { status: string; lastRenderedTime?: string };
  providerStatus: { name: string; model: string; connected: boolean };
  glossaryCount: number | null;
  qaFindingsCount: number | null;
  qaPhase: string | null;

  // Actions
  initWorkspace: (path: string) => Promise<void>;
  refreshChapter: (chapterId: string) => Promise<void>;
  refreshUsage: () => Promise<void>;
  refreshCounts: () => Promise<void>;
  selectChapter: (id: string) => void;
  setActiveTab: (tab: WorkspaceTab) => void;
  setRailWidth: (width: number) => void;
  toggleRailCollapsed: () => void;
  setLogDrawerOpen: (open: boolean) => void;
  toggleLogDrawer: () => void;
  setLogDrawerHeight: (height: number) => void;
  setLogFilter: (filter: "all" | "warn_error" | "selected_chapter") => void;
  startRun: () => Promise<void>;
  translateChapterOnly: (chapterId: string) => Promise<void>;
  startQa: () => Promise<void>;
  retryDocxRender: () => Promise<void>;
  openDocx: () => Promise<void>;
  pauseRun: () => Promise<void>;
  resumeRun: () => Promise<void>;
  stopRun: () => Promise<void>;
  addLog: (entry: Omit<LogEntry, "id">) => void;
  handleEngineEvent: (event: EngineEvent) => void;
}

const LIVE_PHASES = ["planned", "translating", "validating", "repairing", "running"];

/** Maps engine chapter statuses onto the UI vocabulary used by tags and rulers. */
export function normalizeChapterStatus(raw: string, runActive: boolean): string {
  if (raw === "done" || raw === "needs_attention" || raw === "blocked" || raw === "frozen") {
    return raw;
  }
  if (LIVE_PHASES.some((p) => raw.startsWith(p))) {
    return runActive ? "running" : "paused";
  }
  return "pending";
}

/** Render time for the status bar: the clock time today, the date and time before that. */
function renderedLabel(iso: string): string {
  const when = new Date(iso);
  if (Number.isNaN(when.getTime())) return iso;
  const sameDay = when.toDateString() === new Date().toDateString();
  if (!sameDay) return formatDateTime(when);
  return when.toLocaleTimeString(i18n.language || "id", {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
}

function nowHHMM(): string {
  return new Date().toLocaleTimeString(i18n.language || "id", {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
}

/** Builds real ruler cells from persisted segments and current translations. */
function buildSegmentItems(
  detail: ChapterDetail,
  activePart: ActivePart | null,
  runActive: boolean
): SegmentItem[] {
  const covered = new Set<number>();
  for (const t of detail.translations) {
    for (let i = t.seg_from; i <= t.seg_to; i++) covered.add(i);
  }
  const liveRange =
    activePart && activePart.chapterId === detail.chapter.id
      ? detail.parts.find((p) => p.part_index === activePart.current)
      : undefined;
  const failed = detail.chapter.status === "needs_attention";

  return detail.segments
    .filter((s) => s.kind !== "title")
    .map((s) => {
      const isFixed = s.kind === "scene" || s.kind === "image" || s.kind === "image_missing";
      let status: SegmentItem["status"] = "pending";
      if (isFixed || covered.has(s.idx) || detail.chapter.status === "done") {
        status = "done";
      } else if (liveRange && s.idx >= liveRange.seg_from && s.idx <= liveRange.seg_to) {
        status = runActive ? "live" : "paused";
      } else if (failed) {
        status = "fail";
      }
      return { idx: s.idx, kind: s.kind, status, isFixed };
    });
}

// Each initWorkspace call gets a generation number. Listeners from an older generation
// ignore events, because unlisten resolves asynchronously and React StrictMode (and
// reopening a project) can start a second init before the first listener is registered.
let engineListener: Promise<UnlistenFn> | null = null;
let initGeneration = 0;

export const useWorkspaceStore = create<WorkspaceState>((set, get) => ({
  projectPath: "",
  projectConfig: null,
  chapters: [],
  chapterPhase: {},
  selectedChapterId: null,
  chapterSegments: {},
  chapterParts: {},
  imagePageIds: {},
  runState: {
    state: "idle",
    active: false,
    current_chapter_id: null,
    latest_run: null,
  },
  activePart: null,
  streamChars: 0,
  callStartedAt: null,
  runError: null,
  preflight: null,
  inputTokens: null,
  outputTokens: null,
  cachedTokens: null,
  activeTab: "run",
  railWidth: 300,
  railCollapsed: false,
  logDrawerOpen: false,
  logDrawerHeight: 132,
  logFilter: "all",
  logs: [],
  docxStatus: { status: "unknown" },
  providerStatus: { name: "", model: "", connected: false },
  glossaryCount: null,
  qaFindingsCount: null,
  qaPhase: null,

  initWorkspace: async (path: string) => {
    const generation = ++initGeneration;
    if (engineListener) {
      const previous = engineListener;
      engineListener = null;
      void previous.then((unlisten) => unlisten()).catch(() => undefined);
    }
    engineListener = ipc.onEngineEvent((event) => {
      if (generation === initGeneration) get().handleEngineEvent(event);
    });
    set({
      projectPath: path,
      logs: [],
      runError: null,
      activePart: null,
      streamChars: 0,
      callStartedAt: null,
      chapterSegments: {},
      chapterParts: {},
      imagePageIds: {},
      docxStatus: { status: "unknown" },
    });

    try {
      const openResult = await ipc.openProject(path);
      const runState = await ipc.getRunState(path);
      const runActive = runState.active;
      const phases: Record<string, string> = {};
      for (const ch of openResult.chapters) phases[ch.id] = ch.status;

      set({
        projectConfig: openResult.config,
        chapters: openResult.chapters.map((ch) => ({
          ...ch,
          status: normalizeChapterStatus(ch.status, runActive),
        })),
        chapterPhase: phases,
        selectedChapterId:
          runState.current_chapter_id ?? openResult.chapters[0]?.id ?? null,
        runState,
        preflight: openResult.preflight,
      });

      await Promise.all(openResult.chapters.map((ch) => get().refreshChapter(ch.id)));
      await Promise.all([get().refreshUsage(), get().refreshCounts()]);

      // Translations.docx is rendered after each chapter, so with finished chapters its
      // write time is the last render. Before that it is still the user's template.
      if (openResult.chapters.some((ch) => ch.status === "done")) {
        const modified = await ipc.docxModifiedAt(path).catch(() => null);
        if (modified && get().docxStatus.status === "unknown") {
          set({ docxStatus: { status: "rendered", lastRenderedTime: renderedLabel(modified) } });
        }
      }

      const settings = await ipc.getGlobalSettings();
      const primary = settings.roles.translator.primary;
      const provider = settings.providers.find((p) => p.id === primary.provider);
      const modelItem = openResult.preflight.items.find(
        (i) => i.id === "translator_model_configured"
      );
      set({
        providerStatus: {
          name: provider?.name ?? primary.provider,
          model: primary.model_id,
          connected: modelItem?.status === "pass",
        },
      });
    } catch (err) {
      set({ runError: errorMessage(err) });
    }
  },

  refreshChapter: async (chapterId: string) => {
    const { projectPath, runState, activePart } = get();
    if (!projectPath) return;
    try {
      const detail = await ipc.getChapter(projectPath, chapterId);
      const textSegments = detail.segments.filter(
        (s) => s.kind === "narration" || s.kind === "dialogue"
      );
      set((state) => ({
        chapterSegments: {
          ...state.chapterSegments,
          [chapterId]: buildSegmentItems(detail, activePart, runState.active),
        },
        chapterParts: { ...state.chapterParts, [chapterId]: detail.parts },
        imagePageIds: {
          ...state.imagePageIds,
          [chapterId]: detail.segments.length > 0 && textSegments.length === 0,
        },
      }));
    } catch {
      // A chapter that cannot be read keeps its previous ruler; the error surfaces in the Bab tab.
    }
  },

  refreshUsage: async () => {
    const { projectPath, runState } = get();
    if (!projectPath) return;
    try {
      const usage = await ipc.getProjectUsage(projectPath, runState.latest_run?.id);
      set({
        inputTokens: usage.summary.total_input_tokens,
        outputTokens: usage.summary.total_output_tokens,
        cachedTokens: usage.summary.total_cached_tokens,
      });
    } catch {
      set({ inputTokens: null, outputTokens: null, cachedTokens: null });
    }
  },

  refreshCounts: async () => {
    const { projectPath } = get();
    if (!projectPath) return;
    const [glossary, qa] = await Promise.allSettled([
      ipc.listGlossary(projectPath),
      ipc.getQaSummary(projectPath),
    ]);
    set({
      glossaryCount: glossary.status === "fulfilled" ? glossary.value.length : null,
      qaFindingsCount: qa.status === "fulfilled" ? qa.value.remaining_findings : null,
    });
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

  startRun: async () => {
    const { projectPath } = get();
    if (!projectPath) return;
    set({ runError: null });
    try {
      await ipc.startRun(projectPath);
    } catch (err) {
      const message = errorMessage(err);
      set({ runError: message });
      get().addLog({ timestamp: nowHHMM(), scope: "run", level: "error", message });
    }
  },

  translateChapterOnly: async (chapterId: string) => {
    const { projectPath } = get();
    if (!projectPath) return;
    set({ runError: null, selectedChapterId: chapterId });
    try {
      await ipc.retranslateChapter(projectPath, chapterId);
    } catch (err) {
      const message = errorMessage(err);
      set({ runError: message });
      get().addLog({ timestamp: nowHHMM(), scope: "run", level: "error", message });
    }
  },

  startQa: async () => {
    const { projectPath } = get();
    if (!projectPath) return;
    set({ runError: null });
    try {
      await ipc.startQa(projectPath);
    } catch (err) {
      const message = errorMessage(err);
      set({ runError: message });
      get().addLog({ timestamp: nowHHMM(), scope: "QA", level: "error", message });
    }
  },

  retryDocxRender: async () => {
    const { projectPath } = get();
    if (!projectPath) return;
    try {
      await ipc.renderDocx(projectPath);
    } catch (err) {
      const message = errorMessage(err);
      get().addLog({ timestamp: nowHHMM(), scope: "docx", level: "error", message });
    }
  },

  openDocx: async () => {
    const { projectPath } = get();
    if (!projectPath) return;
    try {
      await ipc.openPath(`${projectPath.replace(/[\\/]+$/, "")}/Translations.docx`);
    } catch (err) {
      const message = errorMessage(err);
      get().addLog({ timestamp: nowHHMM(), scope: "docx", level: "error", message });
    }
  },

  pauseRun: async () => {
    const { projectPath } = get();
    set((state) => ({ runState: { ...state.runState, state: "pausing" } }));
    try {
      await ipc.pauseRun(projectPath);
    } catch (err) {
      const message = errorMessage(err);
      get().addLog({ timestamp: nowHHMM(), scope: "run", level: "error", message });
    }
  },

  resumeRun: async () => {
    const { projectPath } = get();
    try {
      await ipc.resumeRun(projectPath);
    } catch (err) {
      const message = errorMessage(err);
      set({ runError: message });
      get().addLog({ timestamp: nowHHMM(), scope: "run", level: "error", message });
    }
  },

  stopRun: async () => {
    const { projectPath } = get();
    try {
      await ipc.stopRun(projectPath);
    } catch (err) {
      const message = errorMessage(err);
      get().addLog({ timestamp: nowHHMM(), scope: "run", level: "error", message });
    }
  },

  addLog: (entry: Omit<LogEntry, "id">) => {
    const newEntry: LogEntry = {
      ...entry,
      id: `log_${Date.now()}_${get().logs.length}`,
    };
    set((state) => ({ logs: [...state.logs, newEntry] }));
  },

  handleEngineEvent: (event: EngineEvent) => {
    if (event.event === "run:state") {
      const state = event.data.state;
      const active = state === "running" || state === "qa_running";
      const priorState = get().runState.state;
      set((s) => ({
        runState: {
          ...s.runState,
          state,
          active,
          latest_run: event.data.run_id
            ? {
                id: event.data.run_id,
                kind: state === "qa_running" ? "qa" : "translation",
                status: state,
                started_at: new Date().toISOString(),
                ended_at: null,
                settings_snapshot: "",
              }
            : s.runState.latest_run,
        },
        chapters: s.chapters.map((ch) => ({
          ...ch,
          status: normalizeChapterStatus(s.chapterPhase[ch.id] ?? ch.status, active),
        })),
        activePart: active ? s.activePart : null,
        callStartedAt: active ? s.callStartedAt : null,
        qaPhase: state === "qa_running" ? s.qaPhase : null,
      }));
      if (!active) {
        void get().refreshUsage();
        void get().refreshCounts();
      }
      if (state === "completed" && priorState !== "completed") {
        void notifyOS({
          title: i18n.t("notifications.run_completed_title"),
          body: i18n.t("notifications.run_completed_body"),
        });
      }
    } else if (event.event === "chapter:status") {
      const { chapter_id: chapterId, status, part_current, part_total } = event.data;
      const runActive = get().runState.active || status !== "done";
      const isLive = LIVE_PHASES.some((p) => status.startsWith(p));
      set((s) => ({
        chapterPhase: { ...s.chapterPhase, [chapterId]: status },
        chapters: s.chapters.map((ch) =>
          ch.id === chapterId
            ? { ...ch, status: normalizeChapterStatus(status, runActive) }
            : ch
        ),
        runState: isLive ? { ...s.runState, current_chapter_id: chapterId } : s.runState,
        activePart:
          isLive && part_current && part_total
            ? { chapterId, current: part_current, total: part_total }
            : s.activePart,
        streamChars:
          status.startsWith("translating") &&
          (s.activePart?.chapterId !== chapterId || s.activePart?.current !== part_current)
            ? 0
            : s.streamChars,
        callStartedAt: status.startsWith("translating") ? Date.now() : s.callStartedAt,
      }));
      void get().refreshChapter(chapterId);
      if (status === "done") void get().refreshUsage();
      if (status === "needs_attention" || status === "blocked") {
        void notifyOS({
          title: i18n.t("notifications.blocker_title"),
          body: i18n.t("notifications.blocker_body", { chapterId }),
        });
      }
    } else if (event.event === "call:progress") {
      set((s) => ({
        streamChars: s.streamChars + event.data.delta.length,
        callStartedAt: s.callStartedAt ?? Date.now(),
      }));
    } else if (event.event === "log:line") {
      const level =
        event.data.level === "error"
          ? "error"
          : event.data.level === "warn" || event.data.level === "warning"
            ? "warning"
            : "info";
      const ts = new Date(event.data.timestamp);
      get().addLog({
        timestamp: Number.isNaN(ts.getTime())
          ? nowHHMM()
          : ts.toLocaleTimeString(i18n.language || "id", {
              hour: "2-digit",
              minute: "2-digit",
              hour12: false,
            }),
        scope: "run",
        level,
        message: event.data.message,
      });
    } else if (event.event === "docx:render") {
      set({
        docxStatus: {
          status: event.data.status,
          lastRenderedTime: event.data.status === "rendered" ? nowHHMM() : undefined,
        },
      });
    } else if (event.event === "glossary:changed") {
      void get().refreshCounts();
    } else if (event.event === "qa:progress") {
      set({ qaFindingsCount: event.data.findings_count, qaPhase: event.data.phase });
      if (event.data.phase === "done") {
        void get().refreshCounts();
        void notifyOS({
          title: i18n.t("notifications.qa_done_title"),
          body: i18n.t("notifications.qa_done_body", { count: event.data.findings_count }),
        });
      }
    }
  },
}));
