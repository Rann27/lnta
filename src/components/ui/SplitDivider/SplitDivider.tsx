import React from "react";
import styles from "./SplitDivider.module.css";

export interface SplitDividerProps {
  value: number;
  onChange: (newValue: number) => void;
  min?: number;
  max?: number;
  step?: number;
  orientation?: "vertical" | "horizontal";
  label?: string;
  className?: string;
}

export const SplitDivider: React.FC<SplitDividerProps> = ({
  value,
  onChange,
  min = 100,
  max = 800,
  step = 16,
  orientation = "vertical",
  label = "Pemisah panel",
  className,
}) => {
  const clamp = (val: number) => Math.min(Math.max(val, min), max);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    let nextVal = value;
    if (orientation === "vertical") {
      if (e.key === "ArrowLeft") {
        e.preventDefault();
        nextVal = clamp(value - step);
      } else if (e.key === "ArrowRight") {
        e.preventDefault();
        nextVal = clamp(value + step);
      }
    } else {
      if (e.key === "ArrowUp") {
        e.preventDefault();
        nextVal = clamp(value - step);
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        nextVal = clamp(value + step);
      }
    }

    if (e.key === "Home") {
      e.preventDefault();
      nextVal = min;
    } else if (e.key === "End") {
      e.preventDefault();
      nextVal = max;
    }

    if (nextVal !== value) {
      onChange(nextVal);
    }
  };

  const handlePointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    e.currentTarget.setPointerCapture(e.pointerId);
    const startPos = orientation === "vertical" ? e.clientX : e.clientY;
    const startValue = value;

    const handlePointerMove = (moveEvent: PointerEvent) => {
      const currentPos = orientation === "vertical" ? moveEvent.clientX : moveEvent.clientY;
      const delta = currentPos - startPos;
      onChange(clamp(startValue + delta));
    };

    const handlePointerUp = () => {
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
    };

    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", handlePointerUp);
  };

  return (
    <div
      role="separator"
      tabIndex={0}
      aria-orientation={orientation}
      aria-label={label}
      aria-valuenow={value}
      aria-valuemin={min}
      aria-valuemax={max}
      onKeyDown={handleKeyDown}
      onPointerDown={handlePointerDown}
      className={[
        styles.divider,
        orientation === "vertical" ? styles.vertical : styles.horizontal,
        className ?? "",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <div className={styles.line} aria-hidden="true" />
    </div>
  );
};
