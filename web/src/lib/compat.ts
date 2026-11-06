import type { Plugin, PluginVersion } from '../data/plugins';
import { prefs } from './prefs';

/**
 * Pumpkin versions a visitor can pick as "my server". Matches Pumpkin's GitHub releases;
 * nightly/canary are rolling builds and count as newer than every release.
 */
export const PUMPKIN_VERSIONS = ['canary', 'nightly', '0.2.1', '0.2.0', '0.1.0-dev'] as const;

const ROLLING: Record<string, number> = { nightly: 1, canary: 2 };

/** Compare Pumpkin versions. Ignores build metadata (+...); a pre-release sorts below its release. */
export function cmpVersion(a: string, b: string): number {
  const ra = ROLLING[a] ?? 0;
  const rb = ROLLING[b] ?? 0;
  if (ra || rb) return ra - rb;
  const parse = (v: string) => {
    const [core, pre = ''] = v.split('+')[0].split('-');
    return { nums: core.split('.').map((n) => parseInt(n, 10) || 0), pre };
  };
  const x = parse(a);
  const y = parse(b);
  for (let i = 0; i < 3; i++) {
    const d = (x.nums[i] ?? 0) - (y.nums[i] ?? 0);
    if (d) return d;
  }
  if (x.pre === y.pre) return 0;
  if (!x.pre) return 1;
  if (!y.pre) return -1;
  return x.pre.localeCompare(y.pre);
}

export type Compat =
  | { state: 'unknown' }
  | { state: 'ok'; version: PluginVersion }
  | { state: 'older'; version: PluginVersion }
  | { state: 'no'; needs: string };

/**
 * Can the visitor's server run this plugin? Each version's `pumpkin` field is the minimum
 * Pumpkin version it needs. "older" means only a previous release works.
 */
export function compat(p: Plugin, server = prefs().server): Compat {
  if (!server) return { state: 'unknown' };
  const fit = p.versions.find((v) => cmpVersion(server, v.pumpkin) >= 0);
  if (!fit) return { state: 'no', needs: p.versions[p.versions.length - 1]?.pumpkin ?? '?' };
  return fit === p.versions[0] ? { state: 'ok', version: fit } : { state: 'older', version: fit };
}
