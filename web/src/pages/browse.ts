import type { Page } from '../lib/router';
import { setQuery } from '../lib/router';
import { PLUGINS, CATEGORIES, updatedAt, type Plugin } from '../data/plugins';
import { pluginCard, pluginRow, emptyState } from '../components/cards';
import { esc, icon } from '../lib/html';
import { prefs, setPrefs, onPrefs } from '../lib/prefs';
import { reveal } from '../lib/ui';
import { buddySay } from '../components/buddy';

type Sort = 'popular' | 'updated' | 'name' | 'followers';
type El = HTMLElement & { selected?: boolean; checked?: boolean; value?: string };

interface State {
  q: string;
  cats: string[];
  sort: Sort;
  saved: boolean;
  view: 'grid' | 'list';
}

function filter(s: State): Plugin[] {
  const terms = s.q.toLowerCase().split(/\s+/).filter(Boolean);
  const favs = prefs().favorites;
  let list = PLUGINS.filter((p) => {
    if (s.saved && !favs.includes(p.slug)) return false;
    if (s.cats.length && !s.cats.some((c) => p.categories.includes(c))) return false;
    if (!terms.length) return true;
    const hay = `${p.name} ${p.summary} ${p.original.name} ${p.categories.join(' ')} ${p.license}`.toLowerCase();
    return terms.every((t) => hay.includes(t));
  });
  const by: Record<Sort, (a: Plugin, b: Plugin) => number> = {
    popular: (a, b) => b.downloads - a.downloads,
    followers: (a, b) => b.followers - a.followers,
    updated: (a, b) => updatedAt(b).localeCompare(updatedAt(a)),
    name: (a, b) => a.name.localeCompare(b.name),
  };
  return list.sort(by[s.sort]);
}

function results(s: State): string {
  const list = filter(s);
  if (!list.length) {
    return s.saved && !s.q && !s.cats.length
      ? emptyState('ambient/sprout', 'Nothing saved yet', 'Tap the ♥ on any port to keep it here for later.', `<m3e-button variant="tonal" href="/plugins">${icon('extension', { slot: 'icon' })}Browse ports</m3e-button>`)
      : emptyState('grok/scout', 'No ports match that', `Scout searched every chunk for “${esc(s.q || 'those filters')}” and came back empty-handed.`,
          `<m3e-button variant="tonal" id="clear-filters">${icon('filter_alt_off', { slot: 'icon' })}Clear filters</m3e-button>
           <m3e-button variant="text" href="/request">Request it as a port</m3e-button>`);
  }
  return s.view === 'grid'
    ? `<div class="grid">${list.map(pluginCard).join('')}</div>`
    : `<div class="rows">${list.map(pluginRow).join('')}</div>`;
}

const browse: Page = {
  title: (c) => (c.query.get('saved') ? 'Saved plugins' : 'Browse ports'),
  render(c) {
    if (!PLUGINS.length) {
      return `<section class="page-head"><span class="eyebrow">${icon('extension')} Catalog</span><h1 class="display-md">Browse <em>ports</em></h1></section>
      ${emptyState('claude/forge', 'No ports have shipped yet', 'The crew is forging the first ones right now. Request the plugin you want most and it jumps the queue.',
        `<m3e-button variant="filled" href="/request">${icon('add_circle', { slot: 'icon' })}Request a port</m3e-button>
         <m3e-button variant="tonal" href="/roadmap">${icon('view_kanban', { slot: 'icon' })}See the roadmap</m3e-button>`)}`;
    }
    const s = readState(c.query);
    const count = filter(s).length;
    return `
    <section class="page-head">
      <span class="eyebrow">${icon(s.saved ? 'favorite' : 'extension')} ${s.saved ? 'Your collection' : 'Catalog'}</span>
      <h1 class="display-md">${s.saved ? 'Saved <em>plugins</em>' : 'Browse <em>ports</em>'}</h1>
    </section>
    <section class="filters" aria-label="Filters">
      <m3e-search-bar clearable class="page-search">
        ${icon('search', { slot: 'leading' })}
        <input slot="input" data-page-search type="search" placeholder="Search by name, feature or original plugin" value="${esc(s.q)}" aria-label="Search ports" />
      </m3e-search-bar>
      <div class="filter-row">
        <m3e-filter-chip-set multi id="cats" aria-label="Categories">
          <m3e-filter-chip value="__saved"${s.saved ? ' selected' : ''}>${icon('favorite', { slot: 'icon' })}Saved</m3e-filter-chip>
          ${Object.entries(CATEGORIES).map(([k, cat]) => `<m3e-filter-chip value="${k}"${s.cats.includes(k) ? ' selected' : ''}>${icon(cat.icon, { slot: 'icon' })}${esc(cat.label)}</m3e-filter-chip>`).join('')}
        </m3e-filter-chip-set>
      </div>
      <div class="toolbar-row">
        <span class="count" id="count" aria-live="polite">${count} ${count === 1 ? 'port' : 'ports'}</span>
        <div class="toolbar-right">
          <m3e-form-field variant="outlined" class="sort-field">
            <label slot="label" for="sort">Sort by</label>
            <m3e-select id="sort">
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
    if (!PLUGINS.length) return;
    const s = readState(c.query);
    const out = root.querySelector<HTMLElement>('#results')!;
    let stopReveal = reveal(out);
    const update = () => {
      out.innerHTML = results(s);
      stopReveal();
      stopReveal = reveal(out);
      const n = filter(s).length;
      root.querySelector('#count')!.textContent = `${n} ${n === 1 ? 'port' : 'ports'}`;
      const q = new URLSearchParams();
      if (s.q) q.set('q', s.q);
      if (s.cats.length) q.set('cat', s.cats.join(','));
      if (s.sort !== 'popular') q.set('sort', s.sort);
      if (s.saved) q.set('saved', '1');
      setQuery(q);
      out.querySelector('#clear-filters')?.addEventListener('click', clear);
      if (!n && s.q) buddySay('Nothing here… try fewer words?', 'sad');
    };
    const input = root.querySelector<HTMLInputElement>('[data-page-search]')!;
    const clear = () => {
      s.q = '';
      s.cats = [];
      s.saved = false;
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
      const chips = [...(e.currentTarget as HTMLElement).querySelectorAll<El>('m3e-filter-chip')];
      const on = chips.filter((ch) => ch.selected).map((ch) => ch.getAttribute('value')!);
      s.saved = on.includes('__saved');
      s.cats = on.filter((v) => v !== '__saved');
      update();
    });
    root.querySelector('#sort')!.addEventListener('change', (e) => {
      const v = (e.currentTarget as El).value;
      if (typeof v === 'string') {
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
    // un-saving a plugin while viewing Saved should drop it from the list
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
    sort: ['popular', 'updated', 'name', 'followers'].includes(sort) ? sort : 'popular',
    saved: q.get('saved') === '1',
    view: prefs().view,
  };
}

export default browse;
