import React, { useState } from "react";
import { LockSimple } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { Button } from "../Button";
import { TextField } from "../TextField";
import styles from "./SecretField.module.css";

export interface SecretFieldProps {
  label?: string;
  isStored?: boolean;
  value?: string;
  onChange?: (value: string) => void;
  onClear?: () => void;
  onTestConnection?: () => void;
  isTesting?: boolean;
  error?: string | boolean;
  hint?: string;
  placeholder?: string;
  disabled?: boolean;
  className?: string;
}

export const SecretField: React.FC<SecretFieldProps> = ({
  label,
  isStored = false,
  value = "",
  onChange,
  onClear,
  onTestConnection,
  isTesting = false,
  error,
  hint,
  placeholder,
  disabled = false,
  className,
}) => {
  const { t } = useTranslation();
  const [isEditing, setIsEditing] = useState(!isStored);

  // If a key is stored and the user is not actively replacing it,
  // we strictly never render an input with stored secret content.
  if (isStored && !isEditing) {
    return (
      <div className={[styles.container, className ?? ""].filter(Boolean).join(" ")}>
        {label && <span className={styles.label}>{label}</span>}
        <div className={styles.storedBox}>
          <div className={styles.storedBadge}>
            <LockSimple size={16} />
            <span>{t("fields.stored_in_keyring")}</span>
          </div>
          <div className={styles.actions}>
            {onTestConnection && (
              <Button
                size="sm"
                variant="secondary"
                onClick={onTestConnection}
                disabled={disabled || isTesting}
              >
                {t("actions.test_connection")}
              </Button>
            )}
            <Button
              size="sm"
              variant="secondary"
              onClick={() => setIsEditing(true)}
              disabled={disabled}
            >
              {t("actions.change_key")}
            </Button>
            {onClear && (
              <Button
                size="sm"
                variant="ghost"
                onClick={onClear}
                disabled={disabled}
              >
                {t("actions.clear_key")}
              </Button>
            )}
          </div>
        </div>
        {typeof error === "string" && error && (
          <span role="alert" className={styles.errorMsg}>
            {error}
          </span>
        )}
        {hint && <span className={styles.hint}>{hint}</span>}
      </div>
    );
  }

  return (
    <div className={[styles.container, className ?? ""].filter(Boolean).join(" ")}>
      <TextField
        type="password"
        autoComplete="off"
        label={label}
        value={value}
        onChange={(e) => onChange?.(e.target.value)}
        placeholder={placeholder ?? t("fields.secret_placeholder")}
        error={error}
        hint={hint}
        disabled={disabled}
        rightElement={
          isStored ? (
            <Button
              size="sm"
              variant="ghost"
              onClick={() => setIsEditing(false)}
            >
              {t("common.cancel")}
            </Button>
          ) : undefined
        }
      />
    </div>
  );
};
