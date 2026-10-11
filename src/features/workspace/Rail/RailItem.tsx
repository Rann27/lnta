import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { StatusTag, type StatusType } from "../../../components/ui/StatusTag";
import { SegmentRuler, type SegmentItem } from "../../../components/ui/SegmentRuler";
import { Popover } from "../../../components/ui/Popover";
import type { Chapter } from "../../../lib/bindings";
import { ipc } from "../../../lib/ipc";
import styles from "./RailItem.module.css";

export interface RailItemProps {
  chapter: Chapter;
  segments?: SegmentItem[];
  /** A chapter with only image/scene segments and no heading (FR-RAW-08). */
  isImagePage?: boolean;
  projectPath?: string;
  isSelected: boolean;
  isCollapsed: boolean;
  onSelect: (chapterId: string) => void;
  onRetranslate?: (chapterId: string) => void;
}

export const RailItem: React.FC<RailItemProps> = ({
  chapter,
  segments = [],
  isImagePage = false,
  projectPath = "",
  isSelected,
  isCollapsed,
  onSelect,
  onRetranslate,
}) => {
  const { t } = useTranslation();
  const [menuOpen, setMenuOpen] = useState(false);

  const rawFilePath = `${projectPath.replace(/[\\/]+$/, "")}/${chapter.file_name}`;

  // Ordinal formatting: 2 digits (e.g. 01, 02)
  const ordinalFormatted = String(chapter.ordinal).padStart(2, "0");

  // Map chapter status to StatusTag StatusType
  const statusType: StatusType =
    chapter.frozen
      ? "frozen"
      : chapter.status === "running"
      ? "running"
      : chapter.status === "paused"
      ? "paused"
      : chapter.status === "done"
      ? "done"
      : chapter.status === "needs_attention"
      ? "needs_attention"
      : chapter.status === "blocked"
      ? "blocked"
      : "queued";

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    setMenuOpen(true);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.shiftKey && e.code === "F10") {
      e.preventDefault();
      setMenuOpen(true);
    }
  };

  if (isCollapsed) {
    return (
      <button
        type="button"
        className={[
          styles.item,
          styles.itemCollapsed,
          isSelected ? styles.selected : "",
        ]
          .filter(Boolean)
          .join(" ")}
        onClick={() => onSelect(chapter.id)}
        onContextMenu={handleContextMenu}
        title={`${ordinalFormatted} · ${chapter.file_name} (${statusType})`}
        aria-current={isSelected ? "true" : undefined}
      >
        <span className={styles.ordinal}>{ordinalFormatted}</span>
        <span
          className={styles.itemCollapsedDot}
          data-status={statusType}
          style={{
            background:
              statusType === "running"
                ? "var(--color-tally)"
                : statusType === "done"
                ? "var(--color-text)"
                : statusType === "needs_attention"
                ? "var(--color-warn)"
                : statusType === "blocked"
                ? "var(--color-danger)"
                : "var(--color-border-strong)",
          }}
        />
      </button>
    );
  }

  return (
    <Popover
      isOpen={menuOpen}
      onOpenChange={setMenuOpen}
      trigger={
        <button
          type="button"
          className={[
            styles.item,
            isSelected ? styles.selected : "",
          ]
            .filter(Boolean)
            .join(" ")}
          onClick={() => onSelect(chapter.id)}
          onContextMenu={handleContextMenu}
          onKeyDown={handleKeyDown}
          aria-current={isSelected ? "true" : undefined}
        >
          {/* Row 1, Col 1 */}
          <span className={styles.ordinal}>{ordinalFormatted}</span>

          {/* Row 1, Col 2 */}
          <span className={styles.fileName} title={chapter.file_name}>
            {chapter.title_src || chapter.file_name}
          </span>

          {/* Row 1, Col 3 */}
          <span className={styles.status}>
            <StatusTag status={statusType} />
          </span>

          {/* Row 2, Col 2-3 */}
          <div className={styles.track}>
            {isImagePage ? (
              <span className={styles.imagePagesLabel}>
                {t("workspace.rail.image_pages_label", "halaman gambar")}
              </span>
            ) : chapter.frozen ? (
              <div className={styles.frozenBar} title={t("status.frozen", "beku")} />
            ) : (
              <SegmentRuler
                variant="rail"
                segments={segments}
              />
            )}
          </div>
        </button>
      }
      content={
        <div
          role="menu"
          style={{
            display: "grid",
            gap: "var(--space-1)",
            padding: "var(--space-2)",
            background: "var(--color-raised)",
            borderRadius: "var(--radius-2)",
            boxShadow: "var(--shadow-1)",
            border: "var(--hairline) solid var(--color-border)",
            minWidth: "12rem",
          }}
        >
          <button
            type="button"
            role="menuitem"
            style={{
              display: "flex",
              alignItems: "center",
              height: "var(--control-md)",
              padding: "0 var(--space-4)",
              background: "transparent",
              border: 0,
              borderRadius: "var(--radius-1)",
              color: "var(--color-text)",
              font: "var(--text-body)",
              textAlign: "left",
              cursor: "pointer",
            }}
            onClick={() => {
              setMenuOpen(false);
              onRetranslate?.(chapter.id);
            }}
          >
            {t("workspace.rail.retranslate_chapter")}
          </button>
          <div style={{ height: "1px", background: "var(--color-border)", margin: "var(--space-1) 0" }} />
          <button
            type="button"
            role="menuitem"
            style={{
              display: "flex",
              alignItems: "center",
              height: "var(--control-md)",
              padding: "0 var(--space-4)",
              background: "transparent",
              border: 0,
              borderRadius: "var(--radius-1)",
              color: "var(--color-text)",
              font: "var(--text-body)",
              textAlign: "left",
              cursor: "pointer",
            }}
            onClick={() => {
              setMenuOpen(false);
              void ipc.openPath(rawFilePath);
            }}
          >
            {t("workspace.rail.open_raw_file")}
          </button>
          <button
            type="button"
            role="menuitem"
            style={{
              display: "flex",
              alignItems: "center",
              height: "var(--control-md)",
              padding: "0 var(--space-4)",
              background: "transparent",
              border: 0,
              borderRadius: "var(--radius-1)",
              color: "var(--color-text)",
              font: "var(--text-body)",
              textAlign: "left",
              cursor: "pointer",
            }}
            onClick={() => {
              setMenuOpen(false);
              void navigator.clipboard?.writeText(rawFilePath);
            }}
          >
            {t("workspace.rail.copy_path")}
          </button>
        </div>
      }
    />
  );
};
