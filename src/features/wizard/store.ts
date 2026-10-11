import { create } from "zustand";
import { ipc } from "../../lib/ipc";
import { errorMessage } from "../../lib/errors";
import type {
  FrozenChapterMapping,
  PreflightReport,
  ProjectConfig,
  RolesConfig,
} from "../../lib/bindings";

export const PREFLIGHT_STEP = 4;

export interface WizardState {
  currentStep: number;
  folderPath: string;
  config: ProjectConfig | null;
  suggestedProfile: string;
  profile: string;
  instructionFile: string;
  instructionCandidates: string[];
  templateTreatment: string;
  sendFullGlossary: boolean;
  frozenMappings: FrozenChapterMapping[];
  isLoadingFrozen: boolean;
  roles: RolesConfig | null;
  preflight: PreflightReport | null;
  isLoading: boolean;
  isSaving: boolean;
  isPreflightRunning: boolean;
  error: string | null;

  initWizard: (path: string) => Promise<void>;
  setStep: (step: number) => void;
  nextStep: () => void;
  prevStep: () => void;
  setProfile: (profile: string) => Promise<void>;
  setInstructionFile: (file: string) => void;
  setTemplateTreatment: (treatment: string) => void;
  setSendFullGlossary: (val: boolean) => void;
  fetchFrozenProposal: () => Promise<void>;
  toggleFrozenMapping: (prefix: string) => void;
  runPreflightCheck: () => Promise<void>;
  /** Writes the choices so far to `.lnta/project.toml`, creating the project on first save. */
  saveDraft: () => Promise<void>;
  /** Saves, then checks the folder with the saved profile and instruction. */
  proceedToPreflight: () => Promise<void>;
  saveAndProceed: () => Promise<void>;
}

/**
 * Profile suggestion per Story A3: a saved profile wins; otherwise the language heuristic
 * on the instruction file, then which profile has an instruction file at all.
 */
function suggestProfile(
  config: ProjectConfig | null,
  report: PreflightReport | null,
  idCandidates: string[],
  enCandidates: string[]
): string {
  if (config?.profile) return config.profile;
  const mismatch = report?.items.find(
    (i) => i.id === "instruction_language_match" && i.status !== "pass"
  );
  const detected = mismatch?.params.detected_profile;
  if (detected === "ja-id" || detected === "ja-en") return detected;
  if (idCandidates.length === 0 && enCandidates.length > 0) return "ja-en";
  return "ja-id";
}

