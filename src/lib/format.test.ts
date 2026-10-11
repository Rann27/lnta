import { describe, it, expect } from "vitest";
import { formatNumber, parseNumber } from "./format";

describe("parseNumber", () => {
  it("reads Indonesian grouping and decimals", () => {
    expect(parseNumber("1.500", "id")).toBe(1500);
    expect(parseNumber("2.000.000", "id")).toBe(2000000);
    expect(parseNumber("1,5", "id")).toBe(1.5);
  });

  it("reads English grouping and decimals", () => {
    expect(parseNumber("1,500", "en")).toBe(1500);
    expect(parseNumber("2.75", "en")).toBe(2.75);
  });

  it("round-trips formatted values and rejects empty input", () => {
    expect(parseNumber(formatNumber(123456, "id"), "id")).toBe(123456);
    expect(parseNumber(formatNumber(123456, "en"), "en")).toBe(123456);
    expect(parseNumber("", "id")).toBeUndefined();
    expect(parseNumber("-", "en")).toBeUndefined();
  });
});
