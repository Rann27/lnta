import React, { useEffect, useState } from "react";
import { Outlet, Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { ipc } from "../lib/ipc";
import { setTheme, getTheme, type Theme } from "./theme";
import { setLanguage, getLanguage, type SupportedLanguage } from "../lib/i18n";
import { PreviewBanner } from "./PreviewBanner";
import styles from "./Shell.module.css";

export const Shell: React.FC = () => {
  const { t } = useTranslation();
  const [currentTheme, setCurrentThemeState] = useState<Theme>(getTheme());
  const [currentLang, setCurrentLangState] = useState<SupportedLanguage>(getLanguage());

  useEffect(() => {
    ipc
      .getGlobalSettings()
      .then((settings) => {
        if (settings.theme) {
          setTheme(settings.theme as Theme);
          setCurrentThemeState(settings.theme as Theme);
        }
        if (settings.ui_language) {
          void setLanguage(settings.ui_language as SupportedLanguage);
          setCurrentLangState(settings.ui_language as SupportedLanguage);
        }
      })
      .catch(() => {
        // use default fallback
      });
  }, []);

  const handleThemeChange = async (e: React.ChangeEvent<HTMLSelectElement>) => {
    const nextTheme = e.target.value as Theme;
    setTheme(nextTheme);
    setCurrentThemeState(nextTheme);
    try {
      const settings = await ipc.getGlobalSettings();
      await ipc.saveGlobalSettings({ ...settings, theme: nextTheme });
    } catch {
      // ignore in mock/fallback
    }
  };

  const handleLanguageChange = async (e: React.ChangeEvent<HTMLSelectElement>) => {
    const nextLang = e.target.value as SupportedLanguage;
    await setLanguage(nextLang);
    setCurrentLangState(nextLang);
    try {
      const settings = await ipc.getGlobalSettings();
      await ipc.saveGlobalSettings({ ...settings, ui_language: nextLang });
    } catch {
      // ignore in mock/fallback
    }
  };

  return (
    <div className={styles.shell}>
      <PreviewBanner />

      <header className={styles.header}>
        <Link to="/" className={styles.brand}>
          <span>LNTA</span>
        </Link>

        <div className={styles.headerActions}>
          <select
            aria-label={t("projects.language")}
            className={styles.select}
            value={currentLang}
            onChange={handleLanguageChange}
          >
            <option value="id">Bahasa Indonesia</option>
            <option value="en">English</option>
          </select>

          <select
            aria-label={t("projects.theme")}
            className={styles.select}
            value={currentTheme}
            onChange={handleThemeChange}
          >
            <option value="system">System</option>
            <option value="light">Light</option>
            <option value="dark">Dark</option>
          </select>
        </div>
      </header>

      <main className={styles.main}>
        <Outlet />
      </main>
    </div>
  );
};
