import React from "react";
import { Warning } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { isTauri } from "@tauri-apps/api/core";
import styles from "./PreviewBanner.module.css";

const SCENARIOS = [
  { id: "run-in-progress", labelKey: "preview.scenarios.run_in_progress" },
  { id: "no-projects", labelKey: "preview.scenarios.no_projects" },
  { id: "project-idle-blocker", labelKey: "preview.scenarios.project_idle_blocker" },
  { id: "chapter-needs-attention", labelKey: "preview.scenarios.chapter_needs_attention" },
  { id: "run-paused-blocker", labelKey: "preview.scenarios.run_paused_blocker" },
  { id: "qa-finished", labelKey: "preview.scenarios.qa_finished" },
] as const;

export function PreviewBanner(): React.ReactElement | null {
  const { t } = useTranslation();

  if (!import.meta.env.DEV || (typeof window !== "undefined" && isTauri())) {
    return null;
  }

  const currentScenario =
    typeof window !== "undefined"
      ? (new URLSearchParams(window.location.search).get("scenario") ?? "run-in-progress")
      : "run-in-progress";

  const handleScenarioChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const nextScenario = e.target.value;
    const url = new URL(window.location.href);
    url.searchParams.set("scenario", nextScenario);
    window.location.href = url.toString();
  };

  return (
    <div className={styles.previewBanner} role="status">
      <div className={styles.content}>
        <span className={styles.icon} aria-hidden="true">
          <Warning size={20} />
        </span>
        <span className={styles.title}>{t("common.mock_data_banner")}</span>
      </div>
      <div className={styles.controls}>
        <label htmlFor="preview-scenario-select" className={styles.title}>
          {t("preview.switch_scenario")}
        </label>
        <select
          id="preview-scenario-select"
          className={styles.select}
          value={currentScenario}
          onChange={handleScenarioChange}
        >
          {SCENARIOS.map((sc) => (
            <option key={sc.id} value={sc.id}>
              {t(sc.labelKey)}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
}
