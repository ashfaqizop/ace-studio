/**
 * Live data from the public GitHub API (no backend, no token). Unauthenticated calls are
 * limited to 60/hour per visitor, so responses are cached in sessionStorage for 10 minutes
 * and failures fall back to the last good copy.
 */
import { SITE } from '../config';
import type { Stage } from '../data/roadmap';

const API = `https://api.github.com/repos/${SITE.repo}`;
const TTL = 10 * 60 * 1000;
const inflight = new Map<string, Promise<unknown>>();

async function cached<T>(key: string, url: string): Promise<T> {
  const k = `ace:gh:${key}`;
  let stale: T | undefined;
  try {
    const hit = JSON.parse(sessionStorage.getItem(k) || 'null') as { at: number; data: T } | null;
    if (hit) {
      if (Date.now() - hit.at < TTL) return hit.data;
      stale = hit.data;
    }
  } catch {
    /* ignore */
  }
  if (!inflight.has(k)) {
    inflight.set(
      k,
      fetch(url, { headers: { Accept: 'application/vnd.github+json' } })
        .then(async (r) => {
          if (!r.ok) throw new Error(`GitHub ${r.status}`);
          const data = (await r.json()) as T;
          try {
            sessionStorage.setItem(k, JSON.stringify({ at: Date.now(), data }));
          } catch {
            /* quota */
          }
          return data;
        })
        .catch((err) => {
          if (stale !== undefined) return stale;
          throw err;
        })
        .finally(() => inflight.delete(k)),
    );
  }
  return inflight.get(k) as Promise<T>;
}

/* ---------- Issues: port requests and the roadmap ---------- */

interface RawIssue {
  number: number;
  title: string;
  html_url: string;
  state: 'open' | 'closed';
  body: string | null;
  created_at: string;
  updated_at: string;
  labels: { name: string }[];
  reactions?: { '+1': number };
  pull_request?: unknown;
  user: { login: string } | null;
}

export interface PortRequest {
  number: number;
  name: string;
  url: string;
  open: boolean;
  votes: number;
  stage: Stage | 'requested';
  progress: number;
  crew?: string;
  slug?: string;
  modrinth?: string;
  author: string;
  updated: string;
}

const STAGE_LABELS: Stage[] = ['queued', 'translating', 'testing', 'shipped'];

/**
 * All issues labelled `port-request`. Board placement comes from labels:
 * `stage:queued|translating|testing|shipped`, optional `progress:NN`, `crew:<set>/<id>`
 * and `slug:<catalog-slug>`. Requests without a stage label are "requested".
 */
export async function portRequests(): Promise<PortRequest[]> {
  const raw = await cached<RawIssue[]>('requests', `${API}/issues?labels=port-request&state=all&per_page=100&sort=updated`);
  const out: PortRequest[] = [];
  for (const i of raw) {
    if (i.pull_request) continue;
    const names = i.labels.map((l) => l.name.toLowerCase());
    const label = (prefix: string) => names.find((n) => n.startsWith(prefix))?.slice(prefix.length);
    const stage: PortRequest['stage'] = STAGE_LABELS.find((s) => names.includes(`stage:${s}`)) ?? 'requested';
    // a closed issue without a stage label was declined or a duplicate: leave it off the board
    if (i.state === 'closed' && stage === 'requested') continue;
    out.push({
      number: i.number,
      name: i.title.replace(/^port request:\s*/i, '').trim() || `Request #${i.number}`,
      url: i.html_url,
      open: i.state === 'open',
      votes: i.reactions?.['+1'] ?? 0,
      stage,
      progress: Math.min(100, Math.max(0, parseInt(label('progress:') ?? '', 10) || (stage === 'shipped' ? 100 : 0))),
      crew: label('crew:'),
      slug: label('slug:'),
      modrinth: i.body?.match(/modrinth\.com\/(?:plugin|mod|project)\/([\w.-]+)/i)?.[1]?.toLowerCase(),
      author: i.user?.login ?? 'someone',
      updated: i.updated_at,
    });
  }
  return out;
}

/* ---------- Releases: real download counts ---------- */

interface RawRelease {
  tag_name: string;
  assets: { name: string; download_count: number; size: number }[];
}

/** Release asset file name → size in bytes, filled by assetDownloads() (manifests can't know it before CI builds). */
export const assetSizes = new Map<string, number>();

/** Map of release asset file name → download count. */
export async function assetDownloads(): Promise<Map<string, number>> {
  const raw = await cached<RawRelease[]>('releases', `${API}/releases?per_page=100`);
  const map = new Map<string, number>();
  for (const r of raw)
    for (const a of r.assets) {
      map.set(a.name, (map.get(a.name) ?? 0) + a.download_count);
      if (a.size) assetSizes.set(a.name, a.size);
    }
  return map;
}
