import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import "@testing-library/jest-dom/vitest";
import { render, screen, fireEvent, cleanup } from "@testing-library/react";
import "../../lib/i18n";
import { FixDiff } from "../../components/ui/FixDiff";
import { QaTab } from "./QaTab";
import { useQaStore } from "./store";
import { useWorkspaceStore } from "../workspace/store";
import type { QaFindingsSummary, QaFixLogItem } from "../../lib/bindings";

const SAMPLE_SUMMARY: QaFindingsSummary = {
  total_findings: 3,
  remaining_findings: 2,
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

const SAMPLE_FIX_LOG: QaFixLogItem[] = [
  {
    id: "fix_1",
    finding_id: "f2",
    check_code: "Q01",
    chapter_id: "c2",
    chapter_ordinal: 2,
    seg_idx: 12,
    origin: "mechanical",
    applied_at: "2026-10-10T12:00:00Z",
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
    applied_at: "2026-10-10T12:30:00Z",
    reverted_at: "2026-10-10T12:35:00Z",
    text_before: "Ada resiko besar di depan mereka.",
    text_after: "Ada risiko besar di depan mereka.",
  },
];

describe("FixDiff Component", () => {
  afterEach(cleanup);

  it("renders before and after texts and meta", () => {
    const handleUndo = vi.fn();
    render(
      <FixDiff
        checkCode="Q01"
        origin="mechanical"
        appliedAt="2026-10-10T12:00:00Z"
        textBefore="Kalimat asli yang salah"
        textAfter="Kalimat hasil perbaikan"
        locationLabel="Bab 1 : Segmen 4"
        onUndo={handleUndo}
      />
    );

    expect(screen.getByText("Q01")).toBeInTheDocument();
    expect(screen.getByText("Bab 1 : Segmen 4")).toBeInTheDocument();
    expect(screen.getByText("Sebelum")).toBeInTheDocument();
    expect(screen.getByText("Sesudah")).toBeInTheDocument();

    const undoBtn = screen.getByRole("button", { name: /batalkan/i });
    expect(undoBtn).toBeInTheDocument();
    fireEvent.click(undoBtn);
    expect(handleUndo).toHaveBeenCalledTimes(1);
  });

  it("indicates reverted state and hides undo button when revertedAt is set", () => {
    render(
      <FixDiff
        checkCode="Q17"
        origin="mechanical"
        appliedAt="2026-10-10T12:00:00Z"
        revertedAt="2026-10-10T12:05:00Z"
        textBefore="Teks sebelum"
        textAfter="Teks sesudah"
        onUndo={vi.fn()}
      />
    );

    expect(screen.getByText(/dibatalkan/i)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /batalkan/i })).not.toBeInTheDocument();
  });
});

describe("QaTab Component", () => {
  beforeEach(() => {
    useWorkspaceStore.setState({
      projectPath: "D:/LN/TestProject",
      selectedChapterId: "c1",
    });
    useQaStore.setState({
      findingsSummary: null,
      fixLog: [],
      selectedGroupCode: null,
      activeSubTab: "findings",
      selectedFixIds: [],
      loading: false,
      error: null,
    });
  });

  afterEach(cleanup);

  it("renders empty state before the first QA run", () => {
    render(<QaTab />);
    expect(screen.getByText(/belum ada pemindaian qa/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /jalankan qa sekarang/i })).toBeInTheDocument();
  });

  it("renders findings by check with counts and groups filter", () => {
    useQaStore.setState({
      findingsSummary: SAMPLE_SUMMARY,
      fixLog: SAMPLE_FIX_LOG,
    });

    render(<QaTab />);

    // Header stats
    expect(screen.getByText(/total 3 temuan/i)).toBeInTheDocument();
    expect(screen.getByText(/2 temuan tersisa/i)).toBeInTheDocument();

    // Group filter chips
    expect(screen.getByRole("button", { name: /semua temuan/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /^Q01/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /^Q04/i })).toBeInTheDocument();

    // Findings items rendered
    expect(screen.getAllByText("Aksara Jepang tersisa")).toHaveLength(2);
    expect(screen.getByText("Atribusi dialog redundan")).toBeInTheDocument();
  });

  it("filters findings when clicking check code chip", () => {
    useQaStore.setState({
      findingsSummary: SAMPLE_SUMMARY,
      fixLog: SAMPLE_FIX_LOG,
    });

    render(<QaTab />);

    const q04Chip = screen.getByRole("button", { name: /^Q04/i });
    fireEvent.click(q04Chip);

    // Q04 item should remain, Q01 should be filtered out
    expect(screen.getByText("Atribusi dialog redundan")).toBeInTheDocument();
    expect(screen.queryByText("Aksara Jepang tersisa")).not.toBeInTheDocument();
  });

  it("jumps to segment when clicking Buka Segmen", () => {
    const selectChapterSpy = vi.fn();
    const setActiveTabSpy = vi.fn();

    useWorkspaceStore.setState({
      selectChapter: selectChapterSpy,
      setActiveTab: setActiveTabSpy,
    });

    useQaStore.setState({
      findingsSummary: SAMPLE_SUMMARY,
      fixLog: SAMPLE_FIX_LOG,
    });

    render(<QaTab />);

    const jumpButtons = screen.getAllByRole("button", { name: /buka segmen/i });
    expect(jumpButtons.length).toBeGreaterThan(0);
    fireEvent.click(jumpButtons[0]);

    expect(selectChapterSpy).toHaveBeenCalledWith("c1");
    expect(setActiveTabSpy).toHaveBeenCalledWith("chapters");
  });

  it("renders fix log with FixDiff items and switches sub-tabs", () => {
    useQaStore.setState({
      findingsSummary: SAMPLE_SUMMARY,
      fixLog: SAMPLE_FIX_LOG,
    });

    render(<QaTab />);

    // Click Log Perbaikan tab
    const fixLogTab = screen.getByRole("tab", { name: /log perbaikan/i });
    fireEvent.click(fixLogTab);

    // Verifies FixDiff items rendered in Fix Log
    expect(screen.getByText("Q17")).toBeInTheDocument();
    expect(screen.getByText(/risiko/i)).toBeInTheDocument();
  });
});
