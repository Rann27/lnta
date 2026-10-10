import React, { useState, useId } from "react";
import { CaretDown } from "@phosphor-icons/react";
import styles from "./Disclosure.module.css";

export interface DisclosureProps {
  title: React.ReactNode;
  children: React.ReactNode;
  isOpen?: boolean;
  defaultOpen?: boolean;
  onOpenChange?: (open: boolean) => void;
  className?: string;
}

export const Disclosure: React.FC<DisclosureProps> = ({
  title,
  children,
  isOpen: controlledOpen,
  defaultOpen = false,
  onOpenChange,
  className,
}) => {
  const [internalOpen, setInternalOpen] = useState(defaultOpen);
  const panelId = useId();

  const isControlled = controlledOpen !== undefined;
  const open = isControlled ? controlledOpen : internalOpen;

  const toggle = () => {
    const next = !open;
    if (!isControlled) {
      setInternalOpen(next);
    }
    onOpenChange?.(next);
  };

  return (
    <div
      className={[
        styles.container,
        open ? styles.open : "",
        className ?? "",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <button
        type="button"
        aria-expanded={open}
        aria-controls={panelId}
        onClick={toggle}
        className={styles.trigger}
      >
        <span>{title}</span>
        <span className={styles.icon} aria-hidden="true">
          <CaretDown size={16} />
        </span>
      </button>
      {open && (
        <div id={panelId} className={styles.panel}>
          {children}
        </div>
      )}
    </div>
  );
};
