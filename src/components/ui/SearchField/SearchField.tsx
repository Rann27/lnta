import React, { useState, useEffect, useRef } from "react";
import { MagnifyingGlass, X } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { TextField } from "../TextField";
import { IconButton } from "../IconButton";
import styles from "./SearchField.module.css";

export interface SearchFieldProps {
  value?: string;
  onChange?: (value: string) => void;
  debounceMs?: number;
  resultCountText?: string;
  placeholder?: string;
  disabled?: boolean;
  enableShortcut?: boolean;
  className?: string;
}

export const SearchField: React.FC<SearchFieldProps> = ({
  value = "",
  onChange,
  debounceMs = 150,
  resultCountText,
  placeholder,
  disabled = false,
  enableShortcut = true,
  className,
}) => {
  const { t } = useTranslation();
  const inputRef = useRef<HTMLInputElement>(null);
  const [internalValue, setInternalValue] = useState(value);

  useEffect(() => {
    setInternalValue(value);
  }, [value]);

  useEffect(() => {
    if (!onChange) return;
    const timer = setTimeout(() => {
      if (internalValue !== value) {
        onChange(internalValue);
      }
    }, debounceMs);
    return () => clearTimeout(timer);
  }, [internalValue, onChange, debounceMs, value]);

  useEffect(() => {
    if (!enableShortcut) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") {
        e.preventDefault();
        inputRef.current?.focus();
        inputRef.current?.select();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [enableShortcut]);

  const handleClear = () => {
    setInternalValue("");
    onChange?.("");
    inputRef.current?.focus();
  };

  return (
    <div className={[styles.container, className ?? ""].filter(Boolean).join(" ")}>
      <TextField
        ref={inputRef}
        type="search"
        value={internalValue}
        onChange={(e) => setInternalValue(e.target.value)}
        placeholder={placeholder ?? t("fields.search_placeholder")}
        disabled={disabled}
        leftElement={<MagnifyingGlass size={16} />}
        rightElement={
          internalValue ? (
            <IconButton
              size="sm"
              variant="ghost"
              aria-label={t("actions.clear_search")}
              icon={<X size={14} />}
              onClick={handleClear}
            />
          ) : undefined
        }
      />
      {resultCountText && <span className={styles.countText}>{resultCountText}</span>}
    </div>
  );
};
