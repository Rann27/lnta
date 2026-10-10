import React from "react";
import styles from "./StateBlock.module.css";

export interface StateBlockProps extends React.HTMLAttributes<HTMLDivElement> {
  title: string;
  description?: string;
  action?: React.ReactNode;
  children?: React.ReactNode;
}

export const StateBlock: React.FC<StateBlockProps> = ({
  title,
  description,
  action,
  children,
  className,
  ...props
}) => {
  return (
    <div
      className={[styles.stateBlock, className ?? ""].filter(Boolean).join(" ")}
      {...props}
    >
      <h4 className={styles.title}>{title}</h4>
      {description && <p className={styles.description}>{description}</p>}
      {children}
      {action && <div className={styles.actions}>{action}</div>}
    </div>
  );
};
