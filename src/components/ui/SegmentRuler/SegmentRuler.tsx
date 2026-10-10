import React, { useMemo, useRef, useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import styles from "./SegmentRuler.module.css";

export type SegmentStatus =
  | "done"
  | "live"
  | "paused"
  | "flag"
  | "fail"
  | "pending";

export interface SegmentItem {
  idx: number;
  kind?: string;
  status: SegmentStatus;
  isFixed?: boolean;
}

export interface SegmentRulerProps extends React.HTMLAttributes<HTMLDivElement> {
  variant?: "rail" | "header";
  segments: SegmentItem[];
  interactive?: boolean;
  onSelectSegment?: (segmentIndex: number) => void;
  maxBins?: number;
}

interface BinnedCell {
  startIdx: number;
  endIdx: number;
  status: SegmentStatus;
  isFixed: boolean;
}

const STATUS_PRIORITY: Record<SegmentStatus, number> = {
  fail: 6,
  flag: 5,
  live: 4,
  paused: 3,
  pending: 2,
  done: 1,
};

function resolveWorstStatus(items: SegmentItem[]): SegmentStatus {
  let worst: SegmentStatus = "done";
  let highestPri = 0;
  for (const item of items) {
    const pri = STATUS_PRIORITY[item.status] ?? 1;
    if (pri > highestPri) {
      highestPri = pri;
      worst = item.status;
    }
  }
  return worst;
}

export const SegmentRuler: React.FC<SegmentRulerProps> = ({
  variant = "rail",
  segments,
  interactive = false,
  onSelectSegment,
  maxBins,
  className,
  ...props
}) => {
  const { t } = useTranslation();
  const containerRef = useRef<HTMLDivElement>(null);
  const [measuredWidth, setMeasuredWidth] = useState<number>(0);

  useEffect(() => {
    if (typeof ResizeObserver === "undefined" || !containerRef.current) return;
    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        if (entry.contentRect.width > 0) {
          setMeasuredWidth(entry.contentRect.width);
        }
      }
    });
    observer.observe(containerRef.current);
    return () => observer.disconnect();
  }, []);

  const cells = useMemo<BinnedCell[]>(() => {
    if (!segments || segments.length === 0) return [];

    // Calculate max allowed bins based on track width / 3px per design doc
    // If not measured yet, fall back to maxBins prop or safe default (60)
    const effectiveWidth = measuredWidth > 0 ? measuredWidth : (maxBins ? maxBins * 3 : 180);
    const calculatedMaxBins = maxBins ?? Math.max(10, Math.floor(effectiveWidth / 3));

    if (segments.length <= calculatedMaxBins) {
      return segments.map((s) => ({
        startIdx: s.idx,
        endIdx: s.idx,
        status: s.status,
        isFixed: Boolean(s.isFixed || s.kind === "scene" || s.kind === "image" || s.kind === "image_missing"),
      }));
    }

    // Binning logic
    const binCount = calculatedMaxBins;
    const binned: BinnedCell[] = [];
    for (let i = 0; i < binCount; i++) {
      const start = Math.floor((i * segments.length) / binCount);
      const end = Math.floor(((i + 1) * segments.length) / binCount);
      const slice = segments.slice(start, end);
      if (slice.length === 0) continue;

      const allFixed = slice.every((s) => s.isFixed || s.kind === "scene" || s.kind === "image" || s.kind === "image_missing");
      const worst = resolveWorstStatus(slice);

      binned.push({
        startIdx: slice[0].idx,
        endIdx: slice[slice.length - 1].idx,
        status: worst,
        isFixed: allFixed,
      });
    }
    return binned;
  }, [segments, maxBins, measuredWidth]);

  const isInteractive = interactive || variant === "header";

  return (
    <div
      ref={containerRef}
      className={[
        styles.ruler,
        variant === "header" ? styles.variantHeader : "",
        className ?? "",
      ]
        .filter(Boolean)
        .join(" ")}
      aria-hidden={!isInteractive ? "true" : undefined}
      {...props}
    >
      {cells.map((cell) => {
        const isSingle = cell.startIdx === cell.endIdx;
        const segmentLabel = isSingle
          ? t("workspace.segment_single", {
              num: cell.startIdx,
              status: t(`status.${cell.status}`, cell.status),
              defaultValue: `Segmen ${cell.startIdx}: ${cell.status}`,
            })
          : t("workspace.segment_range", {
              from: cell.startIdx,
              to: cell.endIdx,
              status: t(`status.${cell.status}`, cell.status),
              defaultValue: `Segmen ${cell.startIdx}-${cell.endIdx}: ${cell.status}`,
            });

        const statusClass = cell.isFixed ? styles.fixed : styles[cell.status] ?? styles.pending;
        const key = `${cell.startIdx}-${cell.endIdx}`;

        if (isInteractive) {
          return (
            <button
              key={key}
              type="button"
              className={`${styles.cell} ${styles.cellButton} ${statusClass}`}
              aria-label={segmentLabel}
              title={segmentLabel}
              data-status={cell.status}
              onClick={() => onSelectSegment?.(cell.startIdx)}
            />
          );
        }

        return (
          <div
            key={key}
            className={`${styles.cell} ${statusClass}`}
            data-status={cell.status}
          />
        );
      })}
    </div>
  );
};
