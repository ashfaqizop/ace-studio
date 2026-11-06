import { esc } from './html';

export interface Match {
  score: number;
  /** Matched character indices in the original text (for highlighting). */
  hits: number[];
}

/**
 * Fuzzy match `query` against `text`. Tries, in order: contiguous substring, in-order
 * subsequence (with bonuses for word starts and runs), then per-word typo tolerance
 * (Damerau-Levenshtein ≤ 1–2). Returns null when nothing matches.
 */
export function fuzzy(query: string, text: string): Match | null {
  const q = query.toLowerCase().trim();
  const t = text.toLowerCase();
  if (!q) return { score: 0, hits: [] };

  const at = t.indexOf(q);
  if (at >= 0) {
    const wordStart = at === 0 || /\W/.test(t[at - 1]);
    return { score: 100 + q.length * 3 + (wordStart ? 25 : 0) - at * 0.2, hits: range(at, q.length) };
  }

  const qs = q.replace(/\s+/g, '');
  const hits: number[] = [];
  let score = 0;
  let qi = 0;
  let prev = -2;
  for (let i = 0; i < t.length && qi < qs.length; i++) {
    if (t[i] !== qs[qi]) continue;
    const start = i === 0 || /\W/.test(t[i - 1]);
    score += 4 + (i === prev + 1 ? 6 : 0) + (start ? 8 : 0);
    hits.push(i);
    prev = i;
    qi++;
  }
  if (qi === qs.length) {
    const spread = hits[hits.length - 1] - hits[0];
    return { score: score - spread * 0.3, hits };
  }

  // typo tolerance: every query word must be close to some word in the text
  const words = [...t.matchAll(/[a-z0-9]+/g)];
  const typoHits: number[] = [];
  let typoScore = 0;
  for (const w of q.split(/\s+/)) {
    if (w.length < 3) return null;
    const max = w.length > 6 ? 2 : 1;
    let best: { d: number; m: RegExpMatchArray } | null = null;
    for (const m of words) {
      const d = dist(w, m[0].slice(0, Math.max(w.length, Math.min(m[0].length, w.length + max))));
      if (d <= max && (!best || d < best.d)) best = { d, m };
    }
    if (!best) return null;
    typoScore += 20 - best.d * 6;
    typoHits.push(...range(best.m.index!, best.m[0].length));
  }
  return { score: typoScore, hits: typoHits };
}

const range = (start: number, n: number) => Array.from({ length: n }, (_, i) => start + i);

/** Optimal string alignment (Damerau-Levenshtein) distance. */
function dist(a: string, b: string): number {
  const d = Array.from({ length: a.length + 1 }, (_, i) => [i, ...Array(b.length).fill(0)]);
  for (let j = 1; j <= b.length; j++) d[0][j] = j;
  for (let i = 1; i <= a.length; i++) {
    for (let j = 1; j <= b.length; j++) {
      const cost = a[i - 1] === b[j - 1] ? 0 : 1;
      d[i][j] = Math.min(d[i - 1][j] + 1, d[i][j - 1] + 1, d[i - 1][j - 1] + cost);
      if (i > 1 && j > 1 && a[i - 1] === b[j - 2] && a[i - 2] === b[j - 1]) d[i][j] = Math.min(d[i][j], d[i - 2][j - 2] + 1);
    }
  }
  return d[a.length][b.length];
}

/** Escape `text` and wrap matched characters in <mark>. */
export function highlight(text: string, hits: number[]): string {
  if (!hits.length) return esc(text);
  const set = new Set(hits);
  let out = '';
  let open = false;
  for (let i = 0; i < text.length; i++) {
    const on = set.has(i);
    if (on && !open) out += '<mark class="hl">';
    if (!on && open) out += '</mark>';
    open = on;
    out += esc(text[i]);
  }
  return out + (open ? '</mark>' : '');
}
