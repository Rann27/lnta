import fs from "node:fs";
import path from "node:path";
import { execSync } from "node:child_process";

export function checkFixtureTracked(runner = execSync) {
  try {
    const stdout = runner("git ls-files Fixture", { encoding: "utf8" }).trim();
    if (stdout.length > 0) {
      return stdout.split(/\r?\n/).filter(Boolean);
    }
  } catch (err) {
    // If git is unavailable or command fails, treat as error if not in test
    throw err;
  }
  return [];
}

export function checkStorage(content) {
  const matches = [];
  const regex = /\b(localStorage|sessionStorage)\b/g;
  let m;
  while ((m = regex.exec(content)) !== null) {
    matches.push(m[1]);
  }
  return matches;
}

export function checkEmDash(content) {
  return content.includes("\u2014");
}

function stripComments(content) {
  return content
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/\/\/.*$/gm, "");
}

export function checkColorsAndUnits(content) {
  const stripped = stripComments(content);
  const violations = [];

  // Hex color: #123, #1234, #123456, #12345678
  const hexRegex = /#([0-9a-fA-F]{3,4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})\b/g;
  let m;
  while ((m = hexRegex.exec(stripped)) !== null) {
    violations.push(`hex: ${m[0]}`);
  }

  // RGB/RGBA/HSL/HSLA
  const rgbRegex = /\b(rgba?|hsla?)\s*\(/gi;
  while ((m = rgbRegex.exec(stripped)) !== null) {
    violations.push(`rgb/hsl: ${m[0]}`);
  }

  // px literals except 0px and 1px
  // Note: allows 0, 0px, 1px, and %
  const pxRegex = /(?<![\w-])(?!0px\b|1px\b)[+-]?\d+px\b/gi;
  while ((m = pxRegex.exec(stripped)) !== null) {
    violations.push(`px literal: ${m[0]}`);
  }

  return violations;
}

export function checkOutlineNone(content) {
  const stripped = stripComments(content);
  const violations = [];

  // Look for CSS blocks { ... } containing outline: none or outline: 0
  const blockRegex = /\{([^}]+)\}/g;
  let m;
  while ((m = blockRegex.exec(stripped)) !== null) {
    const block = m[1];
    if (/\boutline\s*:\s*(none|0)\b/i.test(block)) {
      // Must have focus ring replacement (box-shadow, --focus-ring, or token outline)
      if (!/(--focus|box-shadow|--color-focus)/i.test(block)) {
        violations.push(`outline: none without focus ring replacement`);
      }
    }
  }

  return violations;
}

/** Custom properties a stylesheet defines (`--name:`). */
export function definedVariables(content) {
  return new Set([...content.matchAll(/(--[a-zA-Z0-9-]+)\s*:/g)].map((m) => m[1]));
}

/** `var(--name)` references that nothing defines; such a declaration silently drops. */
export function checkUndefinedVariables(content, defined) {
  return [...content.matchAll(/var\(\s*(--[a-zA-Z0-9-]+)/g)]
    .map((m) => m[1])
    .filter((name) => !defined.has(name));
}

/** `--text-*` tokens are `font` shorthands; a longhand such as font-size drops them. */
export function checkTypeTokenLonghand(content) {
  return [...content.matchAll(/\bfont-(size|family|weight|style)\s*:\s*var\(--text-/g)].map(
    (m) => `font-${m[1]}: var(--text-*)`
  );
}

function getAllFiles(dir, extensions) {
  if (!fs.existsSync(dir)) return [];
  const results = [];
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name !== "node_modules" && entry.name !== "dist" && entry.name !== ".git") {
        results.push(...getAllFiles(fullPath, extensions));
      }
    } else if (extensions.some(ext => entry.name.endsWith(ext))) {
      results.push(fullPath);
    }
  }
  return results;
}

