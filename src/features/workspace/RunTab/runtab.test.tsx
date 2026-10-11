import { describe, it, expect, beforeEach, afterEach } from "vitest";
import "@testing-library/jest-dom/vitest";
import { render, screen, fireEvent, cleanup } from "@testing-library/react";
import "../../../lib/i18n";
import { RunTab } from "./RunTab";
import { ActiveChapterCard } from "./ActiveChapterCard";
import { RunTabBanners } from "./RunTabBanners";
import { WaitingCounter } from "./WaitingCounter";
import { useWorkspaceStore } from "../store";

describe("ActiveChapterCard states", () => {
  const twoParts = [
    {
      id: "ch_01_part_1",
      chapter_id: "ch_01",
      part_index: 1,
      seg_from: 1,
      seg_to: 4,
      line_from: 1,
      line_to: 425,
      est_in: 0,
      est_out: 0,
      status: "planned",
    },
    {
      id: "ch_01_part_2",
      chapter_id: "ch_01",
      part_index: 2,
      seg_from: 5,
      seg_to: 8,
      line_from: 426,
      line_to: 850,
      est_in: 0,
      est_out: 0,
      status: "planned",
    },
  ];
  const cells = [1, 2, 3, 4, 5, 6, 7, 8].map((idx) => ({
    idx,
    kind: "narration",
    status: (idx <= 2 ? "done" : idx <= 4 ? "live" : "pending") as "done" | "live" | "pending",
    isFixed: false,
  }));

  beforeEach(() => {
    useWorkspaceStore.setState({
      chapters: [
        {
          id: "ch_01",
          ordinal: 1,
          file_name: "01_prologue.txt",
          prefix: "01",
          title_src: "第1話 出会い",
          title_tgt: "Bab 1 Pertemuan",
          src_hash: "hash",
          line_count: 850,
          char_count: 3200,
          status: "pending",
          frozen: false,
          parts_planned: 2,
          updated_at: "2026-10-10T10:00:00Z",
        },
      ],
      selectedChapterId: "ch_01",
      chapterParts: { ch_01: twoParts },
      chapterSegments: {},
      chapterPhase: {},
      imagePageIds: {},
      activePart: null,
      callStartedAt: null,
      streamChars: 0,
      runState: {
        state: "idle",
        active: false,
        current_chapter_id: null,
        latest_run: null,
      },
      preflight: { can_run: true, items: [] },
      qaFindingsCount: 0,
      qaPhase: null,
    });
  });

  afterEach(() => {
    cleanup();
  });

  it("renders idle state with the next chapter and its real part count", () => {
    render(<ActiveChapterCard />);
    expect(
      screen.getByText(/Belum ada run\. Bab berikutnya: 01 · 第1話 出会い \(2 bagian\)\./i)
    ).toBeInTheDocument();
  });

  it("explains that the plan is made later when the chapter has no parts yet", () => {
    useWorkspaceStore.setState({
      chapterParts: {},
      chapterPhase: { ch_01: "planned" },
      runState: { state: "running", active: true, current_chapter_id: "ch_01", latest_run: null },
    });

    render(<ActiveChapterCard />);
    expect(screen.getByText(/Rencana bagian dibuat saat bab mulai diproses/i)).toBeInTheDocument();
  });

  it("renders the running chapter from real parts, segments, and the call start time", () => {
    useWorkspaceStore.setState({
      chapterSegments: { ch_01: cells },
      chapterPhase: { ch_01: "translating(part 1/2)" },
      activePart: { chapterId: "ch_01", current: 1, total: 2 },
      callStartedAt: Date.now() - 5_000,
      runState: { state: "running", active: true, current_chapter_id: "ch_01", latest_run: null },
    });

    render(<ActiveChapterCard />);
    expect(screen.getByText(/01 · 第1話 出会い/i)).toBeInTheDocument();
    expect(screen.getByText(/2 bagian · 850 baris/i)).toBeInTheDocument();
    expect(screen.getByText(/Bagian 1/i)).toBeInTheDocument();
    expect(screen.getByRole("progressbar", { name: /2 dari 4 segmen/i })).toBeInTheDocument();
    expect(screen.getByRole("timer")).toBeInTheDocument();
  });

  it("shows the repairing tag when the engine reports the repair phase", () => {
    useWorkspaceStore.setState({
      chapterSegments: { ch_01: cells },
      chapterPhase: { ch_01: "repairing" },
      runState: { state: "running", active: true, current_chapter_id: "ch_01", latest_run: null },
    });

    render(<ActiveChapterCard />);
    expect(screen.getByText(/memperbaiki segmen/i)).toBeInTheDocument();
  });

  it("renders paused state with paused tag", () => {
    useWorkspaceStore.setState({
      runState: { state: "paused", active: false, current_chapter_id: "ch_01", latest_run: null },
    });

    render(<ActiveChapterCard />);
    expect(screen.getByText(/Dijeda/i)).toBeInTheDocument();
  });

  it("renders completed state with QA summary", () => {
    useWorkspaceStore.setState({
      runState: { state: "completed", active: false, current_chapter_id: null, latest_run: null },
      qaFindingsCount: 4,
    });

    render(<ActiveChapterCard />);
    expect(screen.getByText(/Semua bab selesai\./i)).toBeInTheDocument();
    expect(screen.getByText(/Fase QA: 4 temuan terdeteksi\./i)).toBeInTheDocument();
  });
});

