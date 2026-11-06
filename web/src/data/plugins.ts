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

/**
 * The catalog. Add one entry per shipped port; the site builds every page from this list.
 * Builds are attached to GitHub releases tagged `<slug>-v<version>` (see config.ts).
 *
 * Example entry:
 *
 *   {
 *     slug: 'my-port',
 *     name: 'My Port',
 *     summary: 'One line about what it does.',
 *     description: '## Markdown

Shown on the Overview tab.',
 *     categories: ['utility'],
 *     icon: { shape: 'sunny', symbol: 'extension' },          // any m3e-shape + Material Symbol
 *     license: 'MIT',
 *     original: { name: 'Original Plugin', author: 'author', url: 'https://modrinth.com/plugin/original', license: 'MIT' },
 *     portedBy: 'claude/forge',
 *     downloads: 0,
 *     followers: 0,
 *     permissions: [{ id: 'fs:read:data', why: 'Reads its config.' }],
 *     mapping: [{ from: 'PlayerJoinEvent', to: 'PlayerJoinEvent' }],
 *     versions: [{ version: '1.0.0', date: '2026-11-06', pumpkin: '0.2.1', minecraft: '1.21.11',
 *                  file: 'my-port-1.0.0.wasm', size: 0, changelog: '- First Pumpkin release' }],
 *   },
 */
export const PLUGINS: Plugin[] = [
  // real ports go here
  ...demoPlugins, // dev-only demo entries (empty in production)
];

export const pluginBySlug = (slug: string) => PLUGINS.find((p) => p.slug === slug);
export const latest = (p: Plugin) => p.versions[0];
export const updatedAt = (p: Plugin) => latest(p).date;

export const TOTAL_DOWNLOADS = PLUGINS.reduce((n, p) => n + p.downloads, 0);
