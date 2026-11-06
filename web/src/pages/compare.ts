import type { Page } from '../lib/router';
import { pluginBySlug, latest, CATEGORIES, type Plugin } from '../data/plugins';
import { pluginIcon, permBadge, compatBadge, emptyState } from '../components/cards';
import { bytes, date, esc, icon, num } from '../lib/html';
import { prefs, setPrefs, onPrefs } from '../lib/prefs';
import { assetUrl } from '../config';
import { applyDownloads, downloadsFor } from '../lib/ui';
import { char, sprite } from '../data/crew';

function selection(q: URLSearchParams): Plugin[] {
  const ids = q.get('ids')?.split(',') ?? prefs().compare;
  return ids.map(pluginBySlug).filter((p): p is Plugin => !!p).slice(0, 3);
}

function table(list: Plugin[]): string {
  const rows: [string, string, (p: Plugin) => string][] = [
    ['Latest version', 'sell', (p) => `v${esc(latest(p).version)}${latest(p).channel === 'beta' ? ' <b class="beta">beta</b>' : ''}`],
    ['Your server', 'dns', (p) => compatBadge(p) || '<span class="muted small">Pick “My server” on Browse</span>'],
    ['Needs Pumpkin', 'deployed_code', (p) => `${esc(latest(p).pumpkin)}+`],
    ['Minecraft', 'sports_esports', (p) => esc(latest(p).minecraft)],
    ['Permissions', 'shield', (p) => `${permBadge(p)}${p.permissions.length ? `<ul class="cmp-perms">${p.permissions.map((x) => `<li><code>${esc(x.id)}</code></li>`).join('')}</ul>` : ''}`],
    ['Size', 'sd_card', (p) => bytes(latest(p).size)],
    ['Downloads', 'download', (p) => `<span data-dl="${esc(p.slug)}">${num(downloadsFor(p.slug))}</span>`],
    ['Followers', 'favorite', (p) => num(p.followers)],
    ['Updated', 'update', (p) => date(latest(p).date)],
    ['Releases', 'history', (p) => String(p.versions.length)],
    ['Licence', 'gavel', (p) => esc(p.license)],
    ['Original', 'person', (p) => `${esc(p.original.name)} <span class="muted small">by ${esc(p.original.author)}</span>`],
    ['Categories', 'category', (p) => p.categories.map((c) => `<span class="tag">${esc(CATEGORIES[c]?.label ?? c)}</span>`).join(' ')],
    ['Ported by', 'construction', (p) => `<span class="assignee">${sprite(p.portedBy, 0.5)}${esc(char(p.portedBy).name)}</span>`],
  ];
  return `
  <div class="table-wrap cmp-wrap">
    <table class="map cmp-table" style="--cols:${list.length}">
      <thead><tr><th></th>${list.map((p) => `
        <th class="cmp-col">
          <a href="/plugin/${esc(p.slug)}" class="cmp-head">${pluginIcon(p, 48)}<span>${esc(p.name)}</span></a>
          <m3e-icon-button size="small" data-compare="${esc(p.slug)}" aria-label="Remove ${esc(p.name)}">${icon('close')}</m3e-icon-button>
        </th>`).join('')}</tr></thead>
      <tbody>
        ${rows.map(([label, ic, cell]) => `<tr><th scope="row">${icon(ic)}${label}</th>${list.map((p) => `<td>${cell(p)}</td>`).join('')}</tr>`).join('')}
        <tr><th></th>${list.map((p) => `<td><m3e-button variant="filled" href="${esc(assetUrl(p.slug, latest(p).version, latest(p).file))}" target="_blank" rel="noopener">${icon('download', { slot: 'icon' })}Download</m3e-button></td>`).join('')}</tr>
      </tbody>
    </table>
  </div>`;
}

const compare: Page = {
  title: 'Compare ports',
  render(c) {
    const list = selection(c.query);
    const head = `<section class="page-head"><span class="eyebrow">${icon('compare_arrows')} Compare</span><h1 class="display-md">Compare <em>ports</em></h1>
      <p class="lead">Side by side: what each port needs, what it asks for and how fresh it is.</p></section>`;
    if (list.length < 2) {
      return head + emptyState('claude/inspector', list.length ? 'Pick one more' : 'Nothing to compare yet',
        'Tap the compare button on up to three ports, then come back here.',
        `<m3e-button variant="filled" href="/plugins">${icon('extension', { slot: 'icon' })}Browse ports</m3e-button>`);
    }
    return `${head}<div id="cmp">${table(list)}</div>
      <div class="cmp-share"><m3e-button variant="tonal" data-share="/compare?ids=${list.map((p) => p.slug).join(',')}">${icon('share', { slot: 'icon' })}Share this comparison</m3e-button>
      <m3e-button variant="text" id="cmp-reset">${icon('delete_sweep', { slot: 'icon' })}Clear selection</m3e-button></div>`;
  },
  mount(root, c) {
    // a shared ?ids= link becomes the visitor's own selection
    const fromUrl = c.query.get('ids');
    if (fromUrl) setPrefs({ compare: selection(c.query).map((p) => p.slug) });
    applyDownloads(root);
    root.querySelector('#cmp-reset')?.addEventListener('click', () => setPrefs({ compare: [] }));
    const off = onPrefs(() => {
      const list = prefs().compare.map(pluginBySlug).filter((p): p is Plugin => !!p);
      const box = root.querySelector('#cmp');
      if (list.length < 2 || !box) {
        root.innerHTML = compare.render({ ...c, query: new URLSearchParams() });
        return;
      }
      box.innerHTML = table(list);
      applyDownloads(box);
    });
    return off;
  },
};

export default compare;
