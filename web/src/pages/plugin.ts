import type { Page, Ctx } from '../lib/router';
import { pluginBySlug, waitingBySlug, latest, CATEGORIES, PLUGINS, WAITING, type Plugin } from '../data/plugins';
import { char, sprite } from '../data/crew';
import { pluginIcon, favButton, compareButton, compatBadge, permBadge, newBadge, emptyState, speaker, pluginCard, waitingBadge, waitingCard, waitKind } from '../components/cards';
import { ago, bytes, codeBlock, date, esc, icon, md, num } from '../lib/html';
import { assetUrl, GITHUB } from '../config';
import { pushRecent, prefs, setPrefs, isNew } from '../lib/prefs';
import { compat, cmpVersion, PUMPKIN_VERSIONS } from '../lib/compat';
import { reveal, applyDownloads } from '../lib/ui';
import { brand } from '../lib/brands';
import { logo } from '../components/shell';
import { hydrateModrinthIcons, modrinthSlug } from '../lib/modrinth';

const PERM_ICON: Record<string, string> = {
  'fs.read.data': 'folder_open',
  'fs.write.data': 'save',
  'http.outbound': 'cloud_upload',
  'network.dns': 'dns',
  'network.tcp.connect': 'lan',
  'network.outbound': 'public',
  'sys.env': 'terminal',
};

/** The original plugin’s icon: manifest override, else fetched from Modrinth (see lib/modrinth.ts), else a glyph. */
function originalIcon(p: Plugin): string {
  if (p.original.icon) {
    return `<img class="credit-img" src="${esc(p.original.icon)}" alt="${esc(p.original.name)} icon" width="56" height="56" loading="lazy" referrerpolicy="no-referrer" />`;
  }
  const slug = modrinthSlug(p.original.url);
  const glyph = icon('person', { cls: 'credit-i' });
  return slug ? `<span class="credit-art" data-modrinth-icon="${esc(slug)}" data-alt="${esc(p.original.name)} icon">${glyph}</span>` : glyph;
}

function tomlFor(p: Plugin): string {
  const perms = p.permissions.map((x) => `"${x.id}"`).join(', ');
  return `[plugins.overrides."${p.slug}"]
enabled = true
allowed_permissions = [${perms}]${p.permissions.length ? '' : ' # needs no extra capabilities'}
`;
}

function header(p: Plugin): string {
  const v = latest(p);
  return `
  <m3e-breadcrumb class="crumbs">
    <m3e-breadcrumb-item href="/plugins">${icon('extension', { slot: 'icon' })}Browse</m3e-breadcrumb-item>
    <m3e-breadcrumb-item href="/plugins?cat=${esc(p.categories[0])}">${esc(CATEGORIES[p.categories[0]]?.label ?? p.categories[0])}</m3e-breadcrumb-item>
    <m3e-breadcrumb-item current="page">${esc(p.name)}</m3e-breadcrumb-item>
  </m3e-breadcrumb>
  <section class="p-hero">
    <div class="p-hero-icon">${pluginIcon(p, 112)}</div>
    <div class="p-hero-copy">
      <div class="p-badges">
        ${newBadge(p)}<span data-compat-badge>${compatBadge(p)}</span>${permBadge(p)}
        <span class="tag">${icon('gavel')}${esc(p.license)}</span>
        ${v.channel === 'beta' ? `<span class="tag tag-tertiary">${icon('experiment')}Beta</span>` : ''}
        <span class="tag">${brand('webassembly', { size: 14, tint: true })}WASM</span>
      </div>
      <h1 class="display-md">${esc(p.name)}</h1>
      <p class="lead">${esc(p.summary)}</p>
      <p class="muted p-meta">${icon('download')}<span data-dl="${esc(p.slug)}">${num(p.downloads)}</span> downloads · ${icon('favorite')}${num(p.followers)} followers · ${icon('update')}updated ${ago(v.date)}</p>
    </div>
    <div class="p-actions">
      <m3e-split-button variant="filled" size="large" class="dl-split">
        <m3e-button slot="leading-button" variant="filled" size="large" href="${esc(assetUrl(p.slug, v.version, v.file))}" target="_blank" rel="noopener">${icon('download', { slot: 'icon' })}Download v${esc(v.version)}</m3e-button>
        <m3e-icon-button slot="trailing-button" variant="filled" size="large" aria-label="Other versions">
          ${icon('keyboard_arrow_down')}
          <m3e-menu-trigger for="ver-menu"></m3e-menu-trigger>
        </m3e-icon-button>
      </m3e-split-button>
      <m3e-menu id="ver-menu">
        ${p.versions.map((x) => `<m3e-menu-item href="${esc(assetUrl(p.slug, x.version, x.file))}" target="_blank">${icon('deployed_code', { slot: 'icon' })}v${esc(x.version)}</m3e-menu-item>`).join('')}
      </m3e-menu>
      <div class="p-actions-row">
        ${favButton(p.slug, 'tonal')}
        ${compareButton(p.slug, 'tonal')}
        <m3e-icon-button variant="tonal" data-share="/plugin/${esc(p.slug)}" id="share-btn" aria-label="Share">${icon('share')}</m3e-icon-button>
        <m3e-tooltip for="share-btn">Share</m3e-tooltip>
        <m3e-button variant="outlined" data-copy="${esc(tomlFor(p))}" data-copy-msg="pumpkin.toml snippet copied">${icon('content_paste', { slot: 'icon' })}Copy config</m3e-button>
      </div>
    </div>
  </section>`;
}

