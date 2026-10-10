import React, { useEffect, useRef, useId } from "react";
import styles from "./Dialog.module.css";

export interface DialogProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  description?: string;
  variant?: "default" | "destructive" | "form";
  actions?: React.ReactNode;
  children?: React.ReactNode;
  className?: string;
}

export const Dialog: React.FC<DialogProps> = ({
  isOpen,
  onClose,
  title,
  description,
  variant = "default",
  actions,
  children,
  className,
}) => {
  const dialogRef = useRef<HTMLDivElement>(null);
  const previouslyFocusedRef = useRef<HTMLElement | null>(null);
  const titleId = useId();
  const descId = useId();

  const isDestructive = variant === "destructive";

  // Trap focus and restore focus on close
  useEffect(() => {
    if (!isOpen) return;

    previouslyFocusedRef.current = document.activeElement as HTMLElement | null;

    const dialogNode = dialogRef.current;
    if (!dialogNode) return;

    const getFocusable = () => {
      const selector =
        'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';
      return Array.from(dialogNode.querySelectorAll<HTMLElement>(selector));
    };

    const focusables = getFocusable();
    if (focusables.length > 0) {
      if (isDestructive) {
        // On destructive dialogs, find Cancel button or first non-destructive action
        const cancelBtn = focusables.find(
          (el) =>
            el.textContent?.toLowerCase().includes("batal") ||
            el.textContent?.toLowerCase().includes("cancel"),
        );
        (cancelBtn ?? focusables[0]).focus();
      } else {
        focusables[0].focus();
      }
    } else {
      dialogNode.focus();
    }

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
        return;
      }

      if (e.key === "Tab") {
        const elements = getFocusable();
        if (elements.length === 0) {
          e.preventDefault();
          return;
        }

        const first = elements[0];
        const last = elements[elements.length - 1];

        if (e.shiftKey) {
          if (document.activeElement === first) {
            e.preventDefault();
            last.focus();
          }
        } else {
          if (document.activeElement === last) {
            e.preventDefault();
            first.focus();
          }
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      previouslyFocusedRef.current?.focus();
    };
  }, [isOpen, onClose, isDestructive]);

  if (!isOpen) return null;

  const handleScrimClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget) {
      if (!isDestructive) {
        onClose();
      }
    }
  };

  return (
    <div
      className={styles.scrim}
      onClick={handleScrimClick}
      data-testid="dialog-scrim"
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={description ? descId : undefined}
        tabIndex={-1}
        className={[
          styles.dialog,
          variant === "form" ? styles.formVariant : "",
          className ?? "",
        ]
          .filter(Boolean)
          .join(" ")}
      >
        <div className={styles.header}>
          <h3 id={titleId} className={styles.title}>
            {title}
          </h3>
          {description && (
            <p id={descId} className={styles.description}>
              {description}
            </p>
          )}
        </div>
        {children && <div className={styles.body}>{children}</div>}
        {actions && <div className={styles.actions}>{actions}</div>}
      </div>
    </div>
  );
};
