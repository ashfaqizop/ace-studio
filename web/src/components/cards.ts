import { CATEGORIES, latest, type Plugin } from '../data/plugins';
import { char, sprite, type CharRef } from '../data/crew';
import { ago, esc, icon, num } from '../lib/html';
import { isFavorite } from '../lib/prefs';

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

export function pluginCard(p: Plugin): string {
  const v = latest(p);
  const porter = char(p.portedBy);
  return `
  <m3e-card class="plugin-card" variant="filled" actionable href="/plugin/${esc(p.slug)}" data-reveal>
    <div slot="header" class="pc-head">
      ${pluginIcon(p)}
      <div class="pc-title">
        <h3>${esc(p.name)}</h3>
        <span class="muted">port of ${esc(p.original.name)}</span>
      </div>
      ${favButton(p.slug)}
    </div>
    <div slot="content" class="pc-body">
      <p>${esc(p.summary)}</p>
      <div class="pc-cats">${p.categories.map((c) => `<span class="tag">${icon(CATEGORIES[c]?.icon ?? 'label')}${esc(CATEGORIES[c]?.label ?? c)}</span>`).join('')}</div>
    </div>
    <div slot="footer" class="pc-foot">
      <span title="Downloads">${icon('download')}${num(p.downloads)}</span>
      <span title="Latest version">${icon('sell')}v${esc(v.version)}${v.channel === 'beta' ? ' <b class="beta">beta</b>' : ''}</span>
      <span title="Updated">${icon('update')}${ago(v.date)}</span>
      <span class="pc-porter" title="Ported by ${esc(porter.name)}">${sprite(p.portedBy, 0.5)}</span>
    </div>
  </m3e-card>`;
}

export function pluginRow(p: Plugin): string {
  const v = latest(p);
  return `
  <m3e-card class="plugin-row" variant="outlined" actionable href="/plugin/${esc(p.slug)}" orientation="horizontal" data-reveal>
    <div slot="content" class="pr-body">
      ${pluginIcon(p, 44)}
      <div class="pr-main">
        <h3>${esc(p.name)} <span class="muted">v${esc(v.version)}</span></h3>
        <p class="muted">${esc(p.summary)}</p>
      </div>
      <div class="pr-stats">
        <span>${icon('download')}${num(p.downloads)}</span>
        <span>${icon('update')}${ago(v.date)}</span>
        <span class="tag">${esc(p.license)}</span>
      </div>
      ${favButton(p.slug)}
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
