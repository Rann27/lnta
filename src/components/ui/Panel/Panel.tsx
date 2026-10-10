import React from "react";
import styles from "./Panel.module.css";

export interface PanelProps extends React.HTMLAttributes<HTMLDivElement> {
  flush?: boolean;
  padding?: "none" | "normal" | "dense";
}

export const Panel: React.FC<PanelProps> = ({
  flush = false,
  padding = "normal",
  className,
  children,
  ...props
}) => {
  return (
    <div
      className={[
        styles.panel,
        flush ? styles.flush : "",
        padding === "none" ? styles.paddingNone : "",
        padding === "dense" ? styles.paddingDense : "",
        className ?? "",
      ]
        .filter(Boolean)
        .join(" ")}
      {...props}
    >
      {children}
    </div>
  );
};
