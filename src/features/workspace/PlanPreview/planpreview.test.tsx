import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import "@testing-library/jest-dom/vitest";
import { render, screen, fireEvent, cleanup, waitFor } from "@testing-library/react";
import "../../../lib/i18n";
import { PlanPreviewDialog } from "./PlanPreviewDialog";
import { useWorkspaceStore } from "../store";
import { setIpcTransport } from "../../../lib/ipc";
import { PreviewTransport } from "../../../lib/preview/transport";
import type { ProjectPlanPreview } from "../../../lib/bindings";

describe("PlanPreviewDialog (Story B5 / T54)", () => {
  const mockPlanWithLearnedRatios: ProjectPlanPreview = {
    model_id: "claude-3-5-sonnet",
    model_name: "Claude 3.5 Sonnet",
    provider: "anthropic",
    total_chapters: 2,
    total_parts: 3,
    total_input_tokens: 15400,
    total_output_tokens: 7200,
    estimated_total_cost: 0.154,
    has_cost_estimate: true,
    is_learned_ratio: true,
    completed_chapters_count: 2,
    chars_per_input_token: 1.15,
    chars_per_output_token: 1.85,
    chapters: [
      {
        chapter_id: "chap-1",
        file_name: "01_prologue.md",
        title: "第1話 プロローグ",
        line_count: 850,
        char_count: 5000,
        part_count: 1,
        estimated_input_tokens: 5200,
        estimated_output_tokens: 2700,
        estimated_cost: 0.052,
        status: "done",
        parts: [
          {
            part_index: 1,
            total_parts: 1,
            line_start: 1,
            line_end: 850,
            segment_count: 45,
            estimated_input_tokens: 5200,
            estimated_output_tokens: 2700,
          },
        ],
      },
      {
        chapter_id: "chap-2",
        file_name: "02_chapter.md",
        title: "第2話 旅立ち",
        line_count: 1800,
        char_count: 10000,
        part_count: 2,
        estimated_input_tokens: 10200,
        estimated_output_tokens: 4500,
        estimated_cost: 0.102,
        status: "ready",
        parts: [
          {
            part_index: 1,
            total_parts: 2,
            line_start: 1,
            line_end: 900,
            segment_count: 50,
            estimated_input_tokens: 5100,
            estimated_output_tokens: 2250,
          },
          {
            part_index: 2,
            total_parts: 2,
            line_start: 901,
            line_end: 1800,
            segment_count: 48,
            estimated_input_tokens: 5100,
            estimated_output_tokens: 2250,
          },
        ],
      },
    ],
  };

  beforeEach(() => {
    useWorkspaceStore.setState({
      projectPath: "/test/project",
      providerStatus: { name: "Anthropic", model: "claude-3-5-sonnet", connected: true },
    });
  });

  afterEach(() => {
    cleanup();
    setIpcTransport(null);
  });

  it("does not render when isOpen is false", () => {
    render(<PlanPreviewDialog isOpen={false} onClose={() => {}} />);
    expect(screen.queryByText(/Rencana Chunk & Estimasi Token/i)).not.toBeInTheDocument();
  });

  it("renders plan metrics with estimates and learned ratio badge", async () => {
    const transport = new PreviewTransport();
    transport.getPlanPreview = vi.fn().mockResolvedValue(mockPlanWithLearnedRatios);
    setIpcTransport(transport);

    render(<PlanPreviewDialog isOpen={true} onClose={() => {}} />);

    await waitFor(() => {
      expect(screen.getByText(/Terkalibrasi dari 2 bab selesai/i)).toBeInTheDocument();
    });

    // Check summary metrics
    expect(screen.getByText(/15\.400/)).toBeInTheDocument();
    expect(screen.getByText(/7\.200/)).toBeInTheDocument();
    expect(screen.getByText(/0\.154/)).toBeInTheDocument();

    // Verify estimates are explicitly labeled
    const estimateLabels = screen.getAllByText(/\(estimasi\)/i);
    expect(estimateLabels.length).toBeGreaterThan(0);

    // Verify chapter entries
    expect(screen.getByText("第1話 プロローグ")).toBeInTheDocument();
    expect(screen.getByText("第2話 旅立ち")).toBeInTheDocument();
  });

  it("renders profile default ratio badge when not learned", async () => {
    const uncalibratedPlan: ProjectPlanPreview = {
      ...mockPlanWithLearnedRatios,
      is_learned_ratio: false,
      completed_chapters_count: 0,
      chars_per_input_token: 1.0,
      chars_per_output_token: 2.0,
      has_cost_estimate: false,
      estimated_total_cost: null,
    };
    const transport = new PreviewTransport();
    transport.getPlanPreview = vi.fn().mockResolvedValue(uncalibratedPlan);
    setIpcTransport(transport);

    render(<PlanPreviewDialog isOpen={true} onClose={() => {}} />);

    await waitFor(() => {
      expect(screen.getByText(/Default profil/i)).toBeInTheDocument();
    });

    // Cost unknown displayed when pricing is missing
    expect(screen.getByText(/Harga tidak diketahui/i)).toBeInTheDocument();
  });

  it("expands chapter parts breakdown on clicking a multi-part chapter row", async () => {
    const transport = new PreviewTransport();
    transport.getPlanPreview = vi.fn().mockResolvedValue(mockPlanWithLearnedRatios);
    setIpcTransport(transport);

    render(<PlanPreviewDialog isOpen={true} onClose={() => {}} />);

    await waitFor(() => {
      expect(screen.getByText("第2話 旅立ち")).toBeInTheDocument();
    });

    // Click chapter 2 row
    const chap2Row = screen.getByText("第2話 旅立ち").closest("tr");
    expect(chap2Row).not.toBeNull();
    if (chap2Row) {
      fireEvent.click(chap2Row);
    }

    // Parts breakdown should be visible
    await waitFor(() => {
      expect(screen.getByText(/Bagian 1: Baris 1-900/i)).toBeInTheDocument();
      expect(screen.getByText(/Bagian 2: Baris 901-1800/i)).toBeInTheDocument();
    });
  });

  it("triggers onClose when clicking close button", async () => {
    const transport = new PreviewTransport();
    transport.getPlanPreview = vi.fn().mockResolvedValue(mockPlanWithLearnedRatios);
    setIpcTransport(transport);
    const onClose = vi.fn();

    render(<PlanPreviewDialog isOpen={true} onClose={onClose} />);

    await waitFor(() => {
      expect(screen.getByText("第1話 プロローグ")).toBeInTheDocument();
    });

    const closeBtn = screen.getByRole("button", { name: "Tutup" });
    fireEvent.click(closeBtn);
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
