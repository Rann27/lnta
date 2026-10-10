import React from "react";
import { useTranslation } from "react-i18next";
import { ProgressBar } from "../../../components/ui/ProgressBar";
import { SegmentRuler } from "../../../components/ui/SegmentRuler";
import { WaitingCounter } from "./WaitingCounter";
import { useWorkspaceStore } from "../store";
import styles from "./ActiveChapterCard.module.css";

export interface ActiveChapterCardProps {
  onSelectSegment?: (segmentIndex: number) => void;
}

export const ActiveChapterCard: React.FC<ActiveChapterCardProps> = ({
  onSelectSegment,
}) => {
  const { t } = useTranslation();
  const {
    chapters,
    selectedChapterId,
    runState,
    chapterSegments,
    providerStatus,
    qaFindingsCount,
    setActiveTab,
  } = useWorkspaceStore();

  const state = runState.state;

  // Resolve current active chapter
  const activeChapterId = runState.current_chapter_id || selectedChapterId;
  const currentChapter =
    chapters.find((c) => c.id === activeChapterId) ?? chapters[0];

  const nextPendingChapter = chapters.find((c) => c.status === "pending" || c.status === "queued") ?? chapters[0];

  const segments = currentChapter ? (chapterSegments[currentChapter.id] ?? []) : [];
  const partsPlanned = currentChapter?.parts_planned || 2;
  const lineCount = currentChapter?.line_count || 1200;

  // 1. Idle state
  if (state === "idle" && !runState.latest_run) {
    const nextOrd = nextPendingChapter ? String(nextPendingChapter.ordinal).padStart(2, "0") : "01";
    const nextTitle = nextPendingChapter?.title_src || nextPendingChapter?.file_name || "Bab 1";
    const nextParts = nextPendingChapter?.parts_planned || 1;

    return (
      <section className={styles.card} aria-label="Active Chapter Card">
        <div className={styles.emptyState}>
          <h3 className={styles.emptyTitle}>
            {t("workspace.run_tab.no_run_yet", {
              ordinal: nextOrd,
              title: nextTitle,
              parts: nextParts,
            })}
          </h3>
          <p className={styles.emptyDesc}>
            {t("workspace.run_tab.start_helper")}
          </p>
        </div>
      </section>
    );
  }

  // 2. Planning state
  if (state === "planning") {
    const ordText = currentChapter ? String(currentChapter.ordinal).padStart(2, "0") : "01";
    const titleText = currentChapter?.title_src || currentChapter?.file_name || "Bab";

    return (
      <section className={styles.card} aria-label="Active Chapter Card">
        <div className={styles.emptyState}>
          <h3 className={styles.emptyTitle}>
            {t("workspace.run_tab.planning_for", {
              ordinal: ordText,
              title: titleText,
            })}
          </h3>
          <div style={{ marginTop: "var(--space-3)" }}>
            <ProgressBar label="Merencanakan" value={50} max={100} variant="live" />
          </div>
        </div>
      </section>
    );
  }

  // 3. Completed state (Volume finished / QA phase)
  if (state === "completed" || state === "qa_running") {
    return (
      <section className={styles.card} aria-label="Active Chapter Card">
        <div className={styles.head}>
          <h3 className={styles.title}>
            {state === "qa_running"
              ? t("workspace.run_tab.all_completed")
              : t("workspace.run_tab.all_completed_done")}
          </h3>
          <span className={styles.planMeta}>
            {t("workspace.run_tab.qa_summary", { findings: qaFindingsCount })}
          </span>
        </div>

        <div style={{ marginTop: "var(--space-2)" }}>
          <ProgressBar
            label="QA Progress"
            value={state === "completed" ? 100 : 65}
            max={100}
            variant={state === "qa_running" ? "live" : "default"}
          />
        </div>

        <div className={styles.callRow}>
          <span className={styles.callItem}>
            <span>Fase:</span>
            <b className={styles.callItemBold}>
              {state === "qa_running" ? "Pindai C (Hotspots)" : "Selesai"}
            </b>
          </span>
          <span className={styles.callItem}>
            <span>Model:</span>
            <b className={styles.callItemBold}>
              {providerStatus.name} · {providerStatus.model}
            </b>
          </span>
          {state === "qa_running" && (
            <WaitingCounter initialSeconds={14} tokensReceived={3400} isActive={true} />
          )}
        </div>
      </section>
    );
  }

  // 4, 5, 6: Translating / Validating / Paused state
  const isRunning = state === "running";
  const isPaused = state === "paused";
  const isValidating = state === "validating";
  const isRepairing = state === "repairing";

  const chapterTitle = currentChapter?.title_src || currentChapter?.file_name || "Bab 1";
  const ordinalStr = currentChapter ? String(currentChapter.ordinal).padStart(2, "0") : "01";

  const planSummaryText = t("workspace.run_tab.plan_summary", {
    parts: partsPlanned,
    lines: lineCount.toLocaleString(),
  });

  return (
    <section className={styles.card} aria-label="Active Chapter Card">
      {/* Header */}
      <div className={styles.head}>
        <h3 className={styles.title}>
          Bab {ordinalStr} · {chapterTitle}
        </h3>
        <span className={styles.planMeta}>{planSummaryText}</span>
      </div>

      {/* Segment Ruler */}
      <SegmentRuler
        variant="header"
        segments={segments}
        interactive
        onSelectSegment={(idx) => {
          onSelectSegment?.(idx);
          setActiveTab("chapters");
        }}
      />

      {/* Parts List */}
      <div className={styles.parts}>
        {Array.from({ length: partsPlanned }).map((_, index) => {
          const partNum = index + 1;
          const isCurrentPart = partNum === 1; // Simulated active part
          const isDonePart = partNum < 1;

          let progressVal = 0;
          let progressVariant: "default" | "live" | "paused" = "default";

          if (isDonePart) {
            progressVal = 100;
          } else if (isCurrentPart) {
            progressVal = 58;
            progressVariant = isPaused ? "paused" : "live";
          }

          const lineStart = Math.round((lineCount / partsPlanned) * index) + 1;
          const lineEnd = Math.round((lineCount / partsPlanned) * (index + 1));

          return (
            <div key={partNum} className={styles.partRow}>
              <span className={styles.partLabel}>
                {t("workspace.run_tab.part_label", { part: partNum })}
              </span>

              <ProgressBar
                label={`Bagian ${partNum}`}
                value={progressVal}
                max={100}
                variant={progressVariant}
              />

              <span className={styles.partLines}>
                {t("workspace.run_tab.line_range", { from: lineStart, to: lineEnd })}
              </span>
            </div>
          );
        })}
      </div>

      {/* Active Call Row */}
      <div className={styles.callRow}>
        <span className={styles.callItem}>
          <span>Peran:</span>
          <b className={styles.callItemBold}>{t("workspace.run_tab.translator_role")}</b>
        </span>

        <span className={styles.callItem}>
          <span>Model:</span>
          <b className={styles.callItemBold}>
            {providerStatus.name} · {providerStatus.model}
          </b>
        </span>

        {isValidating && (
          <span className={`${styles.stateTag} ${styles.stateTagWarning}`}>
            {t("workspace.run_tab.validating_label")}
          </span>
        )}

        {isRepairing && (
          <span className={`${styles.stateTag} ${styles.stateTagWarning}`}>
            {t("workspace.run_tab.repairing_label", { count: 3 })}
          </span>
        )}

        {isPaused && (
          <span className={`${styles.stateTag} ${styles.stateTagPaused}`}>
            {t("workspace.run_tab.paused_label")}
          </span>
        )}

        {isRunning && (
          <WaitingCounter
            initialSeconds={18}
            tokensReceived={2410}
            isActive={true}
          />
        )}
      </div>
    </section>
  );
};