function tabs(p: Plugin): string {
  const porter = char(p.portedBy);
  return `
  <m3e-tabs class="p-tabs" stretch>
    <m3e-tab for="t-overview" selected>${icon('description', { slot: 'icon' })}Overview</m3e-tab>
    <m3e-tab for="t-versions">${icon('history', { slot: 'icon' })}Versions</m3e-tab>
    <m3e-tab for="t-perms">${icon('verified_user', { slot: 'icon' })}Permissions</m3e-tab>
    <m3e-tab for="t-notes">${icon('swap_horiz', { slot: 'icon' })}Port notes</m3e-tab>
    <m3e-tab for="t-credits">${icon('handshake', { slot: 'icon' })}Credits</m3e-tab>

    <m3e-tab-panel id="t-overview">
      <div class="overview">
        <article id="readme" class="prose">${md(p.description)}</article>
        <m3e-toc for="readme" max-depth="3" class="toc"><span slot="overline">On this page</span></m3e-toc>
      </div>
    </m3e-tab-panel>

    <m3e-tab-panel id="t-versions">
      <m3e-accordion class="versions">
        ${p.versions.map((v, i) => `
        <m3e-expansion-panel${i === 0 ? ' open' : ''}>
          <span slot="header" class="ver-head">
            <strong>v${esc(v.version)}</strong>
            ${i === 0 ? '<span class="tag tag-primary">Latest</span>' : ''}
            ${v.channel === 'beta' ? '<span class="tag tag-tertiary">Beta</span>' : ''}
            ${isNew(v.date) ? `<span class="tag tag-new">${icon('new_releases', { filled: true })}New</span>` : ''}
            ${prefs().server ? (cmpVersion(prefs().server!, v.pumpkin) >= 0 ? `<span class="tag tag-safe">${icon('check_circle', { filled: true })}Works on your server</span>` : `<span class="tag tag-warn">${icon('upgrade')}Needs Pumpkin ${esc(v.pumpkin)}+</span>`) : ''}
            <span class="muted">${date(v.date)} · Pumpkin ${esc(v.pumpkin)} · MC ${esc(v.minecraft)}</span>
          </span>
          <div class="ver-body">
            <div class="prose">${md(v.changelog)}</div>
            <div class="ver-file">
              ${icon('deployed_code')}<code>${esc(v.file)}</code><span class="muted" data-size-file="${esc(v.file)}">${bytes(v.size)}</span><span class="muted" data-dl-file="${esc(v.file)}"></span>
              <m3e-button variant="tonal" href="${esc(assetUrl(p.slug, v.version, v.file))}" target="_blank" rel="noopener">${icon('download', { slot: 'icon' })}Download</m3e-button>
            </div>
          </div>
        </m3e-expansion-panel>`).join('')}
      </m3e-accordion>
    </m3e-tab-panel>

    <m3e-tab-panel id="t-perms">
      <div class="perms">
        ${speaker('codex/sentinel', p.permissions.length ? 'Here is every capability this plugin asks Pumpkin for, and why.' : 'Good news: this plugin runs without any extra capabilities.')}
        ${p.permissions.length ? `<m3e-list class="perm-list">${p.permissions.map((x) => `
          <m3e-list-item>${icon(PERM_ICON[x.id] ?? 'key', { slot: 'leading' })}<code>${esc(x.id)}</code><span slot="supporting-text">${esc(x.why)}</span></m3e-list-item>`).join('')}
        </m3e-list>` : ''}
        <h3 class="title">Pre-approve in <code>pumpkin.toml</code></h3>
        <p class="muted">Pumpkin asks in the console when a plugin requests a capability. To approve it up front, add this override:</p>
        ${codeBlock(tomlFor(p), 'pumpkin.toml')}
      </div>
    </m3e-tab-panel>

    <m3e-tab-panel id="t-notes">
      <p class="muted">How the original plugin's APIs were translated to Pumpkin.</p>
      <div class="table-wrap"><table class="map">
        <thead><tr><th>Original</th><th></th><th>Pumpkin</th><th>Notes</th></tr></thead>
        <tbody>${p.mapping.map((m) => `<tr><td><code>${esc(m.from)}</code></td><td>${icon('arrow_forward')}</td><td><code>${esc(m.to)}</code></td><td class="muted">${esc(m.note ?? '')}</td></tr>`).join('')}</tbody>
      </table></div>
    </m3e-tab-panel>

    <m3e-tab-panel id="t-credits">
      <div class="credits">
        <m3e-card variant="outlined">
          <div slot="content" class="credit">
            ${originalIcon(p)}
            <div><span class="label">Original plugin</span><h3>${esc(p.original.name)}</h3>
            <p class="muted">by ${esc(p.original.author)} · ${esc(p.original.license)}</p>
            ${p.original.url ? `<m3e-button variant="tonal" href="${esc(p.original.url)}" target="_blank" rel="noopener">${brand('modrinth', { slot: 'icon', size: 18, tint: true })}View on Modrinth</m3e-button>` : ''}</div>
          </div>
        </m3e-card>
        <m3e-card variant="outlined">
          <div slot="content" class="credit">
            ${sprite(p.portedBy, 1)}
            <div><span class="label">Ported by</span><h3>${esc(porter.name)}</h3><p class="muted">${esc(porter.role)} at Ace Studio</p></div>
          </div>
        </m3e-card>
      </div>
      <m3e-callout variant="secondary" class="credit-note">${icon('info', { slot: 'icon' })}<span slot="label">Licence</span>
        This port is distributed under ${esc(p.license)}, matching the original's ${esc(p.original.license)} licence. Ace Studio is not affiliated with the original author.
      </m3e-callout>
    </m3e-tab-panel>
  </m3e-tabs>`;
}

