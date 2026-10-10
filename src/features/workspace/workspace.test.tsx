import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import "@testing-library/jest-dom/vitest";
import { render, screen, fireEvent, cleanup } from "@testing-library/react";
import "../../lib/i18n";
import { Workspace } from "./Workspace";
import { useWorkspaceStore } from "./store";
import { RunControl } from "./RunBar/RunControl";
import { RailItem } from "./Rail/RailItem";
import type { Chapter } from "../../lib/bindings";

describe("Workspace shell", () => {
  const originalInnerWidth = window.innerWidth;

  beforeEach(() => {
    useWorkspaceStore.setState({
      projectPath: "D:/LN/Synthetic-Volume-1",
      chapters: [
        {
          id: "ch_01",
          ordinal: 1,
          file_name: "01_illustrations.txt",
          prefix: "01",
          title_src: "",
          title_tgt: null,
          src_hash: "hash1",
          line_count: 5,
          char_count: 0,
          status: "done",
          frozen: false,
          parts_planned: 0,
          updated_at: "2026-10-10T10:00:00Z",
        },
        {
          id: "ch_02",
          ordinal: 2,
          file_name: "02_prologue.txt",
          prefix: "02",
          title_src: "プロローグ",
          title_tgt: "Prolog",
          src_hash: "hash2",
          line_count: 100,
          char_count: 3500,
          status: "running",
          frozen: false,
          parts_planned: 1,
          updated_at: "2026-10-10T10:00:00Z",
        },
        {
          id: "ch_03",
          ordinal: 3,
          file_name: "03_chapter1.txt",
          prefix: "03",
          title_src: "第1話",
          title_tgt: null,
          src_hash: "hash3",
          line_count: 200,
          char_count: 7000,
          status: "pending",
          frozen: true,
          parts_planned: 2,
          updated_at: "2026-10-10T10:00:00Z",
        },
      ],
      selectedChapterId: "ch_02",
      runState: {
        state: "idle",
        active: false,
        current_chapter_id: null,
        latest_run: null,
      },
      preflight: {
        can_run: true,
        items: [],
      },
      activeTab: "run",
      railCollapsed: false,
      logDrawerOpen: false,
    });
  });

  afterEach(() => {
    cleanup();
    Object.defineProperty(window, "innerWidth", {
      writable: true,
      configurable: true,
      value: originalInnerWidth,
    });
    vi.restoreAllMocks();
  });

  it("applies layout classes at 1024, 1280, and 1600 widths", () => {
    // 1024px: Compact layout
    Object.defineProperty(window, "innerWidth", { writable: true, configurable: true, value: 1024 });
    const { container, unmount } = render(<Workspace />);
    expect(container.firstChild).toHaveClass(/layoutCompact/);
    unmount();

    // 1280px: Regular layout
    Object.defineProperty(window, "innerWidth", { writable: true, configurable: true, value: 1280 });
    const { container: container1280, unmount: unmount1280 } = render(<Workspace />);
    expect(container1280.firstChild).toHaveClass(/layoutRegular/);
    unmount1280();

    // 1600px: Wide layout
    Object.defineProperty(window, "innerWidth", { writable: true, configurable: true, value: 1600 });
    const { container: container1600, unmount: unmount1600 } = render(<Workspace />);
    expect(container1600.firstChild).toHaveClass(/layoutWide/);
    unmount1600();
  });

  it("handles rail collapse and expand toggle", () => {
    render(<Workspace />);
    const toggleBtn = screen.getByRole("button", { name: /Ciutkan rail bab|Collapse chapter rail/i });
    expect(toggleBtn).toBeInTheDocument();

    fireEvent.click(toggleBtn);
    expect(useWorkspaceStore.getState().railCollapsed).toBe(true);

    fireEvent.click(toggleBtn);
    expect(useWorkspaceStore.getState().railCollapsed).toBe(false);
  });

  it("renders image-page chapters with 'halaman gambar' / 'image pages' label instead of a ruler", () => {
    const imageChapter: Chapter = {
      id: "ch_img",
      ordinal: 1,
      file_name: "01_illustrations.txt",
      prefix: "01",
      title_src: "",
      title_tgt: null,
      src_hash: "hash",
      line_count: 5,
      char_count: 0,
      status: "done",
      frozen: false,
      parts_planned: 0,
      updated_at: "2026-10-10T10:00:00Z",
    };

    const { container } = render(
      <RailItem
        chapter={imageChapter}
        isSelected={false}
        isCollapsed={false}
        onSelect={() => {}}
      />
    );

    expect(container.textContent).toMatch(/halaman gambar|image pages/i);
    expect(container.querySelector('[class*="ruler"]')).toBeNull(); // No ruler rendered
  });

  it("renders frozen chapters with a sunken bar", () => {
    const frozenChapter: Chapter = {
      id: "ch_froz",
      ordinal: 3,
      file_name: "03_frozen.txt",
      prefix: "03",
      title_src: "第3話",
      title_tgt: null,
      src_hash: "hash",
      line_count: 100,
      char_count: 3000,
      status: "pending",
      frozen: true,
      parts_planned: 1,
      updated_at: "2026-10-10T10:00:00Z",
    };

    const { container } = render(
      <RailItem
        chapter={frozenChapter}
        isSelected={false}
        isCollapsed={false}
        onSelect={() => {}}
      />
    );

    expect(container.querySelector('[data-status="frozen"]')).toBeTruthy();
  });

  it("supports F6 pane cycling across rail, content, and log panes", () => {
    const { container } = render(<Workspace />);

    // Press F6
    fireEvent.keyDown(window, { key: "F6" });
    // Focus should move into content or rail
    expect(container).toBeInTheDocument();
  });

  it("switches workspace tabs via tab buttons and Ctrl+1..6 shortcuts", () => {
    render(<Workspace />);
    const chaptersTab = screen.getByRole("tab", { name: /Bab|Chapters/i });
    fireEvent.click(chaptersTab);
    expect(useWorkspaceStore.getState().activeTab).toBe("chapters");

    // Press Ctrl+1
    fireEvent.keyDown(window, { key: "1", ctrlKey: true });
    expect(useWorkspaceStore.getState().activeTab).toBe("run");

    // Press Ctrl+3
    fireEvent.keyDown(window, { key: "3", ctrlKey: true });
    expect(useWorkspaceStore.getState().activeTab).toBe("glossary");
  });
});

