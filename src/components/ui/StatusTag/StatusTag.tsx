import React from "react";
import {
  Check,
  Warning,
  Pause,
  LockSimple,
  ArrowsClockwise,
} from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { TallyLamp } from "../TallyLamp";
import styles from "./StatusTag.module.css";

export type StatusType =
  | "queued"
  | "running"
  | "paused"
  | "done"
  | "needs_attention"
  | "blocked"
  | "frozen"
  | "source_changed";

export interface StatusTagProps extends React.HTMLAttributes<HTMLSpanElement> {
  status: StatusType;
  label?: string;
}

export const StatusTag: React.FC<StatusTagProps> = ({
  status,
  label,
  className,
  ...props
}) => {
  const { t } = useTranslation();
  const displayLabel = label ?? t(`status.${status}`, status);

  const renderVisualCue = () => {
    switch (status) {
      case "running":
        return <TallyLamp />;
      case "blocked":
        return <span className={styles.hazard} aria-hidden="true" />;
      case "done":
        return (
          <span className={styles.icon} aria-hidden="true">
            <Check size={14} />
          </span>
        );
      case "needs_attention":
        return (
          <span className={styles.icon} aria-hidden="true">
            <Warning size={14} />
          </span>
        );
      case "paused":
        return (
          <span className={styles.icon} aria-hidden="true">
            <Pause size={14} />
          </span>
        );
      case "frozen":
        return (
          <span className={styles.icon} aria-hidden="true">
            <LockSimple size={14} />
          </span>
        );
      case "source_changed":
        return (
          <span className={styles.icon} aria-hidden="true">
            <ArrowsClockwise size={14} />
          </span>
        );
      case "queued":
      default:
        return null;
    }
  };

  return (
    <span
      data-status={status}
      className={[styles.tag, styles[status], className ?? ""].filter(Boolean).join(" ")}
      {...props}
    >
      {renderVisualCue()}
      <span>{displayLabel}</span>
    </span>
  );
};
