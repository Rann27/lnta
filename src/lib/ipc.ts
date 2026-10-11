import { invoke, isTauri } from "@tauri-apps/api/core";
import { PreviewTransport } from "./preview/transport";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { openPath as openWithDefaultApp, revealItemInDir } from "@tauri-apps/plugin-opener";
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
  GlossaryImportPreview,
} from "./bindings";

export interface IpcTransport {
  // App
  getAppInfo(): Promise<AppInfo>;

  // Project
  openProject(path: string): Promise<ProjectOpenResult>;
  listRecentProjects(): Promise<RecentProjectEntry[]>;
  resetProject(path: string): Promise<void>;
  createProject(path: string, profile: string): Promise<ProjectOpenResult>;
  proposeFrozenChapters(projectPath: string): Promise<FrozenChapterMapping[]>;
  confirmFrozenChapters(projectPath: string, mappings: FrozenChapterMapping[]): Promise<void>;
  getPlanPreview(path: string, model?: ModelReference | null): Promise<ProjectPlanPreview>;
  createDefaultTemplate(path: string): Promise<void>;
  importTemplate(path: string, source: string): Promise<void>;
  listInstructionCandidates(path: string, profile: string): Promise<string[]>;

  // Preflight
  runPreflight(path: string): Promise<PreflightReport>;

  // Settings
  getGlobalSettings(): Promise<GlobalSettings>;
  saveGlobalSettings(settings: GlobalSettings): Promise<void>;
  getProjectConfig(path: string): Promise<ProjectConfig>;
  saveProjectConfig(path: string, config: ProjectConfig): Promise<void>;

  // Providers & Keys
  listProviders(): Promise<ProviderConfigEntry[]>;
  saveProvider(entry: ProviderConfigEntry): Promise<void>;
  deleteProvider(id: string): Promise<void>;
  setApiKey(providerId: string, key: string): Promise<void>;
  hasApiKey(providerId: string): Promise<boolean>;
  testProvider(providerId: string): Promise<ProviderTestResult>;

  // Models
  listModels(): Promise<ModelEntry[]>;
  registerModel(entry: ModelEntry): Promise<void>;
  fetchProviderModels(
    providerType: string,
    baseUrl: string,
    apiKey?: string
  ): Promise<ModelEntry[]>;

  // Run
  startRun(path: string): Promise<string>;
  pauseRun(path: string): Promise<void>;
  resumeRun(path: string): Promise<void>;
  stopRun(path: string): Promise<void>;
  getRunState(path: string): Promise<RunStateResponse>;
  startQa(path: string): Promise<string>;
  renderDocx(path: string): Promise<void>;
  openPath(path: string): Promise<void>;
  revealPath(path: string): Promise<void>;
  retranslateChapter(
    path: string,
    chapterId: string,
    model?: ModelReference
  ): Promise<string>;

  // Chapters & Segments
  listChapters(path: string): Promise<Chapter[]>;
  getChapter(path: string, chapterId: string): Promise<ChapterDetail>;
  listSegments(path: string, chapterId: string): Promise<Segment[]>;
  listTranslations(path: string, chapterId: string): Promise<Translation[]>;
  getSegmentHistory(
    path: string,
    chapterId: string,
    segFrom: number,
    segTo: number
  ): Promise<Translation[]>;
  editSegment(
    path: string,
    chapterId: string,
    segFrom: number,
    segTo: number,
    text: string
  ): Promise<Translation>;
  restoreSegmentVersion(
    path: string,
    chapterId: string,
    translationId: string
  ): Promise<Translation>;
  retranslateSegment(
    path: string,
    chapterId: string,
    segFrom: number,
    segTo: number,
    model?: ModelReference
  ): Promise<Translation>;

  // Glossary
  listGlossary(path: string): Promise<GlossaryEntry[]>;
  addGlossaryEntry(path: string, entry: GlossaryEntryInput): Promise<GlossaryEntry>;
  updateGlossaryEntry(path: string, entry: GlossaryEntryInput): Promise<void>;
  fixGlossaryMacrons(path: string): Promise<number>;
  deleteGlossaryEntry(path: string, source: string): Promise<void>;
  inspectGlossaryImport(targetPath: string, sourcePath: string): Promise<GlossaryImportPreview>;
  applyGlossaryImport(targetPath: string, entries: GlossaryEntryInput[]): Promise<number>;

  // QA
  getQaSummary(path: string): Promise<QaFindingsSummary>;
  getQaFixLog(path: string): Promise<QaFixLogItem[]>;
  undoQaFix(path: string, fixId: string): Promise<void>;
  undoQaBatch(path: string, fixIds: string[]): Promise<void>;
  undoQaRun(path: string, runId: string): Promise<void>;

  // Usage
  getProjectUsage(path: string, runId?: string): Promise<ProjectUsageResponse>;

