import React, { useState, useEffect, useRef } from "react";
import { Check, Info, Warning, X } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { IconButton } from "../IconButton";
import styles from "./Toast.module.css";

export type ToastVariant = "info" | "success" | "warn" | "error";

export interface ToastProps {
  isOpen: boolean;
  onClose: () => void;
  message: string;
  title?: string;
  variant?: ToastVariant;
  durationMs?: number;
  action?: React.ReactNode;
  className?: string;
}

export const Toast: React.FC<ToastProps> = ({
  isOpen,
  onClose,
  message,
  title,
  variant = "info",
  durationMs = 6000,
  action,
  className,
}) => {
  const { t } = useTranslation();
  const [isPaused, setIsPaused] = useState(false);
  const remainingTimeRef = useRef(durationMs);
  const startTimeRef = useRef<number>(Date.now());

  useEffect(() => {
    if (!isOpen) return;

    remainingTimeRef.current = durationMs;
    startTimeRef.current = Date.now();
  }, [isOpen, durationMs]);

  useEffect(() => {
    if (!isOpen || isPaused) return;

    startTimeRef.current = Date.now();
    const timer = setTimeout(() => {
      onClose();
    }, remainingTimeRef.current);

    return () => {
      clearTimeout(timer);
      const elapsed = Date.now() - startTimeRef.current;
      remainingTimeRef.current = Math.max(0, remainingTimeRef.current - elapsed);
    };
  }, [isOpen, isPaused, onClose]);

  if (!isOpen) return null;

  const renderIcon = () => {
    switch (variant) {
      case "success":
        return <Check size={16} />;
      case "warn":
      case "error":
        return <Warning size={16} />;
      case "info":
      default:
        return <Info size={16} />;
    }
  };

  return (
    <div
      role={variant === "error" ? "alert" : "status"}
      aria-live="polite"
      onMouseEnter={() => setIsPaused(true)}
      onMouseLeave={() => setIsPaused(false)}
      onFocus={() => setIsPaused(true)}
      onBlur={() => setIsPaused(false)}
      data-testid="toast-container"
      className={[styles.toast, className ?? ""].filter(Boolean).join(" ")}
    >
      <span className={styles.icon} aria-hidden="true">
        {renderIcon()}
      </span>
      <div className={styles.content}>
        {title && <h4 className={styles.title}>{title}</h4>}
        <p className={styles.message}>{message}</p>
      </div>
      <div className={styles.actions}>
        {action}
        <IconButton
          size="sm"
          variant="ghost"
          aria-label={t("common.close")}
          icon={<X size={14} />}
          onClick={onClose}
        />
      </div>
    </div>
  );
};
