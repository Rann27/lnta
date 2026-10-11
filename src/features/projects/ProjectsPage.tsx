import React, { useEffect, useState } from "react";
import { FolderOpen } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "../../components/ui/Button";
import { StateBlock } from "../../components/ui/StateBlock";
import { RecentProjectCard } from "./RecentProjectCard";
import { useProjectsStore } from "./store";
import styles from "./ProjectsPage.module.css";

export const ProjectsPage: React.FC = () => {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { recentProjects, isLoading, loadRecentProjects, openProject } =
    useProjectsStore();
  const [isDragging, setIsDragging] = useState(false);

  useEffect(() => {
    loadRecentProjects();
  }, [loadRecentProjects]);

  const handleOpenFolder = async () => {
    try {
      if (typeof window !== "undefined" && isTauri()) {
        const selected = await open({
          directory: true,
          multiple: false,
          title: t("actions.open_folder"),
        });
        if (selected && typeof selected === "string") {
          await openProject(selected);
          navigate("/workspace");
        }
      } else {
        // Fallback for browser preview
        const fallbackPath =
          recentProjects[0]?.path ?? "D:/LN/Synthetic-Volume-1";
        await openProject(fallbackPath);
        navigate("/workspace");
      }
    } catch {
      // dialog cancelled or open error
    }
  };

  const handleSelectProject = async (path: string) => {
    try {
      await openProject(path);
      navigate("/workspace");
    } catch {
      // handle error
    }
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(true);
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);
  };

  const handleDrop = async (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);

    let droppedPath: string | null = null;
    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      const file = e.dataTransfer.files[0] as unknown as { path?: string };
      droppedPath = file.path ?? null;
    }
    if (!droppedPath) {
      const text = e.dataTransfer.getData("text/plain");
      if (text && text.trim().length > 0) {
        droppedPath = text.trim();
      }
    }

    if (droppedPath) {
      await handleSelectProject(droppedPath);
    }
  };

  return (
    <div
      className={styles.container}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      {isDragging && (
        <div className={styles.dropOverlay}>
          <div className={styles.dropMessage}>{t("projects.drop_hint")}</div>
        </div>
      )}

      <header className={styles.hero}>
        <h1 className={styles.title}>{t("app.title")}</h1>
        <p className={styles.subtitle}>{t("projects.subtitle")}</p>
      </header>

      {/* One primary action per screen: the empty state carries its own button. */}
      {(isLoading || recentProjects.length > 0) && (
        <div className={styles.actionsBar}>
          <Button
            variant="primary"
            icon={<FolderOpen size={16} />}
            onClick={handleOpenFolder}
          >
            {t("actions.open_folder")}
          </Button>
        </div>
      )}

      <section className={styles.section}>
        <div className={styles.sectionHeader}>
          <h2 className={styles.sectionTitle}>{t("projects.recent_title")}</h2>
        </div>

        {isLoading ? (
          <StateBlock title={t("projects.loading")} />
        ) : recentProjects.length === 0 ? (
          <StateBlock
            title={t("projects.empty_title")}
            description={t("projects.empty_desc")}
            action={
              <Button
                variant="primary"
                icon={<FolderOpen size={16} />}
                onClick={handleOpenFolder}
              >
                {t("actions.open_folder")}
              </Button>
            }
          />
        ) : (
          <div className={styles.grid}>
            {recentProjects.map((p) => (
              <RecentProjectCard
                key={p.path}
                project={p}
                onSelect={handleSelectProject}
              />
            ))}
          </div>
        )}
      </section>
    </div>
  );
};
