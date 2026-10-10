import React, { useEffect, useState } from "react";
import { ArrowLeft, Info } from "@phosphor-icons/react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/Button";
import { ProvidersSection } from "./ProvidersSection";
import { ModelsSection } from "./ModelsSection";
import { RolesSection } from "./RolesSection";
import { AboutDialog } from "./AboutDialog";
import { useSettingsStore } from "./store";
import styles from "./SettingsPage.module.css";

type SettingsTab = "providers" | "models" | "roles";

export const SettingsPage: React.FC = () => {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<SettingsTab>("providers");
  const [isAboutOpen, setIsAboutOpen] = useState(false);

  const { loadSettings, isLoading, error } = useSettingsStore();

  useEffect(() => {
    void loadSettings();
  }, [loadSettings]);

  return (
    <div className={styles.container}>
      <div className={styles.topBar}>
        <div className={styles.titleArea}>
          <Link to="/" className={styles.backLink}>
            <ArrowLeft size={16} />
            <span>{t("projects.title")}</span>
          </Link>
          <h1 className={styles.title}>{t("settings.title")}</h1>
          <p className={styles.subtitle}>{t("settings.subtitle")}</p>
        </div>

        <Button
          variant="secondary"
          icon={<Info size={16} />}
          onClick={() => setIsAboutOpen(true)}
        >
          {t("about.button")}
        </Button>
      </div>

      <nav className={styles.navTabs} aria-label={t("settings.nav_label")}>
        <button
          type="button"
          role="tab"
          aria-selected={activeTab === "providers"}
          className={[
            styles.tabButton,
            activeTab === "providers" ? styles.tabActive : "",
          ]
            .filter(Boolean)
            .join(" ")}
          onClick={() => setActiveTab("providers")}
        >
          {t("settings.tabs.providers")}
        </button>

        <button
          type="button"
          role="tab"
          aria-selected={activeTab === "models"}
          className={[
            styles.tabButton,
            activeTab === "models" ? styles.tabActive : "",
          ]
            .filter(Boolean)
            .join(" ")}
          onClick={() => setActiveTab("models")}
        >
          {t("settings.tabs.models")}
        </button>

        <button
          type="button"
          role="tab"
          aria-selected={activeTab === "roles"}
          className={[
            styles.tabButton,
            activeTab === "roles" ? styles.tabActive : "",
          ]
            .filter(Boolean)
            .join(" ")}
          onClick={() => setActiveTab("roles")}
        >
          {t("settings.tabs.roles")}
        </button>
      </nav>

      {error && (
        <div className={styles.errorBanner} role="alert">
          <span>{error}</span>
        </div>
      )}

      {isLoading ? (
        <div className={styles.loading}>{t("common.estimate")}...</div>
      ) : (
        <div className={styles.contentArea}>
          {activeTab === "providers" && <ProvidersSection />}
          {activeTab === "models" && <ModelsSection />}
          {activeTab === "roles" && <RolesSection />}
        </div>
      )}

      <AboutDialog isOpen={isAboutOpen} onClose={() => setIsAboutOpen(false)} />
    </div>
  );
};
