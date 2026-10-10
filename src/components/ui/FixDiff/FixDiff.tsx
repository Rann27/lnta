import React from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../Button";
import styles from "./FixDiff.module.css";

export interface FixDiffProps {
  id?: string;
  checkCode?: string;
  origin?: "system" | "mechanical" | "ai" | "rename" | string;
  appliedAt?: string;
  revertedAt?: string | null;
  textBefore: string;
  textAfter: string;
  locationLabel?: string;
  onUndo?: () => void;
  undoDisabled?: boolean;
}

interface DiffToken {
  type: "same" | "deleted" | "added";
  value: string;
}

/**
 * Basic word-level difference utility.
 */
function computeWordDiff(before: string, after: string): { beforeTokens: DiffToken[]; afterTokens: DiffToken[] } {
  const wordsBefore = before.split(/(\s+)/);
  const wordsAfter = after.split(/(\s+)/);

  // Simple token set check for highlighting
  const setAfter = new Set(wordsAfter.filter((w) => w.trim().length > 0));
  const setBefore = new Set(wordsBefore.filter((w) => w.trim().length > 0));

  const beforeTokens: DiffToken[] = wordsBefore.map((w) => {
    if (w.trim().length === 0) return { type: "same", value: w };
    return {
      type: setAfter.has(w) ? "same" : "deleted",
      value: w,
    };
  });

  const afterTokens: DiffToken[] = wordsAfter.map((w) => {
    if (w.trim().length === 0) return { type: "same", value: w };
    return {
      type: setBefore.has(w) ? "same" : "added",
      value: w,
    };
  });

  return { beforeTokens, afterTokens };
}

export const FixDiff: React.FC<FixDiffProps> = ({
  checkCode,
  origin,
  appliedAt,
  revertedAt,
  textBefore,
  textAfter,
  locationLabel,
  onUndo,
  undoDisabled = false,
}) => {
  const { t } = useTranslation();
  const isReverted = Boolean(revertedAt);
  const { beforeTokens, afterTokens } = computeWordDiff(textBefore, textAfter);

  return (
    <article
      className={[styles.container, isReverted ? styles.reverted : ""].filter(Boolean).join(" ")}
      aria-label={t("qa.fixDiff.label", { code: checkCode ?? "Fix" })}
    >
      <header className={styles.header}>
        <div className={styles.meta}>
          {checkCode && <span className={styles.badge}>{checkCode}</span>}
          {locationLabel && <span className={styles.location}>{locationLabel}</span>}
          {origin && (
            <span className={styles.origin}>
              {origin === "mechanical" || origin === "system"
                ? t("qa.origin.system")
                : origin === "rename"
                ? t("qa.origin.rename")
                : t("qa.origin.ai")}
            </span>
          )}
          {appliedAt && (
            <time className={styles.time} dateTime={appliedAt}>
              {new Date(appliedAt).toLocaleTimeString()}
            </time>
          )}
          {isReverted && (
            <span className={styles.badge}>
              {t("qa.status.reverted")}
            </span>
          )}
        </div>

        {onUndo && !isReverted && (
          <Button
            variant="secondary"
            size="sm"
            onClick={onUndo}
            disabled={undoDisabled}
          >
            {t("qa.action.undoFix")}
          </Button>
        )}
      </header>

      <div className={styles.content}>
        <div className={styles.row}>
          <span className={styles.rowLabel}>{t("qa.fixDiff.before")}</span>
          <p className={styles.rowText}>
            {beforeTokens.map((tok, i) =>
              tok.type === "deleted" ? (
                <span key={i} className={styles.deleted}>
                  <span className={styles.srOnly}>({t("qa.fixDiff.deleted")}: </span>
                  {tok.value}
                  <span className={styles.srOnly}>)</span>
                </span>
              ) : (
                <span key={i}>{tok.value}</span>
              )
            )}
          </p>
        </div>

        <div className={styles.row}>
          <span className={styles.rowLabel}>{t("qa.fixDiff.after")}</span>
          <p className={styles.rowText}>
            {afterTokens.map((tok, i) =>
              tok.type === "added" ? (
                <span key={i} className={styles.added}>
                  <span className={styles.srOnly}>({t("qa.fixDiff.added")}: </span>
                  {tok.value}
                  <span className={styles.srOnly}>)</span>
                </span>
              ) : (
                <span key={i}>{tok.value}</span>
              )
            )}
          </p>
        </div>
      </div>
    </article>
  );
};
