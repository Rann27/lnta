import { describe, it, expect } from "vitest";
import {
  checkFixtureTracked,
  checkStorage,
  checkEmDash,
  checkColorsAndUnits,
  checkOutlineNone,
} from "./check-guards.mjs";
import { compareKeys } from "./check-i18n.mjs";
import {
  isImagePlaceholder,
  countJapaneseChars,
  findLeaks,
} from "./check-fixture-leak.mjs";

describe("Guard checks on bad samples", () => {
  it("fails when fixture file is tracked in git", () => {
    const mockRunner = () => "Fixture/secret.md\n";
    const tracked = checkFixtureTracked(mockRunner);
    expect(tracked).toContain("Fixture/secret.md");
  });

  it("fails when localStorage or sessionStorage is used", () => {
    const badCode = "const data = window.localStorage.getItem('token');";
    expect(checkStorage(badCode)).toContain("localStorage");

    const badSession = "sessionStorage.setItem('key', 'val');";
    expect(checkStorage(badSession)).toContain("sessionStorage");
  });

  it("fails when em dash U+2014 is present", () => {
    const badText = "Kalimat dengan tanda \u2014 em dash.";
    expect(checkEmDash(badText)).toBe(true);
  });

  it("fails on raw hex color", () => {
    const badCss = ".card { color: #f43f5e; }";
    const violations = checkColorsAndUnits(badCss);
    expect(violations.some((v: string) => v.includes("#f43f5e"))).toBe(true);
  });

  it("fails on raw rgb/rgba color", () => {
    const badCss = ".card { background: rgba(0, 0, 0, 0.8); }";
    const violations = checkColorsAndUnits(badCss);
    expect(violations.some((v: string) => v.includes("rgba("))).toBe(true);
  });

  it("fails on raw px literals except 0 and 1px", () => {
    const badCss = ".box { margin: 12px; padding: 1px; border: 0; }";
    const violations = checkColorsAndUnits(badCss);
    expect(violations.some((v: string) => v.includes("12px"))).toBe(true);
    expect(violations.some((v: string) => v.includes("1px"))).toBe(false);
  });

  it("fails on outline: none without focus ring", () => {
    const badCss = ".btn { outline: none; }";
    expect(checkOutlineNone(badCss).length).toBeGreaterThan(0);

    const goodCss = ".btn { outline: none; box-shadow: var(--focus-ring); }";
    expect(checkOutlineNone(goodCss).length).toBe(0);
  });
});

describe("i18n parity check on bad samples", () => {
  it("detects keys missing in either catalog", () => {
    const idObj = { common: { save: "Simpan" }, onlyId: "ID" };
    const enObj = { common: { save: "Save" }, onlyEn: "EN" };
    const { missingInEn, missingInId } = compareKeys(idObj, enObj);
    expect(missingInEn).toContain("onlyId");
    expect(missingInId).toContain("onlyEn");
  });
});

describe("Fixture leak check on bad samples", () => {
  it("identifies placeholder lines to skip", () => {
    expect(isImagePlaceholder("Image PlaceHolder: 1_01.jpg")).toBe(true);
    expect(isImagePlaceholder("![cover](img.png)")).toBe(true);
    expect(isImagePlaceholder("普通の一行テキスト")).toBe(false);
  });

  it("counts Japanese characters accurately", () => {
    expect(countJapaneseChars("こんにちは世界")).toBe(7);
    expect(countJapaneseChars("これはテストです。十二文字以上の日本語文章。")).toBe(20);
  });

  it("detects leaked lines in tracked files", () => {
    const secretLine = "これはテストです。十二文字以上の日本語文章。";
    const fixtureLines = new Set([secretLine]);
    const trackedFiles = {
      "src/sample.ts": `const leak = "${secretLine}";`,
    };
    const leaks = findLeaks(fixtureLines, trackedFiles);
    expect(leaks).toHaveLength(1);
    expect(leaks[0].file).toBe("src/sample.ts");
  });
});
