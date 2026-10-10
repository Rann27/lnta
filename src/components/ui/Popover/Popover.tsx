import React, { useState, useRef, useEffect } from "react";
import styles from "./Popover.module.css";

export interface PopoverProps {
  trigger: React.ReactNode;
  content: React.ReactNode;
  isOpen?: boolean;
  onOpenChange?: (open: boolean) => void;
  placement?: "bottom" | "top";
  className?: string;
}

export const Popover: React.FC<PopoverProps> = ({
  trigger,
  content,
  isOpen: controlledOpen,
  onOpenChange,
  placement = "bottom",
  className,
}) => {
  const [internalOpen, setInternalOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const isControlled = controlledOpen !== undefined;
  const open = isControlled ? controlledOpen : internalOpen;

  const setOpen = (next: boolean) => {
    if (!isControlled) {
      setInternalOpen(next);
    }
    onOpenChange?.(next);
  };

  useEffect(() => {
    if (!open) return;

    const handleClickOutside = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setOpen(false);
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleKeyDown);

    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [open]);

  return (
    <div ref={containerRef} className={styles.container}>
      <div
        onClick={() => setOpen(!open)}
        aria-haspopup="dialog"
        aria-expanded={open}
      >
        {trigger}
      </div>
      {open && (
        <div
          role="dialog"
          className={[
            styles.popover,
            placement === "top" ? styles.placementTop : "",
            className ?? "",
          ]
            .filter(Boolean)
            .join(" ")}
        >
          {content}
        </div>
      )}
    </div>
  );
};
