import React from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { Warning, BookBookmark, FileDoc, Cpu } from "@phosphor-icons/react";
import { useWorkspaceStore } from "../store";
import styles from "./StatusBar.module.css";

export const StatusBar: React.FC = () => {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { providerStatus, docxStatus, glossaryCount, setActiveTab, retryDocxRender } =
    useWorkspaceStore();

  const isDocxLocked = docxStatus.status === "locked";
  const docxText = isDocxLocked
    ? t("workspace.statusbar.docx_locked")
    : docxStatus.lastRenderedTime
      ? t("workspace.statusbar.docx_rendered", { time: docxStatus.lastRenderedTime })
      : t("workspace.statusbar.docx_not_rendered");

  return (
    <footer className={styles.statusBar} role="contentinfo">
      <button
        type="button"
        className={[styles.item, !providerStatus.connected ? styles.warning : ""]
          .filter(Boolean)
          .join(" ")}
        onClick={() => navigate("/settings")}
        title={t("workspace.statusbar.provider_settings_title")}
      >
        {providerStatus.connected ? <Cpu size={14} /> : <Warning size={14} />}
        <span>
          {providerStatus.model
            ? `${providerStatus.name} · ${providerStatus.model}`
            : t("workspace.statusbar.provider_none")}
        </span>
      </button>

      {isDocxLocked ? (
        <button
          type="button"
          className={[styles.item, styles.warning].join(" ")}
          onClick={() => void retryDocxRender()}
          title={t("workspace.statusbar.docx_retry_title")}
        >
          <Warning size={14} />
          <span>{docxText}</span>
        </button>
      ) : (
        <span className={styles.item} style={{ cursor: "default" }}>
          <FileDoc size={14} />
          <span>{docxText}</span>
        </span>
      )}

      {glossaryCount !== null && (
        <button
          type="button"
          className={styles.item}
          onClick={() => setActiveTab("glossary")}
          title={t("workspace.statusbar.glossary_tab_title")}
        >
          <BookBookmark size={14} />
          <span>{t("workspace.statusbar.glossary_entries", { count: glossaryCount })}</span>
        </button>
      )}
    </footer>
  );
};
