import React, { useState, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { SidebarSimple } from "@phosphor-icons/react";
import { IconButton } from "../../../components/ui/IconButton";
import { RailItem } from "./RailItem";
import { useWorkspaceStore } from "../store";
import styles from "./ChapterRail.module.css";

export const ChapterRail: React.FC = () => {
  const { t } = useTranslation();
  const {
    chapters,
    selectedChapterId,
    chapterSegments,
    imagePageIds,
    projectPath,
    railWidth,
    railCollapsed,
    selectChapter,
    setRailWidth,
    toggleRailCollapsed,
    translateChapterOnly,
  } = useWorkspaceStore();

  const [isResizing, setIsResizing] = useState(false);
  const startXRef = useRef(0);
  const startWidthRef = useRef(railWidth);

  const handleMouseDown = (e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
    startXRef.current = e.clientX;
    startWidthRef.current = railWidth;
  };

  useEffect(() => {
    if (!isResizing) return;

    const handleMouseMove = (e: MouseEvent) => {
      const delta = e.clientX - startXRef.current;
      const newWidth = startWidthRef.current + delta;
      setRailWidth(newWidth);
    };

    const handleMouseUp = () => {
      setIsResizing(false);
    };

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [isResizing, setRailWidth]);

  const effectiveWidth = railCollapsed ? 56 : railWidth;

  return (
    <aside
      className={styles.rail}
      style={{ width: `${effectiveWidth}px` }}
      aria-label={t("workspace.panes.rail")}
      tabIndex={-1}
    >
      <div className={styles.header}>
        {!railCollapsed && (
          <h2 className={styles.headerTitle}>
            {t("workspace.rail.chapters_header", { count: chapters.length })}
          </h2>
        )}
        <IconButton
          aria-label={
            railCollapsed
              ? t("workspace.rail.expand_rail")
              : t("workspace.rail.collapse_rail")
          }
          icon={<SidebarSimple size={16} />}
          onClick={toggleRailCollapsed}
        />
      </div>

      <ul className={styles.list} role="listbox" aria-label={t("workspace.panes.rail")}>
        {chapters.map((ch) => (
          <li key={ch.id} role="option" aria-selected={ch.id === selectedChapterId}>
            <RailItem
              chapter={ch}
              segments={chapterSegments[ch.id]}
              isImagePage={imagePageIds[ch.id] ?? false}
              projectPath={projectPath}
              isSelected={ch.id === selectedChapterId}
              isCollapsed={railCollapsed}
              onSelect={selectChapter}
              onRetranslate={(id) => void translateChapterOnly(id)}
            />
          </li>
        ))}
      </ul>

      {!railCollapsed && (
        <div
          className={[styles.resizer, isResizing ? styles.resizing : ""].filter(Boolean).join(" ")}
          onMouseDown={handleMouseDown}
          role="separator"
          aria-orientation="vertical"
          aria-valuenow={railWidth}
        />
      )}
    </aside>
  );
};