function aside(p: Plugin): string {
  const v = latest(p);
  const porter = char(p.portedBy);
  return `
  <aside class="p-aside">
    <m3e-card variant="filled">
      <div slot="content" class="aside-block">
        <h3 class="label">Compatibility</h3>
        <dl class="kv">
          <dt><span class="dt-logo">${logo()}</span>Pumpkin</dt><dd>${esc(v.pumpkin)}</dd>
          <dt>${icon('sports_esports')}Minecraft</dt><dd>${esc(v.minecraft)}</dd>
          <dt>${brand('webassembly', { size: 16, tint: true })}Runtime</dt><dd>WASM component</dd>
          <dt>${icon('sd_card')}Size</dt><dd data-size-file="${esc(v.file)}">${bytes(v.size)}</dd>
        </dl>
        <m3e-form-field variant="outlined" class="aside-server">
          <label slot="label" for="p-server">My server</label>
          ${icon('dns', { slot: 'prefix' })}
          <m3e-select id="p-server">
            <m3e-option value=""${!prefs().server ? ' selected' : ''}>Not set</m3e-option>
            ${PUMPKIN_VERSIONS.map((x) => `<m3e-option value="${x}"${prefs().server === x ? ' selected' : ''}>Pumpkin ${x}</m3e-option>`).join('')}
          </m3e-select>
        </m3e-form-field>
        <div class="aside-compat" data-compat-badge>${compatBadge(p)}</div>
      </div>
    </m3e-card>
    <m3e-card variant="filled">
      <div slot="content" class="aside-block porter">
        ${sprite(p.portedBy, 2)}
        <div><span class="label">Ported by</span><strong>${esc(porter.name)}</strong><span class="muted small">“${esc(porter.line)}”</span></div>
      </div>
    </m3e-card>
    <m3e-card variant="filled">
      <div slot="content" class="aside-block">
        <h3 class="label">Categories</h3>
        <m3e-chip-set>${p.categories.map((c) => `<m3e-assist-chip href="/plugins?cat=${esc(c)}">${icon(CATEGORIES[c]?.icon ?? 'label', { slot: 'icon' })}${esc(CATEGORIES[c]?.label ?? c)}</m3e-assist-chip>`).join('')}</m3e-chip-set>
        <h3 class="label">Links</h3>
        <div class="links">
          <a href="${GITHUB}/issues/new?title=${encodeURIComponent(`[${p.name}] `)}&labels=bug" target="_blank" rel="noopener">${brand('github', { size: 18 })}Report a bug</a>
          <a href="${GITHUB}/releases" target="_blank" rel="noopener">${brand('github', { size: 18 })}All releases</a>
          ${p.original.url ? `<a href="${esc(p.original.url)}" target="_blank" rel="noopener">${brand('modrinth', { size: 18, tint: true })}Original on Modrinth</a>` : ''}
          <a href="/docs">${icon('menu_book')}Install guide</a>
        </div>
      </div>
    </m3e-card>
  </aside>`;
}

