import { CATEGORIES, latest, type Plugin } from '../data/plugins';
import { char, sprite, type CharRef } from '../data/crew';
import { ago, esc, icon, num } from '../lib/html';
import { isFavorite, isNew, prefs } from '../lib/prefs';
import { compat } from '../lib/compat';

/** The plugin's expressive icon: an m3e-shape with a Material Symbol inside. */
export function pluginIcon(p: Plugin, size = 56): string {
  return `<m3e-shape class="plugin-icon" name="${esc(p.icon.shape)}" style="--m3e-shape-size:${size}px">${icon(p.icon.symbol, { filled: true })}</m3e-shape>`;
}

export function favButton(slug: string, variant = 'standard'): string {
  const on = isFavorite(slug);
  return `<m3e-icon-button class="fav" toggle variant="${variant}" data-fav="${esc(slug)}"${on ? ' selected' : ''} aria-label="Save plugin">
    ${icon('favorite')}${icon('favorite', { slot: 'selected', filled: true })}
  </m3e-icon-button>`;
}

export function compareButton(slug: string, variant = 'standard'): string {
  const on = prefs().compare.includes(slug);
  return `<m3e-icon-button class="cmp" toggle variant="${variant}" data-compare="${esc(slug)}"${on ? ' selected' : ''} aria-label="Compare">
    ${icon('compare_arrows')}${icon('library_add_check', { slot: 'selected', filled: true })}
  </m3e-icon-button>`;
}

/** Sandbox badge: how many Pumpkin capabilities the plugin asks for. */
export function permBadge(p: Plugin): string {
  const n = p.permissions.length;
  return n === 0
    ? `<span class="tag tag-safe" title="Runs with no extra Pumpkin capabilities">${icon('verified_user', { filled: true })}No permissions</span>`
    : `<span class="tag" title="${esc(p.permissions.map((x) => x.id).join(', '))}">${icon('shield')}${n} permission${n === 1 ? '' : 's'}</span>`;
}

/** Compatibility with the visitor's chosen Pumpkin version (empty until they pick one). */
export function compatBadge(p: Plugin): string {
  const c = compat(p);
  switch (c.state) {
    case 'ok':
      return `<span class="tag tag-safe" title="Works on Pumpkin ${esc(prefs().server)}">${icon('check_circle', { filled: true })}Works on your server</span>`;
    case 'older':
      return `<span class="tag tag-tertiary" title="The latest release needs a newer Pumpkin">${icon('history')}v${esc(c.version.version)} works for you</span>`;
    case 'no':
      return `<span class="tag tag-warn" title="Update your server to run this">${icon('upgrade')}Needs Pumpkin ${esc(c.needs)}+</span>`;
    default:
      return '';
  }
}

export const newBadge = (p: Plugin) => (isNew(latest(p).date) ? `<span class="tag tag-new">${icon('new_releases', { filled: true })}New</span>` : '');

export function pluginCard(p: Plugin, nameHtml?: string): string {
  const v = latest(p);
  const porter = char(p.portedBy);
  return `
  <m3e-card class="plugin-card" variant="filled" actionable href="/plugin/${esc(p.slug)}" data-reveal>
    <div slot="header" class="pc-head">
      ${pluginIcon(p)}
      <div class="pc-title">
        <h3>${nameHtml ?? esc(p.name)}</h3>
        <span class="muted">port of ${esc(p.original.name)}</span>
      </div>
      <div class="pc-btns">${compareButton(p.slug)}${favButton(p.slug)}</div>
    </div>
    <div slot="content" class="pc-body">
      <p>${esc(p.summary)}</p>
      <div class="pc-cats">
        ${newBadge(p)}${compatBadge(p)}${permBadge(p)}
        ${p.categories.map((c) => `<span class="tag">${icon(CATEGORIES[c]?.icon ?? 'label')}${esc(CATEGORIES[c]?.label ?? c)}</span>`).join('')}
      </div>
    </div>
    <div slot="footer" class="pc-foot">
      <span title="Downloads">${icon('download')}<span data-dl="${esc(p.slug)}">${num(p.downloads)}</span></span>
      <span title="Latest version">${icon('sell')}v${esc(v.version)}${v.channel === 'beta' ? ' <b class="beta">beta</b>' : ''}</span>
      <span title="Updated">${icon('update')}${ago(v.date)}</span>
      <span class="pc-porter" title="Ported by ${esc(porter.name)}">${sprite(p.portedBy, 0.5)}</span>
    </div>
  </m3e-card>`;
}

