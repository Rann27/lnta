import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  ArrowClockwise,
  ArrowCounterClockwise,
  Check,
  X,
  Warning,
} from "@phosphor-icons/react";
import { Button } from "../../components/ui/Button";
import { StatusTag } from "../../components/ui/StatusTag";
import { renderRuby } from "./ruby";
import { ipc } from "../../lib/ipc";
import type { SegmentPair } from "./types";
import type { Translation } from "../../lib/bindings";
import styles from "./SegmentEditor.module.css";

export interface SegmentEditorProps {
  pair: SegmentPair;
  chapterId: string;
  projectPath: string;
  targetLanguage?: string;
  onClose: () => void;
  onSaved: () => void;
}

export const SegmentEditor: React.FC<SegmentEditorProps> = ({
  pair,
  chapterId,
  projectPath,
  targetLanguage = "id",
  onClose,
  onSaved,
}) => {
  const { t } = useTranslation();
  const [editText, setEditText] = useState(pair.translation?.text ?? "");
  const [isSaving, setIsSaving] = useState(false);
  const [isRetranslating, setIsRetranslating] = useState(false);
  const [isRestoringId, setIsRestoringId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [history, setHistory] = useState<Translation[]>([]);

  const segFrom = pair.primaryIdx;
  const segTo = pair.isRange
    ? pair.segments[pair.segments.length - 1].idx
    : pair.primaryIdx;

  const loadHistory = async () => {
    if (!projectPath || !chapterId) return;
    try {
      const list = await ipc.getSegmentHistory(projectPath, chapterId, segFrom, segTo);
      setHistory(list);
    } catch {
      // ignore
    }
  };

  useEffect(() => {
    void loadHistory();
  }, [projectPath, chapterId, segFrom, segTo]);

  // Client-side validation checks
  const clientWarnings = (() => {
    const warnings: string[] = [];
    const trimmed = editText.trim();
    if (!trimmed && pair.segments.some((s) => s.src_text.trim())) {
      warnings.push(t("chapters.validation_warning_empty"));
    }
    // Check residual Japanese kana or kanji
    if (/[\u3040-\u309f\u30a0-\u30ff\u4e00-\u9faf]/.test(trimmed)) {
      warnings.push(t("chapters.validation_warning_residual_jp"));
    }
    // Check double quotes count
    const quotesCount = (trimmed.match(/"/g) || []).length;
    if (quotesCount % 2 !== 0) {
      warnings.push(t("chapters.validation_warning_unbalanced_quotes"));
    }
    return warnings;
  })();

  const handleSave = async () => {
    if (!projectPath || !chapterId) return;
    setIsSaving(true);
    setError(null);
    try {
      await ipc.editSegment(projectPath, chapterId, segFrom, segTo, editText);
      setIsSaving(false);
      onSaved();
    } catch (err) {
      setIsSaving(false);
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleRetranslate = async () => {
    if (!projectPath || !chapterId) return;
    setIsRetranslating(true);
    setError(null);
    try {
      const res = await ipc.retranslateSegment(projectPath, chapterId, segFrom, segTo);
      setEditText(res.text);
      setIsRetranslating(false);
      void loadHistory();
      onSaved();
    } catch (err) {
      setIsRetranslating(false);
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleRestore = async (translationId: string) => {
    if (!projectPath || !chapterId) return;
    setIsRestoringId(translationId);
    setError(null);
    try {
      const res = await ipc.restoreSegmentVersion(projectPath, chapterId, translationId);
      setEditText(res.text);
      setIsRestoringId(null);
      void loadHistory();
      onSaved();
    } catch (err) {
      setIsRestoringId(null);
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  // Keyboard shortcut: Ctrl+Enter / Cmd+Enter to save, Esc to cancel
  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
      e.preventDefault();
      void handleSave();
    } else if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  };

  return (
    <div className={styles.container} role="region" aria-label={t("chapters.edit_segment_title", { index: pair.indexLabel })}>
      <header className={styles.header}>
        <h3 className={styles.title}>
          {t("chapters.edit_segment_title", { index: pair.indexLabel })}
        </h3>
        <Button
          variant="secondary"
          size="sm"
          icon={<X size={14} />}
          onClick={onClose}
          aria-label={t("common.close")}
        />
      </header>

      {/* Read-only Source Box */}
      <div className={styles.sourceBox} lang="ja">
        {pair.isRange ? (
          <div>
            {pair.segments.map((s) => (
              <div key={s.id}>{renderRuby(s.src_text)}</div>
            ))}
          </div>
        ) : (
          renderRuby(pair.segments[0]?.src_text || "")
        )}
      </div>

      {/* Editable Target Textarea */}
      <textarea
        className={styles.textarea}
        lang={targetLanguage}
        value={editText}
        onChange={(e) => setEditText(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder={t("chapters.not_translated")}
        autoFocus
      />

      <p className={styles.formatHelp}>{t("chapters.editor_format_help")}</p>

      {/* Validation Warnings / Error */}
      {error && (
        <div className={styles.warningBox}>
          <Warning size={14} aria-hidden="true" />
          <span>{error}</span>
        </div>
      )}

      {clientWarnings.length > 0 && !error && (
        <div className={styles.warningBox}>
          <Warning size={14} aria-hidden="true" />
          <span>{clientWarnings.join(" ")}</span>
        </div>
      )}

      {/* Action Buttons */}
      <div className={styles.actions}>
        <Button
          variant="secondary"
          size="sm"
          icon={<ArrowClockwise size={14} />}
          disabled={isRetranslating || isSaving}
          onClick={handleRetranslate}
        >
          {isRetranslating
            ? t("chapters.retranslating_segment")
            : t("chapters.retranslate_segment")}
        </Button>

        <div className={styles.actionGroup}>
          <Button variant="secondary" size="sm" onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button
            variant="primary"
            size="sm"
            icon={<Check size={14} />}
            disabled={isSaving || isRetranslating}
            onClick={handleSave}
          >
            {isSaving ? t("chapters.saving") : t("common.save")}
          </Button>
        </div>
      </div>

      {/* Version History Section */}
      {history.length > 0 && (
        <section className={styles.historySection}>
          <h4 className={styles.historyTitle}>
            {t("chapters.history_title", { count: history.length })}
          </h4>
          <div className={styles.historyList}>
            {history.map((ver) => (
              <div
                key={ver.id}
                className={[
                  styles.historyItem,
                  ver.is_current ? styles.historyItemCurrent : "",
                ]
                  .filter(Boolean)
                  .join(" ")}
              >
                <div className={styles.historyMeta}>
                  <div className={styles.historyHeader}>
                    <strong>
                      {t("chapters.history_version", { version: ver.version })}
                    </strong>
                    <StatusTag
                      status={ver.is_current ? "done" : "queued"}
                      label={ver.is_current ? t("chapters.history_current") : ver.origin}
                    />
                    <span>{new Date(ver.created_at).toLocaleTimeString()}</span>
                  </div>
                  <div className={styles.historyText} lang={targetLanguage}>
                    {ver.text}
                  </div>
                </div>

                {!ver.is_current && (
                  <Button
                    variant="secondary"
                    size="sm"
                    icon={<ArrowCounterClockwise size={14} />}
                    disabled={isRestoringId === ver.id}
                    onClick={() => handleRestore(ver.id)}
                  >
                    {isRestoringId === ver.id
                      ? t("chapters.restoring")
                      : t("chapters.restore_version")}
                  </Button>
                )}
              </div>
            ))}
          </div>
        </section>
      )}
    </div>
  );
};
