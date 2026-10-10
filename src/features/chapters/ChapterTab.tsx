import React, { useEffect, useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";
import { useVirtualizer } from "@tanstack/react-virtual";
import { Breadcrumb, type BreadcrumbItem } from "../../components/ui/Breadcrumb";
import { SearchField } from "../../components/ui/SearchField";
import {
  SegmentedControl,
  type SegmentOption,
} from "../../components/ui/SegmentedControl";
import { StateBlock } from "../../components/ui/StateBlock";
import { Button } from "../../components/ui/Button";
import { SegmentPairRow } from "./SegmentPairRow";
import { useChaptersStore, buildSegmentPairs } from "./store";
import { useWorkspaceStore } from "../workspace/store";
import type { SegmentFilter } from "./types";
import styles from "./ChapterTab.module.css";

export const ChapterTab: React.FC = () => {
  const { t, i18n } = useTranslation();
  const {
    projectPath,
    selectedChapterId,
    chapters,
    selectChapter,
  } = useWorkspaceStore();

  const {
    chapter,
    segments,
    translations,
    loading,
    error,
    selectedPairId,
    searchQuery,
    filter,
    fetchChapter,
    selectPair,
    setSearchQuery,
    setFilter,
    resetFilters,
  } = useChaptersStore();

  const scrollContainerRef = useRef<HTMLDivElement>(null);

  // Auto-select first chapter if none selected
  const activeChapterId = selectedChapterId || (chapters.length > 0 ? chapters[0].id : null);

  useEffect(() => {
    if (!selectedChapterId && chapters.length > 0) {
      selectChapter(chapters[0].id);
    }
  }, [selectedChapterId, chapters, selectChapter]);

  // Fetch chapter details on active chapter change
  useEffect(() => {
    if (activeChapterId && projectPath) {
      fetchChapter(projectPath, activeChapterId);
    }
  }, [activeChapterId, projectPath, fetchChapter]);

  // Build paired segment rows
  const pairs = useMemo(() => {
    return buildSegmentPairs(segments, translations, chapter?.status);
  }, [segments, translations, chapter?.status]);

  // Filter pairs
  const filteredPairs = useMemo(() => {
    let list = pairs;

    if (filter === "text") {
      list = list.filter((p) => !p.isFixed);
    } else if (filter === "fixed") {
      list = list.filter((p) => p.isFixed);
    } else if (filter === "findings") {
      list = list.filter((p) => p.isFailed || p.hasFindings);
    } else if (filter === "translated") {
      list = list.filter((p) => p.translation !== null);
    } else if (filter === "pending") {
      list = list.filter((p) => !p.isFixed && p.translation === null);
    }

    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase().trim();
      list = list.filter((p) => {
        const srcMatch = p.segments.some(
          (s) =>
            s.src_text.toLowerCase().includes(q) ||
            (s.image_file && s.image_file.toLowerCase().includes(q))
        );
        if (srcMatch) return true;
        if (p.translation?.text.toLowerCase().includes(q)) return true;
        if (p.indexLabel.includes(q)) return true;
        return false;
      });
    }

    return list;
  }, [pairs, filter, searchQuery]);

  // Find currently selected pair
  const selectedPair = useMemo(() => {
    return pairs.find((p) => p.id === selectedPairId) ?? null;
  }, [pairs, selectedPairId]);

  // TanStack Virtualizer
  const rowVirtualizer = useVirtualizer({
    count: filteredPairs.length,
    getScrollElement: () => scrollContainerRef.current,
    estimateSize: () => 44,
    overscan: 10,
  });

  // Keyboard navigation: J (next), K (prev), N (next finding), Shift+N (prev finding)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const targetTag = (e.target as HTMLElement)?.tagName?.toLowerCase();
      if (targetTag === "input" || targetTag === "textarea") {
        return;
      }

      if (e.key === "j" || e.key === "J") {
        e.preventDefault();
        const currentIndex = filteredPairs.findIndex((p) => p.id === selectedPairId);
        const nextIndex =
          currentIndex < filteredPairs.length - 1 ? currentIndex + 1 : 0;
        if (filteredPairs[nextIndex]) {
          selectPair(filteredPairs[nextIndex].id);
          rowVirtualizer.scrollToIndex(nextIndex, { align: "auto" });
        }
      } else if (e.key === "k" || e.key === "K") {
        e.preventDefault();
        const currentIndex = filteredPairs.findIndex((p) => p.id === selectedPairId);
        const prevIndex =
          currentIndex > 0 ? currentIndex - 1 : filteredPairs.length - 1;
        if (filteredPairs[prevIndex]) {
          selectPair(filteredPairs[prevIndex].id);
          rowVirtualizer.scrollToIndex(prevIndex, { align: "auto" });
        }
      } else if (e.key === "n" || e.key === "N") {
        if (e.shiftKey) {
          // Shift+N: Previous finding or failure
          e.preventDefault();
          const currentIndex = filteredPairs.findIndex((p) => p.id === selectedPairId);
          for (let i = currentIndex - 1; i >= 0; i--) {
            if (filteredPairs[i].isFailed || filteredPairs[i].hasFindings) {
              selectPair(filteredPairs[i].id);
              rowVirtualizer.scrollToIndex(i, { align: "auto" });
              return;
            }
          }
          for (let i = filteredPairs.length - 1; i > currentIndex; i--) {
            if (filteredPairs[i].isFailed || filteredPairs[i].hasFindings) {
              selectPair(filteredPairs[i].id);
              rowVirtualizer.scrollToIndex(i, { align: "auto" });
              return;
            }
          }
        } else {
          // N: Next finding or failure
          e.preventDefault();
          const currentIndex = filteredPairs.findIndex((p) => p.id === selectedPairId);
          for (let i = currentIndex + 1; i < filteredPairs.length; i++) {
            if (filteredPairs[i].isFailed || filteredPairs[i].hasFindings) {
              selectPair(filteredPairs[i].id);
              rowVirtualizer.scrollToIndex(i, { align: "auto" });
              return;
            }
          }
          for (let i = 0; i < currentIndex; i++) {
            if (filteredPairs[i].isFailed || filteredPairs[i].hasFindings) {
              selectPair(filteredPairs[i].id);
              rowVirtualizer.scrollToIndex(i, { align: "auto" });
              return;
            }
          }
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [filteredPairs, selectedPairId, selectPair, rowVirtualizer]);

  // Breadcrumb items
  const breadcrumbItems: BreadcrumbItem[] = useMemo(() => {
    if (!chapter) {
      return [{ label: t("chapters.title") }];
    }
    const ordinalFormatted = String(chapter.ordinal).padStart(2, "0");
    const title = chapter.title_src || chapter.file_name;
    const chapterLabel = t("chapters.breadcrumb_chapter", {
      ordinal: ordinalFormatted,
      title,
    });

    const items: BreadcrumbItem[] = [{ label: chapterLabel }];
    if (selectedPair) {
      items.push({
        label: t("chapters.breadcrumb_segment", {
          index: selectedPair.indexLabel,
        }),
      });
    }
    return items;
  }, [chapter, selectedPair, t]);

  const filterOptions: SegmentOption<SegmentFilter>[] = [
    { value: "all", label: t("chapters.filters.all") },
    { value: "text", label: t("chapters.filters.text") },
    { value: "fixed", label: t("chapters.filters.fixed") },
    { value: "findings", label: t("chapters.filters.findings") },
    { value: "translated", label: t("chapters.filters.translated") },
    { value: "pending", label: t("chapters.filters.pending") },
  ];

  const targetLang = i18n.language.startsWith("en") ? "en" : "id";

  // Loading state
  if (loading && segments.length === 0) {
    return (
      <div className={styles.container}>
        <div className={styles.stateContainer}>
          <StateBlock title={t("chapters.loading")} />
        </div>
      </div>
    );
  }

  // Error state
  if (error && segments.length === 0) {
    return (
      <div className={styles.container}>
        <div className={styles.stateContainer}>
          <StateBlock
            title={t("chapters.error_loading")}
            description={error}
            action={
              <Button
                variant="secondary"
                onClick={() => activeChapterId && fetchChapter(projectPath, activeChapterId)}
              >
                {t("chapters.retry")}
              </Button>
            }
          />
        </div>
      </div>
    );
  }

  // No chapter selected
  if (!activeChapterId) {
    return (
      <div className={styles.container}>
        <div className={styles.stateContainer}>
          <StateBlock
            title={t("chapters.empty.no_chapter_title")}
            description={t("chapters.empty.no_chapter_desc")}
          />
        </div>
      </div>
    );
  }

  return (
    <div className={styles.container}>
      {/* Top Bar: Breadcrumb + Filters */}
      <div className={styles.topBar}>
        <div className={styles.breadcrumbRow}>
          <Breadcrumb items={breadcrumbItems} />
        </div>

        <div className={styles.filterRow}>
          <div className={styles.searchContainer}>
            <SearchField
              value={searchQuery}
              onChange={setSearchQuery}
              placeholder={t("chapters.search_placeholder")}
            />
            {pairs.length > 0 && (
              <span className={styles.resultCount}>
                {t("chapters.search_results_count", {
                  filtered: filteredPairs.length,
                  total: pairs.length,
                })}
              </span>
            )}
          </div>

          <SegmentedControl<SegmentFilter>
            options={filterOptions}
            value={filter}
            onChange={setFilter}
            label="Filter segments"
          />
        </div>
      </div>

      {/* Table Header */}
      <div className={styles.tableHeader} role="row">
        <span role="columnheader">{t("chapters.columns.index")}</span>
        <span role="columnheader">{t("chapters.columns.source")}</span>
        <span role="columnheader">{t("chapters.columns.target")}</span>
        <span role="columnheader">{t("chapters.columns.findings")}</span>
      </div>

      {/* Table Content */}
      {pairs.length === 0 ? (
        <div className={styles.stateContainer}>
          <StateBlock
            title={t("chapters.empty.no_segments_title")}
            description={t("chapters.empty.no_segments_desc")}
          />
        </div>
      ) : filteredPairs.length === 0 ? (
        <div className={styles.stateContainer}>
          <StateBlock
            title={t("chapters.empty.no_results_title")}
            description={t("chapters.empty.no_results_desc")}
            action={
              <Button variant="secondary" onClick={resetFilters}>
                {t("chapters.empty.reset_filter")}
              </Button>
            }
          />
        </div>
      ) : (
        <div
          ref={scrollContainerRef}
          className={styles.tableScroll}
          tabIndex={0}
          role="grid"
          aria-label={chapter?.title_src || t("chapters.title")}
        >
          <div
            style={{
              height: `${rowVirtualizer.getTotalSize()}px`,
              width: "100%",
              position: "relative",
            }}
          >
            {rowVirtualizer.getVirtualItems().map((virtualRow) => {
              const pair = filteredPairs[virtualRow.index];
              return (
                <div
                  key={pair.id}
                  ref={rowVirtualizer.measureElement}
                  data-index={virtualRow.index}
                  style={{
                    position: "absolute",
                    top: 0,
                    left: 0,
                    width: "100%",
                    transform: `translateY(${virtualRow.start}px)`,
                  }}
                >
                  <SegmentPairRow
                    pair={pair}
                    isSelected={selectedPairId === pair.id}
                    targetLanguage={targetLang}
                    onClick={() => selectPair(pair.id)}
                  />
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
};
