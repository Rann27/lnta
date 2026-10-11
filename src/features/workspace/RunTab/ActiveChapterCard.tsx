import React from "react";
import { useTranslation } from "react-i18next";
import { ProgressBar } from "../../../components/ui/ProgressBar";
import { SegmentRuler } from "../../../components/ui/SegmentRuler";
import { formatNumber } from "../../../lib/format";
import { WaitingCounter } from "./WaitingCounter";
import { useWorkspaceStore } from "../store";
import styles from "./ActiveChapterCard.module.css";

export interface ActiveChapterCardProps {
  onSelectSegment?: (segmentIndex: number) => void;
}

const QA_PHASES = ["scan", "mechanical", "ai_fix", "review", "done"];

export const ActiveChapterCard: React.FC<ActiveChapterCardProps> = ({
  onSelectSegment,
}) => {
  const { t } = useTranslation();
  const {
    chapters,
    chapterPhase,
    chapterSegments,
    chapterParts,
    imagePageIds,
    runState,
    activePart,
    streamChars,
    callStartedAt,
    providerStatus,
    qaFindingsCount,
    qaPhase,
    setActiveTab,
  } = useWorkspaceStore();

  const state = runState.state;
  const ordinalOf = (ordinal: number) => String(ordinal).padStart(2, "0");

  // QA phase or finished volume
  if (state === "qa_running" || state === "completed") {
    const phaseIndex = qaPhase ? QA_PHASES.indexOf(qaPhase) + 1 : 0;
    return (
      <section className={styles.card} aria-label={t("workspace.tabs.run")}>
        <div className={styles.head}>
          <h3 className={styles.title}>
            {state === "qa_running"
              ? t("workspace.run_tab.all_completed")
              : t("workspace.run_tab.all_completed_done")}
          </h3>
          {qaFindingsCount !== null && (
            <span className={styles.planMeta}>
              {t("workspace.run_tab.qa_summary", { findings: qaFindingsCount })}
            </span>
          )}
        </div>
        {state === "qa_running" && phaseIndex > 0 && (
          <>
            <div className={styles.parts}>
              <ProgressBar
                label={t("workspace.run_tab.phase_label")}
                value={phaseIndex}
                max={QA_PHASES.length}
                variant="live"
              />
            </div>
            <div className={styles.callRow}>
              <span className={styles.callItem}>
                <span>{t("workspace.run_tab.phase_label")}:</span>
                <b className={styles.callItemBold}>{t(`workspace.qa_phase.${qaPhase}`)}</b>
              </span>
            </div>
          </>
        )}
      </section>
    );
  }

  const currentChapter = chapters.find((c) => c.id === runState.current_chapter_id);
  const isLive = state === "running";
  const isPaused = state === "paused" || state === "pausing";

  // Nothing in progress: point at the next chapter that still needs work.
  if (!currentChapter || (!isLive && !isPaused)) {
    const next = chapters.find((c) => c.status !== "done" && !c.frozen);
    if (!next) {
      return (
        <section className={styles.card} aria-label={t("workspace.tabs.run")}>
          <div className={styles.emptyState}>
            <h3 className={styles.emptyTitle}>{t("workspace.run_tab.all_completed_done")}</h3>
          </div>
        </section>
      );
    }
    const nextParts = chapterParts[next.id]?.length || 1;
    return (
      <section className={styles.card} aria-label={t("workspace.tabs.run")}>
        <div className={styles.emptyState}>
          <h3 className={styles.emptyTitle}>
            {imagePageIds[next.id]
              ? t("workspace.run_tab.image_pages_card", { ordinal: ordinalOf(next.ordinal) })
              : t("workspace.run_tab.no_run_yet", {
                  ordinal: ordinalOf(next.ordinal),
                  title: next.title_src || next.file_name,
                  parts: nextParts,
                })}
          </h3>
          <p className={styles.emptyDesc}>{t("workspace.run_tab.start_helper")}</p>
        </div>
      </section>
    );
  }

  const segments = chapterSegments[currentChapter.id] ?? [];
  const parts = chapterParts[currentChapter.id] ?? [];
  const phase = chapterPhase[currentChapter.id] ?? "";
  const currentPart =
    activePart && activePart.chapterId === currentChapter.id ? activePart.current : null;

  return (
    <section className={styles.card} aria-label={t("workspace.tabs.run")}>
      <div className={styles.head}>
        <h3 className={styles.title}>
          {ordinalOf(currentChapter.ordinal)} · {currentChapter.title_src || currentChapter.file_name}
        </h3>
        {parts.length > 0 && (
          <span className={styles.planMeta}>
            {t("workspace.run_tab.plan_summary", {
              parts: parts.length,
              lines: formatNumber(currentChapter.line_count),
            })}
          </span>
        )}
      </div>

      {segments.length > 0 && (
        <SegmentRuler
          variant="header"
          segments={segments}
          interactive
          onSelectSegment={(idx) => {
            onSelectSegment?.(idx);
            setActiveTab("chapters");
          }}
        />
      )}

      {parts.length === 0 ? (
        <p className={styles.emptyDesc}>{t("workspace.run_tab.plan_pending")}</p>
      ) : (
        <div className={styles.parts}>
          {parts.map((part) => {
            const cells = segments.filter(
              (s) => !s.isFixed && s.idx >= part.seg_from && s.idx <= part.seg_to
            );
            const done = cells.filter((s) => s.status === "done").length;
            const isCurrent = part.part_index === currentPart;
            const variant = isCurrent ? (isPaused ? "paused" : "live") : "default";
            return (
              <div key={part.id} className={styles.partRow}>
                <span className={styles.partLabel}>
                  {t("workspace.run_tab.part_label", { part: part.part_index })}
                </span>
                <ProgressBar
                  label={t("workspace.run_tab.part_progress", { done, total: cells.length })}
                  value={done}
                  max={Math.max(cells.length, 1)}
                  variant={variant}
                />
                <span className={styles.partLines}>
                  {t("workspace.run_tab.line_range", {
                    from: part.line_from,
                    to: part.line_to,
                  })}
                </span>
              </div>
            );
          })}
        </div>
      )}

      <div className={styles.callRow}>
        <span className={styles.callItem}>
          <span>{t("workspace.run_tab.role_label")}:</span>
          <b className={styles.callItemBold}>{t("workspace.run_tab.translator_role")}</b>
        </span>
        {providerStatus.model && (
          <span className={styles.callItem}>
            <span>{t("workspace.run_tab.model_label")}:</span>
            <b className={styles.callItemBold}>
              {providerStatus.name} · {providerStatus.model}
            </b>
          </span>
        )}
        {phase === "validating" && (
          <span className={`${styles.stateTag} ${styles.stateTagWarning}`}>
            {t("workspace.run_tab.validating_label")}
          </span>
        )}
        {phase === "repairing" && (
          <span className={`${styles.stateTag} ${styles.stateTagWarning}`}>
            {t("workspace.run_tab.repairing_label_plain")}
          </span>
        )}
        {isPaused && (
          <span className={`${styles.stateTag} ${styles.stateTagPaused}`}>
            {t("workspace.run_tab.paused_label")}
          </span>
        )}
        {isLive && phase.startsWith("translating") && (
          <WaitingCounter startedAt={callStartedAt} charsReceived={streamChars} isActive />
        )}
      </div>
    </section>
  );
};
