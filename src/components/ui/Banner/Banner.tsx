import React from "react";
import { HandPalm, Warning, X } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { IconButton } from "../IconButton";
import styles from "./Banner.module.css";

export type BannerVariant = "blocker" | "attention";

export interface BannerProps {
  variant: BannerVariant;
  title: string;
  description?: string;
  action?: React.ReactNode;
  onClose?: () => void;
  className?: string;
}

export const Banner: React.FC<BannerProps> = ({
  variant,
  title,
  description,
  action,
  onClose,
  className,
}) => {
  const { t } = useTranslation();
  const isBlocker = variant === "blocker";

  return (
    <div
      role={isBlocker ? "alert" : "status"}
      className={[
        styles.banner,
        isBlocker ? styles.blocker : styles.attention,
        className ?? "",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <span className={styles.icon} aria-hidden="true">
        {isBlocker ? <HandPalm size={20} /> : <Warning size={20} />}
      </span>
      <div className={styles.content}>
        <h4 className={styles.title}>{title}</h4>
        {description && <p className={styles.description}>{description}</p>}
      </div>
      <div className={styles.actions}>
        {action}
        {!isBlocker && onClose && (
          <IconButton
            size="sm"
            variant="ghost"
            aria-label={t("common.close")}
            icon={<X size={16} />}
            onClick={onClose}
          />
        )}
      </div>
    </div>
  );
};
