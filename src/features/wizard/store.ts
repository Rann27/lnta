import { create } from "zustand";
import { ipc } from "../../lib/ipc";
import type { PreflightReport, ProjectConfig, RolesConfig } from "../../lib/bindings";

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
  roles: RolesConfig | null;
  preflight: PreflightReport | null;
  isLoading: boolean;
  isPreflightRunning: boolean;
  error: string | null;

  initWizard: (path: string) => Promise<void>;
  setStep: (step: number) => void;
  nextStep: () => void;
  prevStep: () => void;
  setProfile: (profile: string) => void;
  setInstructionFile: (file: string) => void;
  setTemplateTreatment: (treatment: string) => void;
  setSendFullGlossary: (val: boolean) => void;
  runPreflightCheck: () => Promise<void>;
  saveAndProceed: () => Promise<void>;
}

export const useWizardStore = create<WizardState>((set, get) => ({
  currentStep: 0,
  folderPath: "",
  config: null,
  suggestedProfile: "ja-id",
  profile: "ja-id",
  instructionFile: "instruction.md",
  instructionCandidates: ["instruction.md", "instruction-en.md"],
  templateTreatment: "keep_front_matter",
  sendFullGlossary: false,
  roles: null,
  preflight: null,
  isLoading: false,
  isPreflightRunning: false,
  error: null,

  initWizard: async (path: string) => {
    set({ folderPath: path, isLoading: true, error: null });
    try {
      const [cfg, preflightReport, globalSettings] = await Promise.all([
        ipc.getProjectConfig(path).catch(() => null),
        ipc.runPreflight(path).catch(() => null),
        ipc.getGlobalSettings().catch(() => null),
      ]);

      const detectedProfile =
        cfg?.profile ??
        (path.toLowerCase().includes(" en") ? "ja-en" : "ja-id");

      set({
        config: cfg,
        profile: detectedProfile,
        suggestedProfile: detectedProfile,
        instructionFile: cfg?.instruction_file ?? (detectedProfile === "ja-en" ? "instruction-en.md" : "instruction.md"),
        templateTreatment: cfg?.template_treatment ?? "keep_front_matter",
        sendFullGlossary: cfg?.send_full_glossary ?? false,
        preflight: preflightReport,
        roles: globalSettings?.roles ?? null,
        isLoading: false,
      });
    } catch (err: unknown) {
      set({ error: String(err), isLoading: false });
    }
  },

  setStep: (step: number) => {
    set({ currentStep: Math.max(0, Math.min(4, step)) });
  },

  nextStep: () => {
    const { currentStep } = get();
    set({ currentStep: Math.min(4, currentStep + 1) });
  },

  prevStep: () => {
    const { currentStep } = get();
    set({ currentStep: Math.max(0, currentStep - 1) });
  },

  setProfile: (profile: string) => {
    const currentInstruction = get().instructionFile;
    let nextInstruction = currentInstruction;
    if (profile === "ja-en" && currentInstruction === "instruction.md") {
      nextInstruction = "instruction-en.md";
    } else if (profile === "ja-id" && currentInstruction === "instruction-en.md") {
      nextInstruction = "instruction.md";
    }
    set({ profile, instructionFile: nextInstruction });
  },

  setInstructionFile: (file: string) => {
    set({ instructionFile: file });
  },

  setTemplateTreatment: (treatment: string) => {
    set({ templateTreatment: treatment });
  },

  setSendFullGlossary: (val: boolean) => {
    set({ sendFullGlossary: val });
  },

  runPreflightCheck: async () => {
    const { folderPath } = get();
    set({ isPreflightRunning: true });
    try {
      const report = await ipc.runPreflight(folderPath);
      set({ preflight: report, isPreflightRunning: false });
    } catch (err: unknown) {
      set({ error: String(err), isPreflightRunning: false });
    }
  },

  saveAndProceed: async () => {
    const { folderPath, config, profile, instructionFile, templateTreatment, sendFullGlossary } = get();
    const updated: ProjectConfig = {
      profile,
      target_language: profile === "ja-en" ? "en" : "id",
      instruction_file: instructionFile,
      chunk_threshold_lines: config?.chunk_threshold_lines ?? 400,
      context_k_segments: config?.context_k_segments ?? 5,
      send_full_glossary: sendFullGlossary,
      title_source: config?.title_source ?? "heading",
      strip_title_brackets: config?.strip_title_brackets ?? true,
      normalize_ruby: config?.normalize_ruby ?? true,
      template_treatment: templateTreatment,
      budget_token_limit: config?.budget_token_limit ?? null,
      budget_cost_limit: config?.budget_cost_limit ?? null,
    };
    await ipc.saveProjectConfig(folderPath, updated);
  },
}));
