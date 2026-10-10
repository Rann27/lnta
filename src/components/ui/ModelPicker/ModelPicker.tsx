import React, { useState, useRef, useEffect, useId, useMemo } from "react";
import { CaretDown, Warning, MagnifyingGlass } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import type { ModelEntry, ModelReference } from "../../../lib/bindings";
import { ipc } from "../../../lib/ipc";
import styles from "./ModelPicker.module.css";

export interface ModelPickerProps {
  label?: string;
  value?: ModelReference | null;
  onChange?: (val: ModelReference) => void;
  models?: ModelEntry[];
  placeholder?: string;
  disabled?: boolean;
  onOpenSettings?: () => void;
  className?: string;
}

export const ModelPicker: React.FC<ModelPickerProps> = ({
  label,
  value,
  onChange,
  models: externalModels,
  placeholder,
  disabled = false,
  onOpenSettings,
  className,
}) => {
  const { t } = useTranslation();
  const [isOpen, setIsOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [highlightedIndex, setHighlightedIndex] = useState(0);
  const [internalModels, setInternalModels] = useState<ModelEntry[]>([]);

  const containerRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const listboxId = useId();

  // Load models from IPC if not provided externally
  useEffect(() => {
    if (!externalModels) {
      let isMounted = true;
      ipc
        .listModels()
        .then((m) => {
          if (isMounted) setInternalModels(m);
        })
        .catch(() => {
          // ignore error in preview or uninitialized state
        });
      return () => {
        isMounted = false;
      };
    }
  }, [externalModels]);

  const allModels = externalModels ?? internalModels;

  const filteredModels = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return allModels;
    return allModels.filter(
      (m) =>
        m.display_name.toLowerCase().includes(q) ||
        m.model_id.toLowerCase().includes(q) ||
        m.provider.toLowerCase().includes(q)
    );
  }, [allModels, query]);

  // Group filtered models by provider
  const grouped = useMemo(() => {
    const groups: { provider: string; items: ModelEntry[] }[] = [];
    for (const m of filteredModels) {
      const existing = groups.find((g) => g.provider === m.provider);
      if (existing) {
        existing.items.push(m);
      } else {
        groups.push({ provider: m.provider, items: [m] });
      }
    }
    return groups;
  }, [filteredModels]);

  const selectedModel = useMemo(() => {
    if (!value) return null;
    return (
      allModels.find(
        (m) => m.provider === value.provider && m.model_id === value.model_id
      ) ?? null
    );
  }, [allModels, value]);

  // Click outside and Esc listener
  useEffect(() => {
    if (!isOpen) return;

    const handleClickOutside = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setIsOpen(false);
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [isOpen]);

  // Focus search input when popover opens
  useEffect(() => {
    if (isOpen) {
      setQuery("");
      setHighlightedIndex(0);
      setTimeout(() => {
        searchInputRef.current?.focus();
      }, 30);
    }
  }, [isOpen]);

  const handleSelect = (model: ModelEntry) => {
    onChange?.({
      provider: model.provider,
      model_id: model.model_id,
    });
    setIsOpen(false);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (!isOpen) {
      if (e.key === "Enter" || e.key === " " || e.key === "ArrowDown") {
        e.preventDefault();
        setIsOpen(true);
      }
      return;
    }

    if (e.key === "Escape") {
      e.preventDefault();
      setIsOpen(false);
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      if (filteredModels.length > 0) {
        setHighlightedIndex((prev) => (prev + 1) % filteredModels.length);
      }
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (filteredModels.length > 0) {
        setHighlightedIndex(
          (prev) => (prev - 1 + filteredModels.length) % filteredModels.length
        );
      }
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (filteredModels[highlightedIndex]) {
        handleSelect(filteredModels[highlightedIndex]);
      }
    }
  };

  const activeOptionId =
    filteredModels[highlightedIndex] && isOpen
      ? `${listboxId}-opt-${highlightedIndex}`
      : undefined;

  let flatIndexCounter = 0;

  return (
    <div
      ref={containerRef}
      className={[styles.container, className ?? ""].filter(Boolean).join(" ")}
    >
      {label && <label className={styles.label}>{label}</label>}

      <button
        type="button"
        role="combobox"
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        aria-controls={listboxId}
        aria-activedescendant={activeOptionId}
        disabled={disabled}
        onClick={() => setIsOpen(!isOpen)}
        onKeyDown={handleKeyDown}
        className={styles.trigger}
      >
        <span className={styles.triggerContent}>
          {selectedModel ? (
            <>
              <span className={styles.triggerName}>{selectedModel.display_name}</span>
              <span className={styles.triggerModelId}>{selectedModel.model_id}</span>
            </>
          ) : value ? (
            <>
              <span className={styles.triggerName}>{value.model_id}</span>
              <span className={styles.triggerModelId}>({value.provider})</span>
            </>
          ) : (
            <span className={styles.placeholder}>
              {placeholder ?? t("model_picker.placeholder")}
            </span>
          )}
        </span>
        <span className={styles.chevron} aria-hidden="true">
          <CaretDown size={14} />
        </span>
      </button>

      {isOpen && (
        <div className={styles.popover} onKeyDown={handleKeyDown}>
          <div className={styles.searchWrap}>
            <MagnifyingGlass size={16} className={styles.searchIcon} />
            <input
              ref={searchInputRef}
              type="text"
              value={query}
              onChange={(e) => {
                setQuery(e.target.value);
                setHighlightedIndex(0);
              }}
              placeholder={t("model_picker.search_placeholder")}
              className={styles.searchInput}
              aria-label={t("model_picker.search_placeholder")}
            />
          </div>

          <div
            id={listboxId}
            role="listbox"
            aria-label={label ?? t("model_picker.placeholder")}
            className={styles.listbox}
          >
            {allModels.length === 0 ? (
              <div className={styles.empty}>
                <p>{t("model_picker.empty_no_models")}</p>
                {onOpenSettings ? (
                  <button
                    type="button"
                    onClick={() => {
                      setIsOpen(false);
                      onOpenSettings();
                    }}
                    className={styles.settingsLink}
                  >
                    {t("model_picker.go_to_settings")}
                  </button>
                ) : (
                  <Link
                    to="/settings"
                    onClick={() => setIsOpen(false)}
                    className={styles.settingsLink}
                  >
                    {t("model_picker.go_to_settings")}
                  </Link>
                )}
              </div>
            ) : filteredModels.length === 0 ? (
              <div className={styles.empty}>
                <p>{t("model_picker.empty_no_results")}</p>
              </div>
            ) : (
              grouped.map((group) => (
                <div key={group.provider} role="group" aria-label={group.provider}>
                  <div className={styles.groupHeader}>{group.provider}</div>
                  {group.items.map((m) => {
                    const currentIndex = flatIndexCounter++;
                    const isHighlighted = currentIndex === highlightedIndex;
                    const isSelected =
                      value?.provider === m.provider && value?.model_id === m.model_id;
                    const isMissingOutput = !m.max_output_tokens || m.max_output_tokens <= 0;
                    const isExperimental =
                      m.provider.startsWith("cli_") || m.provider === "cli";

                    return (
                      <div
                        key={`${m.provider}:${m.model_id}`}
                        id={`${listboxId}-opt-${currentIndex}`}
                        role="option"
                        aria-selected={isSelected}
                        onClick={() => handleSelect(m)}
                        onMouseEnter={() => setHighlightedIndex(currentIndex)}
                        className={[
                          styles.option,
                          isHighlighted ? styles.optionHighlighted : "",
                          isSelected ? styles.optionSelected : "",
                        ]
                          .filter(Boolean)
                          .join(" ")}
                      >
                        <div className={styles.optionHeader}>
                          <span className={styles.modelName}>{m.display_name}</span>
                          <span className={styles.modelId}>{m.model_id}</span>
                        </div>
                        <div className={styles.optionMeta}>
                          <span>
                            {t("model_picker.context_tokens", {
                              count: m.context_window,
                            })}
                          </span>
                          {isMissingOutput ? (
                            <span className={styles.warningOutput}>
                              <Warning size={14} />
                              <span>{t("model_picker.missing_output_limit")}</span>
                            </span>
                          ) : (
                            <span>
                              {t("model_picker.output_tokens", {
                                count: m.max_output_tokens,
                              })}
                            </span>
                          )}
                          {isExperimental && (
                            <span className={styles.experimentalBadge}>
                              {t("model_picker.experimental")}
                            </span>
                          )}
                        </div>
                      </div>
                    );
                  })}
                </div>
              ))
            )}
          </div>
        </div>
      )}
    </div>
  );
};
