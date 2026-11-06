import type { Page } from '../lib/router';
import { setQuery } from '../lib/router';
import { PLUGINS, WAITING, CATEGORIES, updatedAt, type Plugin } from '../data/plugins';
import { pluginCard, pluginRow, emptyState, waitingCard, waitingBadge } from '../components/cards';
import { esc, icon } from '../lib/html';
import { prefs, setPrefs, onPrefs } from '../lib/prefs';
import { reveal, applyDownloads, downloadsFor } from '../lib/ui';
import { fuzzy, highlight } from '../lib/fuzzy';
import { compat, PUMPKIN_VERSIONS } from '../lib/compat';
import { buddySay } from '../components/buddy';

type Sort = 'relevance' | 'popular' | 'updated' | 'name' | 'followers';
type El = HTMLElement & { selected?: boolean; checked?: boolean; value?: string | null };

interface State {
  q: string;
  cats: string[];
  sort: Sort;
  saved: boolean;
  safe: boolean;
  works: boolean;
  view: 'grid' | 'list';
}

interface Hit {
  p: Plugin;
  score: number;
  nameHtml?: string;
}

/** Score a plugin against the query: name matches count most, then original name, summary, tags. */
function score(p: Plugin, q: string): Hit | null {
  if (!q) return { p, score: 0 };
  const name = fuzzy(q, p.name);
  const fields = [
    name && { s: name.score * 3 },
    fuzzy(q, p.original.name) && { s: fuzzy(q, p.original.name)!.score * 2 },
    fuzzy(q, p.summary) && { s: fuzzy(q, p.summary)!.score },
    fuzzy(q, `${p.categories.map((c) => CATEGORIES[c]?.label ?? c).join(' ')} ${p.license}`) && { s: 10 },
  ].filter(Boolean) as { s: number }[];
  if (!fields.length) return null;
  return { p, score: Math.max(...fields.map((f) => f.s)), nameHtml: name ? highlight(p.name, name.hits) : undefined };
}

function filter(s: State): Hit[] {
  const favs = prefs().favorites;
  const hits: Hit[] = [];
  for (const p of PLUGINS) {
    if (s.saved && !favs.includes(p.slug)) continue;
    if (s.safe && p.permissions.length) continue;
    if (s.works && prefs().server && compat(p).state === 'no') continue;
    if (s.cats.length && !s.cats.some((c) => p.categories.includes(c))) continue;
    const h = score(p, s.q);
    if (h) hits.push(h);
  }
  const sort: Sort = s.sort === 'relevance' && !s.q ? 'popular' : s.sort;
  const by: Record<Sort, (a: Hit, b: Hit) => number> = {
    relevance: (a, b) => b.score - a.score,
    popular: (a, b) => downloadsFor(b.p.slug) - downloadsFor(a.p.slug),
    followers: (a, b) => b.p.followers - a.p.followers,
    updated: (a, b) => updatedAt(b.p).localeCompare(updatedAt(a.p)),
    name: (a, b) => a.p.name.localeCompare(b.p.name),
  };
  return hits.sort(by[sort]);
}

/** Ports frozen until the Pumpkin plugin API can support them; same text/category/saved filters. */
function waiting(s: State): string {
  if (s.safe || s.works) return '';
  const favs = prefs().favorites;
  const hits = WAITING.filter((p) => (!s.saved || favs.includes(p.slug)) && (!s.cats.length || s.cats.some((c) => p.categories.includes(c))))
    .map((p) => score(p, s.q))
    .filter((h): h is Hit => !!h);
  if (!hits.length) return '';
  return `
  <section class="band waiting-band" aria-labelledby="waiting-h">
    <div class="band-head"><div>
      <h2 class="headline" id="waiting-h">${waitingBadge()}</h2>
      <p class="muted">We attempted these ports, but Pumpkin's plugin API can't support them yet. They're frozen, not forgotten: each page lists exactly what's missing, and we build them as soon as Pumpkin ships it.</p>
    </div></div>
    <div class="grid">${hits.map((h) => waitingCard(h.p, h.nameHtml)).join('')}</div>
  </section>`;
}

