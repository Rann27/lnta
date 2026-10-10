import { create } from "zustand";
import { ipc } from "../../lib/ipc";
import type { Segment, Translation } from "../../lib/bindings";
import type { SegmentFilter, SegmentPair, ChapterState } from "./types";

interface ChaptersStoreState extends ChapterState {
  fetchChapter: (projectPath: string, chapterId: string) => Promise<void>;
  selectPair: (id: string | null) => void;
  setSearchQuery: (query: string) => void;
  setFilter: (filter: SegmentFilter) => void;
  resetFilters: () => void;
}

export function buildSegmentPairs(
  segments: Segment[],
  translations: Translation[],
  chapterStatus?: string
): SegmentPair[] {
  if (!segments || segments.length === 0) return [];

  const sortedSegments = [...segments].sort((a, b) => a.idx - b.idx);
  const currentTranslations = translations.filter((t) => t.is_current !== false);

  // Map segment index to translation for O(1) lookup
  const trLookup = new Map<number, Translation>();
  for (const t of currentTranslations) {
    for (let s = t.seg_from; s <= t.seg_to; s++) {
      trLookup.set(s, t);
    }
  }

  const pairs: SegmentPair[] = [];
  let i = 0;

  while (i < sortedSegments.length) {
    const seg = sortedSegments[i];
    const tr = trLookup.get(seg.idx);

    if (tr && tr.seg_to > tr.seg_from) {
      // Merged range
      const rangeSegs: Segment[] = [];
      while (i < sortedSegments.length && sortedSegments[i].idx <= tr.seg_to) {
        rangeSegs.push(sortedSegments[i]);
        i++;
      }

      pairs.push({
        id: `${tr.seg_from}-${tr.seg_to}`,
        indexLabel: `${tr.seg_from}-${tr.seg_to}`,
        primaryIdx: tr.seg_from,
        isRange: true,
        segments: rangeSegs,
        translation: tr,
        isFixed: false,
        isFailed: false,
        isManual: tr.origin === "manual",
        updatedAt: tr.created_at,
      });
    } else {
      // Single segment
      const isFixed =
        seg.kind === "scene" ||
        seg.kind === "image" ||
        seg.kind === "image_missing";

      const isFailed =
        !isFixed &&
        !tr &&
        (chapterStatus === "done" || chapterStatus === "needs_attention");

      pairs.push({
        id: String(seg.idx),
        indexLabel: String(seg.idx),
        primaryIdx: seg.idx,
        isRange: false,
        segments: [seg],
        translation: tr ?? null,
        isFixed,
        isFailed,
        isManual: tr?.origin === "manual",
        updatedAt: tr?.created_at,
      });

      i++;
    }
  }

  return pairs;
}

export const useChaptersStore = create<ChaptersStoreState>((set) => ({
  chapter: null,
  segments: [],
  translations: [],
  loading: false,
  error: null,
  selectedPairId: null,
  searchQuery: "",
  filter: "all",

  fetchChapter: async (projectPath: string, chapterId: string) => {
    if (!projectPath || !chapterId) return;

    set({ loading: true, error: null });
    try {
      const detail = await ipc.getChapter(projectPath, chapterId);
      set({
        chapter: detail.chapter,
        segments: detail.segments,
        translations: detail.translations,
        loading: false,
        error: null,
      });
    } catch (err) {
      set({
        loading: false,
        error: err instanceof Error ? err.message : String(err),
      });
    }
  },

  selectPair: (id: string | null) => {
    set({ selectedPairId: id });
  },

  setSearchQuery: (query: string) => {
    set({ searchQuery: query });
  },

  setFilter: (filter: SegmentFilter) => {
    set({ filter });
  },

  resetFilters: () => {
    set({ searchQuery: "", filter: "all" });
  },
}));
