import { pluginBySlug } from '../data/plugins';
import { esc, icon } from '../lib/html';
import { onPrefs, prefs, setPrefs, MAX_COMPARE } from '../lib/prefs';
import { pluginIcon } from './cards';

/** Floating tray listing ports queued for comparison. Hidden on the compare page itself. */
export function mountCompareTray(): () => void {
  const el = document.createElement('div');
  el.className = 'cmp-tray';
  el.setAttribute('role', 'region');
  el.setAttribute('aria-label', 'Compare selection');
  document.body.append(el);
  const render = () => {
    const list = prefs().compare.map(pluginBySlug).filter(Boolean);
    const hide = !list.length || location.pathname === '/compare';
    el.toggleAttribute('hidden', hide);
    if (hide) return;
    el.innerHTML = `
      <div class="cmp-items">
        ${list.map((p) => `<span class="cmp-chip">${pluginIcon(p!, 24)}<span>${esc(p!.name)}</span>
          <button type="button" data-compare="${esc(p!.slug)}" aria-label="Remove ${esc(p!.name)}">${icon('close')}</button></span>`).join('')}
        ${Array.from({ length: MAX_COMPARE - list.length }, () => '<span class="cmp-slot" aria-hidden="true"></span>').join('')}
      </div>
      <div class="cmp-actions">
        <m3e-button variant="text" id="cmp-clear">Clear</m3e-button>
        <m3e-button variant="filled" href="/compare"${list.length < 2 ? ' disabled' : ''}>${icon('compare_arrows', { slot: 'icon' })}Compare ${list.length}</m3e-button>
      </div>`;
    el.querySelector('#cmp-clear')!.addEventListener('click', () => setPrefs({ compare: [] }));
  };
  const off = onPrefs(render);
  addEventListener('popstate', render);
  render();
  return () => {
    off();
    el.remove();
  };
}

/** Re-evaluate visibility after client-side navigation. */
export function syncCompareTray(): void {
  const el = document.querySelector<HTMLElement>('.cmp-tray');
  if (el) el.toggleAttribute('hidden', !prefs().compare.length || location.pathname === '/compare');
}
