import React, { useState, useRef, useEffect, useId } from "react";
import styles from "./Tooltip.module.css";

export interface TooltipProps {
  content: React.ReactNode;
  placement?: "top" | "bottom" | "left" | "right";
  children: React.ReactElement;
}

export const Tooltip: React.FC<TooltipProps> = ({
  content,
  placement = "top",
  children,
}) => {
  const [visible, setVisible] = useState(false);
  const timerRef = useRef<number | null>(null);
  const tooltipId = useId();

  const handleMouseEnter = () => {
    timerRef.current = window.setTimeout(() => {
      setVisible(true);
    }, 500);
  };

  const handleMouseLeave = () => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    setVisible(false);
  };

  const handleFocus = () => {
    setVisible(true);
  };

  const handleBlur = () => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    setVisible(false);
  };

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && visible) {
        setVisible(false);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [visible]);

  return (
    <div
      className={styles.container}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      onFocus={handleFocus}
      onBlur={handleBlur}
    >
      {React.cloneElement(children, {
        "aria-describedby": visible ? tooltipId : undefined,
      } as React.HTMLAttributes<HTMLElement>)}
      {visible && content && (
        <div
          id={tooltipId}
          role="tooltip"
          className={[styles.tooltip, styles[placement]].join(" ")}
        >
          {content}
        </div>
      )}
    </div>
  );
};
