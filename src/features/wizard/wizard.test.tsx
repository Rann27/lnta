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

    const actionButton = screen.getByText(/Pilih|Browse/i);
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
      const runBtn = screen.getByRole("button", { name: /Jalankan volume|Run volume/i });
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
      const runBtn = screen.getByRole("button", { name: /Jalankan volume|Run volume/i });
      expect(runBtn).not.toBeDisabled();
    });
  });

  it("shows profile suggestion and template treatment choices", async () => {
    setIpcTransport(new PreviewTransport("project-idle-blocker"));
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

    expect(screen.getByText(/Pertahankan halaman depan|Keep front matter/i)).toBeInTheDocument();
    expect(screen.getByText(/Abaikan isi dokumen|Ignore document content/i)).toBeInTheDocument();
    expect(screen.getByText(/Impor bab sebagai bab beku|Import chapters as frozen/i)).toBeInTheDocument();
  });

  it("allows user to trigger create default template action", async () => {
    const transport = new PreviewTransport("project-idle-blocker");
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

    const createBtn = screen.getByText(/Buat Translations\.docx bawaan|Generate default Translations\.docx/i);
    expect(createBtn).toBeInTheDocument();
    fireEvent.click(createBtn);

    await vi.waitFor(() => {
      expect(createTplSpy).toHaveBeenCalledWith("D:/LN/Synthetic-Volume-1");
    });
  });
});
