import type { UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppInfo,
  Chapter,
  ChapterDetail,
  EngineEvent,
  FrozenChapterMapping,
  GlobalSettings,
  GlossaryEntry,
  GlossaryEntryInput,
  ModelEntry,
  ModelReference,
  PreflightReport,
  ProjectConfig,
  ProjectOpenResult,
  ProviderConfigEntry,
  ProviderTestResult,
  QaFindingsSummary,
  QaFixLogItem,
  RecentProjectEntry,
  RunStateResponse,
  Segment,
  Translation,
  ProjectUsageResponse,
  ProjectPlanPreview,
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
  private inMemoryProviders: ProviderConfigEntry[] = [...PREVIEW_PROVIDERS];
  private inMemoryModels: ModelEntry[] = [...PREVIEW_MODELS];
  private inMemoryKeys: Map<string, string> = new Map([
    ["anthropic", "preview-key"],
    ["openai", "preview-key"],
    ["gemini", "preview-key"],
  ]);
  private inMemoryGlobalSettings: GlobalSettings | null = null;

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
      license: "MIT + Commons Clause",
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

  async proposeFrozenChapters(projectPath: string): Promise<FrozenChapterMapping[]> {
    void projectPath;
    return this.activeScenario.chapters.slice(0, 2).map((ch, idx) => ({
      raw_chapter_prefix: ch.prefix,
      raw_chapter_file: ch.file_name,
      raw_chapter_title: ch.title_src,
      docx_heading: `Bab ${idx + 1}: Terjemahan Beku`,
      paragraph_count: 12,
      is_frozen: true,
    }));
  }

  async confirmFrozenChapters(projectPath: string, mappings: FrozenChapterMapping[]): Promise<void> {
    void projectPath;
    void mappings;
  }

  async getPlanPreview(path: string, model?: ModelReference | null): Promise<ProjectPlanPreview> {
    void path;
    const chapters = this.activeScenario.chapters.map((ch) => ({
      chapter_id: ch.id,
      file_name: ch.file_name,
      title: ch.title_tgt ?? ch.title_src,
      line_count: ch.line_count,
      char_count: ch.char_count,
      part_count: ch.parts_planned || 1,
      estimated_input_tokens: 4500,
      estimated_output_tokens: 2200,
      estimated_cost: 0.045,
      status: ch.status,
      parts: [
        {
          part_index: 1,
          total_parts: 1,
          line_start: 1,
          line_end: ch.line_count,
          segment_count: 50,
          estimated_input_tokens: 4500,
          estimated_output_tokens: 2200,
        },
      ],
    }));

    return {
      model_id: model?.model_id ?? "claude-3-5-sonnet",
      model_name: model?.model_id ?? "Claude 3.5 Sonnet",
      provider: model?.provider ?? "anthropic",
      total_chapters: chapters.length,
      total_parts: chapters.reduce((acc, c) => acc + c.part_count, 0),
      total_input_tokens: chapters.reduce((acc, c) => acc + c.estimated_input_tokens, 0),
      total_output_tokens: chapters.reduce((acc, c) => acc + c.estimated_output_tokens, 0),
      estimated_total_cost: 0.225,
      has_cost_estimate: true,
      is_learned_ratio: true,
      completed_chapters_count: 2,
      chars_per_input_token: 1.1,
      chars_per_output_token: 1.9,
      chapters,
    };
  }

  // Preflight
  async runPreflight(path: string): Promise<PreflightReport> {
    void path;
    return this.activeScenario.preflight;
  }

  // Settings
  async getGlobalSettings(): Promise<GlobalSettings> {
    if (this.inMemoryGlobalSettings) {
      return this.inMemoryGlobalSettings;
    }
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
      providers: this.inMemoryProviders,
      custom_models: [],
      recent_projects: this.activeScenario.recentProjects.map((p) => p.path),
    };
  }

  async saveGlobalSettings(settings: GlobalSettings): Promise<void> {
    this.inMemoryGlobalSettings = settings;
    if (settings.providers) {
      this.inMemoryProviders = settings.providers;
    }
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
    return this.inMemoryProviders;
  }

  async saveProvider(entry: ProviderConfigEntry): Promise<void> {
    const idx = this.inMemoryProviders.findIndex((p) => p.id === entry.id);
    if (idx >= 0) {
      this.inMemoryProviders[idx] = entry;
    } else {
      this.inMemoryProviders.push(entry);
    }
  }

  async deleteProvider(id: string): Promise<void> {
    this.inMemoryProviders = this.inMemoryProviders.filter((p) => p.id !== id);
    this.inMemoryKeys.delete(id);
  }

  async setApiKey(providerId: string, key: string): Promise<void> {
    if (key.trim()) {
      this.inMemoryKeys.set(providerId, key.trim());
    } else {
      this.inMemoryKeys.delete(providerId);
    }
  }

  async hasApiKey(providerId: string): Promise<boolean> {
    return this.inMemoryKeys.has(providerId);
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
    return this.inMemoryModels;
  }

  async registerModel(entry: ModelEntry): Promise<void> {
    const idx = this.inMemoryModels.findIndex(
      (m) => m.provider === entry.provider && m.model_id === entry.model_id
    );
    if (idx >= 0) {
      this.inMemoryModels[idx] = entry;
    } else {
      this.inMemoryModels.push(entry);
    }
  }

  async fetchProviderModels(
    providerType: string,
    baseUrl: string,
    apiKey?: string
  ): Promise<ModelEntry[]> {
    void providerType;
    void baseUrl;
    void apiKey;
    return this.inMemoryModels;
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

  // QA
  async getQaSummary(path: string): Promise<QaFindingsSummary> {
    void path;
    if (this.activeScenario.name === "qa-completed" || this.activeScenario.name === "volume-completed") {
      return {
        total_findings: 12,
        remaining_findings: 3,
        groups: [
          {
            check_code: "Q01",
            name: "Q01",
            count: 2,
            findings: [
              {
                id: "f1",
                check_code: "Q01",
                chapter_id: "c1",
                chapter_ordinal: 1,
                segment_id: "s1",
                seg_idx: 4,
                severity: "warning",
                message: "Aksara Jepang tersisa",
                matched_text: "はい",
                status: "open",
              },
              {
                id: "f2",
                check_code: "Q01",
                chapter_id: "c2",
                chapter_ordinal: 2,
                segment_id: "s2",
                seg_idx: 12,
                severity: "warning",
                message: "Aksara Jepang tersisa",
                matched_text: "ええ",
                status: "fixed",
              },
            ],
          },
          {
            check_code: "Q04",
            name: "Q04",
            count: 1,
            findings: [
              {
                id: "f3",
                check_code: "Q04",
                chapter_id: "c2",
                chapter_ordinal: 2,
                segment_id: "s3",
                seg_idx: 18,
                severity: "info",
                message: "Atribusi dialog redundan",
                matched_text: "Kenta berkata begitu.",
                status: "open",
              },
            ],
          },
        ],
        findings: [
          {
            id: "f1",
            check_code: "Q01",
            chapter_id: "c1",
            chapter_ordinal: 1,
            segment_id: "s1",
            seg_idx: 4,
            severity: "warning",
            message: "Aksara Jepang tersisa",
            matched_text: "はい",
            status: "open",
          },
          {
            id: "f2",
            check_code: "Q01",
            chapter_id: "c2",
            chapter_ordinal: 2,
            segment_id: "s2",
            seg_idx: 12,
            severity: "warning",
            message: "Aksara Jepang tersisa",
            matched_text: "ええ",
            status: "fixed",
          },
          {
            id: "f3",
            check_code: "Q04",
            chapter_id: "c2",
            chapter_ordinal: 2,
            segment_id: "s3",
            seg_idx: 18,
            severity: "info",
            message: "Atribusi dialog redundan",
            matched_text: "Kenta berkata begitu.",
            status: "open",
          },
        ],
      };
    }

    return {
      total_findings: 0,
      remaining_findings: 0,
      groups: [],
      findings: [],
    };
  }

  async getQaFixLog(path: string): Promise<QaFixLogItem[]> {
    void path;
    if (this.activeScenario.name === "qa-completed" || this.activeScenario.name === "volume-completed") {
      return [
        {
          id: "fix_1",
          finding_id: "f2",
          check_code: "Q01",
          chapter_id: "c2",
          chapter_ordinal: 2,
          seg_idx: 12,
          origin: "mechanical",
          applied_at: new Date(Date.now() - 3600000).toISOString(),
          reverted_at: null,
          text_before: "\"Ya, saya mengerti, はい,\" jawab Kenta.",
          text_after: "\"Ya, saya mengerti,\" jawab Kenta.",
        },
        {
          id: "fix_2",
          finding_id: null,
          check_code: "Q17",
          chapter_id: "c1",
          chapter_ordinal: 1,
          seg_idx: 5,
          origin: "mechanical",
          applied_at: new Date(Date.now() - 1800000).toISOString(),
          reverted_at: null,
          text_before: "Ada resiko besar di depan mereka.",
          text_after: "Ada risiko besar di depan mereka.",
        },
      ];
    }
    return [];
  }

  async undoQaFix(path: string, fixId: string): Promise<void> {
    void path;
    void fixId;
  }

  async undoQaBatch(path: string, fixIds: string[]): Promise<void> {
    void path;
    void fixIds;
  }

  async undoQaRun(path: string, runId: string): Promise<void> {
    void path;
    void runId;
  }

  async retranslateChapter(
    path: string,
    chapterId: string,
    model?: ModelReference
  ): Promise<string> {
    void path;
    void chapterId;
    void model;
    return "started";
  }

  async getSegmentHistory(
    path: string,
    chapterId: string,
    segFrom: number,
    segTo: number
  ): Promise<Translation[]> {
    void path;
    void chapterId;
    return [
      {
        id: `tr_${chapterId}_${segFrom}_v1`,
        chapter_id: chapterId,
        seg_from: segFrom,
        seg_to: segTo,
        text: "Terjemahan versi 1 terdahulu.",
        origin: "model",
        call_id: null,
        version: 1,
        is_current: false,
        created_at: new Date(Date.now() - 3600000).toISOString(),
      },
      {
        id: `tr_${chapterId}_${segFrom}_v2`,
        chapter_id: chapterId,
        seg_from: segFrom,
        seg_to: segTo,
        text: "Terjemahan versi 2 terkini.",
        origin: "manual",
        call_id: null,
        version: 2,
        is_current: true,
        created_at: new Date().toISOString(),
      },
    ];
  }

  async editSegment(
    path: string,
    chapterId: string,
    segFrom: number,
    segTo: number,
    text: string
  ): Promise<Translation> {
    void path;
    return {
      id: `tr_${chapterId}_${segFrom}_v_new`,
      chapter_id: chapterId,
      seg_from: segFrom,
      seg_to: segTo,
      text,
      origin: "manual",
      call_id: null,
      version: 2,
      is_current: true,
      created_at: new Date().toISOString(),
    };
  }

  async restoreSegmentVersion(
    path: string,
    chapterId: string,
    translationId: string
  ): Promise<Translation> {
    void path;
    void chapterId;
    return {
      id: translationId,
      chapter_id: chapterId,
      seg_from: 1,
      seg_to: 1,
      text: "Versi dipulihkan.",
      origin: "model",
      call_id: null,
      version: 1,
      is_current: true,
      created_at: new Date().toISOString(),
    };
  }

  async retranslateSegment(
    path: string,
    chapterId: string,
    segFrom: number,
    segTo: number,
    model?: ModelReference
  ): Promise<Translation> {
    void path;
    void model;
    return {
      id: `tr_${chapterId}_${segFrom}_v_retranslated`,
      chapter_id: chapterId,
      seg_from: segFrom,
      seg_to: segTo,
      text: "Terjemahan ulang AI berhasil.",
      origin: "ai",
      call_id: null,
      version: 3,
      is_current: true,
      created_at: new Date().toISOString(),
    };
  }

  async getProjectUsage(path: string, runId?: string): Promise<ProjectUsageResponse> {
    void path;
    void runId;
    return {
      summary: {
        total_input_tokens: 15420,
        total_cached_tokens: 6100,
        total_output_tokens: 8340,
        total_reasoning_tokens: 0,
        total_cost: 0.084,
        has_unknown_cost: false,
        total_calls: 12,
      },
      by_chapter: [
        {
          chapter_id: "c1",
          chapter_ordinal: 1,
          chapter_title: "Bab 1: Awal Mula",
          input_tokens: 7200,
          cached_tokens: 3000,
          output_tokens: 3900,
          reasoning_tokens: 0,
          cost: 0.039,
          call_count: 6,
        },
        {
          chapter_id: "c2",
          chapter_ordinal: 2,
          chapter_title: "Bab 2: Pertemuan",
          input_tokens: 8220,
          cached_tokens: 3100,
          output_tokens: 4440,
          reasoning_tokens: 0,
          cost: 0.045,
          call_count: 6,
        },
      ],
      by_role: [
        {
          role: "translator",
          input_tokens: 13500,
          cached_tokens: 5500,
          output_tokens: 7500,
          reasoning_tokens: 0,
          cost: 0.075,
          call_count: 10,
        },
        {
          role: "repair",
          input_tokens: 1920,
          cached_tokens: 600,
          output_tokens: 840,
          reasoning_tokens: 0,
          cost: 0.009,
          call_count: 2,
        },
      ],
      by_model: [
        {
          provider: "anthropic",
          model: "claude-3-5-sonnet-20241022",
          input_tokens: 15420,
          cached_tokens: 6100,
          output_tokens: 8340,
          reasoning_tokens: 0,
          cost: 0.084,
          call_count: 12,
          pricing_source: "default_registry",
        },
      ],
    };
  }

  // Events
  async onEngineEvent(handler: (event: EngineEvent) => void): Promise<UnlistenFn> {
    this.listeners.add(handler);
    return () => {
      this.listeners.delete(handler);
    };
  }
}
