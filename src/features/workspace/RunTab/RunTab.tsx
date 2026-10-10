import React, { useState, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { RunTabBanners } from "./RunTabBanners";
import { ActiveChapterCard } from "./ActiveChapterCard";
import { useWorkspaceStore } from "../store";
import styles from "./RunTab.module.css";

export const RunTab: React.FC = () => {
  const { t } = useTranslation();
  const { runState } = useWorkspaceStore();
  const [announcement, setAnnouncement] = useState("");
  const prevStateRef = useRef(runState.state);

  // Announce only when runState.state changes
  useEffect(() => {
    if (runState.state !== prevStateRef.current) {
      prevStateRef.current = runState.state;
      let text: string;
      switch (runState.state) {
        case "running":
          text = t("workspace.run_tab.state_announcement_running");
          break;
        case "paused":
          text = t("workspace.run_tab.state_announcement_paused");
          break;
        case "completed":
          text = t("workspace.run_tab.state_announcement_completed");
          break;
        case "idle":
          text = t("workspace.run_tab.state_announcement_idle");
          break;
        default:
          text = t(`status.${runState.state}`, runState.state);
      }
      setAnnouncement(text);
    }
  }, [runState.state, t]);

  return (
    <div className={styles.runTab}>
      {/* Live region: announces run state changes only */}
      <div
        className={styles.srOnly}
        role="status"
        aria-label="Pengumuman status run"
        aria-live="polite"
        aria-atomic="true"
      >
        {announcement}
      </div>

      {/* Blocker and attention banners */}
      <RunTabBanners />

      {/* Active Chapter Card */}
      <ActiveChapterCard />
    </div>
  );
};
