import React from "react";
import { useTranslation } from "react-i18next";
import {
  Warning,
  BookBookmark,
  FileDoc,
  Cpu,
  Eye,
} from "@phosphor-icons/react";
import { useWorkspaceStore } from "../store";
import styles from "./StatusBar.module.css";

export const StatusBar: React.FC = () => {
  const { t } = useTranslation();
  const {
    providerStatus,
    docxStatus,
    glossaryCount,
    setActiveTab,
  } = useWorkspaceStore();

  const isDocxLocked = docxStatus.status === "locked";
  const docxText = isDocxLocked
    ? t("workspace.statusbar.docx_locked")
    : t("workspace.statusbar.docx_rendered", {
        time: docxStatus.lastRenderedTime || "10:32",
      });

  return (
    <footer className={styles.statusBar} role="contentinfo">
      {/* 1. Translator provider status */}
      <button
        type="button"
        className={[
          styles.item,
          !providerStatus.connected ? styles.warning : "",
        ]
          .filter(Boolean)
          .join(" ")}
        onClick={() => setActiveTab("project")}
        title="Buka pengaturan provider"
      >
        <Cpu size={14} />
        <span>
          {providerStatus.name} · {providerStatus.model}
        </span>
      </button>

      {/* 2. docx render status */}
      <button
        type="button"
        className={[
          styles.item,
          isDocxLocked ? styles.warning : "",
        ]
          .filter(Boolean)
          .join(" ")}
        onClick={() => {
          // If locked, retry render
        }}
        title={isDocxLocked ? "Klik untuk render ulang" : undefined}
      >
        {isDocxLocked ? <Warning size={14} /> : <FileDoc size={14} />}
        <span>{docxText}</span>
      </button>

      {/* 3. Glossary entries count */}
      <button
        type="button"
        className={styles.item}
        onClick={() => setActiveTab("glossary")}
        title="Buka tab Glosarium"
      >
        <BookBookmark size={14} />
        <span>
          {t("workspace.statusbar.glossary_entries", { count: glossaryCount })}
        </span>
      </button>

      {/* 4. Folder watcher status */}
      <span className={styles.item} style={{ cursor: "default" }}>
        <Eye size={14} />
        <span>{t("workspace.statusbar.watcher_active")}</span>
      </span>
    </footer>
  );
};
