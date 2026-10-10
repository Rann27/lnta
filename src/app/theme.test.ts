import { describe, it, expect, beforeEach } from "vitest";
import { getTheme, setTheme } from "./theme";

describe("Theme management", () => {
  beforeEach(() => {
    document.documentElement.removeAttribute("data-theme");
  });

  it("defaults to system when no data-theme attribute is present", () => {
    expect(getTheme()).toBe("system");
    expect(document.documentElement.hasAttribute("data-theme")).toBe(false);
  });

  it("sets light theme attribute", () => {
    setTheme("light");
    expect(getTheme()).toBe("light");
    expect(document.documentElement.getAttribute("data-theme")).toBe("light");
  });

  it("sets dark theme attribute", () => {
    setTheme("dark");
    expect(getTheme()).toBe("dark");
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
  });

  it("removes attribute when switching back to system", () => {
    setTheme("dark");
    setTheme("system");
    expect(getTheme()).toBe("system");
    expect(document.documentElement.hasAttribute("data-theme")).toBe(false);
  });
});
