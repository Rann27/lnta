import React from "react";
import type { LogEntry } from "../store";
import styles from "./LogLine.module.css";

export interface LogLineProps {
  entry: LogEntry;
}

export const LogLine: React.FC<LogLineProps> = ({ entry }) => {
  return (
    <div
      className={[
        styles.line,
        entry.level === "warning" ? styles.warning : "",
        entry.level === "error" ? styles.error : "",
      ]
        .filter(Boolean)
        .join(" ")}
      role="listitem"
    >
      <span className={styles.timestamp}>{entry.timestamp}</span>
      <span className={styles.scope}>{entry.scope}</span>
      <span className={styles.message}>{entry.message}</span>
    </div>
  );
};
