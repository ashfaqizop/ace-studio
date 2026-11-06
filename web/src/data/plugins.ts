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
  original: { name: string; author: string; url?: string; license: string };
  portedBy: CharRef;
  downloads: number;
  followers: number;
  featured?: boolean;
  /** Pumpkin WASM capabilities the plugin requests (see docs: [plugins] allowed_permissions). */
  permissions: { id: string; why: string }[];
  /** How original-platform APIs map to Pumpkin. */
  mapping: { from: string; to: string; note?: string }[];
  versions: PluginVersion[];
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

const PORTS: Plugin[] = Object.values(manifests)
  .map((m) => ({ downloads: 0, followers: 0, ...m }))
  .sort((a, b) => b.versions[0].date.localeCompare(a.versions[0].date));

export const PLUGINS: Plugin[] = [
  ...PORTS,
  ...demoPlugins, // dev-only demo entries (empty in production)
];

export const pluginBySlug = (slug: string) => PLUGINS.find((p) => p.slug === slug);
export const latest = (p: Plugin) => p.versions[0];
export const updatedAt = (p: Plugin) => latest(p).date;

export const TOTAL_DOWNLOADS = PLUGINS.reduce((n, p) => n + p.downloads, 0);
