import type { CharRef } from './crew';
import { plugins as demoPlugins } from 'virtual:demo-data';

export interface PluginVersion {
  version: string;
  date: string;
  pumpkin: string;
  minecraft: string;
  file: string;
  size: number;
  changelog: string;
  channel?: 'release' | 'beta';
}

export interface Plugin {
  slug: string;
  name: string;
  summary: string;
  description: string;
  categories: string[];
  /** m3e-shape + Material Symbol used as the plugin's icon. */
  icon: { shape: string; symbol: string };
  license: string;
  /** `icon`: the original's Modrinth icon URL (shown in Credits). */
  original: { name: string; author: string; url?: string; icon?: string; license: string };
  portedBy: CharRef;
  downloads: number;
  followers: number;
  featured?: boolean;
  /** Pumpkin WASM capabilities the plugin requests (see docs: [plugins] allowed_permissions). */
  permissions: { id: string; why: string }[];
  /** How original-platform APIs map to Pumpkin. */
  mapping: { from: string; to: string; note?: string }[];
  versions: PluginVersion[];
  /** Pumpkin Market listing; release CI uploads each version there and ships the Market-signed binary. */
  market?: {
    /** Set once the listing exists (plugins-market workflow prints it). */
    id?: number;
    track?: 'stable' | 'beta' | 'alpha';
    /** Listing details, pushed by .github/scripts/market_listing.py */
    category?: string;
    keywords?: string;
    commands?: { name: string; aliases?: string[]; permission?: string; description: string }[];
  };
  /**
   * `waiting-api`: attempted, but Pumpkin's plugin API can't support it yet. Listed on the site with what's missing,
   * never built. `versions` is empty until it ships.
   * `waiting-permission`: the original isn't openly licensed, so it waits for its author's written permission.
   */
  status?: 'waiting-api' | 'waiting-permission';
  waiting?: {
    /** When the port was attempted (YYYY-MM-DD). */
    since: string;
    /** What has to happen first (missing Pumpkin APIs, or the author's permission), and why. */
    needs: { api: string; why: string }[];
    /** Where the request is tracked (Pumpkin issue / discussion), if any. */
    tracking?: string;
  };
}

export const CATEGORIES: Record<string, { label: string; icon: string }> = {
  utility: { label: 'Utility', icon: 'construction' },
  teleport: { label: 'Teleport', icon: 'move_location' },
  chat: { label: 'Chat', icon: 'forum' },
  economy: { label: 'Economy', icon: 'paid' },
  world: { label: 'World', icon: 'landscape' },
  admin: { label: 'Admin', icon: 'shield_person' },
  social: { label: 'Social', icon: 'groups' },
  qol: { label: 'Quality of life', icon: 'auto_awesome' },
  performance: { label: 'Performance', icon: 'speed' },
};

/** A port's `ace.json`: the catalog entry, minus the live numbers. */
export type PortManifest = Omit<Plugin, 'downloads' | 'followers'> & { downloads?: number; followers?: number };

/**
 * The catalog. Each port describes itself in `plugins/<slug>/ace.json`; vite.config.ts copies those into
 * `./ports/<slug>.json` (committed, since Vercel only sees web/), and every page is built from this list.
 * Builds are attached to GitHub releases tagged `<slug>-v<version>` (see config.ts); download counts and file
 * sizes are filled in live from those releases.
 */
const manifests = import.meta.glob<PortManifest>('./ports/*.json', { eager: true, import: 'default' });

const ALL: Plugin[] = Object.values(manifests).map((m) => ({ downloads: 0, followers: 0, ...m }));

/** Ports frozen until the Pumpkin plugin API can support them (listed, never built). */
export const WAITING: Plugin[] = ALL.filter((p) => !!p.status || !p.versions.length).sort((a, b) =>
  (b.waiting?.since ?? '').localeCompare(a.waiting?.since ?? ''),
);

const PORTS: Plugin[] = ALL.filter((p) => !WAITING.includes(p)).sort((a, b) => b.versions[0].date.localeCompare(a.versions[0].date));

/** Released ports: every entry has at least one version. */
export const PLUGINS: Plugin[] = [
  ...PORTS,
  ...demoPlugins, // dev-only demo entries (empty in production)
];

export const pluginBySlug = (slug: string) => PLUGINS.find((p) => p.slug === slug);
export const waitingBySlug = (slug: string) => WAITING.find((p) => p.slug === slug);
export const latest = (p: Plugin) => p.versions[0];
export const updatedAt = (p: Plugin) => latest(p).date;

export const TOTAL_DOWNLOADS = PLUGINS.reduce((n, p) => n + p.downloads, 0);
