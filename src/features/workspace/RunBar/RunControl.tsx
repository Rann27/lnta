import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import {
  Play,
  Pause,
  ArrowSquareOut,
  CaretDown,
} from "@phosphor-icons/react";
import { Button } from "../../../components/ui/Button";
import { Popover } from "../../../components/ui/Popover";
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
    pauseRun,
    resumeRun,
    stopRun,
    setActiveTab,
  } = useWorkspaceStore();

  const [menuOpen, setMenuOpen] = useState(false);
  const [planPreviewOpen, setPlanPreviewOpen] = useState(false);

  const state = runState.state;
  const hasBlockers = preflight ? !preflight.can_run || preflight.items.some((i) => i.status === "blocker") : false;

  // Shortcuts: Ctrl+Shift+R (run/resume), Ctrl+. (pause)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.shiftKey && e.code === "KeyR") {
        e.preventDefault();
        if (state === "idle" && !hasBlockers) {
          startRun();
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

  if (state === "idle" && hasBlockers) {
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
    mainAction = () => {
      // In dev/tauri, opens docx or triggers open
    };
  }

  const renderMenuItems = () => {
    if (state === "idle" && !hasBlockers) {
      return (
        <>
          <button
            type="button"
            className={styles.menuItem}
            onClick={() => {
              setMenuOpen(false);
              startRun(selectedChapterId ?? undefined);
            }}
          >
            {t("workspace.run_control.start_from_selected")}
          </button>
          <button
            type="button"
            className={styles.menuItem}
            onClick={() => {
              setMenuOpen(false);
              startRun(selectedChapterId ?? undefined);
            }}
          >
            {t("workspace.run_control.selected_only")}
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

    if (state === "idle" && hasBlockers) {
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
      <Button
        variant={mainVariant}
        disabled={mainDisabled}
        disabledReason={disabledReason}
        icon={mainIcon}
        className={styles.mainBtn}
        onClick={mainAction}
      >
        {mainLabel}
      </Button>

      {showMenuButton && (
        <Popover
          isOpen={menuOpen}
          onOpenChange={setMenuOpen}
          trigger={
            <Button
              variant={mainVariant}
              aria-label="Pilihan run lainnya"
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
