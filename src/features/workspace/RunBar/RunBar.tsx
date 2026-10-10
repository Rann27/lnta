import React from "react";
import { useTranslation } from "react-i18next";
import { TallyLamp } from "../../../components/ui/TallyLamp";
import { Tooltip } from "../../../components/ui/Tooltip";
import { RunControl } from "./RunControl";
import { useWorkspaceStore } from "../store";
import styles from "./RunBar.module.css";

export const RunBar: React.FC = () => {
  const { t } = useTranslation();
  const {
    projectPath,
    projectConfig,
    chapters,
    selectedChapterId,
    runState,
    inputTokens,
    outputTokens,
  } = useWorkspaceStore();

  const isLive = runState.state === "running" || runState.state === "qa_running";
  const projectName = projectPath
    ? projectPath.split(/[/\\]/).filter(Boolean).pop() ?? "LNTA"
    : "LNTA";
  const profileLabel = projectConfig?.profile === "ja-en" ? "JP→EN" : "JP→ID";

  const totalChapters = chapters.length;
  const currentChapterIndex =
    chapters.findIndex((c) => c.id === (runState.current_chapter_id || selectedChapterId)) + 1;
  const chapIndexText = currentChapterIndex > 0 ? String(currentChapterIndex).padStart(2, "0") : "01";
  const totalChaptersText = String(totalChapters || 1).padStart(2, "0");

  const positionSummaryText = t("workspace.position_summary", {
    currentChap: chapIndexText,
    totalChap: totalChaptersText,
    currentPart: "1",
    totalPart: "2",
    currentSeg: "41",
    totalSeg: "88",
  });

  const formattedInput = inputTokens.toLocaleString();
  const formattedOutput = outputTokens.toLocaleString();

  return (
    <header className={styles.runBar} role="banner">
      {/* Left zone: max 360px */}
      <div className={styles.left}>
        <strong className={styles.projectName}>{projectName}</strong>
        <span className={styles.profileBadge} title={`Profil: ${profileLabel}`}>
          {profileLabel}
        </span>
        <Tooltip content={t("workspace.folder_path_tooltip", { path: projectPath })}>
          <span className={styles.folderPath} tabIndex={0}>
            {projectPath}
          </span>
        </Tooltip>
      </div>

      {/* Center zone: flexible, truncated */}
      <div className={styles.center} aria-live="polite">
        {isLive && <TallyLamp />}
        <span className={styles.statusLabel}>
          {t(`status.${runState.state}`, runState.state)}
        </span>
        <Tooltip content={positionSummaryText}>
          <span className={styles.positionSummary} tabIndex={0}>
            {positionSummaryText}
          </span>
        </Tooltip>
      </div>

      {/* Right zone: tokens counter and RunControl */}
      <div className={styles.right}>
        <Tooltip content={t("workspace.tokens_tooltip")}>
          <span className={styles.counter} tabIndex={0}>
            Token masuk <b className={styles.counterBold}>{formattedInput}</b> · keluar{" "}
            <b className={styles.counterBold}>{formattedOutput}</b>
          </span>
        </Tooltip>

        <RunControl />
      </div>
    </header>
  );
};
