import React, { useState, useEffect, useRef } from "react";
import styles from "./ContextMenu.module.css";

export interface ContextMenuItem {
  label: string;
  onClick: () => void;
  destructive?: boolean;
  disabled?: boolean;
  icon?: React.ReactNode;
  shortcut?: string;
  separatorBefore?: boolean;
}

export interface ContextMenuProps {
  items: ContextMenuItem[];
  children: React.ReactNode;
  className?: string;
}

export const ContextMenu: React.FC<ContextMenuProps> = ({
  items,
  children,
  className,
}) => {
  const [position, setPosition] = useState<{ x: number; y: number } | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!position) return;

    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setPosition(null);
      }
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setPosition(null);
      } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        const menuNode = menuRef.current;
        if (!menuNode) return;
        const buttons = Array.from(
          menuNode.querySelectorAll<HTMLButtonElement>("button:not([disabled])"),
        );
        if (buttons.length === 0) return;
        const activeIdx = buttons.indexOf(document.activeElement as HTMLButtonElement);
        if (e.key === "ArrowDown") {
          const next = activeIdx < buttons.length - 1 ? activeIdx + 1 : 0;
          buttons[next].focus();
        } else {
          const prev = activeIdx > 0 ? activeIdx - 1 : buttons.length - 1;
          buttons[prev].focus();
        }
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleKeyDown);

    // Focus first actionable item
    const timer = setTimeout(() => {
      const firstBtn = menuRef.current?.querySelector<HTMLButtonElement>(
        "button:not([disabled])",
      );
      firstBtn?.focus();
    }, 10);

    return () => {
      clearTimeout(timer);
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [position]);

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    setPosition({ x: e.clientX, y: e.clientY });
  };

  const handleKeyDownTarget = (e: React.KeyboardEvent) => {
    if ((e.shiftKey && e.key === "F10") || e.key === "ContextMenu") {
      e.preventDefault();
      const rect = e.currentTarget.getBoundingClientRect();
      setPosition({ x: rect.left + 20, y: rect.bottom });
    }
  };

  return (
    <div
      ref={containerRef}
      onContextMenu={handleContextMenu}
      onKeyDown={handleKeyDownTarget}
      className={className}
    >
      {children}
      {position && (
        <div
          ref={menuRef}
          role="menu"
          className={styles.menu}
          style={{ top: position.y, left: position.x }}
        >
          {items.map((item, idx) => (
            <React.Fragment key={idx}>
              {item.separatorBefore && <div className={styles.separator} role="separator" />}
              <button
                type="button"
                role="menuitem"
                disabled={item.disabled}
                onClick={() => {
                  item.onClick();
                  setPosition(null);
                }}
                className={[
                  styles.item,
                  item.destructive ? styles.destructive : "",
                ]
                  .filter(Boolean)
                  .join(" ")}
              >
                <span className={styles.left}>
                  {item.icon}
                  <span>{item.label}</span>
                </span>
                {item.shortcut && <span className={styles.shortcut}>{item.shortcut}</span>}
              </button>
            </React.Fragment>
          ))}
        </div>
      )}
    </div>
  );
};
