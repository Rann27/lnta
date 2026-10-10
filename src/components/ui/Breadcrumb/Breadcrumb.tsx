import React from "react";
import styles from "./Breadcrumb.module.css";

export interface BreadcrumbItem {
  label: string;
  onClick?: () => void;
}

export interface BreadcrumbProps {
  items: BreadcrumbItem[];
  className?: string;
  ariaLabel?: string;
}

export const Breadcrumb: React.FC<BreadcrumbProps> = ({
  items,
  className,
  ariaLabel = "Breadcrumb",
}) => {
  return (
    <nav aria-label={ariaLabel} className={[styles.nav, className].filter(Boolean).join(" ")}>
      <ol className={styles.list}>
        {items.map((item, index) => {
          const isLast = index === items.length - 1;

          return (
            <li key={index} className={styles.item}>
              {index > 0 && (
                <span className={styles.separator} aria-hidden="true">
                  ›
                </span>
              )}
              {isLast ? (
                <span className={styles.current} aria-current="page">
                  {item.label}
                </span>
              ) : item.onClick ? (
                <button
                  type="button"
                  className={styles.button}
                  onClick={item.onClick}
                >
                  {item.label}
                </button>
              ) : (
                <span className={styles.label}>{item.label}</span>
              )}
            </li>
          );
        })}
      </ol>
    </nav>
  );
};
