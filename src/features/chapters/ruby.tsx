import React from "react";

/**
 * Regex matching normalized Japanese ruby in memory:
 * CJK ideograph base followed by 《kana reading》 per FR-RAW-07.
 */
const RUBY_REGEX = /([\u4E00-\u9FFF\u3400-\u4DBF\uF900-\uFAFF々〆ヶ〇]+)《([^》]+)》/g;

/**
 * Renders normalized ruby text `漢字《かな》` using semantic `<ruby>` HTML tags.
 */
export function renderRuby(text: string): React.ReactNode {
  if (!text || !text.includes("《")) {
    return text;
  }

  const parts: React.ReactNode[] = [];
  let lastIndex = 0;
  const regex = new RegExp(RUBY_REGEX.source, "g");
  let match: RegExpExecArray | null;

  while ((match = regex.exec(text)) !== null) {
    if (match.index > lastIndex) {
      parts.push(text.slice(lastIndex, match.index));
    }

    const [fullMatch, base, reading] = match;
    parts.push(
      <ruby key={`${match.index}-${base}`}>
        {base}
        <rp>《</rp>
        <rt>{reading}</rt>
        <rp>》</rp>
      </ruby>
    );

    lastIndex = match.index + fullMatch.length;
  }

  if (lastIndex < text.length) {
    parts.push(text.slice(lastIndex));
  }

  return <>{parts}</>;
}
