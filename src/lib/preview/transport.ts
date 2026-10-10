import type { UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppInfo,
  Chapter,
  ChapterDetail,
  EngineEvent,
  GlobalSettings,
  GlossaryEntry,
  GlossaryEntryInput,
  ModelEntry,
  PreflightReport,
  ProjectConfig,
  ProjectOpenResult,
  ProviderConfigEntry,
  ProviderTestResult,
  RecentProjectEntry,
  RunStateResponse,
  Segment,
  Translation,
} from "../bindings";
import type { IpcTransport } from "../ipc";
import {
  createPreviewScenarios,
  resolveScenarioName,
  type PreviewScenarioData,
  PREVIEW_MODELS,
  PREVIEW_PROVIDERS,
} from "./scenarios";

export class PreviewTransport implements IpcTransport {
  private scenarios: Record<string, PreviewScenarioData>;
  private activeScenario: PreviewScenarioData;
  private listeners: Set<(event: EngineEvent) => void> = new Set();
  private streamTimer: ReturnType<typeof setInterval> | null = null;
  private tokenIndex = 0;
  private syntheticTokens = [
    "Kenta ",
    "melangkah ",
    "perlahan ",
    "melewati ",
    "gerbang ",
    "Akademi ",
    "Sihir. ",
    "Cahaya ",
    "matahari ",
    "pagi ",
    "menyinari ",
    "langkahnya. ",
  ];

  constructor(initialScenarioName?: string) {
    this.scenarios = createPreviewScenarios();
    const scenarioName =
      initialScenarioName ??
      (typeof window !== "undefined"
        ? new URLSearchParams(window.location.search).get("scenario")
        : null);
    const resolved = resolveScenarioName(scenarioName);
    this.activeScenario = this.scenarios[resolved] ?? this.scenarios["run-in-progress"];
    if (this.activeScenario.simulateStreaming) {
      this.startTokenSimulation();
    }
  }

  public getScenarioName(): string {
    return this.activeScenario.name;
  }

  public setScenario(name: string) {
    const resolved = resolveScenarioName(name);
    const next = this.scenarios[resolved];
    if (next) {
      this.activeScenario = next;
      this.stopTokenSimulation();
      if (this.activeScenario.simulateStreaming) {
        this.startTokenSimulation();
      }
      this.emitEvent({
        event: "run:state",
        data: {
          state: this.activeScenario.runState.state,
          run_id: this.activeScenario.runState.latest_run?.id ?? null,
        },
      });
    }
  }

  private emitEvent(event: EngineEvent) {
    for (const listener of this.listeners) {
      try {
        listener(event);
      } catch {
        // ignore listener errors in mock
      }
    }
  }

  private startTokenSimulation() {
    this.stopTokenSimulation();
    this.streamTimer = setInterval(() => {
      if (this.activeScenario.runState.state !== "running") {
        return;
      }
      const token = this.syntheticTokens[this.tokenIndex % this.syntheticTokens.length];
      this.tokenIndex++;
      this.emitEvent({
        event: "call:progress",
        data: {
          call_id: "call_preview_stream",
          delta: token,
          chapter_id: this.activeScenario.runState.current_chapter_id,
          part_index: 0,
        },
      });
    }, 400);
  }

  private stopTokenSimulation() {
    if (this.streamTimer) {
      clearInterval(this.streamTimer);
      this.streamTimer = null;
    }
  }

  // App
  async getAppInfo(): Promise<AppInfo> {
    return {
      version: "0.1.0",
      license: "MIT",
      repo_url: "https://github.com/Rann27/lnta",
    };
  }

  // Project
  async openProject(path: string): Promise<ProjectOpenResult> {
    return {
      path,
      config: this.activeScenario.projectConfig,
      chapters: this.activeScenario.chapters,
      preflight: this.activeScenario.preflight,
      latest_run: this.activeScenario.runState.latest_run,
    };
  }

  async listRecentProjects(): Promise<RecentProjectEntry[]> {
    return this.activeScenario.recentProjects;
  }

