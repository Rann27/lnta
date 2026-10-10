import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { isTauri } from "@tauri-apps/api/core";
import { Dialog } from "../../components/ui/Dialog";
import { Button } from "../../components/ui/Button";
import { PathField } from "../../components/ui/PathField";
import { ipc } from "../../lib/ipc";
import type { GlossaryImportPreview, GlossaryEntryInput } from "../../lib/bindings";
import styles from "./GlossaryImportDialog.module.css";

export interface GlossaryImportDialogProps {
  isOpen: boolean;
  onClose: () => void;
  projectPath: string;
  onImported: () => void;
}

export const GlossaryImportDialog: React.FC<GlossaryImportDialogProps> = ({
  isOpen,
  onClose,
  projectPath,
  onImported,
}) => {
  const { t } = useTranslation();
  const [sourcePath, setSourcePath] = useState("");
  const [preview, setPreview] = useState<GlossaryImportPreview | null>(null);
  const [isInspecting, setIsInspecting] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [resolutions, setResolutions] = useState<Record<string, "current" | "incoming">>({});

  const handleBrowse = async () => {
    try {
      if (isTauri()) {
        const selected = await open({
          directory: true,
          multiple: false,
          title: t("glossary.import_browse_title"),
        });
        if (selected && typeof selected === "string") {
          setSourcePath(selected);
          setPreview(null);
          setErrorMessage(null);
        }
      }
    } catch {
      // browser fallback
    }
  };

  const handleInspect = async () => {
    if (!sourcePath.trim()) return;
    setIsInspecting(true);
    setErrorMessage(null);
    try {
      const res = await ipc.inspectGlossaryImport(projectPath, sourcePath.trim());
      setPreview(res);
      // Default all conflicts to 'incoming'
      const initialRes: Record<string, "current" | "incoming"> = {};
      for (const c of res.conflicts) {
        initialRes[c.source] = "incoming";
      }
      setResolutions(initialRes);
    } catch (err: unknown) {
      setErrorMessage(
        err instanceof Error ? err.message : t("glossary.import_failed_inspect")
      );
    } finally {
      setIsInspecting(false);
    }
  };

  const handleApply = async () => {
    if (!preview) return;
    setIsApplying(true);
    setErrorMessage(null);
    try {
      const entriesToApply: GlossaryEntryInput[] = [
        ...preview.new_entries,
        ...preview.conflicts
          .filter((c) => resolutions[c.source] === "incoming")
          .map((c) => ({
            source: c.source,
            translation: c.incoming_translation,
            note: c.incoming_note,
          })),
      ];

      await ipc.applyGlossaryImport(projectPath, entriesToApply);
      onImported();
      handleClose();
    } catch (err: unknown) {
      setErrorMessage(
        err instanceof Error ? err.message : t("glossary.import_failed_apply")
      );
    } finally {
      setIsApplying(false);
    }
  };

  const handleClose = () => {
    setPreview(null);
    setErrorMessage(null);
    setIsInspecting(false);
    setIsApplying(false);
    onClose();
  };

  const renderContent = () => (
    <div className={styles.container}>
      <div className={styles.section}>
        <PathField
          label={t("glossary.import_source_path")}
          value={sourcePath}
          onChange={(val) => {
            setSourcePath(val);
            setPreview(null);
          }}
          onBrowse={handleBrowse}
          placeholder={t("glossary.import_path_placeholder")}
        />
        <Button
          size="sm"
          variant="secondary"
          onClick={handleInspect}
          disabled={!sourcePath.trim() || isInspecting}
        >
          {isInspecting ? t("glossary.import_inspecting") : t("glossary.import_inspect_btn")}
        </Button>
      </div>

      {errorMessage && <div className={styles.error}>{errorMessage}</div>}

      {preview && (
        <div className={styles.section}>
          <div className={styles.summaryBox}>
            <div className={styles.summaryRow}>
              <span>{t("glossary.import_total_incoming")}</span>
              <span className={styles.summaryValue}>{preview.total_incoming}</span>
            </div>
            <div className={styles.summaryRow}>
              <span>{t("glossary.import_new_added")}</span>
              <span className={styles.summaryValue}>{preview.added_count}</span>
            </div>
            <div className={styles.summaryRow}>
              <span>{t("glossary.import_identical_ignored")}</span>
              <span className={styles.summaryValue}>{preview.identical_count}</span>
            </div>
            <div className={styles.summaryRow}>
              <span>{t("glossary.import_conflicts_count")}</span>
              <span className={styles.summaryValue}>{preview.conflicts.length}</span>
            </div>
          </div>

          {preview.conflicts.length > 0 && (
            <div className={styles.conflictsList}>
              <h3 style={{ fontSize: "var(--font-size-sm)", margin: 0 }}>
                {t("glossary.import_conflicts_title")}
              </h3>
              {preview.conflicts.map((c) => {
                const choice = resolutions[c.source] ?? "incoming";
                return (
                  <div key={c.source} className={styles.conflictCard}>
                    <div className={styles.conflictHeader}>
                      <span className={styles.conflictSource}>{c.source}</span>
                      <span className={styles.conflictBadge}>
                        {t("glossary.import_conflict_badge")}
                      </span>
                    </div>

                    <div className={styles.optionsGroup}>
                      <label
                        className={[
                          styles.optionLabel,
                          choice === "current" ? styles.optionLabelActive : "",
                        ]
                          .filter(Boolean)
                          .join(" ")}
                      >
                        <input
                          type="radio"
                          name={`conflict_${c.source}`}
                          checked={choice === "current"}
                          onChange={() =>
                            setResolutions((prev) => ({ ...prev, [c.source]: "current" }))
                          }
                        />
                        <div className={styles.optionDetails}>
                          <span className={styles.optionTitle}>
                            {t("glossary.import_choice_current")}: {c.current_translation}
                          </span>
                          <span className={styles.optionSub}>
                            {c.current_note || t("glossary.no_note")}
                          </span>
                        </div>
                      </label>

                      <label
                        className={[
                          styles.optionLabel,
                          choice === "incoming" ? styles.optionLabelActive : "",
                        ]
                          .filter(Boolean)
                          .join(" ")}
                      >
                        <input
                          type="radio"
                          name={`conflict_${c.source}`}
                          checked={choice === "incoming"}
                          onChange={() =>
                            setResolutions((prev) => ({ ...prev, [c.source]: "incoming" }))
                          }
                        />
                        <div className={styles.optionDetails}>
                          <span className={styles.optionTitle}>
                            {t("glossary.import_choice_incoming")}: {c.incoming_translation}
                          </span>
                          <span className={styles.optionSub}>
                            {c.incoming_note || t("glossary.no_note")}
                          </span>
                        </div>
                      </label>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      )}
    </div>
  );

  return (
    <Dialog
      isOpen={isOpen}
      onClose={handleClose}
      title={t("glossary.import_dialog_title")}
      description={t("glossary.import_dialog_desc")}
      actions={
        <>
          <Button variant="secondary" onClick={handleClose}>
            {t("common.cancel")}
          </Button>
          <Button
            variant="primary"
            onClick={handleApply}
            disabled={!preview || isApplying}
          >
            {isApplying ? t("glossary.import_applying") : t("glossary.import_apply_btn")}
          </Button>
        </>
      }
    >
      {renderContent()}
    </Dialog>
  );
};
