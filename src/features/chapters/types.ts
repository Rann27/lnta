import type { Segment, Translation, Chapter } from "../../lib/bindings";

export type SegmentFilter =
  | "all"
  | "text"
  | "fixed"
  | "findings"
  | "translated"
  | "pending";

export interface SegmentPair {
  id: string;
  indexLabel: string;
  primaryIdx: number;
  isRange: boolean;
  segments: Segment[];
  translation: Translation | null;
  isFixed: boolean;
  isFailed: boolean;
  isManual: boolean;
  hasFindings?: boolean;
  findingMessage?: string;
  updatedAt?: string;
}

export interface ChapterState {
  chapter: Chapter | null;
  segments: Segment[];
  translations: Translation[];
  loading: boolean;
  error: string | null;
  selectedPairId: string | null;
  searchQuery: string;
  filter: SegmentFilter;
}
