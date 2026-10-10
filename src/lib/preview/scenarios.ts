import type {
  Chapter,
  ChapterDetail,
  EngineEvent,
  GlobalSettings,
  GlossaryEntry,
  ModelEntry,
  PreflightReport,
  ProjectConfig,
  ProviderConfigEntry,
  RecentProjectEntry,
  RunStateResponse,
} from "../bindings";

export interface PreviewScenarioData {
  name: string;
  labelKey: string;
  recentProjects: RecentProjectEntry[];
  activeProjectPath: string;
  projectConfig: ProjectConfig;
  preflight: PreflightReport;
  runState: RunStateResponse;
  chapters: Chapter[];
  chapterDetails: Record<string, ChapterDetail>;
  glossary: GlossaryEntry[];
  simulateStreaming: boolean;
  events?: EngineEvent[];
}

const DEFAULT_GLOBAL_SETTINGS: GlobalSettings = {
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
  providers: [
    {
      id: "anthropic",
      name: "Anthropic",
      provider_type: "anthropic",
      base_url: "https://api.anthropic.com",
      active: true,
    },
    {
      id: "openai",
      name: "OpenAI",
      provider_type: "openai",
      base_url: "https://api.openai.com/v1",
      active: true,
    },
    {
      id: "gemini",
      name: "Google Gemini",
      provider_type: "gemini",
      base_url: "https://generativelanguage.googleapis.com",
      active: true,
    },
  ],
  custom_models: [],
  recent_projects: ["D:/LN/Synthetic-Volume-1"],
};

export const PREVIEW_PROVIDERS: ProviderConfigEntry[] = DEFAULT_GLOBAL_SETTINGS.providers;

export const PREVIEW_MODELS: ModelEntry[] = [
  {
    provider: "anthropic",
    model_id: "claude-3-5-sonnet",
    display_name: "Claude 3.5 Sonnet",
    context_window: 200000,
    max_output_tokens: 8192,
    pricing: {
      input_per_million: 3.0,
      cached_input_per_million: 0.3,
      output_per_million: 15.0,
    },
    value_source: "default_registry",
    supports_reasoning: false,
  },
  {
    provider: "openai",
    model_id: "gpt-4o",
    display_name: "GPT-4o",
    context_window: 128000,
    max_output_tokens: 4096,
    pricing: {
      input_per_million: 2.5,
      cached_input_per_million: 1.25,
      output_per_million: 10.0,
    },
    value_source: "default_registry",
    supports_reasoning: false,
  },
  {
    provider: "gemini",
    model_id: "gemini-1.5-pro",
    display_name: "Gemini 1.5 Pro",
    context_window: 2000000,
    max_output_tokens: 8192,
    pricing: {
      input_per_million: 3.5,
      cached_input_per_million: 0.875,
      output_per_million: 10.5,
    },
    value_source: "default_registry",
    supports_reasoning: false,
  },
];

const SYNTHETIC_CONFIG: ProjectConfig = {
  profile: "ja-id",
  target_language: "id",
  instruction_file: "instruction.md",
  chunk_threshold_lines: 400,
  context_k_segments: 5,
  send_full_glossary: false,
  title_source: "heading",
  strip_title_brackets: true,
  normalize_ruby: true,
  template_treatment: "keep_front_matter",
  budget_token_limit: null,
  budget_cost_limit: null,
};

const SYNTHETIC_GLOSSARY: GlossaryEntry[] = [
  {
    id: "glo_01",
    source: "健太",
    translation: "Kenta",
    note: "Tokoh utama laki-laki",
    tier: 1,
    origin: "manual",
    first_chapter_id: "chap_01",
    occurrences: 42,
    status: "active",
    file_line: 1,
  },
  {
    id: "glo_02",
    source: "美咲",
    translation: "Misaki",
    note: "Sahabat masa kecil",
    tier: 1,
    origin: "manual",
    first_chapter_id: "chap_01",
    occurrences: 38,
    status: "active",
    file_line: 2,
  },
  {
    id: "glo_03",
    source: "魔導学園",
    translation: "Akademi Sihir",
    note: "Nama institusi",
    tier: 2,
    origin: "manual",
    first_chapter_id: "chap_02",
    occurrences: 14,
    status: "active",
    file_line: 3,
  },
];

