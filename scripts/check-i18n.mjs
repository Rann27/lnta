import fs from "node:fs";
import path from "node:path";

export function collectKeys(obj, prefix = "") {
  let keys = [];
  if (!obj || typeof obj !== "object" || Array.isArray(obj)) {
    return [prefix];
  }
  for (const [key, value] of Object.entries(obj)) {
    const nextPrefix = prefix ? `${prefix}.${key}` : key;
    if (value && typeof value === "object" && !Array.isArray(value)) {
      keys.push(...collectKeys(value, nextPrefix));
    } else {
      keys.push(nextPrefix);
    }
  }
  return keys;
}

export function compareKeys(idObj, enObj) {
  const idKeys = new Set(collectKeys(idObj));
  const enKeys = new Set(collectKeys(enObj));

  const missingInEn = [...idKeys].filter(k => !enKeys.has(k));
  const missingInId = [...enKeys].filter(k => !idKeys.has(k));

  return { missingInEn, missingInId };
}

export function runTests() {
  console.log("Running check-i18n self-tests on bad samples...");

  const badId = { common: { ok: "OK", cancel: "Batal" }, onlyId: "Khusus ID" };
  const badEn = { common: { ok: "OK" }, onlyEn: "Only EN" };

  const { missingInEn, missingInId } = compareKeys(badId, badEn);
  if (!missingInEn.includes("common.cancel") || !missingInEn.includes("onlyId")) {
    throw new Error("Test failed: compareKeys should find missing keys in EN");
  }
  if (!missingInId.includes("onlyEn")) {
    throw new Error("Test failed: compareKeys should find missing keys in ID");
  }

  const goodId = { app: { title: "LNTA" } };
  const goodEn = { app: { title: "LNTA" } };
  const goodResult = compareKeys(goodId, goodEn);
  if (goodResult.missingInEn.length > 0 || goodResult.missingInId.length > 0) {
    throw new Error("Test failed: compareKeys should pass on identical keys");
  }

  console.log("All check-i18n self-tests passed!");
}

export function checkI18n(rootDir = process.cwd()) {
  const idPath = path.join(rootDir, "src", "locales", "id.json");
  const enPath = path.join(rootDir, "src", "locales", "en.json");

  if (!fs.existsSync(idPath) && !fs.existsSync(enPath)) {
    console.log("Locales not yet created; skipping parity check.");
    return [];
  }

  if (!fs.existsSync(idPath)) {
    return ["src/locales/id.json is missing"];
  }
  if (!fs.existsSync(enPath)) {
    return ["src/locales/en.json is missing"];
  }

  try {
    const idData = JSON.parse(fs.readFileSync(idPath, "utf8"));
    const enData = JSON.parse(fs.readFileSync(enPath, "utf8"));

    const { missingInEn, missingInId } = compareKeys(idData, enData);
    const errors = [];
    if (missingInEn.length > 0) {
      errors.push(`Keys in id.json but missing in en.json:\n  ${missingInEn.join("\n  ")}`);
    }
    if (missingInId.length > 0) {
      errors.push(`Keys in en.json but missing in id.json:\n  ${missingInId.join("\n  ")}`);
    }
    return errors;
  } catch (err) {
    return [`Error parsing locale files: ${err.message}`];
  }
}

if (process.argv.includes("--test")) {
  try {
    runTests();
    process.exit(0);
  } catch (err) {
    console.error(err.message);
    process.exit(1);
  }
} else if (process.argv[1] && process.argv[1].endsWith("check-i18n.mjs")) {
  const errors = checkI18n();
  if (errors.length > 0) {
    console.error("i18n parity check failed:\n" + errors.join("\n"));
    process.exit(1);
  } else {
    console.log("i18n parity check passed.");
  }
}
