import type { GlossaryEntry } from "../../lib/bindings";

export interface GlossaryLintIssue {
  kind: "macron" | "missing_gender" | "duplicate" | "near_variant";
  message: string;
}

const DEFAULT_GENDER_KEYWORDS = [
  "laki-laki",
  "perempuan",
  "pria",
  "wanita",
  "cowok",
  "cewek",
  "male",
  "female",
  "boy",
  "girl",
  "man",
  "woman",
];

export function normalizeLongVowels(str: string): string {
  return str
    .toLowerCase()
    .replace(/ou/g, "o")
    .replace(/uu/g, "u")
    .replace(/aa/g, "a")
    .replace(/ee/g, "e")
    .replace(/ii/g, "i")
    .replace(/[āâ]/g, "a")
    .replace(/[ēê]/g, "e")
    .replace(/[īî]/g, "i")
    .replace(/[ōô]/g, "o")
    .replace(/[ūû]/g, "u");
}

function levenshteinDistance(a: string, b: string): number {
  const m = a.length;
  const n = b.length;
  const dp: number[][] = Array.from({ length: m + 1 }, () => Array(n + 1).fill(0));

  for (let i = 0; i <= m; i++) dp[i][0] = i;
  for (let j = 0; j <= n; j++) dp[0][j] = j;

  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      if (a[i - 1].toLowerCase() === b[j - 1].toLowerCase()) {
        dp[i][j] = dp[i - 1][j - 1];
      } else {
        dp[i][j] = 1 + Math.min(dp[i - 1][j], dp[i][j - 1], dp[i - 1][j - 1]);
      }
    }
  }

  return dp[m][n];
}

export function isNearVariant(nameA: string, nameB: string): boolean {
  const normA = normalizeLongVowels(nameA);
  const normB = normalizeLongVowels(nameB);
  if (normA === normB && nameA.toLowerCase() !== nameB.toLowerCase()) {
    return true;
  }
  if (nameA.length >= 4 && nameB.length >= 4) {
    const dist = levenshteinDistance(nameA, nameB);
    return dist > 0 && dist <= 2;
  }
  return false;
}

export function lintGlossaryEntry(
  entry: GlossaryEntry,
  allEntries: GlossaryEntry[],
  genderKeywords: string[] = DEFAULT_GENDER_KEYWORDS
): GlossaryLintIssue[] {
  const issues: GlossaryLintIssue[] = [];

  // 1. Macron check on translation and note
  const MACRON_REGEX = /[āēīōūâêîôû]/i;
  if (MACRON_REGEX.test(entry.translation) || (entry.note && MACRON_REGEX.test(entry.note))) {
    issues.push({
      kind: "macron",
      message: "macron",
    });
  }

  // 2. Missing gender on Tier 1
  if (entry.tier === 1) {
    const noteText = (entry.note ?? "").toLowerCase();
    const hasGender = genderKeywords.some((kw) => noteText.includes(kw.toLowerCase()));
    if (!hasGender) {
      issues.push({
        kind: "missing_gender",
        message: "catatan gender kosong",
      });
    }
  }

  // 3. Duplicate source
  const hasDuplicate = allEntries.some(
    (other) =>
      other.id !== entry.id &&
      other.source.trim().toLowerCase() === entry.source.trim().toLowerCase()
  );
  if (hasDuplicate) {
    issues.push({
      kind: "duplicate",
      message: "duplikat",
    });
  }

  // 4. Near-variant name
  const variant = allEntries.find(
    (other) =>
      other.id !== entry.id && isNearVariant(entry.translation, other.translation)
  );
  if (variant) {
    issues.push({
      kind: "near_variant",
      message: `mirip dengan ${variant.translation}`,
    });
  }

  return issues;
}
