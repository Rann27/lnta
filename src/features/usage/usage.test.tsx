import { describe, it, expect, beforeEach, afterEach } from "vitest";
import "@testing-library/jest-dom/vitest";
import { render, screen, fireEvent, cleanup } from "@testing-library/react";
import "../../lib/i18n";
import { UsageTab } from "./UsageTab";
import { useUsageStore } from "./store";
import { useWorkspaceStore } from "../workspace/store";
import type { ProjectUsageResponse } from "../../lib/bindings";

const SAMPLE_USAGE: ProjectUsageResponse = {
  summary: {
    total_calls: 10,
    total_input_tokens: 12000,
    total_cached_tokens: 4000,
    total_output_tokens: 3500,
    total_reasoning_tokens: 0,
    total_cost: 0.054,
    has_unknown_cost: false,
  },
  by_chapter: [
    {
      chapter_id: "ch1",
      chapter_ordinal: 1,
      chapter_title: "Bab 1: Awal Perjalanan",
      call_count: 6,
      input_tokens: 7000,
      cached_tokens: 2500,
      output_tokens: 2000,
      reasoning_tokens: 0,
      cost: 0.032,
    },
    {
      chapter_id: "ch2",
      chapter_ordinal: 2,
      chapter_title: "Bab 2: Pertemuan",
      call_count: 4,
      input_tokens: 5000,
      cached_tokens: 1500,
      output_tokens: 1500,
      reasoning_tokens: 0,
      cost: 0.022,
    },
  ],
  by_role: [
    {
      role: "translator",
      call_count: 8,
      input_tokens: 10000,
      cached_tokens: 3500,
      output_tokens: 3000,
      reasoning_tokens: 0,
      cost: 0.046,
    },
    {
      role: "repair",
      call_count: 2,
      input_tokens: 2000,
      cached_tokens: 500,
      output_tokens: 500,
      reasoning_tokens: 0,
      cost: 0.008,
    },
  ],
  by_model: [
    {
      provider: "anthropic",
      model: "claude-3-5-sonnet",
      call_count: 10,
      input_tokens: 12000,
      cached_tokens: 4000,
      output_tokens: 3500,
      reasoning_tokens: 0,
      cost: 0.054,
      pricing_source: "default_registry",
    },
  ],
};

const SAMPLE_UNKNOWN_COST_USAGE: ProjectUsageResponse = {
  summary: {
    total_calls: 3,
    total_input_tokens: 3000,
    total_cached_tokens: 0,
    total_output_tokens: 1000,
    total_reasoning_tokens: 0,
    total_cost: null,
    has_unknown_cost: true,
  },
  by_chapter: [
    {
      chapter_id: "ch1",
      chapter_ordinal: 1,
      chapter_title: "Bab 1",
      call_count: 3,
      input_tokens: 3000,
      cached_tokens: 0,
      output_tokens: 1000,
      reasoning_tokens: 0,
      cost: null,
    },
  ],
  by_role: [
    {
      role: "translator",
      call_count: 3,
      input_tokens: 3000,
      cached_tokens: 0,
      output_tokens: 1000,
      reasoning_tokens: 0,
      cost: null,
    },
  ],
  by_model: [
    {
      provider: "custom",
      model: "local-llm",
      call_count: 3,
      input_tokens: 3000,
      cached_tokens: 0,
      output_tokens: 1000,
      reasoning_tokens: 0,
      cost: null,
      pricing_source: null,
    },
  ],
};

describe("UsageTab", () => {
  beforeEach(() => {
    useWorkspaceStore.setState({ projectPath: "/mock/project" });
    useUsageStore.setState({
      grouping: "chapter",
      usageData: SAMPLE_USAGE,
      loading: false,
      error: null,
    });
  });

  afterEach(() => {
    cleanup();
  });

  it("renders empty state when there are no recorded calls", () => {
    useUsageStore.setState({
      usageData: {
        summary: {
          total_calls: 0,
          total_input_tokens: 0,
          total_cached_tokens: 0,
          total_output_tokens: 0,
          total_reasoning_tokens: 0,
          total_cost: null,
          has_unknown_cost: false,
        },
        by_chapter: [],
        by_role: [],
        by_model: [],
      },
    });

    render(<UsageTab />);
    expect(
      screen.getByText(/Belum ada panggilan di proyek ini|No calls recorded/i)
    ).toBeInTheDocument();
  });

  it("renders summary metrics when usage data is present", () => {
    render(<UsageTab />);
    // Total calls summary
    expect(screen.getByText(/10 panggilan|10 calls/i)).toBeInTheDocument();
    // Grand total row
    expect(screen.getByText(/Total Keseluruhan|Grand Total/i)).toBeInTheDocument();
  });

  it("renders chapter rows by default", () => {
    render(<UsageTab />);
    expect(screen.getByText("Bab 1: Awal Perjalanan")).toBeInTheDocument();
    expect(screen.getByText("Bab 2: Pertemuan")).toBeInTheDocument();
  });

  it("switches to role grouping when role option is selected", () => {
    render(<UsageTab />);
    const roleBtn = screen.getByRole("radio", { name: /Per Peran|By Role/i });
    fireEvent.click(roleBtn);

    expect(screen.getByText(/Penerjemah|Translator/i)).toBeInTheDocument();
    expect(screen.getByText(/Perbaikan|Repair/i)).toBeInTheDocument();
  });

  it("switches to model grouping when model option is selected", () => {
    render(<UsageTab />);
    const modelBtn = screen.getByRole("radio", { name: /Per Model|By Model/i });
    fireEvent.click(modelBtn);

    expect(screen.getByText("anthropic / claude-3-5-sonnet")).toBeInTheDocument();
    expect(screen.getByText(/Bawaan|Default/i)).toBeInTheDocument();
  });

  it("displays missing cost indicator when cost is null", () => {
    useUsageStore.setState({
      usageData: SAMPLE_UNKNOWN_COST_USAGE,
    });

    render(<UsageTab />);
    const missingElements = screen.getAllByText(/Harga belum ada|Price not set/i);
    expect(missingElements.length).toBeGreaterThan(0);
    expect(
      screen.getByText(/\* Beberapa panggilan menggunakan model tanpa data harga|\* Some calls used models without configured pricing/i)
    ).toBeInTheDocument();
  });
});
