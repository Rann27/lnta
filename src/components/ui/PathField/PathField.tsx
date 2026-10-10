import React, { useId } from "react";
import { FolderOpen } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { Button } from "../Button";
import { TextField } from "../TextField";
import styles from "./PathField.module.css";

export interface PathFieldProps {
  label?: string;
  value?: string;
  onChange?: (path: string) => void;
  onBrowse?: () => void;
  browseLabel?: string;
  placeholder?: string;
  error?: string | boolean;
  hint?: string;
  disabled?: boolean;
  className?: string;
}

export const PathField: React.FC<PathFieldProps> = ({
  label,
  value = "",
  onChange,
  onBrowse,
  browseLabel,
  placeholder,
  error,
  hint,
  disabled,
  className,
}) => {
  const { t } = useTranslation();
  const generatedId = useId();

  return (
    <div className={[styles.container, className ?? ""].filter(Boolean).join(" ")}>
      <div className={styles.row}>
        <div className={styles.inputWrap}>
          <TextField
            id={generatedId}
            label={label}
            value={value}
            onChange={(e) => onChange?.(e.target.value)}
            placeholder={placeholder}
            error={error}
            hint={hint}
            disabled={disabled}
            className={styles.monoInput}
          />
        </div>
        {onBrowse && (
          <Button
            type="button"
            variant="secondary"
            onClick={onBrowse}
            disabled={disabled}
            icon={<FolderOpen size={16} />}
            style={{ marginTop: label ? "var(--space-6)" : "0" }}
          >
            {browseLabel ?? t("actions.browse")}
          </Button>
        )}
      </div>
    </div>
  );
};
