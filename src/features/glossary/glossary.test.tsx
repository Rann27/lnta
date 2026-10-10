import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import "@testing-library/jest-dom/vitest";
import { render, screen, fireEvent, cleanup, waitFor } from "@testing-library/react";
import "../../lib/i18n";
import { useGlossaryStore } from "./store";
import { GlossaryTab } from "./GlossaryTab";
import { GlossaryRow } from "./GlossaryRow";
import { lintGlossaryEntry, isNearVariant } from "./lint";
import { useWorkspaceStore } from "../workspace/store";
import type { GlossaryEntry } from "../../lib/bindings";

const SAMPLE_ENTRIES: GlossaryEntry[] = [
  {
    id: "glo_01",
    source: "健太",
    translation: "Kenta",
    note: "Tokoh utama laki-laki",
    tier: 1,
    origin: "manual",
    first_chapter_id: "01",
    occurrences: 42,
    status: "active",
    file_line: 1,
  },
  {
    id: "glo_02",
    source: "美咲",
    translation: "Misaki",
    note: "Sahabat masa kecil", // missing gender note on Tier 1
    tier: 1,
    origin: "manual",
    first_chapter_id: "01",
    occurrences: 38,
    status: "active",
    file_line: 2,
  },
  {
    id: "glo_03",
    source: "佐藤",
    translation: "Satō", // macron
    note: "Pria paruh baya",
    tier: 1,
    origin: "manual",
    first_chapter_id: "02",
    occurrences: 15,
    status: "active",
    file_line: 3,
  },
  {
    id: "glo_04",
    source: "聖剣",
    translation: "Pedang Suci",
    note: "Senjata legendaris (usulan model: Pedang Kudus)",
    tier: 2,
    origin: "model",
    first_chapter_id: "03",
    occurrences: 5,
    status: "conflict",
    file_line: 4,
  },
];

describe("Glossary Linting", () => {
  it("detects macron characters", () => {
    const lints = lintGlossaryEntry(SAMPLE_ENTRIES[2], SAMPLE_ENTRIES);
    expect(lints.some((l) => l.kind === "macron")).toBe(true);
  });

  it("detects missing gender note on Tier 1", () => {
    const lints = lintGlossaryEntry(SAMPLE_ENTRIES[1], SAMPLE_ENTRIES);
    expect(lints.some((l) => l.kind === "missing_gender")).toBe(true);
  });

  it("detects duplicate sources", () => {
    const duplicateEntry: GlossaryEntry = {
      ...SAMPLE_ENTRIES[0],
      id: "glo_dup",
    };
    const lints = lintGlossaryEntry(duplicateEntry, [...SAMPLE_ENTRIES, duplicateEntry]);
    expect(lints.some((l) => l.kind === "duplicate")).toBe(true);
  });

  it("detects near-variant names", () => {
    expect(isNearVariant("Sato", "Satou")).toBe(true);
    expect(isNearVariant("Kyoya", "Kyouya")).toBe(true);
  });
});

describe("Glossary Store", () => {
  beforeEach(() => {
    useGlossaryStore.setState({
      entries: SAMPLE_ENTRIES,
      isLoading: false,
      error: null,
      searchQuery: "",
      sortColumn: "occurrences",
      sortDirection: "desc",
      editingId: null,
    });
  });

  afterEach(() => {
    cleanup();
  });

  it("filters entries by search query", () => {
    const store = useGlossaryStore.getState();
    store.setSearchQuery("Misaki");
    expect(useGlossaryStore.getState().searchQuery).toBe("Misaki");
  });

  it("changes sort column and toggles direction", () => {
    const store = useGlossaryStore.getState();
    store.setSorting("source");
    expect(useGlossaryStore.getState().sortColumn).toBe("source");
    expect(useGlossaryStore.getState().sortDirection).toBe("asc");

    store.setSorting("source");
    expect(useGlossaryStore.getState().sortDirection).toBe("desc");
  });

  it("accepts model proposal in conflict row", async () => {
    const store = useGlossaryStore.getState();
    const conflictEntry = SAMPLE_ENTRIES[3];
    await store.acceptProposal("/dummy/path", conflictEntry, "Pedang Kudus");

    const updated = useGlossaryStore
      .getState()
      .entries.find((e) => e.id === conflictEntry.id);
    expect(updated?.translation).toBe("Pedang Kudus");
    expect(updated?.status).toBe("active");
  });

  it("ignores model proposal in conflict row", async () => {
    const store = useGlossaryStore.getState();
    const conflictEntry = SAMPLE_ENTRIES[3];
    await store.ignoreProposal("/dummy/path", conflictEntry);

    const updated = useGlossaryStore
      .getState()
      .entries.find((e) => e.id === conflictEntry.id);
    expect(updated?.translation).toBe("Pedang Suci");
    expect(updated?.status).toBe("active");
  });
});

