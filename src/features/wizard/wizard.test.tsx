import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import "@testing-library/jest-dom/vitest";
import "../../lib/i18n";
import { render, screen, cleanup, fireEvent } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { useWizardStore } from "./store";
import { WizardStepper } from "./WizardStepper";
import { PreflightChecklist } from "./PreflightChecklist";
import { WizardPage } from "./WizardPage";
import { setIpcTransport } from "../../lib/ipc";
import { PreviewTransport } from "../../lib/preview/transport";
import type { PreflightReport } from "../../lib/bindings";

describe("Project Wizard and Preflight UI", () => {
  beforeEach(() => {
    useWizardStore.setState({
      currentStep: 0,
      folderPath: "D:/LN/Synthetic-Volume-1",
      profile: "ja-id",
      suggestedProfile: "ja-id",
      instructionFile: "instruction.md",
      instructionCandidates: ["instruction.md", "instruction-en.md"],
      templateTreatment: "keep_front_matter",
      sendFullGlossary: false,
      preflight: null,
      isLoading: false,
      isPreflightRunning: false,
      error: null,
    });
  });

  afterEach(() => {
    cleanup();
  });

  it("renders 5 steps in WizardStepper with proper aria attributes", () => {
    const onSelect = vi.fn();
    render(<WizardStepper currentStep={2} onSelectStep={onSelect} />);

    const steps = screen.getAllByRole("listitem");
    expect(steps.length).toBe(5);

    const stepButtons = screen.getAllByRole("button");
    expect(stepButtons[2]).toHaveAttribute("aria-current", "step");

    fireEvent.click(stepButtons[0]);
    expect(onSelect).toHaveBeenCalledWith(0);
  });

  it("renders grouped preflight items in PreflightChecklist", () => {
    const report: PreflightReport = {
      can_run: false,
      items: [
        {
          id: "check_inst",
          status: "blocker",
          message_code: "errors.instruction_missing",
          params: {},
          actions: [{ id: "browse_inst", label_code: "actions.browse" }],
        },
        {
          id: "check_tmpl",
          status: "warning",
          message_code: "errors.invalid_template",
          params: {},
          actions: [],
        },
        {
          id: "check_done",
          status: "pass",
          message_code: "preflight.ready",
          params: {},
          actions: [],
        },
      ],
    };

    const onRerun = vi.fn();
    const onAction = vi.fn();

    render(
      <PreflightChecklist
        report={report}
        onRerun={onRerun}
        onAction={onAction}
      />
    );

    expect(screen.getByText(/1 blocker/i)).toBeInTheDocument();
    expect(screen.getByText(/1 peringatan|1 warning/i)).toBeInTheDocument();

    const actionButton = screen.getByRole("button", { name: /Pilih|Browse/i });
    expect(actionButton).toBeInTheDocument();
    fireEvent.click(actionButton);
    expect(onAction).toHaveBeenCalledWith("browse_inst", expect.anything());

    const rerunButton = screen.getByText(/Jalankan preflight lagi|Run preflight again/i);
    fireEvent.click(rerunButton);
    expect(onRerun).toHaveBeenCalled();
  });

  it("disables Run button with reason when blockers exist", async () => {
    setIpcTransport(new PreviewTransport("project-idle-blocker"));
    await useWizardStore.getState().initWizard("D:/LN/Synthetic-Volume-1");
    useWizardStore.setState({ currentStep: 4 });

    render(
      <MemoryRouter>
        <WizardPage />
      </MemoryRouter>
    );

    await vi.waitFor(() => {
      const runBtn = screen.getByRole("button", {
        name: /Simpan dan buka workspace|Save and open workspace/i,
      });
      expect(runBtn).toBeDisabled();
      expect(runBtn).toHaveAttribute("aria-describedby");
    });
  });

  it("enables Run button when preflight passes without blockers", async () => {
    setIpcTransport(new PreviewTransport("run-in-progress"));
    await useWizardStore.getState().initWizard("D:/LN/Synthetic-Volume-1");
    useWizardStore.setState({ currentStep: 4 });

    render(
      <MemoryRouter>
        <WizardPage />
      </MemoryRouter>
    );

    await vi.waitFor(() => {
      const runBtn = screen.getByRole("button", {
        name: /Simpan dan buka workspace|Save and open workspace/i,
      });
      expect(runBtn).not.toBeDisabled();
    });
  });

  it("shows profile suggestion and template treatment choices", async () => {
    setIpcTransport(new PreviewTransport("project-idle-blocker"));
    await useWizardStore.getState().initWizard("D:/LN/Synthetic-Volume-1");
    useWizardStore.setState({ currentStep: 1 });

    const { rerender } = render(
      <MemoryRouter>
        <WizardPage />
      </MemoryRouter>
    );

    expect(screen.getByText(/Disarankan dari instruction|Suggested from instruction/i)).toBeInTheDocument();

    // Advance to step 2 (Template & Glossary)
    useWizardStore.setState({ currentStep: 2 });
    rerender(
      <MemoryRouter>
        <WizardPage />
      </MemoryRouter>
    );

    // The scenario template has old text, so the treatment choice is offered.
    expect(screen.getByText(/Pertahankan halaman depan|Keep front matter/i)).toBeInTheDocument();
    expect(screen.getByText(/Abaikan isi dokumen|Ignore document content/i)).toBeInTheDocument();
    expect(screen.getByText(/Impor bab sebagai bab beku|Import chapters as frozen/i)).toBeInTheDocument();
  });

  it("offers the default template when Translations.docx is unusable", async () => {
    const transport = new PreviewTransport("run-paused-blocker");
    const createTplSpy = vi.fn().mockResolvedValue(undefined);
    transport.createDefaultTemplate = createTplSpy;
    setIpcTransport(transport);

    await useWizardStore.getState().initWizard("D:/LN/Synthetic-Volume-1");
    useWizardStore.setState({ currentStep: 2 });

    render(
      <MemoryRouter>
        <WizardPage />
      </MemoryRouter>
    );

    // The wizard reads the folder again on mount, so the button appears after that.
    const createBtn = await screen.findByText(
      /Buat Translations\.docx bawaan|Generate default Translations\.docx/i
    );
    expect(createBtn).toBeInTheDocument();
    fireEvent.click(createBtn);

    await vi.waitFor(() => {
      expect(createTplSpy).toHaveBeenCalledWith("D:/LN/Synthetic-Volume-1");
    });
  });

  it("lists the real roles on the model step instead of fixed model names", async () => {
    setIpcTransport(new PreviewTransport("project-idle-blocker"));
    await useWizardStore.getState().initWizard("D:/LN/Synthetic-Volume-1");
    useWizardStore.setState({ currentStep: 3 });

    render(
      <MemoryRouter>
        <WizardPage />
      </MemoryRouter>
    );

    expect(screen.queryByText(/Claude 3\.5 Sonnet/)).not.toBeInTheDocument();
    expect(screen.getByText(/Penerjemah|Translator/)).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /Buka Pengaturan|Open Settings/i })
    ).toBeInTheDocument();
  });

  it("hides preflight actions the screen cannot carry out and dismisses warnings", () => {
    const report: PreflightReport = {
      can_run: true,
      items: [
        {
          id: "instruction_language_match",
          status: "warning",
          message_code: "preflight.instruction_language_mismatch",
          params: { detected_profile: "ja-en", selected_profile: "ja-id" },
          actions: [
            { id: "switch_profile", label_code: "actions.switch_profile" },
            { id: "dismiss", label_code: "actions.dismiss" },
          ],
        },
        {
          id: "chapter_prefixes_unique",
          status: "blocker",
          message_code: "preflight.duplicate_prefix",
          params: { prefix: "03", conflicting_files: "03" },
          actions: [{ id: "show_conflicting_files", label_code: "actions.show_conflicting_files" }],
        },
      ],
    };

    render(
      <PreflightChecklist
        report={report}
        onRerun={vi.fn()}
        onAction={vi.fn()}
        availableActions={new Set(["switch_profile"])}
      />
    );

    expect(screen.getByText(/JP→EN/)).toBeInTheDocument();
    expect(screen.getByText(/Bahasa instruction|Instruction language/)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Lihat file|Show files/i })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Abaikan|Dismiss/i }));
    expect(screen.queryByText(/JP→EN/)).not.toBeInTheDocument();
  });
});
