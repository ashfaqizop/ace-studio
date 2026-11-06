import { M3eSnackbar } from '@m3e/web/snackbar';
import { buddySay } from '../components/buddy';
import { toggleFavorite, toggleCompare, MAX_COMPARE } from './prefs';
import { pluginBySlug, PLUGINS } from '../data/plugins';
import { assetDownloads } from './github';
import { num } from './html';

export function toast(message: string): void {
  M3eSnackbar.open(message, true, { duration: 3200 });
}

export async function copy(text: string, what = 'Copied to clipboard'): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    const ta = Object.assign(document.createElement('textarea'), { value: text });
    document.body.append(ta);
    ta.select();
    document.execCommand('copy');
    ta.remove();
  }
  toast(what);
  buddySay(['Copied!', 'Got it in my pocket.', 'Ready to paste!'][Math.floor(Math.random() * 3)], 'happy');
}

/** Global delegated handlers: [data-copy], [data-fav], [data-share]. */
export function installGlobalActions(): void {
  document.addEventListener(
    'click',
    (e) => {
      for (const n of e.composedPath()) {
        if (!(n instanceof HTMLElement)) continue;
        if (n.dataset.copy != null) {
          e.preventDefault();
          e.stopPropagation();
          copy(n.dataset.copy, n.dataset.copyMsg || 'Copied to clipboard');
          return;
        }
        if (n.dataset.fav != null) {
          e.preventDefault();
          e.stopPropagation();
          const slug = n.dataset.fav;
          const on = toggleFavorite(slug);
          document.querySelectorAll<HTMLElement>(`[data-fav="${CSS.escape(slug)}"]`).forEach((b) => {
            b.toggleAttribute('selected', on);
            (b as HTMLElement & { selected?: boolean }).selected = on;
          });
          const name = pluginBySlug(slug)?.name ?? 'Plugin';
          toast(on ? `${name} saved` : `${name} removed from saved`);
          if (on) buddySay('Saved for later ♥', 'happy');
          return;
        }
        if (n.dataset.compare != null) {
          e.preventDefault();
          e.stopPropagation();
          const slug = n.dataset.compare;
          const res = toggleCompare(slug);
          const on = res === 'added';
          document.querySelectorAll<HTMLElement>(`[data-compare="${CSS.escape(slug)}"]`).forEach((b) => {
            const sel = res === 'full' ? false : on;
            b.toggleAttribute('selected', sel);
            (b as HTMLElement & { selected?: boolean }).selected = sel;
          });
          const name = pluginBySlug(slug)?.name ?? 'Plugin';
          if (res === 'full') toast(`You can compare up to ${MAX_COMPARE} ports at once`);
          else toast(on ? `${name} added to compare` : `${name} removed from compare`);
          return;
        }
        if (n.dataset.share != null) {
          e.preventDefault();
          const url = new URL(n.dataset.share || location.pathname, location.origin).href;
          if (navigator.share) navigator.share({ title: document.title, url }).catch(() => {});
          else copy(url, 'Link copied');
          return;
        }
      }
    },
    true,
  );
}

/** Animate numbers in [data-count] from 0 when they scroll into view. */
export function countUp(root: HTMLElement): () => void {
  const els = root.querySelectorAll<HTMLElement>('[data-count]');
  const reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;
  const io = new IntersectionObserver((entries) => {
    for (const en of entries) {
      if (!en.isIntersecting) continue;
      io.unobserve(en.target);
      const el = en.target as HTMLElement;
      const end = Number(el.dataset.count);
      const fmt = new Intl.NumberFormat('en', { notation: end > 9999 ? 'compact' : 'standard', maximumFractionDigits: 1 });
      if (reduce) {
        el.textContent = fmt.format(end);
        continue;
      }
      const t0 = performance.now();
      const tick = (t: number) => {
        const k = Math.min(1, (t - t0) / 1100);
        el.textContent = fmt.format(Math.round(end * (1 - Math.pow(1 - k, 4))));
        if (k < 1) requestAnimationFrame(tick);
      };
      requestAnimationFrame(tick);
    }
  });
  els.forEach((e) => io.observe(e));
  return () => io.disconnect();
}

/** Reveal [data-reveal] elements with a staggered rise as they enter the viewport. */
export function reveal(root: HTMLElement): () => void {
  const io = new IntersectionObserver(
    (entries) => {
      for (const en of entries) if (en.isIntersecting) {
        en.target.classList.add('revealed');
        io.unobserve(en.target);
      }
    },
    { rootMargin: '0px 0px -8% 0px' },
  );
  root.querySelectorAll('[data-reveal]').forEach((e, i) => {
    (e as HTMLElement).style.setProperty('--stagger', String(i % 6));
    io.observe(e);
  });
  return () => io.disconnect();
}

/* ---------- Live download counts from GitHub release assets ---------- */

let live: Map<string, number> | null = null;

/** Total downloads for a plugin: the larger of the catalog figure and the live asset counts. */
export function downloadsFor(slug: string): number {
  const p = pluginBySlug(slug);
  if (!p) return 0;
  const sum = live ? p.versions.reduce((n, v) => n + (live!.get(v.file) ?? 0), 0) : 0;
  return Math.max(p.downloads, sum);
}

/** Fill [data-dl] (per plugin) and [data-dl-file] (per release asset) with live counts. */
export function applyDownloads(root: ParentNode = document): void {
  if (!live) return;
  root.querySelectorAll<HTMLElement>('[data-dl]').forEach((el) => (el.textContent = num(downloadsFor(el.dataset.dl!))));
  root.querySelectorAll<HTMLElement>('[data-dl-file]').forEach((el) => {
    const n = live!.get(el.dataset.dlFile!);
    if (n != null) el.textContent = `${num(n)} downloads`;
  });
}

/** Fetch release stats once per session (cached) and patch the current page. */
export function loadDownloads(): void {
  if (!PLUGINS.length) return;
  assetDownloads()
    .then((m) => {
      live = m;
      applyDownloads();
    })
    .catch(() => {
      /* rate-limited or offline: keep catalog numbers */
    });
}
