import { it, expect } from "vitest";
import i18n, { setLanguage } from "./i18n";

it("formats plural counts per locale", async () => {
  await setLanguage("id");
  expect(i18n.t("usage.summary.total_calls", { count: 1234 })).toBe("1.234 panggilan");
  await setLanguage("en");
  expect(i18n.t("usage.summary.total_calls", { count: 1 })).toBe("1 call");
  expect(i18n.t("usage.summary.total_calls", { count: 1234 })).toBe("1,234 calls");
  expect(i18n.t("project_tab.image_pages_value", { count: 1 })).toBe("1 image page");
  await setLanguage("id");
});
