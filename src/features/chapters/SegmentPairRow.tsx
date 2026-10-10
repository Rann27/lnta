import React from "react";
import { useTranslation } from "react-i18next";
import { PencilSimple, Image as ImageIcon } from "@phosphor-icons/react";
import { StatusTag } from "../../components/ui/StatusTag";
import { Tooltip } from "../../components/ui/Tooltip";
import { renderRuby } from "./ruby";
import type { SegmentPair } from "./types";
import styles from "./SegmentPairRow.module.css";

export interface SegmentPairRowProps {
  pair: SegmentPair;
  isSelected: boolean;
  targetLanguage?: string;
  onClick: () => void;
  onDoubleClick?: () => void;
}

export const SegmentPairRow: React.FC<SegmentPairRowProps> = ({
  pair,
  isSelected,
  targetLanguage = "id",
  onClick,
  onDoubleClick,
}) => {
  const { t } = useTranslation();

  if (pair.isFixed) {
    const firstSeg = pair.segments[0];
    const isScene = firstSeg?.kind === "scene";
    const isImageMissing = firstSeg?.kind === "image_missing";

    return (
      <div
        className={[styles.fixedRow, isSelected ? styles.selected : ""]
          .filter(Boolean)
          .join(" ")}
        onClick={onClick}
        onDoubleClick={onDoubleClick}
        role="row"
        aria-selected={isSelected}
        tabIndex={0}
      >
        <div className={styles.indexCell}>{pair.indexLabel}</div>
        {isScene ? (
          <div className={styles.sceneMarker}>
            {firstSeg?.src_text || "☆"}
          </div>
        ) : (
          <>
            <ImageIcon size={16} className={styles.fixedIcon} aria-hidden="true" />
            <span className={styles.fixedLabel}>
              {firstSeg?.image_file || firstSeg?.src_text || t("chapters.fixed.image")}
            </span>
            {isImageMissing && (
              <StatusTag status="blocked" label={t("chapters.status.missing")} />
            )}
          </>
        )}
      </div>
    );
  }

  return (
    <div
      className={[styles.row, isSelected ? styles.selected : ""]
        .filter(Boolean)
        .join(" ")}
      onClick={onClick}
      onDoubleClick={onDoubleClick}
      role="row"
      aria-selected={isSelected}
      tabIndex={0}
    >
      <div className={styles.indexCell}>
        <span>{pair.indexLabel}</span>
        {pair.isManual && (
          <Tooltip
            content={
              t("chapters.edited_manually") +
              (pair.updatedAt
                ? ` (${new Date(pair.updatedAt).toLocaleTimeString()})`
                : "")
            }
          >
            <span
              tabIndex={0}
              className={styles.manualIcon}
              aria-label={t("chapters.edited_manually")}
            >
              <PencilSimple size={12} aria-hidden="true" />
            </span>
          </Tooltip>
        )}
      </div>

      <div className={styles.sourceCell} lang="ja">
        {pair.isRange ? (
          <div className={styles.sourceStacked}>
            {pair.segments.map((s) => (
              <div key={s.id}>{renderRuby(s.src_text)}</div>
            ))}
          </div>
        ) : (
          renderRuby(pair.segments[0]?.src_text || "")
        )}
      </div>

      <div className={styles.targetCell} lang={targetLanguage}>
        {pair.translation ? (
          pair.translation.text
        ) : pair.isFailed ? (
          <span className={styles.failedText}>
            {t("chapters.validation_failed")}
          </span>
        ) : (
          <span className={styles.pendingText}>
            {t("chapters.not_translated")}
          </span>
        )}
      </div>

      <div className={styles.findingsCell}>
        {pair.isFailed && (
          <StatusTag status="blocked" label={t("chapters.status.failed")} />
        )}
        {pair.hasFindings && pair.findingMessage && (
          <StatusTag status="needs_attention" label={pair.findingMessage} />
        )}
      </div>
    </div>
  );
};
