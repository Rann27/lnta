import React, { useState } from "react";
import { HandPalm, Warning, Info, Check, ArrowsClockwise } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import type { PreflightItem, PreflightReport } from "../../lib/bindings";
import { Button } from "../../components/ui/Button";
import { Disclosure } from "../../components/ui/Disclosure";
import styles from "./PreflightChecklist.module.css";

export interface PreflightChecklistProps {
  report: PreflightReport | null;
  isLoading?: boolean;
  onRerun: () => void;
  onAction?: (actionId: string, item: PreflightItem) => void;
  /** Actions the host screen can carry out; others are hidden. All are shown when omitted. */
  availableActions?: ReadonlySet<string>;
}

const PROFILE_LABELS: Record<string, string> = { "ja-id": "JP→ID", "ja-en": "JP→EN" };

/** Shows profile ids in params the way the rest of the UI names them. */
function displayParams(params: Record<string, string>): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [key, value] of Object.entries(params)) {
    out[key] = key.endsWith("_profile") ? (PROFILE_LABELS[value] ?? value) : value;
  }
  return out;
}

export const PreflightChecklist: React.FC<PreflightChecklistProps> = ({
  report,
  isLoading = false,
  onRerun,
  onAction,
  availableActions,
}) => {
  const { t } = useTranslation();
  const [dismissed, setDismissed] = useState<ReadonlySet<string>>(new Set());

  if (isLoading) {
    return <div className={styles.checklist}>{t("wizard.preflight_checking")}</div>;
  }

  if (!report) {
    return null;
  }

  // A dismissed warning stays out of sight until the next preflight report.
  const visible = report.items.filter((i) => i.status === "blocker" || !dismissed.has(i.id));
  const blockers = visible.filter((i) => i.status === "blocker");
  const warnings = visible.filter((i) => i.status === "warning");
  const infos = visible.filter((i) => i.status === "info");
  const passed = visible.filter((i) => i.status === "pass");

  const renderItem = (item: PreflightItem) => {
    const actions = item.actions.filter(
      (a) => a.id === "dismiss" || !availableActions || availableActions.has(a.id)
    );
    let icon = <Check size={18} />;
    let iconClass = styles.iconPass;
    let itemClass = "";

    if (item.status === "blocker") {
      icon = <HandPalm size={18} />;
      iconClass = styles.iconBlocker;
      itemClass = styles.itemBlocker;
    } else if (item.status === "warning") {
      icon = <Warning size={18} />;
      iconClass = styles.iconWarning;
      itemClass = styles.itemWarning;
    } else if (item.status === "info") {
      icon = <Info size={18} />;
      iconClass = styles.iconInfo;
    }

    return (
      <div key={item.id} className={[styles.item, itemClass].filter(Boolean).join(" ")}>
        <div className={styles.itemLeft}>
          <span className={[styles.icon, iconClass].join(" ")} aria-hidden="true">
            {icon}
          </span>
          <div className={styles.content}>
            <span className={styles.itemTitle}>
              {t(`preflight_titles.${item.id}`, { defaultValue: item.id })}
            </span>
            <p className={styles.itemMessage}>
              {t(item.message_code, displayParams(item.params as Record<string, string>))}
            </p>
          </div>
        </div>

        {actions.length > 0 && (
          <div className={styles.actions}>
            {actions.map((act) => (
              <Button
                key={act.id}
                size="sm"
                variant="secondary"
                onClick={() => {
                  if (act.id === "dismiss") {
                    setDismissed((prev) => new Set(prev).add(item.id));
                    return;
                  }
                  onAction?.(act.id, item);
                }}
              >
                {t(act.label_code, act.id)}
              </Button>
            ))}
          </div>
        )}
      </div>
    );
  };

  return (
    <div className={styles.checklist}>
      <div className={styles.header}>
        <div className={styles.summary}>
          {blockers.length > 0 && (
            <span style={{ color: "var(--color-danger)" }}>
              {t("wizard.preflight_blockers_count", { count: blockers.length })}
            </span>
          )}
          {warnings.length > 0 && (
            <span style={{ color: "var(--color-warn)" }}>
              {t("wizard.preflight_warnings_count", { count: warnings.length })}
            </span>
          )}
          {infos.length > 0 && (
            <span style={{ color: "var(--color-text-3)" }}>
              {t("wizard.preflight_infos_count", { count: infos.length })}
            </span>
          )}
          {passed.length > 0 && (
            <span>
              {t("wizard.preflight_passed_count", { count: passed.length })}
            </span>
          )}
        </div>

        <Button
          size="sm"
          variant="secondary"
          icon={<ArrowsClockwise size={14} />}
          onClick={onRerun}
        >
          {t("wizard.preflight_rerun")}
        </Button>
      </div>

      {blockers.length > 0 && (
        <section className={[styles.group, styles.groupBlocker].join(" ")}>
          <h4 className={styles.groupTitle}>{t("wizard.blockers")}</h4>
          <div className={styles.list}>{blockers.map(renderItem)}</div>
        </section>
      )}

      {warnings.length > 0 && (
        <section className={[styles.group, styles.groupWarning].join(" ")}>
          <h4 className={styles.groupTitle}>{t("wizard.warnings")}</h4>
          <div className={styles.list}>{warnings.map(renderItem)}</div>
        </section>
      )}

      {infos.length > 0 && (
        <section className={[styles.group, styles.groupInfo].join(" ")}>
          <h4 className={styles.groupTitle}>{t("wizard.infos")}</h4>
          <div className={styles.list}>{infos.map(renderItem)}</div>
        </section>
      )}

      {passed.length > 0 && (
        <section className={[styles.group, styles.groupPass].join(" ")}>
          <Disclosure
            title={`${t("wizard.passed")} (${passed.length})`}
            defaultOpen={false}
          >
            <div className={styles.list} style={{ marginTop: "var(--space-2)" }}>
              {passed.map(renderItem)}
            </div>
          </Disclosure>
        </section>
      )}
    </div>
  );
};