export function runGuards(rootDir = process.cwd()) {
  const errors = [];

  // Guard 1: Tracked Fixture files
  try {
    const trackedFixture = checkFixtureTracked();
    if (trackedFixture.length > 0) {
      errors.push(`Tracked files found under Fixture/:\n${trackedFixture.join("\n")}`);
    }
  } catch (err) {
    errors.push(`Failed to check tracked fixture files: ${err.message}`);
  }

  // Guard 2: localStorage / sessionStorage in src/
  const srcDir = path.join(rootDir, "src");
  const srcFiles = getAllFiles(srcDir, [".ts", ".tsx", ".js", ".jsx"]);
  for (const file of srcFiles) {
    const content = fs.readFileSync(file, "utf8");
    const storage = checkStorage(content);
    if (storage.length > 0) {
      errors.push(`Forbidden storage (${storage.join(", ")}) in ${path.relative(rootDir, file)}`);
    }
  }

  // Guard 3: U+2014 in src/locales/*.json and TSX string literals
  const localeDir = path.join(srcDir, "locales");
  const localeFiles = getAllFiles(localeDir, [".json"]);
  for (const file of localeFiles) {
    const content = fs.readFileSync(file, "utf8");
    if (checkEmDash(content)) {
      errors.push(`Em dash U+2014 found in locale file: ${path.relative(rootDir, file)}`);
    }
  }

  const tsxFiles = getAllFiles(srcDir, [".tsx"]);
  for (const file of tsxFiles) {
    const content = fs.readFileSync(file, "utf8");
    if (checkEmDash(content)) {
      errors.push(`Em dash U+2014 found in TSX file: ${path.relative(rootDir, file)}`);
    }
  }

  // Guard 4: Raw hex/rgb/px in .module.css and TSX
  const styledFiles = [
    ...getAllFiles(srcDir, [".module.css"]),
    ...tsxFiles,
  ];
  for (const file of styledFiles) {
    const content = fs.readFileSync(file, "utf8");
    const violations = checkColorsAndUnits(content);
    if (violations.length > 0) {
      errors.push(`Raw colors/units in ${path.relative(rootDir, file)}: ${violations.join(", ")}`);
    }

    const outlineViolations = checkOutlineNone(content);
    if (outlineViolations.length > 0) {
      errors.push(`Outline none without focus ring in ${path.relative(rootDir, file)}: ${outlineViolations.join(", ")}`);
    }
  }

  // Guard 5: every var(--x) in components must be a real token
  const defined = new Set();
  for (const file of getAllFiles(path.join(srcDir, "styles"), [".css"])) {
    for (const name of definedVariables(fs.readFileSync(file, "utf8"))) defined.add(name);
  }
  for (const file of [...getAllFiles(srcDir, [".css"]), ...tsxFiles]) {
    const content = fs.readFileSync(file, "utf8");
    const local = definedVariables(content);
    const longhand = checkTypeTokenLonghand(content);
    if (longhand.length > 0) {
      errors.push(`Type token in a longhand in ${path.relative(rootDir, file)}: use font: instead`);
    }
    const missing = checkUndefinedVariables(content, new Set([...defined, ...local]));
    if (missing.length > 0) {
      errors.push(
        `Undefined CSS variables in ${path.relative(rootDir, file)}: ${[...new Set(missing)].join(", ")}`
      );
    }
  }

  return errors;
}

export function runTests() {
  console.log("Running check-guards self-tests on bad samples...");

  // Test 1: Tracked Fixture
  const mockTracked = () => "Fixture/secret.txt\n";
  const tracked = checkFixtureTracked(mockTracked);
  if (tracked.length === 0 || tracked[0] !== "Fixture/secret.txt") {
    throw new Error("Test failed: checkFixtureTracked should detect tracked fixture files");
  }

  // Test 2: Storage
  const badStorageSample = "const x = window.localStorage.getItem('foo');";
  const storageResult = checkStorage(badStorageSample);
  if (storageResult.length === 0 || storageResult[0] !== "localStorage") {
    throw new Error("Test failed: checkStorage should detect localStorage");
  }

  // Test 3: Em dash
  const badEmDash = "Text with \u2014 em dash";
  if (!checkEmDash(badEmDash)) {
    throw new Error("Test failed: checkEmDash should detect U+2014");
  }

  // Test 4: Hex, rgb, px
  const badCssHex = ".btn { color: #ff0000; }";
  if (checkColorsAndUnits(badCssHex).length === 0) {
    throw new Error("Test failed: checkColorsAndUnits should detect hex #ff0000");
  }

  const badCssRgb = ".btn { background: rgba(0, 0, 0, 0.5); }";
  if (checkColorsAndUnits(badCssRgb).length === 0) {
    throw new Error("Test failed: checkColorsAndUnits should detect rgb/rgba");
  }

  const badCssPx = ".btn { padding: 16px; border: 1px solid var(--border); margin: 0; }";
  const pxViolations = checkColorsAndUnits(badCssPx);
  if (!pxViolations.some(v => v.includes("16px"))) {
    throw new Error("Test failed: checkColorsAndUnits should detect 16px but allow 1px and 0");
  }

  // Test 5: Outline none
  const badOutline = ".btn { outline: none; }";
  if (checkOutlineNone(badOutline).length === 0) {
    throw new Error("Test failed: checkOutlineNone should detect outline: none without focus ring");
  }

  const goodOutline = ".btn { outline: none; box-shadow: var(--focus-ring); }";
  if (checkOutlineNone(goodOutline).length !== 0) {
    throw new Error("Test failed: checkOutlineNone should accept outline: none when focus ring is specified");
  }

  // Test 6: Undefined CSS variables
  const tokens = definedVariables(":root { --color-text: #000; --space-2: 4px; }");
  const undefinedVars = checkUndefinedVariables(
    ".a { color: var(--color-text); box-shadow: var(--focus-ring); }",
    tokens
  );
  if (undefinedVars.length !== 1 || undefinedVars[0] !== "--focus-ring") {
    throw new Error("Test failed: checkUndefinedVariables should report --focus-ring only");
  }

  // Test 7: Type tokens used as font-size
  if (checkTypeTokenLonghand(".a { font-size: var(--text-small); }").length !== 1) {
    throw new Error("Test failed: checkTypeTokenLonghand should flag font-size: var(--text-small)");
  }
  if (checkTypeTokenLonghand(".a { font: var(--text-small); }").length !== 0) {
    throw new Error("Test failed: checkTypeTokenLonghand should accept the font shorthand");
  }

  console.log("All check-guards self-tests passed!");
}

if (process.argv.includes("--test")) {
  try {
    runTests();
    process.exit(0);
  } catch (err) {
    console.error(err.message);
    process.exit(1);
  }
} else if (process.argv[1] && process.argv[1].endsWith("check-guards.mjs")) {
  const errors = runGuards();
  if (errors.length > 0) {
    console.error("Guard check failed with errors:\n" + errors.join("\n"));
    process.exit(1);
  } else {
    console.log("Guard checks passed.");
  }
}