  async resetProject(path: string): Promise<void> {
    void path;
  }

  async createProject(path: string, profile: string): Promise<ProjectOpenResult> {
    return {
      path,
      config: { ...this.activeScenario.projectConfig, profile },
      chapters: this.activeScenario.chapters,
      preflight: this.activeScenario.preflight,
      latest_run: null,
    };
  }

  // Preflight
  async runPreflight(path: string): Promise<PreflightReport> {
    void path;
    return this.activeScenario.preflight;
  }

  // Settings
  async getGlobalSettings(): Promise<GlobalSettings> {
    return {
      theme: "dark",
      ui_language: "id",
      notifications_enabled: true,
      roles: {
        translator: {
          primary: { provider: "anthropic", model_id: "claude-3-5-sonnet" },
          fallbacks: [{ provider: "openai", model_id: "gpt-4o" }],
          temperature: 0.3,
          max_output_tokens: 4096,
          reasoning_effort: null,
        },
        repair: {
          primary: { provider: "anthropic", model_id: "claude-3-5-sonnet" },
          fallbacks: [{ provider: "openai", model_id: "gpt-4o" }],
          temperature: 0.2,
          max_output_tokens: 4096,
          reasoning_effort: null,
        },
        qa_reviewer: {
          primary: { provider: "openai", model_id: "gpt-4o" },
          fallbacks: [{ provider: "gemini", model_id: "gemini-1.5-pro" }],
          temperature: 0.1,
          max_output_tokens: 4096,
          reasoning_effort: null,
        },
        qa_fixer: {
          primary: { provider: "anthropic", model_id: "claude-3-5-sonnet" },
          fallbacks: [{ provider: "openai", model_id: "gpt-4o" }],
          temperature: 0.2,
          max_output_tokens: 4096,
          reasoning_effort: null,
        },
      },
      providers: PREVIEW_PROVIDERS,
      custom_models: [],
      recent_projects: this.activeScenario.recentProjects.map((p) => p.path),
    };
  }

  async saveGlobalSettings(settings: GlobalSettings): Promise<void> {
    void settings;
  }

  async getProjectConfig(path: string): Promise<ProjectConfig> {
    void path;
    return this.activeScenario.projectConfig;
  }

  async saveProjectConfig(path: string, config: ProjectConfig): Promise<void> {
    void path;
    this.activeScenario.projectConfig = config;
  }

  // Providers & Keys
  async listProviders(): Promise<ProviderConfigEntry[]> {
    return PREVIEW_PROVIDERS;
  }

  async saveProvider(entry: ProviderConfigEntry): Promise<void> {
    void entry;
  }

  async deleteProvider(id: string): Promise<void> {
    void id;
  }

  async setApiKey(providerId: string, key: string): Promise<void> {
    void providerId;
    void key;
  }

  async hasApiKey(providerId: string): Promise<boolean> {
    void providerId;
    return true;
  }

  async testProvider(providerId: string): Promise<ProviderTestResult> {
    void providerId;
    return {
      success: true,
      latency_ms: 120n,
      error: null,
    };
  }

  // Models
  async listModels(): Promise<ModelEntry[]> {
    return PREVIEW_MODELS;
  }

  async registerModel(entry: ModelEntry): Promise<void> {
    void entry;
  }

  async fetchProviderModels(
    providerType: string,
    baseUrl: string,
    apiKey?: string
  ): Promise<ModelEntry[]> {
    void providerType;
    void baseUrl;
    void apiKey;
    return PREVIEW_MODELS;
  }

  // Run
  async startRun(path: string): Promise<string> {
    void path;
    const runId = "run_preview_" + Date.now();
    this.activeScenario.runState = {
      state: "running",
      active: true,
      current_chapter_id: this.activeScenario.chapters[0]?.id ?? null,
      latest_run: {
        id: runId,
        kind: "full",
        status: "running",
        started_at: new Date().toISOString(),
        ended_at: null,
        settings_snapshot: "{}",
      },
    };
    this.startTokenSimulation();
    this.emitEvent({
      event: "run:state",
      data: { state: "running", run_id: runId },
    });
    return runId;
  }

