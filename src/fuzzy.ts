// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

/**
 * Scores how well `query` matches `text`; higher is better, null means no
 * match. Every whitespace-separated token must match, either as a substring
 * (strongly preferred, earlier and word-start hits score higher) or as an
 * in-order subsequence with gap penalties.
 */
export function fuzzyScore(query: string, text: string): number | null {
  const tokens = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (tokens.length === 0) return 0;
  const hay = text.toLowerCase();
  let total = 0;
  for (const token of tokens) {
    const score = scoreToken(token, hay);
    if (score === null) return null;
    total += score;
  }
  return total;
}

function scoreToken(token: string, hay: string): number | null {
  const idx = hay.indexOf(token);
  if (idx >= 0) {
    const wordStart = idx === 0 || /[\s\W_]/.test(hay[idx - 1]);
    return 1000 + token.length * 10 - Math.min(idx, 200) + (wordStart ? 150 : 0);
  }
  // Subsequence match, limited to a window so long texts stay cheap.
  const limit = Math.min(hay.length, 4000);
  let score = 0;
  let pos = 0;
  let last = -1;
  for (const ch of token) {
    let found = -1;
    for (let i = pos; i < limit; i++) {
      if (hay[i] === ch) {
        found = i;
        break;
      }
    }
    if (found < 0) return null;
    score += last >= 0 && found === last + 1 ? 12 : 4;
    if (last >= 0) score -= Math.min(found - last - 1, 20);
    last = found;
    pos = found + 1;
  }
  return score;
}