function results(s: State): string {
  const list = filter(s);
  if (!list.length && waiting(s)) return waiting(s);
  if (!list.length) {
    return s.saved && !s.q && !s.cats.length
      ? emptyState('ambient/sprout', 'Nothing saved yet', 'Tap the ♥ on any port to keep it here for later.', `<m3e-button variant="tonal" href="/plugins">${icon('extension', { slot: 'icon' })}Browse ports</m3e-button>`)
      : emptyState('grok/scout', 'No ports match that', `Scout searched every chunk for “${esc(s.q || 'those filters')}” and came back empty-handed.`,
          `<m3e-button variant="tonal" id="clear-filters">${icon('filter_alt_off', { slot: 'icon' })}Clear filters</m3e-button>
           <m3e-button variant="text" href="/request">Request it as a port</m3e-button>`);
  }
  return (s.view === 'grid'
    ? `<div class="grid">${list.map((h) => pluginCard(h.p, h.nameHtml)).join('')}</div>`
    : `<div class="rows">${list.map((h) => pluginRow(h.p, h.nameHtml)).join('')}</div>`) + waiting(s);
}

const SPECIAL = [
  { v: '__saved', label: 'Saved', icon: 'favorite' },
  { v: '__works', label: 'Works on my server', icon: 'dns' },
  { v: '__safe', label: 'No permissions', icon: 'verified_user' },
];

