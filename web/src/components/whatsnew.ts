import { PLUGINS, latest, type Plugin } from '../data/plugins';
import { ago, esc, icon } from '../lib/html';
import { prefs, isNew } from '../lib/prefs';
import { pluginIcon } from './cards';
import { sprite } from '../data/crew';

/** Ports released or updated since the visitor's previous visit. */
export function newSinceLastVisit(): { p: Plugin; fresh: boolean }[] {
  if (!prefs().prevVisit) return [];
  return PLUGINS.filter((p) => isNew(latest(p).date))
    .map((p) => ({ p, fresh: p.versions.every((v) => isNew(v.date)) }))
    .sort((a, b) => latest(b.p).date.localeCompare(latest(a.p).date));
}

export function whatsNewDialog(): string {
  return `
  <m3e-dialog id="whatsnew" dismissible class="whatsnew">
    <span slot="header" class="appearance-head">${sprite('ambient/tick', 1)} What’s new</span>
    <div class="wn-body" id="wn-body"></div>
    <div slot="actions" end>
      <m3e-button variant="text" href="/plugins?sort=updated"><m3e-dialog-action>All updates</m3e-dialog-action></m3e-button>
      <m3e-button variant="filled"><m3e-dialog-action>Done</m3e-dialog-action></m3e-button>
    </div>
  </m3e-dialog>`;
}

function body(): string {
  const items = newSinceLastVisit();
  const since = prefs().prevVisit;
  if (items.length) {
    return `<p class="muted">Since your last visit ${since ? ago(since) : ''}:</p>
      <m3e-list>${items.map(({ p, fresh }) => `
        <m3e-list-item>
          <span slot="leading">${pluginIcon(p, 40)}</span>
          <a class="li-link" href="/plugin/${esc(p.slug)}">${esc(p.name)}</a>
          <span slot="supporting-text">${fresh ? 'New port' : `Updated to v${esc(latest(p).version)}`} · ${ago(latest(p).date)}</span>
          <span slot="trailing" class="tag ${fresh ? 'tag-new' : 'tag-primary'}">${fresh ? 'New' : 'Update'}</span>
        </m3e-list-item>`).join('')}</m3e-list>`;
  }
  const recent = [...PLUGINS].sort((a, b) => latest(b).date.localeCompare(latest(a).date)).slice(0, 5);
  if (!recent.length) {
    return `<div class="wn-empty">${sprite('claude/forge', 2)}<p>No ports have shipped yet. The first releases will show up here, and anything that lands between your visits gets a <span class="tag tag-new">${icon('new_releases', { filled: true })}New</span> badge.</p></div>`;
  }
  return `<p class="muted">You’re all caught up. Latest releases:</p>
    <m3e-list>${recent.map((p) => `
      <m3e-list-item>
        <span slot="leading">${pluginIcon(p, 40)}</span>
        <a class="li-link" href="/plugin/${esc(p.slug)}">${esc(p.name)}</a>
        <span slot="supporting-text">v${esc(latest(p).version)} · ${ago(latest(p).date)}</span>
      </m3e-list-item>`).join('')}</m3e-list>`;
}

export function mountWhatsNew(root: HTMLElement): void {
  const dlg = root.querySelector<HTMLElement>('#whatsnew')!;
  dlg.addEventListener('opening', () => (dlg.querySelector('#wn-body')!.innerHTML = body()));
  dlg.addEventListener('click', (e) => {
    if ((e.target as HTMLElement).closest('a.li-link')) (dlg as HTMLElement & { hide(): void }).hide();
  });
  const badge = root.querySelector<HTMLElement>('.new-badge');
  const n = newSinceLastVisit().length;
  if (badge) {
    badge.textContent = String(n);
    badge.toggleAttribute('hidden', n === 0);
  }
}
