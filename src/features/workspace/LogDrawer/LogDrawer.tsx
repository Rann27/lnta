import React, { useState, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import {
  CaretUp,
  CaretDown,
  Copy,
} from "@phosphor-icons/react";
import { Button } from "../../../components/ui/Button";
import { IconButton } from "../../../components/ui/IconButton";
import { LogLine } from "./LogLine";
import { useWorkspaceStore } from "../store";
import styles from "./LogDrawer.module.css";

export interface LogDrawerProps {
  isPanel?: boolean;
}

export const LogDrawer: React.FC<LogDrawerProps> = ({ isPanel = false }) => {
  const { t } = useTranslation();
  const {
    logs,
    logDrawerOpen,
    logDrawerHeight,
    logFilter,
    selectedChapterId,
    chapters,
    toggleLogDrawer,
    setLogDrawerHeight,
    setLogFilter,
  } = useWorkspaceStore();

  const [isResizing, setIsResizing] = useState(false);
  const startYRef = useRef(0);
  const startHeightRef = useRef(logDrawerHeight);
  const listRef = useRef<HTMLDivElement>(null);
  const userScrolledUpRef = useRef(false);

  // Resize handling for bottom drawer
  const handleMouseDown = (e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
    startYRef.current = e.clientY;
    startHeightRef.current = logDrawerHeight;
  };

  useEffect(() => {
    if (!isResizing) return;

    const handleMouseMove = (e: MouseEvent) => {
      const delta = startYRef.current - e.clientY;
      const newHeight = startHeightRef.current + delta;
      setLogDrawerHeight(newHeight);
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
  }, [isResizing, setLogDrawerHeight]);

  // Filter logs
  const selectedChapter = chapters.find((c) => c.id === selectedChapterId);
  const selectedScope = selectedChapter
    ? `ch${String(selectedChapter.ordinal).padStart(2, "0")}`
    : "";

  const filteredLogs = logs.filter((entry) => {
    if (logFilter === "warn_error") {
      return entry.level === "warning" || entry.level === "error";
    }
    if (logFilter === "selected_chapter") {
      return !selectedScope || entry.scope === selectedScope || entry.scope === "run";
    }
    return true;
  });

  // Auto-scroll to bottom
  useEffect(() => {
    if (listRef.current && !userScrolledUpRef.current) {
      listRef.current.scrollTop = listRef.current.scrollHeight;
    }
  }, [filteredLogs]);

  const handleScroll = () => {
    if (!listRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = listRef.current;
    userScrolledUpRef.current = scrollHeight - (scrollTop + clientHeight) > 30;
  };

  const handleCopy = () => {
    const text = filteredLogs
      .map((l) => `[${l.timestamp}] [${l.scope}] [${l.level.toUpperCase()}] ${l.message}`)
      .join("\n");
    navigator.clipboard?.writeText(text);
  };

  return (
    <section
      className={[
        styles.container,
        isPanel ? styles.panel : styles.drawer,
      ].join(" ")}
      style={!isPanel && logDrawerOpen ? { height: `${logDrawerHeight}px` } : undefined}
      aria-label={t("workspace.panes.log")}
      tabIndex={-1}
    >
      {!isPanel && logDrawerOpen && (
        <div
          className={[styles.resizerHorizontal, isResizing ? styles.resizing : ""].filter(Boolean).join(" ")}
          onMouseDown={handleMouseDown}
          role="separator"
          aria-orientation="horizontal"
          aria-valuenow={logDrawerHeight}
        />
      )}

      {/* Header */}
      <div className={styles.header}>
        <div style={{ display: "flex", alignItems: "center", gap: "var(--space-3)" }}>
          <h2 className={styles.headerTitle}>{t("workspace.log.title")}</h2>
          <span style={{ font: "var(--text-mono)", color: "var(--color-text-3)", fontSize: "var(--text-small)" }}>
            ({filteredLogs.length})
          </span>
        </div>

        <div className={styles.headerActions}>
          <select
            className={styles.filterSelect}
            value={logFilter}
            onChange={(e) =>
              setLogFilter(e.target.value as "all" | "warn_error" | "selected_chapter")
            }
            aria-label="Filter log"
          >
            <option value="all">{t("workspace.log.filter_all")}</option>
            <option value="warn_error">{t("workspace.log.filter_warn_error")}</option>
            <option value="selected_chapter">{t("workspace.log.filter_selected_chapter")}</option>
          </select>

          <Button
            variant="ghost"
            className={styles.btnSmall}
            onClick={handleCopy}
            icon={<Copy size={14} />}
          >
            {t("workspace.log.copy")}
          </Button>

          {!isPanel && (
            <IconButton
              aria-label={t("workspace.log.toggle")}
              icon={logDrawerOpen ? <CaretDown size={16} /> : <CaretUp size={16} />}
              onClick={toggleLogDrawer}
            />
          )}
        </div>
      </div>

      {/* Body: open drawer or panel */}
      {(isPanel || logDrawerOpen) && (
        <div
          ref={listRef}
          className={styles.logList}
          role="log"
          aria-live="polite"
          onScroll={handleScroll}
        >
          {filteredLogs.length === 0 ? (
            <div className={styles.emptyState}>{t("workspace.log.empty")}</div>
          ) : (
            filteredLogs.map((entry) => <LogLine key={entry.id} entry={entry} />)
          )}
        </div>
      )}
    </section>
  );
};