function buildSyntheticChapterDetails(): {
  chapters: Chapter[];
  details: Record<string, ChapterDetail>;
} {
  const chapters: Chapter[] = [
    {
      id: "chap_01",
      ordinal: 1,
      file_name: "1_hajimari.txt",
      prefix: "1",
      title_src: "第1章 はじまり",
      title_tgt: "Bab 1 Permulaan",
      src_hash: "hash_c1",
      line_count: 85,
      char_count: 3200,
      status: "done",
      frozen: false,
      parts_planned: 1,
      updated_at: "2026-10-10T10:00:00Z",
    },
    {
      id: "chap_02",
      ordinal: 2,
      file_name: "2_deai.txt",
      prefix: "2",
      title_src: "第2章 予期せぬ出会い",
      title_tgt: "Bab 2 Pertemuan Tak Terduga",
      src_hash: "hash_c2",
      line_count: 120,
      char_count: 4800,
      status: "done",
      frozen: false,
      parts_planned: 1,
      updated_at: "2026-10-10T10:15:00Z",
    },
    {
      id: "chap_03",
      ordinal: 3,
      file_name: "3_ketsui.txt",
      prefix: "3",
      title_src: "第3章 決意の朝",
      title_tgt: "Bab 3 Tekad di Pagi Hari",
      src_hash: "hash_c3",
      line_count: 140,
      char_count: 5200,
      status: "running",
      frozen: false,
      parts_planned: 2,
      updated_at: "2026-10-10T10:30:00Z",
    },
    {
      id: "chap_04",
      ordinal: 4,
      file_name: "4_tabidachi.txt",
      prefix: "4",
      title_src: "第4章 旅立ちの時",
      title_tgt: null,
      src_hash: "hash_c4",
      line_count: 110,
      char_count: 4100,
      status: "pending",
      frozen: false,
      parts_planned: 1,
      updated_at: "2026-10-10T10:00:00Z",
    },
    {
      id: "chap_05",
      ordinal: 5,
      file_name: "5_shiren.txt",
      prefix: "5",
      title_src: "第5章 最初の試練",
      title_tgt: null,
      src_hash: "hash_c5",
      line_count: 160,
      char_count: 6000,
      status: "pending",
      frozen: false,
      parts_planned: 2,
      updated_at: "2026-10-10T10:00:00Z",
    },
  ];

  const details: Record<string, ChapterDetail> = {};
  for (const ch of chapters) {
    details[ch.id] = {
      chapter: ch,
      segments: [
        {
          id: `${ch.id}_s001`,
          chapter_id: ch.id,
          idx: 0,
          kind: "title",
          src_text: ch.title_src,
          line_start: 1,
          line_end: 1,
          image_file: null,
          part_index: 0,
        },
        {
          id: `${ch.id}_s002`,
          chapter_id: ch.id,
          idx: 1,
          kind: "narration",
          src_text: "空は青く澄み渡り、心地よい風が吹き抜けていた。",
          line_start: 3,
          line_end: 3,
          image_file: null,
          part_index: 0,
        },
        {
          id: `${ch.id}_s003`,
          chapter_id: ch.id,
          idx: 2,
          kind: "dialogue",
          src_text: "「健太、もう出発の時間よ」",
          line_start: 5,
          line_end: 5,
          image_file: null,
          part_index: 0,
        },
        {
          id: `${ch.id}_s004`,
          chapter_id: ch.id,
          idx: 3,
          kind: "narration",
          src_text: "美咲の声に、健太はゆっくりと荷物を背負い直した。",
          line_start: 7,
          line_end: 7,
          image_file: null,
          part_index: 0,
        },
      ],
      translations:
        ch.status === "done"
          ? [
              {
                id: `tr_${ch.id}_0`,
                chapter_id: ch.id,
                seg_from: 0,
                seg_to: 0,
                text: ch.title_tgt ?? ch.title_src,
                origin: "model",
                call_id: "call_mock_1",
                version: 1,
                is_current: true,
                created_at: ch.updated_at,
              },
              {
                id: `tr_${ch.id}_1`,
                chapter_id: ch.id,
                seg_from: 1,
                seg_to: 1,
                text: "Langit begitu biru cerah, diiringi hembusan angin sejuk.",
                origin: "model",
                call_id: "call_mock_1",
                version: 1,
                is_current: true,
                created_at: ch.updated_at,
              },
              {
                id: `tr_${ch.id}_2`,
                chapter_id: ch.id,
                seg_from: 2,
                seg_to: 2,
                text: "「Kenta, sudah waktunya berangkat.」",
                origin: "model",
                call_id: "call_mock_1",
                version: 1,
                is_current: true,
                created_at: ch.updated_at,
              },
              {
                id: `tr_${ch.id}_3`,
                chapter_id: ch.id,
                seg_from: 3,
                seg_to: 3,
                text: "Mendengar suara Misaki, Kenta perlahan membenahi barang bawaannya.",
                origin: "model",
                call_id: "call_mock_1",
                version: 1,
                is_current: true,
                created_at: ch.updated_at,
              },
            ]
          : [],
    };
  }

  return { chapters, details };
}

