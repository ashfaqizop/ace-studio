import type { Page, Ctx } from '../lib/router';
import { pluginBySlug, latest, CATEGORIES, PLUGINS, type Plugin } from '../data/plugins';
import { char, sprite } from '../data/crew';
import { pluginIcon, favButton, emptyState, speaker, pluginCard } from '../components/cards';
import { ago, bytes, codeBlock, date, esc, icon, md, num } from '../lib/html';
import { assetUrl, GITHUB } from '../config';
import { pushRecent } from '../lib/prefs';
import { reveal } from '../lib/ui';

const PERM_ICON: Record<string, string> = {
  'fs:read:data': 'folder_open',
  'fs:write:data': 'save',
  'network:outbound': 'public',
};

function tomlFor(p: Plugin): string {
  const id = p.slug.replace(/-/g, '_');
  const perms = p.permissions.map((x) => `"${x.id}"`).join(', ');
  return `[plugins.overrides.${id}]
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
        <span class="tag">${icon('gavel')}${esc(p.license)}</span>
        ${v.channel === 'beta' ? `<span class="tag tag-tertiary">${icon('experiment')}Beta</span>` : ''}
        <span class="tag">${icon('memory')}WASM</span>
      </div>
      <h1 class="display-md">${esc(p.name)}</h1>
      <p class="lead">${esc(p.summary)}</p>
      <p class="muted p-meta">${icon('download')}${num(p.downloads)} downloads · ${icon('favorite')}${num(p.followers)} followers · ${icon('update')}updated ${ago(v.date)}</p>
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
        ${p.versions.map((x) => `<m3e-menu-item href="${esc(assetUrl(p.slug, x.version, x.file))}" target="_blank">${icon('deployed_code', { slot: 'icon' })}v${esc(x.version)} · ${esc(bytes(x.size))}</m3e-menu-item>`).join('')}
      </m3e-menu>
      <div class="p-actions-row">
        ${favButton(p.slug, 'tonal')}
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
            <span class="muted">${date(v.date)} · Pumpkin ${esc(v.pumpkin)} · MC ${esc(v.minecraft)}</span>
          </span>
          <div class="ver-body">
            <div class="prose">${md(v.changelog)}</div>
            <div class="ver-file">
              ${icon('deployed_code')}<code>${esc(v.file)}</code><span class="muted">${bytes(v.size)}</span>
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
            ${icon('person', { cls: 'credit-i' })}
            <div><span class="label">Original plugin</span><h3>${esc(p.original.name)}</h3>
            <p class="muted">by ${esc(p.original.author)} · ${esc(p.original.license)}</p>
            ${p.original.url ? `<m3e-button variant="text" href="${esc(p.original.url)}" target="_blank" rel="noopener">View on Modrinth ${icon('open_in_new', { slot: 'trailing-icon' })}</m3e-button>` : ''}</div>
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
          <dt>${icon('deployed_code')}Pumpkin</dt><dd>${esc(v.pumpkin)}</dd>
          <dt>${icon('sports_esports')}Minecraft</dt><dd>${esc(v.minecraft)}</dd>
          <dt>${icon('memory')}Runtime</dt><dd>WASM component</dd>
          <dt>${icon('sd_card')}Size</dt><dd>${bytes(v.size)}</dd>
        </dl>
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
          <a href="${GITHUB}/issues/new?title=${encodeURIComponent(`[${p.name}] `)}&labels=bug" target="_blank" rel="noopener">${icon('bug_report')}Report a bug</a>
          <a href="/docs">${icon('menu_book')}Install guide</a>
        </div>
      </div>
    </m3e-card>
  </aside>`;
}

const plugin: Page = {
  title: (c: Ctx) => pluginBySlug(c.params.slug)?.name ?? 'Plugin not found',
  render(c) {
    const p = pluginBySlug(c.params.slug);
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
        <div class="grid">${more.map(pluginCard).join('')}</div></section>` : ''}
    </div>`;
  },
  mount(root, c) {
    const p = pluginBySlug(c.params.slug);
    if (p) pushRecent(p.slug);
    return reveal(root);
  },
};

export default plugin;