/** Page for a port frozen until the Pumpkin plugin API can support it: what's missing, no downloads. */
function waitingPage(p: Plugin): string {
  const w = p.waiting;
  const porter = char(p.portedBy);
  const needs = w?.needs ?? [];
  const k = waitKind(p);
  const more = WAITING.filter((x) => x.slug !== p.slug).slice(0, 3);
  return `
  <div class="plugin-page waiting-page" style="view-transition-name: plugin-${esc(p.slug)}">
    <m3e-breadcrumb class="crumbs">
      <m3e-breadcrumb-item href="/plugins">${icon('extension', { slot: 'icon' })}Browse</m3e-breadcrumb-item>
      <m3e-breadcrumb-item href="/plugins">Frozen ports</m3e-breadcrumb-item>
      <m3e-breadcrumb-item current="page">${esc(p.name)}</m3e-breadcrumb-item>
    </m3e-breadcrumb>
    <section class="p-hero">
      <div class="p-hero-icon">${pluginIcon(p, 112)}</div>
      <div class="p-hero-copy">
        <div class="p-badges">
          ${waitingBadge(p)}
          <span class="tag">${icon('gavel')}${esc(p.license)}</span>
        </div>
        <h1 class="display-md">${esc(p.name)}</h1>
        <p class="lead">${esc(p.summary)}</p>
        <p class="muted p-meta">${icon('pause_circle')}Attempted ${w?.since ? `${date(w.since)} (${ago(w.since)})` : ''} · ${icon(k.icon)}${k.count}</p>
      </div>
      <div class="p-actions">
        <m3e-callout variant="tertiary" class="waiting-callout">${icon('hourglass_top', { slot: 'icon', filled: true })}<span slot="label">Frozen, not forgotten</span>
          ${k.callout}
        </m3e-callout>
        <div class="p-actions-row">
          ${favButton(p.slug, 'tonal')}
          <m3e-icon-button variant="tonal" data-share="/plugin/${esc(p.slug)}" id="share-btn" aria-label="Share">${icon('share')}</m3e-icon-button>
          <m3e-tooltip for="share-btn">Share</m3e-tooltip>
          ${w?.tracking ? `<m3e-button variant="outlined" href="${esc(w.tracking)}" target="_blank" rel="noopener">${brand('github', { slot: 'icon', size: 18 })}Follow the request</m3e-button>` : ''}
        </div>
      </div>
    </section>
    <div class="p-layout">
      <div class="p-main">
        <m3e-tabs class="p-tabs" stretch>
          <m3e-tab for="t-missing" selected>${icon(k.icon, { slot: 'icon' })}${k.tab}</m3e-tab>
          <m3e-tab for="t-overview">${icon('description', { slot: 'icon' })}Overview</m3e-tab>
          <m3e-tab for="t-notes">${icon('swap_horiz', { slot: 'icon' })}Port notes</m3e-tab>
          <m3e-tab for="t-credits">${icon('handshake', { slot: 'icon' })}Credits</m3e-tab>

          <m3e-tab-panel id="t-missing">
            <div class="perms">
              ${speaker(p.portedBy, k.speech)}
              <m3e-list class="perm-list">${needs.map((n) => `
                <m3e-list-item>${icon(k.icon, { slot: 'leading' })}<code>${esc(n.api)}</code><span slot="supporting-text">${esc(n.why)}</span></m3e-list-item>`).join('')}
              </m3e-list>
              <p class="muted">${k.footnote}</p>
            </div>
          </m3e-tab-panel>
          <m3e-tab-panel id="t-overview">
            <div class="overview">
              <article id="readme" class="prose">${md(p.description)}</article>
              <m3e-toc for="readme" max-depth="3" class="toc"><span slot="overline">On this page</span></m3e-toc>
            </div>
          </m3e-tab-panel>
          <m3e-tab-panel id="t-notes">
            <p class="muted">How the original plugin&rsquo;s APIs map to Pumpkin, including the parts that don&rsquo;t exist yet.</p>
            <div class="table-wrap"><table class="map">
              <thead><tr><th>Original</th><th></th><th>Pumpkin</th><th>Notes</th></tr></thead>
              <tbody>${p.mapping.map((m) => `<tr><td><code>${esc(m.from)}</code></td><td>${icon('arrow_forward')}</td><td><code>${esc(m.to)}</code></td><td class="muted">${esc(m.note ?? '')}</td></tr>`).join('')}</tbody>
            </table></div>
          </m3e-tab-panel>
          <m3e-tab-panel id="t-credits">
            <div class="credits">
              <m3e-card variant="outlined">
                <div slot="content" class="credit">
                  ${originalIcon(p)}
                  <div><span class="label">Original plugin</span><h3>${esc(p.original.name)}</h3>
                  <p class="muted">by ${esc(p.original.author)} · ${esc(p.original.license)}</p>
                  ${p.original.url ? `<m3e-button variant="tonal" href="${esc(p.original.url)}" target="_blank" rel="noopener">${brand('modrinth', { slot: 'icon', size: 18, tint: true })}View on Modrinth</m3e-button>` : ''}</div>
                </div>
              </m3e-card>
              <m3e-card variant="outlined">
                <div slot="content" class="credit">
                  ${sprite(p.portedBy, 1)}
                  <div><span class="label">Attempted by</span><h3>${esc(porter.name)}</h3><p class="muted">${esc(porter.role)} at Ace Studio</p></div>
                </div>
              </m3e-card>
            </div>
          </m3e-tab-panel>
        </m3e-tabs>
      </div>
      <aside class="p-aside">
        <m3e-card variant="filled">
          <div slot="content" class="aside-block">
            <h3 class="label">Status</h3>
            <dl class="kv">
              <dt>${icon('hourglass_top')}State</dt><dd>${k.state}</dd>
              ${w?.since ? `<dt>${icon('event')}Attempted</dt><dd>${date(w.since)}</dd>` : ''}
              <dt>${icon(k.icon)}Blocked by</dt><dd>${k.count}</dd>
              <dt>${icon('gavel')}Licence</dt><dd>${esc(p.license)}</dd>
            </dl>
          </div>
        </m3e-card>
        <m3e-card variant="filled">
          <div slot="content" class="aside-block">
            <h3 class="label">Links</h3>
            <div class="links">
              ${p.original.url ? `<a href="${esc(p.original.url)}" target="_blank" rel="noopener">${brand('modrinth', { size: 18, tint: true })}Original on Modrinth</a>` : ''}
              ${w?.tracking ? `<a href="${esc(w.tracking)}" target="_blank" rel="noopener">${brand('github', { size: 18 })}API request</a>` : ''}
              <a href="/roadmap">${icon('view_kanban')}Roadmap</a>
            </div>
          </div>
        </m3e-card>
      </aside>
    </div>
    ${more.length ? `<section class="band"><div class="band-head"><h2 class="headline">Also frozen</h2></div>
      <div class="grid">${more.map((x) => waitingCard(x)).join('')}</div></section>` : ''}
  </div>`;
}

