import React from "react";
import { Folder } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import type { RecentProjectEntry } from "../../lib/bindings";
import { ProgressBar } from "../../components/ui/ProgressBar";
import { formatDateTime } from "../../lib/format";
import styles from "./RecentProjectCard.module.css";

export interface RecentProjectCardProps {
  project: RecentProjectEntry;
  onSelect: (path: string) => void;
}

export const RecentProjectCard: React.FC<RecentProjectCardProps> = ({
  project,
  onSelect,
}) => {
  const { t } = useTranslation();

  const handleKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      onSelect(project.path);
    }
  };

  return (
    <div
      role="button"
      tabIndex={0}
      className={styles.card}
      onClick={() => onSelect(project.path)}
      onKeyDown={handleKeyDown}
      aria-label={`${project.name}, ${project.chapters_done}/${project.chapters_total}`}
    >
      <div className={styles.header}>
        <div className={styles.titleArea}>
          <Folder size={18} aria-hidden="true" />
          <h3 className={styles.name}>{project.name}</h3>
        </div>
        {/* A profile badge, not a status: nothing is running on the home screen. */}
        <span className={styles.profileBadge}>
          {project.profile === "ja-en" ? "JP→EN" : "JP→ID"}
        </span>
      </div>

      <div className={styles.path} title={project.path}>
        {project.path}
      </div>

      <div className={styles.meta}>
        <ProgressBar
          value={project.chapters_done}
          max={project.chapters_total}
          label={t("projects.chapters_progress", {
            done: project.chapters_done,
            total: project.chapters_total,
          })}
        />
        <div className={styles.stats}>
          <span>
            {t("projects.chapters_progress", {
              done: project.chapters_done,
              total: project.chapters_total,
            })}
          </span>
          {project.last_opened && (
            <span>
              {t("projects.last_opened", {
                time: formatDateTime(project.last_opened),
              })}
            </span>
          )}
        </div>
      </div>
    </div>
  );
};