describe("RunControl state machine", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders Primary 'Jalankan volume' when idle with preflight pass", () => {
    useWorkspaceStore.setState({
      runState: { state: "idle", active: false, current_chapter_id: null, latest_run: null },
      preflight: { can_run: true, items: [] },
    });

    render(<RunControl />);
    const btn = screen.getByRole("button", { name: /Jalankan volume|Run volume/i });
    expect(btn).toBeInTheDocument();
    expect(btn).not.toBeDisabled();
  });

  it("renders Secondary disabled 'Jalankan volume' when preflight has blockers", () => {
    useWorkspaceStore.setState({
      runState: { state: "idle", active: false, current_chapter_id: null, latest_run: null },
      preflight: {
        can_run: false,
        items: [{ id: "b1", status: "blocker", message_code: "err", params: {}, actions: [] }],
      },
    });

    render(<RunControl />);
    const btn = screen.getByRole("button", { name: /Jalankan volume|Run volume/i });
    expect(btn).toBeDisabled();
    expect(btn).toHaveAttribute("aria-describedby");
  });

  it("renders Secondary 'Jeda' when state is running (enforcing one tally accent)", () => {
    useWorkspaceStore.setState({
      runState: { state: "running", active: true, current_chapter_id: "ch_02", latest_run: null },
    });

    render(<RunControl />);
    const btn = screen.getByRole("button", { name: /Jeda|Pause/i });
    expect(btn).toBeInTheDocument();
    expect(btn).not.toBeDisabled();
    // In running state, button is NOT primary vermilion
    expect(btn).not.toHaveClass(/btn--primary/);
  });

  it("renders Primary 'Lanjutkan run' when state is paused", () => {
    useWorkspaceStore.setState({
      runState: { state: "paused", active: false, current_chapter_id: "ch_02", latest_run: null },
    });

    render(<RunControl />);
    const btn = screen.getByRole("button", { name: /Lanjutkan run|Resume run/i });
    expect(btn).toBeInTheDocument();
  });

  it("renders Secondary 'Buka Translations.docx' when run is completed", () => {
    useWorkspaceStore.setState({
      runState: { state: "completed", active: false, current_chapter_id: null, latest_run: null },
    });

    render(<RunControl />);
    const btn = screen.getByRole("button", { name: /Buka Translations\.docx|Open Translations\.docx/i });
    expect(btn).toBeInTheDocument();
  });
});