  // Events
  onEngineEvent(handler: (event: EngineEvent) => void): Promise<UnlistenFn>;
}

export class TauriTransport implements IpcTransport {
  getAppInfo(): Promise<AppInfo> {
    return invoke<AppInfo>("get_app_info");
  }

  openProject(path: string): Promise<ProjectOpenResult> {
    return invoke<ProjectOpenResult>("open_project", { path });
  }

  listRecentProjects(): Promise<RecentProjectEntry[]> {
    return invoke<RecentProjectEntry[]>("list_recent_projects");
  }

  resetProject(path: string): Promise<void> {
    return invoke<void>("reset_project", { path });
  }

  createProject(path: string, profile: string): Promise<ProjectOpenResult> {
    return invoke<ProjectOpenResult>("create_project", { path, profile });
  }

  proposeFrozenChapters(projectPath: string): Promise<FrozenChapterMapping[]> {
    return invoke<FrozenChapterMapping[]>("propose_frozen_chapters", { projectPath });
  }

  confirmFrozenChapters(projectPath: string, mappings: FrozenChapterMapping[]): Promise<void> {
    return invoke<void>("confirm_frozen_chapters", { projectPath, mappings });
  }

  getPlanPreview(path: string, model?: ModelReference | null): Promise<ProjectPlanPreview> {
    return invoke<ProjectPlanPreview>("get_plan_preview", { path, model: model ?? null });
  }

  createDefaultTemplate(path: string): Promise<void> {
    return invoke<void>("create_default_template", { path });
  }

  importTemplate(path: string, source: string): Promise<void> {
    return invoke<void>("import_template", { path, source });
  }

  listInstructionCandidates(path: string, profile: string): Promise<string[]> {
    return invoke<string[]>("list_instruction_candidates", { path, profile });
  }

  runPreflight(path: string): Promise<PreflightReport> {
    return invoke<PreflightReport>("run_preflight", { path });
  }

  getGlobalSettings(): Promise<GlobalSettings> {
    return invoke<GlobalSettings>("get_global_settings");
  }

  saveGlobalSettings(settings: GlobalSettings): Promise<void> {
    return invoke<void>("save_global_settings", { settings });
  }

  getProjectConfig(path: string): Promise<ProjectConfig> {
    return invoke<ProjectConfig>("get_project_config", { path });
  }

  saveProjectConfig(path: string, config: ProjectConfig): Promise<void> {
    return invoke<void>("save_project_config", { path, config });
  }

  listProviders(): Promise<ProviderConfigEntry[]> {
    return invoke<ProviderConfigEntry[]>("list_providers");
  }

  saveProvider(entry: ProviderConfigEntry): Promise<void> {
    return invoke<void>("save_provider", { entry });
  }

  deleteProvider(id: string): Promise<void> {
    return invoke<void>("delete_provider", { id });
  }

  setApiKey(providerId: string, key: string): Promise<void> {
    return invoke<void>("set_api_key", { providerId, key });
  }

  hasApiKey(providerId: string): Promise<boolean> {
    return invoke<boolean>("has_api_key", { providerId });
  }

  testProvider(providerId: string): Promise<ProviderTestResult> {
    return invoke<ProviderTestResult>("test_provider", { providerId });
  }

  listModels(): Promise<ModelEntry[]> {
    return invoke<ModelEntry[]>("list_models");
  }

  registerModel(entry: ModelEntry): Promise<void> {
    return invoke<void>("register_model", { entry });
  }

  fetchProviderModels(
    providerType: string,
    baseUrl: string,
    apiKey?: string
  ): Promise<ModelEntry[]> {
    return invoke<ModelEntry[]>("fetch_provider_models", {
      providerType,
      baseUrl,
      apiKey: apiKey ?? null,
    });
  }

  startRun(path: string): Promise<string> {
    return invoke<string>("start_run", { path });
  }

  pauseRun(path: string): Promise<void> {
    return invoke<void>("pause_run", { path });
  }

  resumeRun(path: string): Promise<void> {
    return invoke<void>("resume_run", { path });
  }

  stopRun(path: string): Promise<void> {
    return invoke<void>("stop_run", { path });
  }

  getRunState(path: string): Promise<RunStateResponse> {
    return invoke<RunStateResponse>("get_run_state", { path });
  }

  startQa(path: string): Promise<string> {
    return invoke<string>("start_qa", { path });
  }

  renderDocx(path: string): Promise<void> {
    return invoke<void>("render_docx", { path });
  }

  openPath(path: string): Promise<void> {
    return openWithDefaultApp(path);
  }

  revealPath(path: string): Promise<void> {
    return revealItemInDir(path);
  }

  retranslateChapter(
    path: string,
    chapterId: string,
    model?: ModelReference
  ): Promise<string> {
    return invoke<string>("retranslate_chapter", { path, chapterId, model });
  }

