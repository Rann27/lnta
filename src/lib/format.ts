import { getLanguage, type SupportedLanguage } from "./i18n";

export function formatNumber(value: number, lang?: SupportedLanguage): string {
  const activeLang = lang ?? getLanguage();
  const locale = activeLang === "id" ? "id-ID" : "en-US";
  return new Intl.NumberFormat(locale).format(value);
}

export function formatDuration(seconds: number, lang?: SupportedLanguage): string {
  const activeLang = lang ?? getLanguage();
  const secUnit = activeLang === "id" ? "d" : "s";
  const minUnit = "m";
  const hrUnit = activeLang === "id" ? "j" : "h";

  const totalSec = Math.max(0, Math.floor(seconds));
  if (totalSec < 60) {
    return `${totalSec} ${secUnit}`;
  }

  const hours = Math.floor(totalSec / 3600);
  const minutes = Math.floor((totalSec % 3600) / 60);
  const remSec = totalSec % 60;

  if (hours > 0) {
    return minutes > 0 ? `${hours} ${hrUnit} ${minutes} ${minUnit}` : `${hours} ${hrUnit}`;
  }

  return remSec > 0 ? `${minutes} ${minUnit} ${remSec} ${secUnit}` : `${minutes} ${minUnit}`;
}

export function formatTime(date: Date | string | number): string {
  const d = typeof date === "object" && date instanceof Date ? date : new Date(date);
  const hours = String(d.getHours()).padStart(2, "0");
  const minutes = String(d.getMinutes()).padStart(2, "0");
  return `${hours}:${minutes}`;
}

export function formatDateTime(date: Date | string | number, lang?: SupportedLanguage): string {
  const activeLang = lang ?? getLanguage();
  const locale = activeLang === "id" ? "id-ID" : "en-US";
  const d = typeof date === "object" && date instanceof Date ? date : new Date(date);
  return new Intl.DateTimeFormat(locale, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).format(d);
}
