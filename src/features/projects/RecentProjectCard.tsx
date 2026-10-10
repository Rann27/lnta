import React from "react";
import { Folder } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import type { RecentProjectEntry } from "../../lib/bindings";
import { ProgressBar } from "../../components/ui/ProgressBar";
import { StatusTag } from "../../components/ui/StatusTag";
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

  const isCompleted =
    project.chapters_total > 0 && project.chapters_done >= project.chapters_total;

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
        <StatusTag
          status={isCompleted ? "done" : "running"}
          label={project.profile}
        />
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
          <span>
            {t("projects.last_opened", {
              time: formatDateTime(project.last_opened),
            })}
          </span>
        </div>
      </div>
    </div>
  );
};
