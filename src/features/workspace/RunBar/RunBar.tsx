import React from "react";
import { useTranslation } from "react-i18next";
import { TallyLamp } from "../../../components/ui/TallyLamp";
import { Tooltip } from "../../../components/ui/Tooltip";
import { formatNumber } from "../../../lib/format";
import { RunControl } from "./RunControl";
import { useWorkspaceStore } from "../store";
import styles from "./RunBar.module.css";

export const RunBar: React.FC = () => {
  const { t } = useTranslation();
  const {
    projectPath,
    projectConfig,
    chapters,
    chapterSegments,
    runState,
    activePart,
    inputTokens,
    outputTokens,
  } = useWorkspaceStore();

  const isLive = runState.state === "running" || runState.state === "qa_running";
  const projectName = projectPath
    ? projectPath.split(/[/\\]/).filter(Boolean).pop() ?? "LNTA"
    : "LNTA";
  const profileLabel = projectConfig?.profile === "ja-en" ? "JP→EN" : "JP→ID";

  // Position is only shown for a chapter the engine actually reported; nothing is guessed.
  const currentChapterId = runState.current_chapter_id;
  const currentIndex = chapters.findIndex((c) => c.id === currentChapterId);
  const positionParts: string[] = [];
  if (currentIndex >= 0 && runState.state !== "idle") {
    positionParts.push(
      t("workspace.position_chapter", {
        currentChap: String(currentIndex + 1).padStart(2, "0"),
        totalChap: String(chapters.length).padStart(2, "0"),
      })
    );
    if (activePart && activePart.chapterId === currentChapterId && activePart.total > 1) {
      positionParts.push(
        t("workspace.position_part", {
          currentPart: activePart.current,
          totalPart: activePart.total,
        })
      );
    }
    const cells = (chapterSegments[currentChapterId ?? ""] ?? []).filter((s) => !s.isFixed);
    if (cells.length > 0) {
      positionParts.push(
        t("workspace.position_segments", {
          done: cells.filter((s) => s.status === "done").length,
          total: cells.length,
        })
      );
    }
  }
  const positionSummaryText = positionParts.join(" · ");
  const hasUsage = (inputTokens ?? 0) > 0 || (outputTokens ?? 0) > 0;

  return (
    <header className={styles.runBar} role="banner">
      <div className={styles.left}>
        <strong className={styles.projectName}>{projectName}</strong>
        <span className={styles.profileBadge} title={profileLabel}>
          {profileLabel}
        </span>
        <Tooltip content={t("workspace.folder_path_tooltip", { path: projectPath })}>
          <span className={styles.folderPath} tabIndex={0}>
            {projectPath}
          </span>
        </Tooltip>
      </div>

      <div className={styles.center} aria-live="polite">
        {isLive && <TallyLamp />}
        <span className={styles.statusLabel}>
          {t(`status.${runState.state}`, runState.state)}
        </span>
        {positionSummaryText && (
          <Tooltip content={positionSummaryText}>
            <span className={styles.positionSummary} tabIndex={0}>
              {positionSummaryText}
            </span>
          </Tooltip>
        )}
      </div>

      <div className={styles.right}>
        {hasUsage && (
          <Tooltip content={t("workspace.tokens_tooltip")}>
            <span className={styles.counter} tabIndex={0}>
              {t("workspace.tokens_in")}{" "}
              <b className={styles.counterBold}>{formatNumber(inputTokens ?? 0)}</b> ·{" "}
              {t("workspace.tokens_out")}{" "}
              <b className={styles.counterBold}>{formatNumber(outputTokens ?? 0)}</b>
            </span>
          </Tooltip>
        )}

        <RunControl />
      </div>
    </header>
  );
};
