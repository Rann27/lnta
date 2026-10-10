import React, { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Dialog } from "../../../components/ui/Dialog";
import { Button } from "../../../components/ui/Button";
import { ModelPicker } from "../../../components/ui/ModelPicker";
import { StatusTag, type StatusType } from "../../../components/ui/StatusTag";
import { StateBlock } from "../../../components/ui/StateBlock";
import { formatNumber } from "../../../lib/format";
import { ipc } from "../../../lib/ipc";
import type { ProjectPlanPreview, ModelReference } from "../../../lib/bindings";
import { useWorkspaceStore } from "../store";
import styles from "./PlanPreviewDialog.module.css";

const mapStatus = (status: string): StatusType => {
  if (status === "done") return "done";
  if (status === "frozen") return "frozen";
  if (status === "running") return "running";
  if (status === "paused") return "paused";
  if (status === "needs_attention") return "needs_attention";
  if (status === "blocked") return "blocked";
  return "queued";
};

export interface PlanPreviewDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

export const PlanPreviewDialog: React.FC<PlanPreviewDialogProps> = ({
  isOpen,
  onClose,
}) => {
  const { t } = useTranslation();
  const { projectPath, providerStatus } = useWorkspaceStore();

  const [selectedModel, setSelectedModel] = useState<ModelReference | null>(null);
  const [plan, setPlan] = useState<ProjectPlanPreview | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [expandedChapterIds, setExpandedChapterIds] = useState<Set<string>>(new Set());

  const fetchPlan = useCallback(
    async (modelRef?: ModelReference | null) => {
      if (!projectPath) return;
      setLoading(true);
      setError(null);
      try {
        const preview = await ipc.getPlanPreview(projectPath, modelRef ?? selectedModel);
        setPlan(preview);
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err);
        setError(msg);
      } finally {
        setLoading(false);
      }
    },
    [projectPath, selectedModel]
  );

  useEffect(() => {
    if (isOpen) {
      fetchPlan(selectedModel);
    }
  }, [isOpen, selectedModel, fetchPlan]);

  const toggleChapterExpand = (chapterId: string) => {
    setExpandedChapterIds((prev) => {
      const next = new Set(prev);
      if (next.has(chapterId)) {
        next.delete(chapterId);
      } else {
        next.add(chapterId);
      }
      return next;
    });
  };

  const handleModelChange = (modelRef: ModelReference) => {
    setSelectedModel(modelRef);
    fetchPlan(modelRef);
  };

  const formatCost = (cost: number | null | undefined): string => {
    if (cost === null || cost === undefined) {
      return "-";
    }
    return `~$${cost.toFixed(3)} (${t("workspace.plan_preview.estimate_tag")})`;
  };

  return (
    <Dialog
      isOpen={isOpen}
      onClose={onClose}
      title={t("workspace.plan_preview.title")}
      description={t("workspace.plan_preview.description")}
    >
      <div className={styles.container}>
        <div className={styles.topSection}>
          <div className={styles.modelRow}>
            <div className={styles.modelPickerWrapper}>
              <ModelPicker
                label={t("workspace.plan_preview.model_selector_label")}
                placeholder={providerStatus?.model || t("workspace.plan_preview.model_selector_placeholder")}
                value={selectedModel}
                onChange={handleModelChange}
              />
            </div>
          </div>

          {plan && (
            <div className={styles.ratioBanner}>
              {plan.is_learned_ratio ? (
                <span>
                  {t("workspace.plan_preview.ratio_learned", {
                    count: plan.completed_chapters_count,
                    inRatio: plan.chars_per_input_token,
                    outRatio: plan.chars_per_output_token,
                  })}
                </span>
              ) : (
                <span>
                  {t("workspace.plan_preview.ratio_default", {
                    inRatio: plan.chars_per_input_token,
                    outRatio: plan.chars_per_output_token,
                  })}
                </span>
              )}
            </div>
          )}
        </div>

        {loading && !plan && (
          <StateBlock
            title={t("workspace.plan_preview.loading")}
            description={t("workspace.plan_preview.description")}
          />
        )}

        {error && (
          <StateBlock
            title={t("workspace.plan_preview.error_loading", { message: error })}
            action={
              <Button variant="secondary" onClick={() => fetchPlan(selectedModel)}>
                {t("common.retry", "Coba lagi")}
              </Button>
            }
          />
        )}

        {plan && (
          <>
            <div className={styles.metricsGrid}>
              <div className={styles.metricCard}>
                <span className={styles.metricLabel}>{t("workspace.plan_preview.total_chapters")}</span>
                <span className={styles.metricValue}>{plan.total_chapters}</span>
              </div>
              <div className={styles.metricCard}>
                <span className={styles.metricLabel}>{t("workspace.plan_preview.total_parts")}</span>
                <span className={styles.metricValue}>{plan.total_parts}</span>
              </div>
              <div className={styles.metricCard}>
                <span className={styles.metricLabel}>{t("workspace.plan_preview.estimated_input")}</span>
                <span className={styles.metricValue}>
                  ~{formatNumber(plan.total_input_tokens)}
                  <span className={styles.metricEstimateTag}>({t("workspace.plan_preview.estimate_tag")})</span>
                </span>
              </div>
              <div className={styles.metricCard}>
                <span className={styles.metricLabel}>{t("workspace.plan_preview.estimated_output")}</span>
                <span className={styles.metricValue}>
                  ~{formatNumber(plan.total_output_tokens)}
                  <span className={styles.metricEstimateTag}>({t("workspace.plan_preview.estimate_tag")})</span>
                </span>
              </div>
              <div className={styles.metricCard}>
                <span className={styles.metricLabel}>{t("workspace.plan_preview.estimated_cost")}</span>
                <span className={styles.metricValue}>
                  {plan.has_cost_estimate && plan.estimated_total_cost !== null ? (
                    <>
                      ~${plan.estimated_total_cost.toFixed(3)}
                      <span className={styles.metricEstimateTag}>({t("workspace.plan_preview.estimate_tag")})</span>
                    </>
                  ) : (
                    t("workspace.plan_preview.cost_unknown")
                  )}
                </span>
              </div>
            </div>

            {plan.chapters.length === 0 ? (
              <StateBlock
                title={t("workspace.plan_preview.empty_chapters")}
                description={t("workspace.plan_preview.empty_action")}
              />
            ) : (
              <div className={styles.tableWrapper}>
                <table className={styles.table}>
                  <thead>
                    <tr>
                      <th className={styles.th}>{t("workspace.plan_preview.th_chapter")}</th>
                      <th className={styles.th}>{t("workspace.plan_preview.th_parts")}</th>
                      <th className={styles.th}>{t("workspace.plan_preview.th_length")}</th>
                      <th className={styles.th}>{t("workspace.plan_preview.th_est_input")}</th>
                      <th className={styles.th}>{t("workspace.plan_preview.th_est_output")}</th>
                      <th className={styles.th}>{t("workspace.plan_preview.th_est_cost")}</th>
                      <th className={styles.th}>{t("workspace.plan_preview.th_status")}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {plan.chapters.map((ch) => {
                      const isExpanded = expandedChapterIds.has(ch.chapter_id);
                      const hasMultiParts = ch.parts.length > 1;

                      return (
                        <React.Fragment key={ch.chapter_id}>
                          <tr
                            className={`${styles.tr} ${hasMultiParts ? styles.trExpandable : ""}`}
                            onClick={() => {
                              if (hasMultiParts) {
                                toggleChapterExpand(ch.chapter_id);
                              }
                            }}
                          >
                            <td className={styles.td}>
                              <strong>{ch.title || ch.file_name}</strong>
                            </td>
                            <td className={styles.td}>
                              {t("workspace.plan_preview.parts_count", { count: ch.part_count })}
                            </td>
                            <td className={styles.td}>
                              {t("workspace.plan_preview.length_format", {
                                lines: formatNumber(ch.line_count),
                                chars: formatNumber(ch.char_count),
                              })}
                            </td>
                            <td className={`${styles.td} ${styles.tdMono}`}>
                              ~{formatNumber(ch.estimated_input_tokens)}
                              <span className={styles.tagEstimate}>
                                ({t("workspace.plan_preview.estimate_tag")})
                              </span>
                            </td>
                            <td className={`${styles.td} ${styles.tdMono}`}>
                              ~{formatNumber(ch.estimated_output_tokens)}
                              <span className={styles.tagEstimate}>
                                ({t("workspace.plan_preview.estimate_tag")})
                              </span>
                            </td>
                            <td className={`${styles.td} ${styles.tdMono}`}>
                              {formatCost(ch.estimated_cost)}
                            </td>
                            <td className={styles.td}>
                              <StatusTag
                                status={mapStatus(ch.status)}
                                label={ch.status}
                              />
                            </td>
                          </tr>

                          {hasMultiParts && isExpanded && (
                            <tr>
                              <td colSpan={7} style={{ padding: 0 }}>
                                <div className={styles.partsBreakdown}>
                                  {ch.parts.map((p) => (
                                    <div key={p.part_index} className={styles.partRow}>
                                      <span>
                                        {t("workspace.plan_preview.part_label", { index: p.part_index })}:{" "}
                                        {t("workspace.plan_preview.part_range", {
                                          start: p.line_start,
                                          end: p.line_end,
                                          segments: p.segment_count,
                                        })}
                                      </span>
                                      <div className={styles.partTokens}>
                                        <span>
                                          In: ~{formatNumber(p.estimated_input_tokens)} (
                                          {t("workspace.plan_preview.estimate_tag")})
                                        </span>
                                        <span>
                                          Out: ~{formatNumber(p.estimated_output_tokens)} (
                                          {t("workspace.plan_preview.estimate_tag")})
                                        </span>
                                      </div>
                                    </div>
                                  ))}
                                </div>
                              </td>
                            </tr>
                          )}
                        </React.Fragment>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            )}
          </>
        )}

        <div className={styles.actions}>
          <Button variant="secondary" onClick={onClose}>
            {t("workspace.plan_preview.close")}
          </Button>
        </div>
      </div>
    </Dialog>
  );
};