  listChapters(path: string): Promise<Chapter[]> {
    return invoke<Chapter[]>("list_chapters", { path });
  }

  getChapter(path: string, chapterId: string): Promise<ChapterDetail> {
    return invoke<ChapterDetail>("get_chapter", { path, chapterId });
  }

  listSegments(path: string, chapterId: string): Promise<Segment[]> {
    return invoke<Segment[]>("list_segments", { path, chapterId });
  }

  listTranslations(path: string, chapterId: string): Promise<Translation[]> {
    return invoke<Translation[]>("list_translations", { path, chapterId });
  }

  getSegmentHistory(
    path: string,
    chapterId: string,
    segFrom: number,
    segTo: number
  ): Promise<Translation[]> {
    return invoke<Translation[]>("get_segment_history", {
      path,
      chapterId,
      segFrom,
      segTo,
    });
  }

  editSegment(
    path: string,
    chapterId: string,
    segFrom: number,
    segTo: number,
    text: string
  ): Promise<Translation> {
    return invoke<Translation>("edit_segment", {
      path,
      chapterId,
      segFrom,
      segTo,
      text,
    });
  }

  restoreSegmentVersion(
    path: string,
    chapterId: string,
    translationId: string
  ): Promise<Translation> {
    return invoke<Translation>("restore_segment_version", {
      path,
      chapterId,
      translationId,
    });
  }

  retranslateSegment(
    path: string,
    chapterId: string,
    segFrom: number,
    segTo: number,
    model?: ModelReference
  ): Promise<Translation> {
    return invoke<Translation>("retranslate_segment", {
      path,
      chapterId,
      segFrom,
      segTo,
      model,
    });
  }

  listGlossary(path: string): Promise<GlossaryEntry[]> {
    return invoke<GlossaryEntry[]>("list_glossary", { path });
  }

  addGlossaryEntry(path: string, entry: GlossaryEntryInput): Promise<GlossaryEntry> {
    return invoke<GlossaryEntry>("add_glossary_entry", { path, entry });
  }

  fixGlossaryMacrons(path: string): Promise<number> {
    return invoke<number>("fix_glossary_macrons", { path });
  }

  updateGlossaryEntry(path: string, entry: GlossaryEntryInput): Promise<void> {
    return invoke<void>("update_glossary_entry", { path, entry });
  }

  deleteGlossaryEntry(path: string, source: string): Promise<void> {
    return invoke<void>("delete_glossary_entry", { path, source });
  }

  inspectGlossaryImport(targetPath: string, sourcePath: string): Promise<GlossaryImportPreview> {
    return invoke<GlossaryImportPreview>("inspect_glossary_import", { targetPath, sourcePath });
  }

  applyGlossaryImport(targetPath: string, entries: GlossaryEntryInput[]): Promise<number> {
    return invoke<number>("apply_glossary_import", { targetPath, entriesToApply: entries });
  }

  getQaSummary(path: string): Promise<QaFindingsSummary> {
    return invoke<QaFindingsSummary>("get_qa_summary", { path });
  }

  getQaFixLog(path: string): Promise<QaFixLogItem[]> {
    return invoke<QaFixLogItem[]>("get_qa_fix_log", { path });
  }

  undoQaFix(path: string, fixId: string): Promise<void> {
    return invoke<void>("undo_qa_fix", { path, fixId });
  }

  undoQaBatch(path: string, fixIds: string[]): Promise<void> {
    return invoke<void>("undo_qa_batch", { path, fixIds });
  }

  undoQaRun(path: string, runId: string): Promise<void> {
    return invoke<void>("undo_qa_run", { path, runId });
  }

  getProjectUsage(path: string, runId?: string): Promise<ProjectUsageResponse> {
    return invoke<ProjectUsageResponse>("get_project_usage", { path, runId });
  }

  onEngineEvent(handler: (event: EngineEvent) => void): Promise<UnlistenFn> {
    return listen<EngineEvent>("lnta:event", (ev) => {
      handler(ev.payload);
    });
  }
}

let activeTransport: IpcTransport | null = null;

export function setIpcTransport(transport: IpcTransport | null) {
  activeTransport = transport;
}

export function getIpc(): IpcTransport {
  if (activeTransport) {
    return activeTransport;
  }
  if (import.meta.env.DEV && !isTauri()) {
    activeTransport = new PreviewTransport();
  } else {
    activeTransport = new TauriTransport();
  }
  return activeTransport;
}

export const ipc = new Proxy({} as IpcTransport, {
  get(_target, prop) {
    const transport = getIpc();
    const val = transport[prop as keyof IpcTransport];
    if (typeof val === "function") {
      return val.bind(transport);
    }
    return val;
  },
});
