import React from "react";
import { ArrowSquareOut } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { openUrl } from "@tauri-apps/plugin-opener";
import { isTauri } from "@tauri-apps/api/core";
import { Dialog } from "../../components/ui/Dialog";
import { Button } from "../../components/ui/Button";
import { useSettingsStore } from "./store";
import styles from "./AboutDialog.module.css";

export interface AboutDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

export const AboutDialog: React.FC<AboutDialogProps> = ({ isOpen, onClose }) => {
  const { t } = useTranslation();
  const appInfo = useSettingsStore((s) => s.appInfo);

  const version = appInfo?.version ?? "0.1.0";
  const license = appInfo?.license ?? "MIT + Commons Clause";
  const repoUrl = appInfo?.repo_url ?? "https://github.com/Rann27/lnta";

  const handleOpenRepo = async () => {
    if (isTauri()) {
      try {
        await openUrl(repoUrl);
        return;
      } catch {
        // fallback to window.open if plugin fails
      }
    }
    window.open(repoUrl, "_blank", "noopener,noreferrer");
  };

  return (
    <Dialog
      isOpen={isOpen}
      onClose={onClose}
      title={t("about.title")}
      actions={
        <Button variant="primary" onClick={onClose}>
          {t("common.close")}
        </Button>
      }
    >
      <div className={styles.content}>
        <div className={styles.headerInfo}>
          <h2 className={styles.appName}>LNTA</h2>
          <p className={styles.byLine}>{t("app.by")}</p>
        </div>

        <div className={styles.metaRow}>
          <div className={styles.metaItem}>
            <span className={styles.metaLabel}>{t("about.version")}</span>
            <span className={styles.metaValue}>v{version}</span>
          </div>

          <div className={styles.metaItem}>
            <span className={styles.metaLabel}>{t("about.license")}</span>
            <span className={styles.metaValue}>{license}</span>
          </div>
        </div>

        <Button
          variant="secondary"
          icon={<ArrowSquareOut size={16} />}
          onClick={handleOpenRepo}
          className={styles.repoButton}
        >
          {repoUrl}
        </Button>
      </div>
    </Dialog>
  );
};
