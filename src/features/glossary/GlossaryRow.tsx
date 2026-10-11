import React, { useState, useEffect, useRef } from "react";
import {
  PencilSimple,
  Trash,
  Check,
  X,
  Warning,
  ArrowsClockwise,
} from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/Button";
import { useGlossaryStore } from "./store";
import { lintGlossaryEntry } from "./lint";
import type { GlossaryEntry } from "../../lib/bindings";
import styles from "./GlossaryRow.module.css";

export interface GlossaryRowProps {
  entry: GlossaryEntry;
  /** First chapter as shown in the rail ("02"), or "-" when unknown. */
  chapterLabel: string;
  allEntries: GlossaryEntry[];
  projectPath: string;
  isEditing: boolean;
  onStartEdit: () => void;
  onCancelEdit: () => void;
}

export const GlossaryRow: React.FC<GlossaryRowProps> = ({
  entry,
  chapterLabel,
  allEntries,
  projectPath,
  isEditing,
  onStartEdit,
  onCancelEdit,
}) => {
  const { t } = useTranslation();
  const { updateEntry, deleteEntry, acceptProposal, ignoreProposal } =
    useGlossaryStore();

  const [editSource, setEditSource] = useState(entry.source);
  const [editTranslation, setEditTranslation] = useState(entry.translation);
  const [editNote, setEditNote] = useState(entry.note ?? "");

  const sourceInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (isEditing) {
      setEditSource(entry.source);
      setEditTranslation(entry.translation);
      setEditNote(entry.note ?? "");
      setTimeout(() => {
        sourceInputRef.current?.focus();
      }, 30);
    }
  }, [isEditing, entry]);

  const lints = lintGlossaryEntry(entry, allEntries);

  const handleSave = async () => {
    if (!editSource.trim() || !editTranslation.trim()) return;
    await updateEntry(projectPath, {
      source: editSource.trim(),
      translation: editTranslation.trim(),
      note: editNote.trim() || null,
    });
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      void handleSave();
    } else if (e.key === "Escape") {
      e.preventDefault();
      onCancelEdit();
    }
  };

  // Check for proposal in note (e.g. "(usulan model: Pedang Kudus)")
  const proposalMatch = (entry.note ?? "").match(/usulan(?:\s+model)?:\s*([^)]+)/i);
  const proposedText = proposalMatch ? proposalMatch[1].trim() : null;
  const isConflict = entry.status === "conflict";

  return (
    <div
      className={[
        styles.row,
        isEditing ? styles.rowEditing : "",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <div className={styles.cellsGrid}>
        {isEditing ? (
          <>
            <div>
              <input
                ref={sourceInputRef}
                type="text"
                value={editSource}
                onChange={(e) => setEditSource(e.target.value)}
                onKeyDown={handleKeyDown}
                className={styles.editInput}
                lang="ja"
                aria-label={t("glossary.columns.source")}
              />
            </div>
            <div>
              <input
                type="text"
                value={editTranslation}
                onChange={(e) => setEditTranslation(e.target.value)}
                onKeyDown={handleKeyDown}
                className={styles.editInput}
                aria-label={t("glossary.columns.translation")}
              />
            </div>
            <div>
              <input
                type="text"
                value={editNote}
                onChange={(e) => setEditNote(e.target.value)}
                onKeyDown={handleKeyDown}
                className={styles.editInput}
                aria-label={t("glossary.columns.note")}
              />
            </div>
            <div className={styles.cellTier}>
              <span
                className={[
                  styles.tierBadge,
                  entry.tier === 1 ? styles.tier1Badge : "",
                ]
                  .filter(Boolean)
                  .join(" ")}
              >
                T{entry.tier}
              </span>
            </div>
            <div className={styles.cellChapter}>{chapterLabel}</div>
            <div className={styles.cellOccurrences}>
              {entry.occurrences.toLocaleString()}
            </div>
            <div className={styles.cellActions}>
              <Button
                size="sm"
                variant="primary"
                icon={<Check size={14} />}
                onClick={handleSave}
                aria-label={t("common.save")}
              />
              <Button
                size="sm"
                variant="ghost"
                icon={<X size={14} />}
                onClick={onCancelEdit}
                aria-label={t("common.cancel")}
              />
            </div>
          </>
        ) : (
          <>
            <div className={styles.cellSource} lang="ja" title={entry.source}>
              {entry.source}
            </div>
            <div className={styles.cellTranslation} title={entry.translation}>
              {entry.translation}
            </div>
            <div className={styles.cellNote} title={entry.note ?? ""}>
              {entry.note || "-"}
            </div>
            <div className={styles.cellTier}>
              <span
                className={[
                  styles.tierBadge,
                  entry.tier === 1 ? styles.tier1Badge : "",
                ]
                  .filter(Boolean)
                  .join(" ")}
              >
                T{entry.tier}
              </span>
            </div>
            <div className={styles.cellChapter}>{chapterLabel}</div>
            <div className={styles.cellOccurrences}>
              {entry.occurrences.toLocaleString()}
            </div>
            <div className={styles.cellActions}>
              <Button
                size="sm"
                variant="ghost"
                icon={<PencilSimple size={14} />}
                onClick={onStartEdit}
                aria-label={t("actions.change_model")}
              />
              <Button
                size="sm"
                variant="ghost"
                icon={<Trash size={14} />}
                onClick={() => deleteEntry(projectPath, entry.source)}
                aria-label={t("common.delete")}
              />
            </div>
          </>
        )}
      </div>

      {/* Lint Messages Under Row */}
      {lints.length > 0 && (
        <div className={styles.lintArea}>
          {lints.map((lint, i) => (
            <span key={i} className={styles.lintItem}>
              <Warning size={12} />
              <span>{lint.message}</span>
            </span>
          ))}
        </div>
      )}

      {/* Conflict Proposal Subrow */}
      {isConflict && (
        <div className={styles.conflictSubrow}>
          <div className={styles.conflictInfo}>
            <ArrowsClockwise size={14} />
            <span>
              {proposedText
                ? t("glossary.conflict.proposed", { proposed: proposedText })
                : t("glossary.conflict.title")}
            </span>
          </div>

          <div className={styles.conflictActions}>
            {proposedText && (
              <Button
                size="sm"
                variant="secondary"
                onClick={() =>
                  acceptProposal(projectPath, entry, proposedText)
                }
              >
                {t("glossary.conflict.use_proposal")}
              </Button>
            )}
            <Button
              size="sm"
              variant="ghost"
              onClick={() => ignoreProposal(projectPath, entry)}
            >
              {t("glossary.conflict.ignore")}
            </Button>
          </div>
        </div>
      )}
    </div>
  );
};