describe("RunTabBanners", () => {
  beforeEach(() => {
    useWorkspaceStore.setState({
      preflight: { can_run: true, items: [] },
      docxStatus: { status: "rendered" },
      chapters: [],
    });
  });

  afterEach(() => {
    cleanup();
  });

  it("renders blocker banner when preflight has blockers", () => {
    useWorkspaceStore.setState({
      preflight: {
        can_run: false,
        items: [{ id: "b1", status: "blocker", message_code: "err", params: {}, actions: [] }],
      },
    });

    render(<RunTabBanners />);
    const alertBanner = screen.getByRole("alert");
    expect(alertBanner).toBeInTheDocument();
    expect(alertBanner).toHaveTextContent(/Run terblokir oleh preflight/i);
    expect(screen.getByRole("button", { name: /Periksa preflight/i })).toBeInTheDocument();
  });

  it("renders attention banner when docx is locked and allows dismissal", () => {
    useWorkspaceStore.setState({
      docxStatus: { status: "locked" },
    });

    render(<RunTabBanners />);
    const statusBanner = screen.getByRole("status");
    expect(statusBanner).toHaveTextContent(/File docx terkunci/i);

    const closeBtn = screen.getByRole("button", { name: /Tutup|Close/i });
    fireEvent.click(closeBtn);
    expect(screen.queryByText(/File docx terkunci/i)).toBeNull();
  });

  it("renders attention banner for chapter needing attention with working action", () => {
    useWorkspaceStore.setState({
      chapters: [
        {
          id: "ch_attn",
          ordinal: 4,
          file_name: "04_attention.txt",
          prefix: "04",
          title_src: "第4話",
          title_tgt: null,
          src_hash: "hash",
          line_count: 100,
          char_count: 2000,
          status: "needs_attention",
          frozen: false,
          parts_planned: 1,
          updated_at: "2026-10-10T10:00:00Z",
        },
      ],
    });

    render(<RunTabBanners />);
    const viewBtn = screen.getByRole("button", { name: /Lihat bab/i });
    expect(viewBtn).toBeInTheDocument();

    fireEvent.click(viewBtn);
    expect(useWorkspaceStore.getState().selectedChapterId).toBe("ch_attn");
    expect(useWorkspaceStore.getState().activeTab).toBe("chapters");
  });
});

describe("WaitingCounter component", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders waiting response before any text arrives, without spinners", () => {
    const { container } = render(<WaitingCounter startedAt={Date.now() - 12_000} charsReceived={0} />);
    expect(screen.getByText(/Menunggu respons, 12 d|Waiting for response, 12 s/i)).toBeInTheDocument();
    // Verify zero svg/spinners in WaitingCounter
    expect(container.querySelector("svg")).toBeNull();
  });

  it("renders streamed characters and elapsed seconds", () => {
    render(<WaitingCounter startedAt={Date.now() - 48_000} charsReceived={6204} />);
    expect(
      screen.getByText(/Diterima 6\.204 karakter · 48 d|Received 6,204 characters · 48 s/i)
    ).toBeInTheDocument();
  });

  it("renders nothing when no call is running", () => {
    const { container } = render(<WaitingCounter startedAt={null} />);
    expect(container.textContent).toBe("");
  });
});

describe("RunTab live region", () => {
  beforeEach(() => {
    useWorkspaceStore.setState({
      runState: { state: "idle", active: false, current_chapter_id: null, latest_run: null },
    });
  });

  afterEach(() => {
    cleanup();
  });

  it("announces run state transitions in aria-live polite region", () => {
    const { rerender } = render(<RunTab />);
    const liveRegion = screen.getByRole("status", { name: "Pengumuman status run" });
    expect(liveRegion).toHaveAttribute("aria-live", "polite");

    // Transition to running
    useWorkspaceStore.setState({
      runState: { state: "running", active: true, current_chapter_id: "ch_01", latest_run: null },
    });
    rerender(<RunTab />);
    expect(liveRegion).toHaveTextContent(/Run sedang berjalan\.|Run is running\./i);

    // Transition to paused
    useWorkspaceStore.setState({
      runState: { state: "paused", active: false, current_chapter_id: "ch_01", latest_run: null },
    });
    rerender(<RunTab />);
    expect(liveRegion).toHaveTextContent(/Run dijeda\.|Run is paused\./i);
  });
});