const browse: Page = {
  title: (c) => (c.query.get('saved') ? 'Saved plugins' : 'Browse ports'),
  render(c) {
    if (!PLUGINS.length && !WAITING.length) {
      return `<section class="page-head"><span class="eyebrow">${icon('extension')} Catalog</span><h1 class="display-md">Browse <em>ports</em></h1></section>
      ${emptyState('claude/forge', 'No ports have shipped yet', 'The crew is forging the first ones right now. Request the plugin you want most and it jumps the queue.',
        `<m3e-button variant="filled" href="/request">${icon('add_circle', { slot: 'icon' })}Request a port</m3e-button>
         <m3e-button variant="tonal" href="/roadmap">${icon('view_kanban', { slot: 'icon' })}See the roadmap</m3e-button>`)}`;
    }
    const s = readState(c.query);
    const count = filter(s).length;
    const on = (v: string) => (v === '__saved' ? s.saved : v === '__safe' ? s.safe : s.works);
    return `
    <section class="page-head">
      <span class="eyebrow">${icon(s.saved ? 'favorite' : 'extension')} ${s.saved ? 'Your collection' : 'Catalog'}</span>
      <h1 class="display-md">${s.saved ? 'Saved <em>plugins</em>' : 'Browse <em>ports</em>'}</h1>
    </section>
    <section class="filters" aria-label="Filters">
      <m3e-search-bar clearable class="page-search">
        ${icon('search', { slot: 'leading' })}
        <input slot="input" data-page-search type="search" placeholder="Search by name, feature or original plugin (typos are fine)" value="${esc(s.q)}" aria-label="Search ports" />
      </m3e-search-bar>
      <div class="filter-row">
        <m3e-filter-chip-set multi id="cats" aria-label="Filters">
          ${SPECIAL.map((x) => `<m3e-filter-chip value="${x.v}"${on(x.v) ? ' selected' : ''}>${icon(x.icon, { slot: 'icon' })}${x.label}</m3e-filter-chip>`).join('')}
          ${Object.entries(CATEGORIES).map(([k, cat]) => `<m3e-filter-chip value="${k}"${s.cats.includes(k) ? ' selected' : ''}>${icon(cat.icon, { slot: 'icon' })}${esc(cat.label)}</m3e-filter-chip>`).join('')}
        </m3e-filter-chip-set>
      </div>
      <div class="toolbar-row">
        <span class="count" id="count" aria-live="polite">${count} ${count === 1 ? 'port' : 'ports'}</span>
        <div class="toolbar-right">
          <m3e-form-field variant="outlined" class="sort-field">
            <label slot="label" for="server">My server</label>
            ${icon('dns', { slot: 'prefix' })}
            <m3e-select id="server">
              <m3e-option value=""${!prefs().server ? ' selected' : ''}>Any version</m3e-option>
              ${PUMPKIN_VERSIONS.map((v) => `<m3e-option value="${v}"${prefs().server === v ? ' selected' : ''}>Pumpkin ${v}</m3e-option>`).join('')}
            </m3e-select>
          </m3e-form-field>
          <m3e-form-field variant="outlined" class="sort-field">
            <label slot="label" for="sort">Sort by</label>
            <m3e-select id="sort">
              <m3e-option value="relevance"${s.sort === 'relevance' ? ' selected' : ''}>Best match</m3e-option>
              <m3e-option value="popular"${s.sort === 'popular' ? ' selected' : ''}>Most downloaded</m3e-option>
              <m3e-option value="followers"${s.sort === 'followers' ? ' selected' : ''}>Most followed</m3e-option>
              <m3e-option value="updated"${s.sort === 'updated' ? ' selected' : ''}>Recently updated</m3e-option>
              <m3e-option value="name"${s.sort === 'name' ? ' selected' : ''}>Name A–Z</m3e-option>
            </m3e-select>
          </m3e-form-field>
          <m3e-segmented-button id="view" aria-label="Layout" hide-selection-indicator>
            <m3e-button-segment value="grid"${s.view === 'grid' ? ' checked' : ''} aria-label="Grid">${icon('grid_view', { slot: 'icon' })}</m3e-button-segment>
            <m3e-button-segment value="list"${s.view === 'list' ? ' checked' : ''} aria-label="List">${icon('view_agenda', { slot: 'icon' })}</m3e-button-segment>
          </m3e-segmented-button>
        </div>
      </div>
    </section>
    <section id="results" class="results">${results(s)}</section>`;
  },
  mount(root, c) {
    if (!PLUGINS.length && !WAITING.length) return;
    const s = readState(c.query);
    const out = root.querySelector<HTMLElement>('#results')!;
    let stopReveal = reveal(out);
    const update = () => {
      out.innerHTML = results(s);
      stopReveal();
      stopReveal = reveal(out);
      applyDownloads(out);
      const n = filter(s).length;
      root.querySelector('#count')!.textContent = `${n} ${n === 1 ? 'port' : 'ports'}`;
      const q = new URLSearchParams();
      if (s.q) q.set('q', s.q);
      if (s.cats.length) q.set('cat', s.cats.join(','));
      if (s.sort !== 'relevance') q.set('sort', s.sort);
      if (s.saved) q.set('saved', '1');
      if (s.safe) q.set('safe', '1');
      if (s.works) q.set('works', '1');
      setQuery(q);
      out.querySelector('#clear-filters')?.addEventListener('click', clear);
      if (!n && s.q) buddySay('Nothing here… try fewer words?', 'sad');
    };
    const input = root.querySelector<HTMLInputElement>('[data-page-search]')!;
    const clear = () => {
      Object.assign(s, { q: '', cats: [], saved: false, safe: false, works: false });
      input.value = '';
      root.querySelectorAll<El>('#cats m3e-filter-chip').forEach((ch) => (ch.selected = false));
      update();
    };
    let t = 0;
    input.addEventListener('input', () => {
      clearTimeout(t);
      t = window.setTimeout(() => {
        s.q = input.value.trim();
        update();
      }, 120);
    });
    root.querySelector('.page-search')!.addEventListener('clear', () => {
      s.q = '';
      update();
    });
    root.querySelector('#cats')!.addEventListener('change', (e) => {
      const on = [...(e.currentTarget as HTMLElement).querySelectorAll<El>('m3e-filter-chip')].filter((ch) => ch.selected).map((ch) => ch.getAttribute('value')!);
      s.saved = on.includes('__saved');
      s.safe = on.includes('__safe');
      s.works = on.includes('__works');
      if (s.works && !prefs().server) buddySay('Pick your Pumpkin version under “My server” first.', 'talk');
      s.cats = on.filter((v) => !v.startsWith('__'));
      update();
    });
    root.querySelector('#server')!.addEventListener('change', (e) => {
      const v = (e.currentTarget as El).value;
      setPrefs({ server: typeof v === 'string' && v ? v : null });
      update();
    });
    root.querySelector('#sort')!.addEventListener('change', (e) => {
      const v = (e.currentTarget as El).value;
      if (typeof v === 'string' && v) {
        s.sort = v as Sort;
        update();
      }
    });
    root.querySelector('#view')!.addEventListener('change', (e) => {
      const on = [...(e.currentTarget as HTMLElement).querySelectorAll<El>('m3e-button-segment')].find((x) => x.checked);
      if (!on) return;
      s.view = on.getAttribute('value') as State['view'];
      setPrefs({ view: s.view });
      update();
    });
    out.querySelector('#clear-filters')?.addEventListener('click', clear);
    const off = onPrefs(() => {
      if (s.saved) update();
    });
    if (c.query.get('q')) input.focus();
    return () => {
      stopReveal();
      off();
    };
  },
};

function readState(q: URLSearchParams): State {
  const sort = q.get('sort') as Sort;
  return {
    q: q.get('q') ?? '',
    cats: (q.get('cat') ?? '').split(',').filter((c) => c in CATEGORIES),
    sort: ['relevance', 'popular', 'updated', 'name', 'followers'].includes(sort) ? sort : 'relevance',
    saved: q.get('saved') === '1',
    safe: q.get('safe') === '1',
    works: q.get('works') === '1',
    view: prefs().view,
  };
}

export default browse;
