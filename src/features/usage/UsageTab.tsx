import React, { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { SegmentedControl, type SegmentOption } from "../../components/ui/SegmentedControl";
import { StateBlock } from "../../components/ui/StateBlock";
import { Button } from "../../components/ui/Button";
import { useWorkspaceStore } from "../workspace/store";
import { useUsageStore } from "./store";
import { formatNumber } from "../../lib/format";
import type { UsageGrouping } from "./types";
import styles from "./UsageTab.module.css";

function formatCost(cost: number | null | undefined): string {
  if (cost === null || cost === undefined) return "";
  if (cost === 0) return "$0.00";
  if (cost < 0.01) return `$${cost.toFixed(4)}`;
  return `$${cost.toFixed(3)}`;
}

export const UsageTab: React.FC = () => {
  const { t } = useTranslation();
  const { projectPath, runState } = useWorkspaceStore();
  const { grouping, usageData, loading, error, setGrouping, fetchUsage } = useUsageStore();

  const currentRunId = runState.latest_run?.id;

  useEffect(() => {
    if (projectPath) {
      void fetchUsage(projectPath, currentRunId ?? undefined);
    }
  }, [projectPath, currentRunId, fetchUsage]);

  const groupingOptions: SegmentOption<UsageGrouping>[] = [
    { value: "chapter", label: t("usage.grouping.chapter") },
    { value: "role", label: t("usage.grouping.role") },
    { value: "model", label: t("usage.grouping.model") },
  ];

  if (loading && !usageData) {
    return (
      <div className={styles.container}>
        <div className={styles.stateContainer}>
          <StateBlock title={t("usage.loading")} />
        </div>
      </div>
    );
  }

  if (error && !usageData) {
    return (
      <div className={styles.container}>
        <div className={styles.stateContainer}>
          <StateBlock
            title={t("usage.error_loading")}
            description={error}
            action={
              <Button
                variant="secondary"
                onClick={() => projectPath && fetchUsage(projectPath, currentRunId ?? undefined)}
              >
                {t("chapters.retry")}
              </Button>
            }
          />
        </div>
      </div>
    );
  }

  const summary = usageData?.summary;
  const isEmpty = !summary || summary.total_calls === 0;

  if (isEmpty) {
    return (
      <div className={styles.container}>
        <div className={styles.stateContainer}>
          <StateBlock
            title={t("usage.empty_title")}
            description={t("usage.empty_desc")}
          />
        </div>
      </div>
    );
  }

  const totalTokens =
    summary.total_input_tokens + summary.total_output_tokens;

  return (
    <div className={styles.container}>
      {/* Top Bar: Grouping selector & Summary metrics */}
      <div className={styles.topBar}>
        <SegmentedControl<UsageGrouping>
          options={groupingOptions}
          value={grouping}
          onChange={setGrouping}
          label="Pengelompokan pemakaian"
        />

        <div className={styles.summaryGroup}>
          <span className={styles.summaryMetric}>
            {t("usage.summary.total_calls", { count: formatNumber(summary.total_calls) })}
          </span>
          <span>·</span>
          <span className={styles.summaryMetric}>
            {t("usage.summary.total_tokens", { tokens: formatNumber(totalTokens) })}
          </span>
          {summary.total_cost !== null && summary.total_cost !== undefined && (
            <>
              <span>·</span>
              <span className={styles.summaryMetric}>
                {t("usage.summary.total_cost", { cost: formatCost(summary.total_cost).replace("$", "") })}
              </span>
            </>
          )}
          {summary.has_unknown_cost && (
            <span className={styles.unknownCostNote}>
              {t("usage.summary.unknown_cost_note")}
            </span>
          )}
        </div>
      </div>

      {/* Usage Data Table */}
      <div className={styles.tableWrapper}>
        <table className={styles.table}>
          <thead>
            {grouping === "chapter" && (
              <tr>
                <th className={styles.th} style={{ width: "3.5rem" }}>
                  {t("usage.columns.ordinal")}
                </th>
                <th className={styles.th}>{t("usage.columns.chapter")}</th>
                <th className={[styles.th, styles.thNumeric].join(" ")}>
                  {t("usage.columns.calls")}
                </th>
                <th className={[styles.th, styles.thNumeric].join(" ")}>
                  {t("usage.columns.input_tokens")}
                </th>
                <th className={[styles.th, styles.thNumeric].join(" ")}>
                  {t("usage.columns.cached_tokens")}
                </th>
                <th className={[styles.th, styles.thNumeric].join(" ")}>
                  {t("usage.columns.output_tokens")}
                </th>
                <th className={[styles.th, styles.thNumeric].join(" ")}>
                  {t("usage.columns.reasoning_tokens")}
                </th>
                <th className={[styles.th, styles.thNumeric].join(" ")}>
                  {t("usage.columns.cost")}
                </th>
              </tr>
            )}

            {grouping === "role" && (
              <tr>
                <th className={styles.th}>{t("usage.columns.role")}</th>
                <th className={[styles.th, styles.thNumeric].join(" ")}>
                  {t("usage.columns.calls")}
                </th>
                <th className={[styles.th, styles.thNumeric].join(" ")}>
                  {t("usage.columns.input_tokens")}
                </th>
                <th className={[styles.th, styles.thNumeric].join(" ")}>
                  {t("usage.columns.cached_tokens")}
                </th>
                <th className={[styles.th, styles.thNumeric].join(" ")}>
                  {t("usage.columns.output_tokens")}
                </th>
                <th className={[styles.th, styles.thNumeric].join(" ")}>
                  {t("usage.columns.reasoning_tokens")}
                </th>
                <th className={[styles.th, styles.thNumeric].join(" ")}>
                  {t("usage.columns.cost")}
                </th>
              </tr>
            )}

            {grouping === "model" && (
              <tr>
                <th className={styles.th}>{t("usage.columns.model")}</th>
                <th className={styles.th}>{t("usage.columns.pricing_source")}</th>
                <th className={[styles.th, styles.thNumeric].join(" ")}>
                  {t("usage.columns.calls")}
                </th>
                <th className={[styles.th, styles.thNumeric].join(" ")}>
                  {t("usage.columns.input_tokens")}
                </th>
                <th className={[styles.th, styles.thNumeric].join(" ")}>
                  {t("usage.columns.cached_tokens")}
                </th>
                <th className={[styles.th, styles.thNumeric].join(" ")}>
                  {t("usage.columns.output_tokens")}
                </th>
                <th className={[styles.th, styles.thNumeric].join(" ")}>
                  {t("usage.columns.reasoning_tokens")}
                </th>
                <th className={[styles.th, styles.thNumeric].join(" ")}>
                  {t("usage.columns.cost")}
                </th>
              </tr>
            )}
          </thead>

          <tbody>
            {grouping === "chapter" &&
              usageData.by_chapter.map((row) => (
                <tr key={row.chapter_id} className={styles.tr}>
                  <td className={[styles.td, styles.tdNumeric].join(" ")}>
                    {row.chapter_ordinal < 9999 ? row.chapter_ordinal : "-"}
                  </td>
                  <td className={styles.td}>{row.chapter_title}</td>
                  <td className={[styles.td, styles.tdNumeric].join(" ")}>
                    {formatNumber(row.call_count)}
                  </td>
                  <td className={[styles.td, styles.tdNumeric].join(" ")}>
                    {formatNumber(row.input_tokens)}
                  </td>
                  <td className={[styles.td, styles.tdNumeric].join(" ")}>
                    {formatNumber(row.cached_tokens)}
                  </td>
                  <td className={[styles.td, styles.tdNumeric].join(" ")}>
                    {formatNumber(row.output_tokens)}
                  </td>
                  <td className={[styles.td, styles.tdNumeric].join(" ")}>
                    {formatNumber(row.reasoning_tokens)}
                  </td>
                  <td className={[styles.td, styles.tdNumeric].join(" ")}>
                    {row.cost !== null && row.cost !== undefined ? (
                      formatCost(row.cost)
                    ) : (
                      <span className={styles.costMissing} title={t("usage.cost_missing")}>
                        {t("usage.cost_missing")}
                      </span>
                    )}
                  </td>
                </tr>
              ))}

            {grouping === "role" &&
              usageData.by_role.map((row) => {
                const roleKey = `usage.roles.${row.role}`;
                const localizedRole = t(roleKey, row.role);
                return (
                  <tr key={row.role} className={styles.tr}>
                    <td className={styles.td}>{localizedRole}</td>
                    <td className={[styles.td, styles.tdNumeric].join(" ")}>
                      {formatNumber(row.call_count)}
                    </td>
                    <td className={[styles.td, styles.tdNumeric].join(" ")}>
                      {formatNumber(row.input_tokens)}
                    </td>
                    <td className={[styles.td, styles.tdNumeric].join(" ")}>
                      {formatNumber(row.cached_tokens)}
                    </td>
                    <td className={[styles.td, styles.tdNumeric].join(" ")}>
                      {formatNumber(row.output_tokens)}
                    </td>
                    <td className={[styles.td, styles.tdNumeric].join(" ")}>
                      {formatNumber(row.reasoning_tokens)}
                    </td>
                    <td className={[styles.td, styles.tdNumeric].join(" ")}>
                      {row.cost !== null && row.cost !== undefined ? (
                        formatCost(row.cost)
                      ) : (
                        <span className={styles.costMissing} title={t("usage.cost_missing")}>
                          {t("usage.cost_missing")}
                        </span>
                      )}
                    </td>
                  </tr>
                );
              })}

            {grouping === "model" &&
              usageData.by_model.map((row) => {
                const sourceKey = row.pricing_source
                  ? `usage.sources.${row.pricing_source}`
                  : null;
                const sourceLabel = sourceKey && row.pricing_source
                  ? t(sourceKey, { defaultValue: row.pricing_source })
                  : "-";

                return (
                  <tr key={`${row.provider}_${row.model}`} className={styles.tr}>
                    <td className={[styles.td, styles.tdMono].join(" ")}>
                      {row.provider} / {row.model}
                    </td>
                    <td className={styles.td}>
                      {row.pricing_source ? (
                        <span className={styles.sourceBadge}>{sourceLabel}</span>
                      ) : (
                        "-"
                      )}
                    </td>
                    <td className={[styles.td, styles.tdNumeric].join(" ")}>
                      {formatNumber(row.call_count)}
                    </td>
                    <td className={[styles.td, styles.tdNumeric].join(" ")}>
                      {formatNumber(row.input_tokens)}
                    </td>
                    <td className={[styles.td, styles.tdNumeric].join(" ")}>
                      {formatNumber(row.cached_tokens)}
                    </td>
                    <td className={[styles.td, styles.tdNumeric].join(" ")}>
                      {formatNumber(row.output_tokens)}
                    </td>
                    <td className={[styles.td, styles.tdNumeric].join(" ")}>
                      {formatNumber(row.reasoning_tokens)}
                    </td>
                    <td className={[styles.td, styles.tdNumeric].join(" ")}>
                      {row.cost !== null && row.cost !== undefined ? (
                        formatCost(row.cost)
                      ) : (
                        <span className={styles.costMissing} title={t("usage.cost_missing")}>
                          {t("usage.cost_missing")}
                        </span>
                      )}
                    </td>
                  </tr>
                );
              })}

            {/* Bottom Grand Total Row */}
            <tr className={[styles.tr, styles.totalRow].join(" ")}>
              <td
                className={styles.td}
                colSpan={grouping === "chapter" ? 2 : grouping === "model" ? 2 : 1}
              >
                {t("usage.total")}
              </td>
              <td className={[styles.td, styles.tdNumeric].join(" ")}>
                {formatNumber(summary.total_calls)}
              </td>
              <td className={[styles.td, styles.tdNumeric].join(" ")}>
                {formatNumber(summary.total_input_tokens)}
              </td>
              <td className={[styles.td, styles.tdNumeric].join(" ")}>
                {formatNumber(summary.total_cached_tokens)}
              </td>
              <td className={[styles.td, styles.tdNumeric].join(" ")}>
                {formatNumber(summary.total_output_tokens)}
              </td>
              <td className={[styles.td, styles.tdNumeric].join(" ")}>
                {formatNumber(summary.total_reasoning_tokens)}
              </td>
              <td className={[styles.td, styles.tdNumeric].join(" ")}>
                {summary.total_cost !== null && summary.total_cost !== undefined ? (
                  formatCost(summary.total_cost)
                ) : (
                  <span className={styles.costMissing} title={t("usage.cost_missing")}>
                    {t("usage.cost_missing")}
                  </span>
                )}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  );
};
