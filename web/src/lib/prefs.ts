import { ACCENTS } from '../config';

export type Scheme = 'auto' | 'light' | 'dark';

export interface Prefs {
  scheme: Scheme;
  accent: string;
  variant: string;
  expressive: boolean;
  buddy: boolean;
  view: 'grid' | 'list';
  favorites: string[];
  recent: string[];
  greeted: boolean;
  /** The visitor's Pumpkin server version, for compatibility badges. */
  server: string | null;
  /** Up to 3 plugin slugs queued for side-by-side comparison. */
  compare: string[];
  /** ISO time of this visit and of the previous one ("new since your last visit"). */
  lastVisit: string | null;
  prevVisit: string | null;
}

const KEY = 'ace:prefs';
const DEFAULTS: Prefs = {
  scheme: 'auto',
  accent: ACCENTS[0].color,
  variant: 'vibrant',
  expressive: true,
  buddy: true,
  view: 'grid',
  favorites: [],
  recent: [],
  greeted: false,
  server: null,
  compare: [],
  lastVisit: null,
  prevVisit: null,
};

let state: Prefs = load();
const listeners = new Set<(p: Prefs) => void>();

function load(): Prefs {
  try {
    return { ...DEFAULTS, ...JSON.parse(localStorage.getItem(KEY) || '{}') };
  } catch {
    return { ...DEFAULTS };
  }
}

export const prefs = (): Readonly<Prefs> => state;

export function setPrefs(patch: Partial<Prefs>): void {
  state = { ...state, ...patch };
  try {
    localStorage.setItem(KEY, JSON.stringify(state));
  } catch {
    /* private mode: keep in memory */
  }
  applyTheme();
  listeners.forEach((fn) => fn(state));
}

export function onPrefs(fn: (p: Prefs) => void): () => void {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

export function isFavorite(slug: string): boolean {
  return state.favorites.includes(slug);
}

export function toggleFavorite(slug: string): boolean {
  const on = !isFavorite(slug);
  setPrefs({ favorites: on ? [...state.favorites, slug] : state.favorites.filter((s) => s !== slug) });
  return on;
}

export const MAX_COMPARE = 3;

export function toggleCompare(slug: string): 'added' | 'removed' | 'full' {
  if (state.compare.includes(slug)) {
    setPrefs({ compare: state.compare.filter((s) => s !== slug) });
    return 'removed';
  }
  if (state.compare.length >= MAX_COMPARE) return 'full';
  setPrefs({ compare: [...state.compare, slug] });
  return 'added';
}

/** Record this visit. A gap of 30+ minutes starts a new visit. */
export function markVisit(): void {
  const now = new Date();
  const last = state.lastVisit ? new Date(state.lastVisit) : null;
  if (!last) setPrefs({ lastVisit: now.toISOString() });
  else if (now.getTime() - last.getTime() > 30 * 60 * 1000) setPrefs({ prevVisit: state.lastVisit, lastVisit: now.toISOString() });
  else setPrefs({ lastVisit: now.toISOString() });
}

/** True when `iso` (a release date) is newer than the visitor's previous visit. */
export function isNew(iso: string): boolean {
  return !!state.prevVisit && new Date(iso).getTime() > new Date(state.prevVisit).getTime();
}

export function pushRecent(slug: string): void {
  setPrefs({ recent: [slug, ...state.recent.filter((s) => s !== slug)].slice(0, 8) });
}

/** The scheme actually in effect (resolves "auto"). */
export function effectiveScheme(): 'light' | 'dark' {
  if (state.scheme !== 'auto') return state.scheme;
  return matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

export function applyTheme(): void {
  const theme = document.getElementById('theme');
  if (!theme) return;
  theme.setAttribute('color', state.accent);
  theme.setAttribute('scheme', state.scheme);
  theme.setAttribute('variant', state.variant);
  theme.setAttribute('motion', state.expressive ? 'expressive' : 'standard');
  document.documentElement.dataset.scheme = effectiveScheme();
  document.querySelector('meta[name="theme-color"]')?.setAttribute('content', state.accent);
}

matchMedia('(prefers-color-scheme: dark)').addEventListener('change', applyTheme);