export function createPreviewScenarios(): Record<string, PreviewScenarioData> {
  const { chapters, details } = buildSyntheticChapterDetails();

  // 1. no-projects
  const noProjects: PreviewScenarioData = {
    name: "no-projects",
    labelKey: "preview.scenarios.no_projects",
    recentProjects: [],
    activeProjectPath: "",
    projectConfig: SYNTHETIC_CONFIG,
    preflight: { items: [], can_run: false },
    runState: { state: "idle", active: false, current_chapter_id: null, latest_run: null },
    chapters: [],
    chapterDetails: {},
    glossary: [],
    simulateStreaming: false,
  };

  // 2. project-idle-blocker
  const idleBlocker: PreviewScenarioData = {
    name: "project-idle-blocker",
    labelKey: "preview.scenarios.project_idle_blocker",
    recentProjects: [
      {
        path: "D:/LN/Synthetic-Volume-1",
        name: "Synthetic-Volume-1",
        profile: "ja-id",
        chapters_done: 0,
        chapters_total: 4,
        last_opened: "2026-10-10T09:00:00Z",
      },
    ],
    activeProjectPath: "D:/LN/Synthetic-Volume-1",
    projectConfig: SYNTHETIC_CONFIG,
    preflight: {
      can_run: false,
      items: [
        {
          id: "check_instruction",
          status: "blocker",
          message_code: "errors.instruction_missing",
          params: {},
          actions: [{ id: "browse_instruction", label_code: "actions.browse" }],
        },
        {
          id: "check_chapters",
          status: "pass",
          message_code: "preflight.chapters_found",
          params: { count: "4" },
          actions: [],
        },
        {
          id: "check_template",
          status: "pass",
          message_code: "preflight.template_ready",
          params: {},
          actions: [],
        },
      ],
    },
    runState: { state: "idle", active: false, current_chapter_id: null, latest_run: null },
    chapters: chapters.slice(0, 4).map((c) => ({ ...c, status: "pending", title_tgt: null })),
    chapterDetails: details,
    glossary: SYNTHETIC_GLOSSARY,
    simulateStreaming: false,
  };

  // 3. run-in-progress (streaming tokens)
  const runInProgress: PreviewScenarioData = {
    name: "run-in-progress",
    labelKey: "preview.scenarios.run_in_progress",
    recentProjects: [
      {
        path: "D:/LN/Synthetic-Volume-1",
        name: "Synthetic-Volume-1",
        profile: "ja-id",
        chapters_done: 2,
        chapters_total: 5,
        last_opened: "2026-10-10T10:15:00Z",
      },
    ],
    activeProjectPath: "D:/LN/Synthetic-Volume-1",
    projectConfig: SYNTHETIC_CONFIG,
    preflight: {
      can_run: true,
      items: [
        {
          id: "check_all",
          status: "pass",
          message_code: "preflight.ready",
          params: {},
          actions: [],
        },
      ],
    },
    runState: {
      state: "running",
      active: true,
      current_chapter_id: "chap_03",
      latest_run: {
        id: "run_preview_01",
        kind: "full",
        status: "running",
        started_at: "2026-10-10T10:00:00Z",
        ended_at: null,
        settings_snapshot: "{}",
      },
    },
    chapters,
    chapterDetails: details,
    glossary: SYNTHETIC_GLOSSARY,
    simulateStreaming: true,
  };

  // 4. chapter-needs-attention
  const chAttention = chapters.map((c) => {
    if (c.id === "chap_03") {
      return { ...c, status: "needs_attention" };
    }
    return c;
  });
  const chapterNeedsAttention: PreviewScenarioData = {
    name: "chapter-needs-attention",
    labelKey: "preview.scenarios.chapter_needs_attention",
    recentProjects: [
      {
        path: "D:/LN/Synthetic-Volume-1",
        name: "Synthetic-Volume-1",
        profile: "ja-id",
        chapters_done: 2,
        chapters_total: 5,
        last_opened: "2026-10-10T10:20:00Z",
      },
    ],
    activeProjectPath: "D:/LN/Synthetic-Volume-1",
    projectConfig: SYNTHETIC_CONFIG,
    preflight: {
      can_run: true,
      items: [],
    },
    runState: {
      state: "needs_attention",
      active: false,
      current_chapter_id: "chap_03",
      latest_run: {
        id: "run_preview_attn",
        kind: "full",
        status: "needs_attention",
        started_at: "2026-10-10T10:00:00Z",
        ended_at: null,
        settings_snapshot: "{}",
      },
    },
    chapters: chAttention,
    chapterDetails: details,
    glossary: SYNTHETIC_GLOSSARY,
    simulateStreaming: false,
  };

  // 5. run-paused-blocker
  const runPausedBlocker: PreviewScenarioData = {
    name: "run-paused-blocker",
    labelKey: "preview.scenarios.run_paused_blocker",
    recentProjects: [
      {
        path: "D:/LN/Synthetic-Volume-1",
        name: "Synthetic-Volume-1",
        profile: "ja-id",
        chapters_done: 1,
        chapters_total: 4,
        last_opened: "2026-10-10T10:25:00Z",
      },
    ],
    activeProjectPath: "D:/LN/Synthetic-Volume-1",
    projectConfig: SYNTHETIC_CONFIG,
    preflight: {
      can_run: false,
      items: [
        {
          id: "check_docx_locked",
          status: "blocker",
          message_code: "errors.docx_locked",
          params: {},
          actions: [{ id: "render_again", label_code: "actions.render_again" }],
        },
      ],
    },
    runState: {
      state: "paused",
      active: false,
      current_chapter_id: "chap_02",
      latest_run: {
        id: "run_preview_paused",
        kind: "full",
        status: "paused",
        started_at: "2026-10-10T10:00:00Z",
        ended_at: null,
        settings_snapshot: "{}",
      },
    },
    chapters: chapters.slice(0, 4).map((c) => (c.id === "chap_02" ? { ...c, status: "paused" } : c)),
    chapterDetails: details,
    glossary: SYNTHETIC_GLOSSARY,
    simulateStreaming: false,
  };

  // 6. qa-finished
  const allDoneChapters = chapters.slice(0, 4).map((c) => ({
    ...c,
    status: "done",
    title_tgt: c.title_tgt ?? `Bab ${c.ordinal}`,
  }));
  const qaFinished: PreviewScenarioData = {
    name: "qa-finished",
    labelKey: "preview.scenarios.qa_finished",
    recentProjects: [
      {
        path: "D:/LN/Synthetic-Volume-1",
        name: "Synthetic-Volume-1",
        profile: "ja-id",
        chapters_done: 4,
        chapters_total: 4,
        last_opened: "2026-10-10T10:50:00Z",
      },
    ],
    activeProjectPath: "D:/LN/Synthetic-Volume-1",
    projectConfig: SYNTHETIC_CONFIG,
    preflight: {
      can_run: true,
      items: [],
    },
    runState: {
      state: "done",
      active: false,
      current_chapter_id: null,
      latest_run: {
        id: "run_preview_done",
        kind: "full",
        status: "completed",
        started_at: "2026-10-10T10:00:00Z",
        ended_at: "2026-10-10T10:45:00Z",
        settings_snapshot: "{}",
      },
    },
    chapters: allDoneChapters,
    chapterDetails: details,
    glossary: SYNTHETIC_GLOSSARY,
    simulateStreaming: false,
  };

  return {
    "no-projects": noProjects,
    "project-idle-blocker": idleBlocker,
    "run-in-progress": runInProgress,
    "chapter-needs-attention": chapterNeedsAttention,
    "run-paused-blocker": runPausedBlocker,
    "qa-finished": qaFinished,
  };
}

export function resolveScenarioName(name: string | null): string {
  if (!name) return "run-in-progress";
  const normalized = name.toLowerCase().replace(/_/g, "-");
  switch (normalized) {
    case "no-projects":
    case "empty":
    case "none":
      return "no-projects";
    case "project-idle-blocker":
    case "idle-blocker":
    case "preflight-blockers":
    case "blocker":
      return "project-idle-blocker";
    case "run-in-progress":
    case "running":
    case "streaming":
    case "streaming-tokens":
      return "run-in-progress";
    case "chapter-needs-attention":
    case "needs-attention":
    case "attention":
      return "chapter-needs-attention";
    case "run-paused-blocker":
    case "paused-blocker":
    case "paused":
      return "run-paused-blocker";
    case "qa-finished":
    case "qa-done":
    case "qa":
      return "qa-finished";
    default:
      return "run-in-progress";
  }
}
