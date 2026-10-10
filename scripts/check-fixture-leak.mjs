import fs from "node:fs";
import path from "node:path";
import { execSync } from "node:child_process";

// Japanese character range: Hiragana, Katakana, CJK Unified Ideographs
const JAPANESE_CHAR_REGEX = /[\u3040-\u309F\u30A0-\u30FF\u3400-\u4DBF\u4E00-\u9FFF\uF900-\uFAFF]/g;

export function countJapaneseChars(text) {
  const matches = text.match(JAPANESE_CHAR_REGEX);
  return matches ? matches.length : 0;
}

export function isImagePlaceholder(line) {
  const lower = line.toLowerCase();
  return (
    lower.includes("image placeholder") ||
    lower.includes("illustration placeholder") ||
    line.startsWith("![")
  );
}

export function collectFixtureLines(fixtureDir) {
  const lines = new Set();
  if (!fs.existsSync(fixtureDir)) return lines;

  function walk(dir) {
    const entries = fs.readdirSync(dir, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        walk(fullPath);
      } else if (entry.isFile() && /^\d+_.*\.md$/.test(entry.name)) {
        const content = fs.readFileSync(fullPath, "utf8");
        const fileLines = content.split(/\r?\n/);
        for (const line of fileLines) {
          const trimmed = line.trim();
          if (!trimmed || isImagePlaceholder(trimmed)) continue;
          if (countJapaneseChars(trimmed) >= 12) {
            lines.add(trimmed);
          }
        }
      }
    }
  }

  walk(fixtureDir);
  return lines;
}

export function findLeaks(fixtureLines, trackedFilesMap) {
  const leaks = [];
  if (fixtureLines.size === 0) return leaks;

  for (const [filePath, content] of Object.entries(trackedFilesMap)) {
    for (const line of fixtureLines) {
      if (content.includes(line)) {
        leaks.push({ file: filePath, line });
      }
    }
  }
  return leaks;
}

export function runTests() {
  console.log("Running check-fixture-leak self-tests on bad samples...");

  // Test placeholder skipping
  const placeholder = "Image PlaceHolder: 1_表紙イラストを描いた美麗な画像";
  if (!isImagePlaceholder(placeholder)) {
    throw new Error("Test failed: isImagePlaceholder should recognize placeholder lines");
  }

  // Test short Japanese count
  const shortJp = "こんにちは、世界！"; // 9 chars
  if (countJapaneseChars(shortJp) >= 12) {
    throw new Error("Test failed: countJapaneseChars should count strictly");
  }

  // Test 12+ Japanese chars
  const longJp = "この文章は十二文字以上の日本語を含んでいます。"; // 23 chars
  if (countJapaneseChars(longJp) < 12) {
    throw new Error("Test failed: countJapaneseChars should recognize 12+ Japanese chars");
  }

  // Test leak detection on bad sample
  const fixtureSet = new Set([longJp]);
  const trackedBad = {
    "src/example.ts": `// Some comment\nconst leaked = "${longJp}";\n`,
  };

  const leaks = findLeaks(fixtureSet, trackedBad);
  if (leaks.length === 0 || leaks[0].file !== "src/example.ts") {
    throw new Error("Test failed: findLeaks should detect leaked line in tracked file");
  }

  const trackedGood = {
    "src/example.ts": `// Some comment\nconst safe = "Safe text";\n`,
  };
  const noLeaks = findLeaks(fixtureSet, trackedGood);
  if (noLeaks.length !== 0) {
    throw new Error("Test failed: findLeaks should not flag safe files");
  }

  console.log("All check-fixture-leak self-tests passed!");
}

export function checkFixtureLeak(rootDir = process.cwd(), runner = execSync) {
  const fixtureDir = path.join(rootDir, "Fixture");
  if (!fs.existsSync(fixtureDir)) {
    console.log("Fixture/ directory not present; skipping fixture leak check.");
    return [];
  }

  const fixtureLines = collectFixtureLines(fixtureDir);
  if (fixtureLines.size === 0) {
    console.log("No Japanese lines with >= 12 characters found in Fixture/.");
    return [];
  }

  let trackedList = [];
  try {
    const stdout = runner("git ls-files", { encoding: "utf8", cwd: rootDir });
    trackedList = stdout.split(/\r?\n/).filter(Boolean);
  } catch (err) {
    return [`Failed to query git tracked files: ${err.message}`];
  }

  const trackedFilesMap = {};
  for (const relPath of trackedList) {
    const fullPath = path.join(rootDir, relPath);
    if (fs.existsSync(fullPath)) {
      try {
        trackedFilesMap[relPath] = fs.readFileSync(fullPath, "utf8");
      } catch {
        // Skip binary or unreadable files
      }
    }
  }

  const leaks = findLeaks(fixtureLines, trackedFilesMap);
  const errors = [];
  for (const leak of leaks) {
    errors.push(`Fixture text leaked into tracked file ${leak.file}: "${leak.line}"`);
  }
  return errors;
}

if (process.argv.includes("--test")) {
  try {
    runTests();
    process.exit(0);
  } catch (err) {
    console.error(err.message);
    process.exit(1);
  }
} else if (process.argv[1] && process.argv[1].endsWith("check-fixture-leak.mjs")) {
  const errors = checkFixtureLeak();
  if (errors.length > 0) {
    console.error("Fixture leak check FAILED:\n" + errors.join("\n"));
    process.exit(1);
  } else {
    console.log("Fixture leak check passed.");
  }
}