/** Wording for a frozen port: blocked on the Pumpkin plugin API, or on the original author's permission. */
export function waitKind(p: Plugin) {
  const n = p.waiting?.needs.length ?? 0;
  if (p.status === 'author-declined')
    return {
      badge: 'Port declined by author',
      title: "The original's author doesn't allow ports or translations of their code",
      icon: 'block',
      state: 'Not planned',
      count: 'Port declined',
      tab: 'Why it&rsquo;s not planned',
      callout: 'We asked, and the original&rsquo;s author doesn&rsquo;t allow ports or translations of their code. We respect that, so nothing of theirs will be ported.',
      speech: 'The author said no to a port, so none of their code will be used. Here’s what that leaves.',
      footnote: 'A clean-room implementation (our own code, written only from observed behaviour, never from the original source) is the only route left. It isn&rsquo;t started.',
    };
  return p.status === 'waiting-permission'
    ? {
        badge: 'Waiting on author permission',
        title: "The original isn't openly licensed, so we only port it with its author's written permission",
        icon: 'front_hand',
        state: 'Waiting on permission',
        count: 'Needs permission',
        tab: 'What we&rsquo;re waiting on',
        callout: 'The original plugin isn&rsquo;t openly licensed, so we&rsquo;ve asked its author for permission to port it. Nothing ships until they agree.',
        speech: 'The original is All Rights Reserved, so I haven’t ported any of it. We’ve asked the author, and this is what has to happen first.',
        footnote: 'We only port closed-source or All Rights Reserved plugins with written permission. If the author says no, this listing comes down.',
      }
    : {
        badge: 'Waiting on Pumpkin Plugin API',
        title: "We attempted this port, but Pumpkin's plugin API can't support it yet",
        icon: 'extension_off',
        state: 'Waiting on API',
        count: `${n} missing API${n === 1 ? '' : 's'}`,
        tab: 'What&rsquo;s missing',
        callout: 'We attempted this port, but Pumpkin&rsquo;s plugin API can&rsquo;t support it yet. It ships as soon as Pumpkin adds what&rsquo;s listed under <strong>What&rsquo;s missing</strong>.',
        speech: `I mapped the whole plugin, but Pumpkin’s plugin API is missing ${n === 1 ? 'one piece' : `${n} pieces`} it depends on. Here’s exactly what, and why.`,
        footnote: 'We don&rsquo;t ship workarounds that only half-match the original. When a Pumpkin plugin API release adds these, the crew picks the port back up.',
      };
}

/** Status badge for a frozen port (see waitKind). Without a plugin it's the generic "frozen" heading tag. */
export const waitingBadge = (p?: Plugin) => {
  const k = p ? waitKind(p) : { badge: 'Frozen ports', title: 'Ports we attempted that are waiting on something outside our control' };
  return `<span class="tag tag-waiting" title="${esc(k.title)}">${icon('hourglass_top', { filled: true })}${k.badge}</span>`;
};

/** Card for a waiting port: same shape as a plugin card, but no download, version or compare. */
export function waitingCard(p: Plugin, nameHtml?: string): string {
  const porter = char(p.portedBy);
  const k = waitKind(p);
  return `
  <m3e-card class="plugin-card waiting-card" variant="outlined" actionable href="/plugin/${esc(p.slug)}" data-reveal>
    <div slot="header" class="pc-head">
      ${pluginIcon(p)}
      <div class="pc-title">
        <h3>${nameHtml ?? esc(p.name)}</h3>
        <span class="muted">port of ${esc(p.original.name)}</span>
      </div>
      <div class="pc-btns">${favButton(p.slug)}</div>
    </div>
    <div slot="content" class="pc-body">
      <p>${esc(p.summary)}</p>
      <div class="pc-cats">
        ${waitingBadge(p)}
        ${p.categories.map((c) => `<span class="tag">${icon(CATEGORIES[c]?.icon ?? 'label')}${esc(CATEGORIES[c]?.label ?? c)}</span>`).join('')}
      </div>
    </div>
    <div slot="footer" class="pc-foot">
      <span title="${esc(k.state)}">${icon(k.icon)}${k.count}</span>
      ${p.waiting?.since ? `<span title="Attempted">${icon('pause_circle')}since ${ago(p.waiting.since)}</span>` : ''}
      <span class="pc-porter" title="Attempted by ${esc(porter.name)}">${sprite(p.portedBy, 0.5)}</span>
    </div>
  </m3e-card>`;
}

export function pluginRow(p: Plugin, nameHtml?: string): string {
  const v = latest(p);
  return `
  <m3e-card class="plugin-row" variant="outlined" actionable href="/plugin/${esc(p.slug)}" orientation="horizontal" data-reveal>
    <div slot="content" class="pr-body">
      ${pluginIcon(p, 44)}
      <div class="pr-main">
        <h3>${nameHtml ?? esc(p.name)} <span class="muted">v${esc(v.version)}</span> ${newBadge(p)}</h3>
        <p class="muted">${esc(p.summary)}</p>
      </div>
      <div class="pr-stats">
        ${compatBadge(p)}${permBadge(p)}
        <span>${icon('download')}<span data-dl="${esc(p.slug)}">${num(p.downloads)}</span></span>
        <span>${icon('update')}${ago(v.date)}</span>
        <span class="tag">${esc(p.license)}</span>
      </div>
      ${compareButton(p.slug)}${favButton(p.slug)}
    </div>
  </m3e-card>`;
}

/** A character-led empty state. */
export function emptyState(ref: CharRef, title: string, body: string, actions = ''): string {
  return `
  <div class="empty" role="status">
    <div class="empty-art">
      <m3e-shape name="soft-burst" class="empty-shape"></m3e-shape>
      ${sprite(ref, 3, 'empty-sprite')}
    </div>
    <h2 class="serif">${esc(title)}</h2>
    <p class="muted">${body}</p>
    ${actions ? `<div class="empty-actions">${actions}</div>` : ''}
  </div>`;
}

/** Speech bubble attached to a character. */
export function speaker(ref: CharRef, line: string, scale = 2): string {
  const c = char(ref);
  return `<figure class="speaker">${sprite(ref, scale)}<figcaption class="bubble"><strong>${esc(c.name)}</strong>${esc(line)}</figcaption></figure>`;
}