  async pauseRun(path: string): Promise<void> {
    void path;
    this.activeScenario.runState.state = "paused";
    this.activeScenario.runState.active = false;
    this.stopTokenSimulation();
    this.emitEvent({
      event: "run:state",
      data: {
        state: "paused",
        run_id: this.activeScenario.runState.latest_run?.id ?? null,
      },
    });
  }

  async resumeRun(path: string): Promise<void> {
    void path;
    this.activeScenario.runState.state = "running";
    this.activeScenario.runState.active = true;
    this.startTokenSimulation();
    this.emitEvent({
      event: "run:state",
      data: {
        state: "running",
        run_id: this.activeScenario.runState.latest_run?.id ?? null,
      },
    });
  }

  async stopRun(path: string): Promise<void> {
    void path;
    this.activeScenario.runState.state = "idle";
    this.activeScenario.runState.active = false;
    this.stopTokenSimulation();
    this.emitEvent({
      event: "run:state",
      data: {
        state: "idle",
        run_id: this.activeScenario.runState.latest_run?.id ?? null,
      },
    });
  }

  async getRunState(path: string): Promise<RunStateResponse> {
    void path;
    return this.activeScenario.runState;
  }

  // Chapters & Segments
  async listChapters(path: string): Promise<Chapter[]> {
    void path;
    return this.activeScenario.chapters;
  }

  async getChapter(path: string, chapterId: string): Promise<ChapterDetail> {
    void path;
    const detail = this.activeScenario.chapterDetails[chapterId];
    if (detail) {
      return detail;
    }
    const ch =
      this.activeScenario.chapters.find((c) => c.id === chapterId) ?? {
        id: chapterId,
        ordinal: 1,
        file_name: `${chapterId}.txt`,
        prefix: "1",
        title_src: "Mock Chapter",
        title_tgt: null,
        src_hash: "mock_hash",
        line_count: 10,
        char_count: 100,
        status: "pending",
        frozen: false,
        parts_planned: 1,
        updated_at: new Date().toISOString(),
      };
    return {
      chapter: ch,
      segments: [],
      translations: [],
    };
  }

  async listSegments(path: string, chapterId: string): Promise<Segment[]> {
    void path;
    return this.activeScenario.chapterDetails[chapterId]?.segments ?? [];
  }

  async listTranslations(path: string, chapterId: string): Promise<Translation[]> {
    void path;
    return this.activeScenario.chapterDetails[chapterId]?.translations ?? [];
  }

  // Glossary
  async listGlossary(path: string): Promise<GlossaryEntry[]> {
    void path;
    return this.activeScenario.glossary;
  }

  async addGlossaryEntry(path: string, entry: GlossaryEntryInput): Promise<GlossaryEntry> {
    void path;
    const newEntry: GlossaryEntry = {
      id: `glo_${Date.now()}`,
      source: entry.source,
      translation: entry.translation,
      note: entry.note,
      tier: 1,
      origin: "manual",
      first_chapter_id: null,
      occurrences: 1,
      status: "active",
      file_line: this.activeScenario.glossary.length + 1,
    };
    this.activeScenario.glossary.push(newEntry);
    return newEntry;
  }

  async updateGlossaryEntry(path: string, entry: GlossaryEntryInput): Promise<void> {
    void path;
    const existing = this.activeScenario.glossary.find((g) => g.source === entry.source);
    if (existing) {
      existing.translation = entry.translation;
      existing.note = entry.note;
    }
  }

  async deleteGlossaryEntry(path: string, source: string): Promise<void> {
    void path;
    this.activeScenario.glossary = this.activeScenario.glossary.filter(
      (g) => g.source !== source
    );
  }

  // Events
  async onEngineEvent(handler: (event: EngineEvent) => void): Promise<UnlistenFn> {
    this.listeners.add(handler);
    return () => {
      this.listeners.delete(handler);
    };
  }
}
