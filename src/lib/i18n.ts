import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import idCatalog from "../locales/id.json";
import enCatalog from "../locales/en.json";

export type SupportedLanguage = "id" | "en";

export const defaultNS = "translation";
export const resources = {
  id: { translation: idCatalog },
  en: { translation: enCatalog },
} as const;

i18n.use(initReactI18next).init({
  resources,
  lng: "id",
  fallbackLng: "en",
  interpolation: {
    escapeValue: false,
  },
});

export function setLanguage(lang: SupportedLanguage): Promise<void> {
  return i18n.changeLanguage(lang).then(() => {
    if (typeof document !== "undefined") {
      document.documentElement.lang = lang;
    }
  });
}

export function getLanguage(): SupportedLanguage {
  const current = i18n.language;
  return current && current.startsWith("en") ? "en" : "id";
}

export default i18n;