const plugin: Page = {
  title: (c: Ctx) => (pluginBySlug(c.params.slug) ?? waitingBySlug(c.params.slug))?.name ?? 'Plugin not found',
  render(c) {
    const p = pluginBySlug(c.params.slug);
    const waitingPort = p ? undefined : waitingBySlug(c.params.slug);
    if (waitingPort) return waitingPage(waitingPort);
    if (!p) {
      return emptyState('claude/patch', 'That port doesn’t exist', `We couldn’t find “${esc(c.params.slug)}”. It may have been renamed.`,
        `<m3e-button variant="filled" href="/plugins">${icon('extension', { slot: 'icon' })}Browse ports</m3e-button>`);
    }
    const more = PLUGINS.filter((x) => x.slug !== p.slug && x.categories.some((cat) => p.categories.includes(cat))).slice(0, 3);
    return `
    <div class="plugin-page" style="view-transition-name: plugin-${esc(p.slug)}">
      ${header(p)}
      <div class="p-layout">
        <div class="p-main">${tabs(p)}</div>
        ${aside(p)}
      </div>
      ${more.length ? `<section class="band"><div class="band-head"><h2 class="headline">More like this</h2></div>
        <div class="grid">${more.map((x) => pluginCard(x)).join('')}</div></section>` : ''}
    </div>`;
  },
  mount(root, c) {
    const p = pluginBySlug(c.params.slug);
    void hydrateModrinthIcons(root);
    if (!p) return waitingBySlug(c.params.slug) ? reveal(root) : undefined;
    pushRecent(p.slug);
    applyDownloads(root);
    root.querySelector('#p-server')?.addEventListener('change', (e) => {
      const v = (e.currentTarget as HTMLElement & { value?: string | null }).value;
      setPrefs({ server: typeof v === 'string' && v ? v : null });
      root.querySelectorAll<HTMLElement>('[data-compat-badge]').forEach((el) => (el.innerHTML = compatBadge(p)));
      const c2 = compat(p);
      if (c2.state === 'older') root.querySelector<HTMLElement>('[data-compat-badge].aside-compat')!.insertAdjacentHTML('beforeend', `<p class="small muted">The latest release needs a newer Pumpkin. Grab v${esc(c2.version.version)} from the Versions tab.</p>`);
    });
    return reveal(root);
  },
};

export default plugin;