export const useWizardStore = create<WizardState>((set, get) => ({
  currentStep: 0,
  folderPath: "",
  config: null,
  suggestedProfile: "ja-id",
  profile: "ja-id",
  instructionFile: "",
  instructionCandidates: [],
  templateTreatment: "keep_front_matter",
  sendFullGlossary: false,
  frozenMappings: [],
  isLoadingFrozen: false,
  roles: null,
  preflight: null,
  isLoading: false,
  isSaving: false,
  isPreflightRunning: false,
  error: null,

  initWizard: async (path: string) => {
    const sameFolder = get().folderPath === path;
    set({
      folderPath: path,
      currentStep: sameFolder ? get().currentStep : 0,
      preflight: null,
      frozenMappings: [],
      isLoading: true,
      error: null,
    });
    try {
      let preflightError: unknown = null;
      const [cfg, preflightReport, globalSettings, idCandidates, enCandidates] =
        await Promise.all([
          ipc.getProjectConfig(path).catch(() => null),
          ipc.runPreflight(path).catch((err: unknown) => {
            preflightError = err;
            return null;
          }),
          ipc.getGlobalSettings().catch(() => null),
          ipc.listInstructionCandidates(path, "ja-id").catch(() => [] as string[]),
          ipc.listInstructionCandidates(path, "ja-en").catch(() => [] as string[]),
        ]);

      const profile = suggestProfile(cfg, preflightReport, idCandidates, enCandidates);
      const candidates = profile === "ja-en" ? enCandidates : idCandidates;

      set({
        config: cfg,
        profile,
        suggestedProfile: profile,
        instructionCandidates: candidates,
        instructionFile: cfg?.instruction_file ?? candidates[0] ?? "",
        templateTreatment: cfg?.template_treatment ?? "keep_front_matter",
        sendFullGlossary: cfg?.send_full_glossary ?? false,
        preflight: preflightReport,
        roles: globalSettings?.roles ?? null,
        isLoading: false,
        // A folder that cannot be read has no report; say why instead of waiting forever.
        error: preflightError ? errorMessage(preflightError) : null,
      });
    } catch (err: unknown) {
      set({ error: errorMessage(err), isLoading: false });
    }
  },

  setStep: (step: number) => {
    const target = Math.max(0, Math.min(PREFLIGHT_STEP, step));
    if (target === PREFLIGHT_STEP && get().currentStep !== PREFLIGHT_STEP) {
      void get().proceedToPreflight();
      return;
    }
    set({ currentStep: target });
  },

  nextStep: () => {
    get().setStep(get().currentStep + 1);
  },

  prevStep: () => {
    const { currentStep } = get();
    set({ currentStep: Math.max(0, currentStep - 1) });
  },

  setProfile: async (profile: string) => {
    const { folderPath, instructionFile, instructionCandidates } = get();
    // A file the user picked by hand stays; an auto-detected one follows the profile.
    const custom = instructionFile !== "" && !instructionCandidates.includes(instructionFile);
    set({ profile });
    const candidates = await ipc
      .listInstructionCandidates(folderPath, profile)
      .catch(() => [] as string[]);
    set({
      instructionCandidates: candidates,
      instructionFile:
        custom || candidates.includes(instructionFile) ? instructionFile : (candidates[0] ?? ""),
    });
  },

  setInstructionFile: (file: string) => {
    set({ instructionFile: file });
  },

  setTemplateTreatment: (treatment: string) => {
    set({ templateTreatment: treatment });
    if (treatment === "import_frozen") {
      void get().fetchFrozenProposal();
    }
  },

  setSendFullGlossary: (val: boolean) => {
    set({ sendFullGlossary: val });
  },

  fetchFrozenProposal: async () => {
    const { folderPath } = get();
    if (!folderPath) return;
    set({ isLoadingFrozen: true });
    try {
      const mappings = await ipc.proposeFrozenChapters(folderPath);
      set({ frozenMappings: mappings, isLoadingFrozen: false });
    } catch {
      set({ isLoadingFrozen: false });
    }
  },

  toggleFrozenMapping: (prefix: string) => {
    const { frozenMappings } = get();
    set({
      frozenMappings: frozenMappings.map((m) =>
        m.raw_chapter_prefix === prefix ? { ...m, is_frozen: !m.is_frozen } : m
      ),
    });
  },

  runPreflightCheck: async () => {
    const { folderPath } = get();
    set({ isPreflightRunning: true });
    try {
      const report = await ipc.runPreflight(folderPath);
      set({ preflight: report, isPreflightRunning: false });
    } catch (err: unknown) {
      set({ error: errorMessage(err), isPreflightRunning: false });
    }
  },

  saveDraft: async () => {
    const {
      folderPath,
      profile,
      instructionFile,
      templateTreatment,
      sendFullGlossary,
      frozenMappings,
    } = get();
    let base = get().config;
    if (!base) {
      base = (await ipc.createProject(folderPath, profile)).config;
    }
    const importFrozen = templateTreatment === "import_frozen";
    const updated: ProjectConfig = {
      ...base,
      profile,
      target_language: profile === "ja-en" ? "en" : "id",
      instruction_file: instructionFile || null,
      send_full_glossary: sendFullGlossary,
      // confirm_frozen_chapters records the frozen treatment itself.
      template_treatment: importFrozen ? base.template_treatment : templateTreatment,
    };
    await ipc.saveProjectConfig(folderPath, updated);
    if (importFrozen && frozenMappings.some((m) => m.is_frozen)) {
      await ipc.confirmFrozenChapters(folderPath, frozenMappings);
    }
    set({ config: updated });
  },

  proceedToPreflight: async () => {
    set({ isSaving: true, error: null });
    try {
      await get().saveDraft();
      set({ currentStep: PREFLIGHT_STEP, isSaving: false });
      await get().runPreflightCheck();
    } catch (err: unknown) {
      set({ error: errorMessage(err), isSaving: false });
    }
  },

  saveAndProceed: async () => {
    set({ isSaving: true, error: null });
    try {
      await get().saveDraft();
      set({ isSaving: false });
    } catch (err: unknown) {
      set({ error: errorMessage(err), isSaving: false });
      throw err;
    }
  },
}));
