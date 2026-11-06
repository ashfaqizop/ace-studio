/**
 * Original-plugin icons from Modrinth, for every port automatically: elements with `data-modrinth-icon="<slug>"`
 * are filled with the project's icon. One batched API call per page, cached for the session (Modrinth's API is CORS-open).
 */

const KEY = 'ace:modrinth-icons';
const memo: Record<string, string | null> = (() => {
  try {
    return JSON.parse(sessionStorage.getItem(KEY) ?? '{}');
  } catch {
    return {};
  }
})();

/** The Modrinth project slug from a modrinth.com URL (`/plugin/x`, `/mod/x`, `/datapack/x`, `/project/x`…). */
export function modrinthSlug(url?: string): string | null {
  if (!url) return null;
  const m = /modrinth\.com\/(?:plugin|mod|datapack|project|resourcepack|shader|modpack)\/([^/?#]+)/i.exec(url);
  return m ? decodeURIComponent(m[1]).toLowerCase() : null;
}

function paint(el: HTMLElement, src: string, alt: string): void {
  const img = document.createElement('img');
  img.className = 'credit-img';
  img.src = src;
  img.alt = alt;
  img.width = 56;
  img.height = 56;
  img.loading = 'lazy';
  img.referrerPolicy = 'no-referrer';
  el.replaceChildren(img);
}

/** Fills every `[data-modrinth-icon]` under `root`; keeps the fallback glyph when a project has no icon. */
export async function hydrateModrinthIcons(root: ParentNode = document): Promise<void> {
  const els = [...root.querySelectorAll<HTMLElement>('[data-modrinth-icon]')];
  if (!els.length) return;
  const missing = [...new Set(els.map((e) => e.dataset.modrinthIcon!).filter((s) => !(s in memo)))];
  if (missing.length) {
    try {
      const res = await fetch(`https://api.modrinth.com/v2/projects?ids=${encodeURIComponent(JSON.stringify(missing))}`);
      const list: { slug: string; id: string; icon_url: string | null }[] = res.ok ? await res.json() : [];
      for (const s of missing) memo[s] = null;
      for (const p of list) memo[p.slug.toLowerCase()] = p.icon_url;
      sessionStorage.setItem(KEY, JSON.stringify(memo));
    } catch {
      return; // offline or rate-limited: keep the fallback glyphs
    }
  }
  for (const el of els) {
    const src = memo[el.dataset.modrinthIcon!];
    if (src) paint(el, src, el.dataset.alt ?? '');
  }
}
