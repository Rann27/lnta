import React, { useState, useEffect, useId } from "react";
import { useTranslation } from "react-i18next";
import {
  Play,
  Pause,
  ArrowSquareOut,
  CaretDown,
} from "@phosphor-icons/react";
import { Button } from "../../../components/ui/Button";
import { Popover } from "../../../components/ui/Popover";
import { Tooltip } from "../../../components/ui/Tooltip";
import { PlanPreviewDialog } from "../PlanPreview/PlanPreviewDialog";
import { useWorkspaceStore } from "../store";
import styles from "./RunControl.module.css";

export const RunControl: React.FC = () => {
  const { t } = useTranslation();
  const {
    runState,
    preflight,
    selectedChapterId,
    startRun,
    translateChapterOnly,
    startQa,
    openDocx,
    pauseRun,
    resumeRun,
    stopRun,
    setActiveTab,
  } = useWorkspaceStore();

  const [menuOpen, setMenuOpen] = useState(false);
  const reasonId = useId();
  const [planPreviewOpen, setPlanPreviewOpen] = useState(false);

  const state = runState.state;
  const hasBlockers = preflight ? !preflight.can_run || preflight.items.some((i) => i.status === "blocker") : false;

  // Shortcuts: Ctrl+Shift+R (run/resume), Ctrl+. (pause)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.shiftKey && e.code === "KeyR") {
        e.preventDefault();
        if ((state === "idle" || state === "failed") && !hasBlockers) {
          void startRun();
        } else if (state === "paused") {
          resumeRun();
        }
      } else if (e.ctrlKey && e.code === "Period") {
        e.preventDefault();
        if (state === "running" || state === "qa_running") {
          pauseRun();
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [state, hasBlockers, startRun, pauseRun, resumeRun]);

  // Determine state configuration
  let mainLabel = t("workspace.run_control.run_volume");
  let mainVariant: "primary" | "secondary" = "primary";
  let mainDisabled = false;
  let mainIcon: React.ReactNode = <Play size={16} weight="fill" />;
  let mainAction: () => void | Promise<void> = () => startRun();
  let disabledReason: string | undefined = undefined;
  let showMenuButton = true;

  if ((state === "idle" || state === "failed") && hasBlockers) {
    mainVariant = "secondary";
    mainDisabled = true;
    disabledReason = t("workspace.run_control.preflight_blocker_reason");
  } else if (state === "running") {
    mainLabel = t("workspace.run_control.pause");
    mainVariant = "secondary";
    mainIcon = <Pause size={16} />;
    mainAction = () => pauseRun();
  } else if (state === "pausing") {
    mainLabel = t("workspace.run_control.pausing");
    mainVariant = "secondary";
    mainDisabled = true;
    mainIcon = <Pause size={16} />;
    showMenuButton = false;
  } else if (state === "paused") {
    mainLabel = t("workspace.run_control.resume_run");
    mainVariant = "primary";
    mainIcon = <Play size={16} weight="fill" />;
    mainAction = () => resumeRun();
  } else if (state === "qa_running") {
    mainLabel = t("workspace.run_control.pause_qa");
    mainVariant = "secondary";
    mainIcon = <Pause size={16} />;
    mainAction = () => pauseRun();
  } else if (state === "completed") {
    mainLabel = t("workspace.run_control.open_docx");
    mainVariant = "secondary";
    mainIcon = <ArrowSquareOut size={16} />;
    mainAction = () => openDocx();
  }

  const renderMenuItems = () => {
    if ((state === "idle" || state === "failed") && !hasBlockers) {
      return (
        <>
          <button
            type="button"
            className={styles.menuItem}
            disabled={!selectedChapterId}
            onClick={() => {
              setMenuOpen(false);
              if (selectedChapterId) void translateChapterOnly(selectedChapterId);
            }}
          >
            {t("workspace.run_control.translate_selected_only")}
          </button>
          <div className={styles.divider} />
          <button
            type="button"
            className={styles.menuItem}
            onClick={() => {
              setMenuOpen(false);
              setPlanPreviewOpen(true);
            }}
          >
            {t("workspace.run_control.view_plan")}
          </button>
        </>
      );
    }

    if ((state === "idle" || state === "failed") && hasBlockers) {
      return (
        <button
          type="button"
          className={styles.menuItem}
          onClick={() => {
            setMenuOpen(false);
            setActiveTab("project");
          }}
        >
          {t("workspace.run_control.run_preflight")}
        </button>
      );
    }

    if (state === "running") {
      return (
        <button
          type="button"
          className={`${styles.menuItem} ${styles.menuItemDanger}`}
          onClick={() => {
            setMenuOpen(false);
            stopRun();
          }}
        >
          {t("workspace.run_control.stop_now")}
        </button>
      );
    }

    if (state === "paused") {
      return (
        <button
          type="button"
          className={`${styles.menuItem} ${styles.menuItemDanger}`}
          onClick={() => {
            setMenuOpen(false);
            stopRun();
          }}
        >
          {t("workspace.run_control.stop_run")}
        </button>
      );
    }

    if (state === "qa_running") {
      return (
        <button
          type="button"
          className={`${styles.menuItem} ${styles.menuItemDanger}`}
          onClick={() => {
            setMenuOpen(false);
            stopRun();
          }}
        >
          {t("workspace.run_control.stop_qa")}
        </button>
      );
    }

    if (state === "completed") {
      return (
        <button
          type="button"
          className={styles.menuItem}
          onClick={() => {
            setMenuOpen(false);
            setActiveTab("qa");
            void startQa();
          }}
        >
          {t("workspace.run_control.rerun_qa")}
        </button>
      );
    }

    return null;
  };

  return (
    <div className={styles.runControl}>
      {disabledReason ? (
        // The reason lives in a tooltip: inline text would overflow the 48 px run bar.
        <Tooltip content={disabledReason}>
          <span tabIndex={0} aria-label={`${mainLabel}. ${disabledReason}`}>
            <Button
              variant={mainVariant}
              disabled
              icon={mainIcon}
              className={styles.mainBtn}
              aria-describedby={reasonId}
            >
              {mainLabel}
            </Button>
            <span id={reasonId} className={styles.srOnly}>
              {disabledReason}
            </span>
          </span>
        </Tooltip>
      ) : (
        <Button
          variant={mainVariant}
          disabled={mainDisabled}
          icon={mainIcon}
          className={styles.mainBtn}
          onClick={mainAction}
        >
          {mainLabel}
        </Button>
      )}

      {showMenuButton && (
        <Popover
          isOpen={menuOpen}
          onOpenChange={setMenuOpen}
          trigger={
            <Button
              variant={mainVariant}
              aria-label={t("workspace.run_control.more_options")}
              aria-haspopup="menu"
              aria-expanded={menuOpen}
              className={styles.menuTrigger}
              onClick={() => setMenuOpen(!menuOpen)}
            >
              <CaretDown size={14} />
            </Button>
          }
          content={
            <div className={styles.menu} role="menu">
              {renderMenuItems()}
            </div>
          }
        />
      )}

      <PlanPreviewDialog
        isOpen={planPreviewOpen}
        onClose={() => setPlanPreviewOpen(false)}
      />
    </div>
  );
};