describe("GlossaryRow", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders row cells, tier badge, occurrences, and lint warnings", () => {
    render(
      <GlossaryRow
        entry={SAMPLE_ENTRIES[1]}
        allEntries={SAMPLE_ENTRIES}
        projectPath="/dummy/path"
        isEditing={false}
        onStartEdit={vi.fn()}
        onCancelEdit={vi.fn()}
      />
    );

    expect(screen.getByText("美咲")).toBeInTheDocument();
    expect(screen.getByText("Misaki")).toBeInTheDocument();
    expect(screen.getByText("Sahabat masa kecil")).toBeInTheDocument();
    expect(screen.getByText("T1")).toBeInTheDocument();
    expect(screen.getByText("catatan gender kosong")).toBeInTheDocument();
  });

  it("renders conflict subrow with proposal and action buttons", () => {
    render(
      <GlossaryRow
        entry={SAMPLE_ENTRIES[3]}
        allEntries={SAMPLE_ENTRIES}
        projectPath="/dummy/path"
        isEditing={false}
        onStartEdit={vi.fn()}
        onCancelEdit={vi.fn()}
      />
    );

    expect(screen.getByText("Usulan model: Pedang Kudus")).toBeInTheDocument();
    expect(screen.getByText("Pakai usulan")).toBeInTheDocument();
    expect(screen.getByText("Abaikan")).toBeInTheDocument();
  });

  it("enters inline editing mode and responds to Escape key to cancel", () => {
    const onCancel = vi.fn();
    render(
      <GlossaryRow
        entry={SAMPLE_ENTRIES[0]}
        allEntries={SAMPLE_ENTRIES}
        projectPath="/dummy/path"
        isEditing={true}
        onStartEdit={vi.fn()}
        onCancelEdit={onCancel}
      />
    );

    const input = screen.getByDisplayValue("Kenta");
    fireEvent.keyDown(input, { key: "Escape" });
    expect(onCancel).toHaveBeenCalled();
  });
});

describe("GlossaryTab", () => {
  beforeEach(() => {
    useWorkspaceStore.setState({ projectPath: "/dummy/path" });
    useGlossaryStore.setState({
      entries: SAMPLE_ENTRIES,
      isLoading: false,
      error: null,
      searchQuery: "",
      sortColumn: "occurrences",
      sortDirection: "desc",
      editingId: null,
    });
  });

  afterEach(() => {
    cleanup();
  });

  it("renders table header and rows, filterable via search input", async () => {
    render(<GlossaryTab />);

    expect(screen.getByText("美咲")).toBeInTheDocument();
    expect(screen.getByText("健太")).toBeInTheDocument();

    const searchInput = screen.getByRole("searchbox");
    fireEvent.change(searchInput, { target: { value: "Misaki" } });

    await waitFor(() => {
      expect(screen.getByText("美咲")).toBeInTheDocument();
      expect(screen.queryByText("健太")).not.toBeInTheDocument();
    });
  });

  it("shows empty state explaining entries appear as chapters finish when empty", () => {
    useGlossaryStore.setState({ entries: [] });
    render(<GlossaryTab />);

    expect(screen.getByText("Belum ada entri glosarium")).toBeInTheDocument();
    expect(
      screen.getByText(
        /Entri glosarium akan muncul secara otomatis saat bab selesai diterjemahkan/i
      )
    ).toBeInTheDocument();
  });
});
