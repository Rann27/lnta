import { describe, it, expect, beforeEach } from "vitest";
import i18n, { setLanguage, getLanguage } from "./i18n";
import { formatNumber, formatDuration, formatTime } from "./format";
import idCatalog from "../locales/id.json";
import enCatalog from "../locales/en.json";

describe("i18n infrastructure", () => {
  beforeEach(async () => {
    await setLanguage("id");
  });

  it("switches language at runtime without reload", async () => {
    expect(getLanguage()).toBe("id");
    expect(i18n.t("common.save")).toBe("Simpan");

    await setLanguage("en");
    expect(getLanguage()).toBe("en");
    expect(i18n.t("common.save")).toBe("Save");
  });

  it("contains errors.* namespace in both catalogs", () => {
    expect(idCatalog.errors).toBeDefined();
    expect(enCatalog.errors).toBeDefined();
    expect(idCatalog.errors.docx_locked).toContain("Translations.docx");
    expect(enCatalog.errors.docx_locked).toContain("Translations.docx");
  });
});

describe("Format helpers", () => {
  it("formats numbers per active language", () => {
    // 12480 -> "12.480" in id, "12,480" in en
    expect(formatNumber(12480, "id")).toBe("12.480");
    expect(formatNumber(12480, "en")).toBe("12,480");
  });

  it("formats duration per active language", () => {
    expect(formatDuration(48, "id")).toBe("48 d");
    expect(formatDuration(48, "en")).toBe("48 s");

    expect(formatDuration(192, "id")).toBe("3 m 12 d");
    expect(formatDuration(192, "en")).toBe("3 m 12 s");
  });

  it("formats time in 24-hour format", () => {
    const fixedDate = new Date("2026-10-10T14:35:00Z");
    const result = formatTime(fixedDate);
    expect(result).toMatch(/^\d{2}:\d{2}$/);
  });
});
