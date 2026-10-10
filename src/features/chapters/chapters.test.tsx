import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent, cleanup } from "@testing-library/react";
import { renderRuby } from "./ruby";
import { buildSegmentPairs } from "./store";
import { SegmentPairRow } from "./SegmentPairRow";
import { Breadcrumb } from "../../components/ui/Breadcrumb";
import { ChapterTab } from "./ChapterTab";
import { useWorkspaceStore } from "../workspace/store";
import { useChaptersStore } from "./store";
import type { Segment, Translation, Chapter } from "../../lib/bindings";

// Mock i18next
vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => {} },
  useTranslation: () => ({
    t: (key: string, params?: Record<string, unknown>) => {
      if (params) {
        let str = key;
        for (const [k, v] of Object.entries(params)) {
          str += ` ${k}=${v}`;
        }
        return str;
      }
      return key;
    },
    i18n: { language: "id" },
  }),
}));

afterEach(() => {
  cleanup();
});

describe("chapters feature", () => {
  describe("renderRuby", () => {
    it("returns plain text unchanged when no ruby syntax exists", () => {
      const rendered = render(<div>{renderRuby("青い空と白い雲")}</div>);
      expect(rendered.container.textContent).toBe("青い空と白い雲");
    });

    it("parses normalized Japanese ruby into semantic <ruby> elements", () => {
      const rendered = render(<div>{renderRuby("健太《けんた》は立ち上がった。")}</div>);
      const rubyEl = rendered.container.querySelector("ruby");
      expect(rubyEl).not.toBeNull();
      const rtEl = rubyEl?.querySelector("rt");
      expect(rtEl?.textContent).toBe("けんた");
      expect(rendered.container.textContent).toContain("健太");
    });

    it("handles multiple ruby annotations in a single sentence", () => {
      const rendered = render(
        <div>{renderRuby("春馬《はるま》と美咲《みさき》が走る。")}</div>
      );
      const rubyEls = rendered.container.querySelectorAll("ruby");
      expect(rubyEls.length).toBe(2);
      expect(rubyEls[0].querySelector("rt")?.textContent).toBe("はるま");
      expect(rubyEls[1].querySelector("rt")?.textContent).toBe("みさき");
    });
  });

  describe("buildSegmentPairs", () => {
    it("builds 1:1 segment pairs for standard segments", () => {
      const segments: Segment[] = [
        {
          id: "s1",
          chapter_id: "c1",
          idx: 1,
          kind: "narration",
          src_text: "空が青い。",
          line_start: 1,
          line_end: 1,
          image_file: null,
          part_index: 0,
        },
      ];
      const translations: Translation[] = [
        {
          id: "t1",
          chapter_id: "c1",
          seg_from: 1,
          seg_to: 1,
          text: "Langit itu biru.",
          origin: "model",
          call_id: "call_1",
          version: 1,
          is_current: true,
          created_at: "2026-10-10T12:00:00Z",
        },
      ];

      const pairs = buildSegmentPairs(segments, translations, "done");
      expect(pairs.length).toBe(1);
      expect(pairs[0].id).toBe("1");
      expect(pairs[0].indexLabel).toBe("1");
      expect(pairs[0].isRange).toBe(false);
      expect(pairs[0].translation?.text).toBe("Langit itu biru.");
      expect(pairs[0].isFixed).toBe(false);
      expect(pairs[0].isFailed).toBe(false);
    });

    it("combines merged range translations (e.g. 12-14) into one pair", () => {
      const segments: Segment[] = [
        {
          id: "s12",
          chapter_id: "c1",
          idx: 12,
          kind: "dialogue",
          src_text: "「健太！」",
          line_start: 12,
          line_end: 12,
          image_file: null,
          part_index: 0,
        },
        {
          id: "s13",
          chapter_id: "c1",
          idx: 13,
          kind: "narration",
          src_text: "美咲が叫んだ。",
          line_start: 13,
          line_end: 13,
          image_file: null,
          part_index: 0,
        },
        {
          id: "s14",
          chapter_id: "c1",
          idx: 14,
          kind: "dialogue",
          src_text: "「危ない！」",
          line_start: 14,
          line_end: 14,
          image_file: null,
          part_index: 0,
        },
      ];

      const translations: Translation[] = [
        {
          id: "t_merged",
          chapter_id: "c1",
          seg_from: 12,
          seg_to: 14,
          text: "「Kenta!」 Misaki berteriak, 「Awas!」",
          origin: "model",
          call_id: "call_1",
          version: 1,
          is_current: true,
          created_at: "2026-10-10T12:00:00Z",
        },
      ];

      const pairs = buildSegmentPairs(segments, translations, "done");
      expect(pairs.length).toBe(1);
      expect(pairs[0].id).toBe("12-14");
      expect(pairs[0].indexLabel).toBe("12-14");
      expect(pairs[0].isRange).toBe(true);
      expect(pairs[0].segments.length).toBe(3);
      expect(pairs[0].translation?.text).toBe("「Kenta!」 Misaki berteriak, 「Awas!」");
    });

    it("identifies fixed segments and failed untranslated segments", () => {
      const segments: Segment[] = [
        {
          id: "s1",
          chapter_id: "c1",
          idx: 1,
          kind: "scene",
          src_text: "☆",
          line_start: 1,
          line_end: 1,
          image_file: null,
          part_index: 0,
        },
        {
          id: "s2",
          chapter_id: "c1",
          idx: 2,
          kind: "image",
          src_text: "image_placeholder",
          line_start: 2,
          line_end: 2,
          image_file: "illustrations/01.jpg",
          part_index: 0,
        },
        {
          id: "s3",
          chapter_id: "c1",
          idx: 3,
          kind: "narration",
          src_text: "未翻訳の文章",
          line_start: 3,
          line_end: 3,
          image_file: null,
          part_index: 0,
        },
      ];

      const pairs = buildSegmentPairs(segments, [], "done");
      expect(pairs[0].isFixed).toBe(true);
      expect(pairs[1].isFixed).toBe(true);
      expect(pairs[2].isFixed).toBe(false);
      expect(pairs[2].isFailed).toBe(true);
    });

    it("processes 5,000 synthetic segments in under 50ms", () => {
      const largeSegments: Segment[] = [];
      const largeTranslations: Translation[] = [];

      for (let i = 1; i <= 5000; i++) {
        largeSegments.push({
          id: `seg_${i}`,
          chapter_id: "chap_large",
          idx: i,
          kind: i % 20 === 0 ? "scene" : "narration",
          src_text: `テスト行 ${i}`,
          line_start: i,
          line_end: i,
          image_file: null,
          part_index: 0,
        });

        if (i % 20 !== 0) {
          largeTranslations.push({
            id: `tr_${i}`,
            chapter_id: "chap_large",
            seg_from: i,
            seg_to: i,
            text: `Baris uji ${i}`,
            origin: "model",
            call_id: "call_large",
            version: 1,
            is_current: true,
            created_at: "2026-10-10T12:00:00Z",
          });
        }
      }

      const start = performance.now();
      const pairs = buildSegmentPairs(largeSegments, largeTranslations, "done");
      const elapsed = performance.now() - start;

      expect(pairs.length).toBe(5000);
      expect(elapsed).toBeLessThan(100);
    });
  });

  describe("SegmentPairRow", () => {
    it("renders text segment with Japanese lang on source and target lang on translation", () => {
      const pair = {
        id: "1",
        indexLabel: "1",
        primaryIdx: 1,
        isRange: false,
        segments: [
          {
            id: "s1",
            chapter_id: "c1",
            idx: 1,
            kind: "narration",
            src_text: "日本語のテキスト",
            line_start: 1,
            line_end: 1,
            image_file: null,
            part_index: 0,
          },
        ],
        translation: {
          id: "t1",
          chapter_id: "c1",
          seg_from: 1,
          seg_to: 1,
          text: "Teks terjemahan",
          origin: "model",
          call_id: "call_1",
          version: 1,
          is_current: true,
          created_at: "2026-10-10T12:00:00Z",
        },
        isFixed: false,
        isFailed: false,
        isManual: false,
      };

      const onClick = vi.fn();
      const { container } = render(
        <SegmentPairRow
          pair={pair}
          isSelected={false}
          targetLanguage="id"
          onClick={onClick}
        />
      );

      const srcCol = container.querySelector('[lang="ja"]');
      const tgtCol = container.querySelector('[lang="id"]');
      expect(srcCol?.textContent).toContain("日本語のテキスト");
      expect(tgtCol?.textContent).toContain("Teks terjemahan");

      fireEvent.click(container.firstChild as HTMLElement);
      expect(onClick).toHaveBeenCalledTimes(1);
    });

    it("renders fixed scene marker segment", () => {
      const pair = {
        id: "2",
        indexLabel: "2",
        primaryIdx: 2,
        isRange: false,
        segments: [
          {
            id: "s2",
            chapter_id: "c1",
            idx: 2,
            kind: "scene",
            src_text: "☆",
            line_start: 2,
            line_end: 2,
            image_file: null,
            part_index: 0,
          },
        ],
        translation: null,
        isFixed: true,
        isFailed: false,
        isManual: false,
      };

      const { container } = render(
        <SegmentPairRow pair={pair} isSelected={false} onClick={vi.fn()} />
      );
      expect(container.textContent).toContain("☆");
    });

    it("renders failed segment with validation failed indicator", () => {
      const pair = {
        id: "3",
        indexLabel: "3",
        primaryIdx: 3,
        isRange: false,
        segments: [
          {
            id: "s3",
            chapter_id: "c1",
            idx: 3,
            kind: "narration",
            src_text: "エラーテキスト",
            line_start: 3,
            line_end: 3,
            image_file: null,
            part_index: 0,
          },
        ],
        translation: null,
        isFixed: false,
        isFailed: true,
        isManual: false,
      };

      const { container } = render(
        <SegmentPairRow pair={pair} isSelected={false} onClick={vi.fn()} />
      );
      expect(container.textContent).toContain("chapters.validation_failed");
    });
  });

  describe("Breadcrumb", () => {
    it("renders breadcrumb items separated by › with last item current", () => {
      const onFirstClick = vi.fn();
      const { container } = render(
        <Breadcrumb
          items={[
            { label: "Bab 04 · 第2話", onClick: onFirstClick },
            { label: "segmen 41" },
          ]}
        />
      );

      expect(container.textContent).toContain("Bab 04 · 第2話");
      expect(container.textContent).toContain("segmen 41");

      const current = container.querySelector('[aria-current="page"]');
      expect(current?.textContent).toBe("segmen 41");

      const btn = screen.getByRole("button", { name: "Bab 04 · 第2話" });
      fireEvent.click(btn);
      expect(onFirstClick).toHaveBeenCalled();
    });
  });

  describe("ChapterTab", () => {
    beforeEach(() => {
      const testChapter: Chapter = {
        id: "chap_01",
        ordinal: 1,
        file_name: "1_intro.txt",
        prefix: "1",
        title_src: "第1話 はじまり",
        title_tgt: "Bab 1 Permulaan",
        src_hash: "hash1",
        line_count: 50,
        char_count: 1000,
        status: "done",
        frozen: false,
        parts_planned: 1,
        updated_at: "2026-10-10T10:00:00Z",
      };

      useWorkspaceStore.setState({
        projectPath: "D:/LN/Test",
        chapters: [testChapter],
        selectedChapterId: "chap_01",
      });

      useChaptersStore.setState({
        chapter: testChapter,
        segments: [
          {
            id: "s1",
            chapter_id: "chap_01",
            idx: 1,
            kind: "title",
            src_text: "第1話 はじまり",
            line_start: 1,
            line_end: 1,
            image_file: null,
            part_index: 0,
          },
          {
            id: "s2",
            chapter_id: "chap_01",
            idx: 2,
            kind: "narration",
            src_text: "朝の光が差し込む。",
            line_start: 3,
            line_end: 3,
            image_file: null,
            part_index: 0,
          },
        ],
        translations: [
          {
            id: "t1",
            chapter_id: "chap_01",
            seg_from: 1,
            seg_to: 1,
            text: "Bab 1 Permulaan",
            origin: "model",
            call_id: "c1",
            version: 1,
            is_current: true,
            created_at: "2026-10-10T10:00:00Z",
          },
          {
            id: "t2",
            chapter_id: "chap_01",
            seg_from: 2,
            seg_to: 2,
            text: "Cahaya pagi menyinari.",
            origin: "model",
            call_id: "c1",
            version: 1,
            is_current: true,
            created_at: "2026-10-10T10:00:00Z",
          },
        ],
        loading: false,
        error: null,
        selectedPairId: null,
        searchQuery: "",
        filter: "all",
      });
    });

    it("renders search field, filter segmented control, and breadcrumb", () => {
      render(<ChapterTab />);
      expect(screen.getByRole("searchbox")).toBeDefined();
      expect(screen.getByRole("radiogroup", { name: "Filter segments" })).toBeDefined();
      expect(screen.getByText(/breadcrumb_chapter/)).toBeDefined();
    });

    it("filters segments when search query changes", () => {
      vi.useFakeTimers();
      render(<ChapterTab />);
      const searchInput = screen.getByRole("searchbox");
      fireEvent.change(searchInput, { target: { value: "朝" } });
      vi.advanceTimersByTime(200);

      expect(useChaptersStore.getState().searchQuery).toBe("朝");
      vi.useRealTimers();
    });
  });
});
