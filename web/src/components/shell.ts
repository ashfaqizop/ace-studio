import { GITHUB, SITE } from '../config';
import { icon, esc } from '../lib/html';
import { navigate } from '../lib/router';
import { prefs, onPrefs } from '../lib/prefs';
import { appearanceDialog, mountAppearance } from './appearance';
import { brand } from '../lib/brands';
import { sprite } from '../data/crew';

export const NAV: { href: string; label: string; icon: string; mobile: boolean }[] = [
  { href: '/', label: 'Home', icon: 'home', mobile: true },
  { href: '/plugins', label: 'Browse', icon: 'extension', mobile: true },
  { href: '/roadmap', label: 'Roadmap', icon: 'view_kanban', mobile: true },
  { href: '/request', label: 'Request', icon: 'add_circle', mobile: true },
  { href: '/docs', label: 'Install', icon: 'menu_book', mobile: true },
  { href: '/crew', label: 'Crew', icon: 'groups', mobile: false },
];

const navItem = (n: (typeof NAV)[number]) =>
  `<m3e-nav-item href="${n.href}" data-nav="${n.href}">${icon(n.icon, { slot: 'icon' })}${icon(n.icon, { slot: 'selected-icon', filled: true })}${esc(n.label)}</m3e-nav-item>`;

export function renderShell(root: HTMLElement): HTMLElement {
  root.innerHTML = `
  <a class="skip" href="#outlet">Skip to content</a>
  <div class="shell">
    <aside class="rail">
      <a class="rail-logo" href="/" aria-label="${SITE.name} home">${logo()}</a>
      <m3e-nav-rail mode="compact" aria-label="Main">${NAV.map(navItem).join('')}</m3e-nav-rail>
      <div class="rail-foot">${sprite('ambient/spin', 1, 'rail-dj')}</div>
    </aside>
    <div class="main">
      <header class="topbar">
        <a class="brand" href="/">${logo()}<span class="brand-name">Ace <em>Studio</em></span></a>
        <form class="topsearch" role="search" action="/plugins">
          <m3e-search-bar clearable>
            ${icon('search', { slot: 'leading' })}
            <input slot="input" name="q" type="search" placeholder="Search ports…" aria-label="Search ports" autocomplete="off" />
            <kbd slot="trailing" class="kbd-hint" aria-hidden="true">/</kbd>
          </m3e-search-bar>
        </form>
        <div class="topactions">
          <span class="saved-wrap">
            <m3e-icon-button id="saved-btn" href="/plugins?saved=1" aria-label="Saved plugins">${icon('favorite')}</m3e-icon-button>
            <m3e-badge for="saved-btn" class="saved-badge" size="small" hidden></m3e-badge>
          </span>
          <m3e-tooltip for="saved-btn">Saved plugins</m3e-tooltip>
          <m3e-icon-button id="theme-btn" aria-label="Appearance">
            ${icon('palette')}
            <m3e-dialog-trigger for="appearance"></m3e-dialog-trigger>
          </m3e-icon-button>
          <m3e-tooltip for="theme-btn">Appearance</m3e-tooltip>
          <m3e-icon-button id="gh-btn" href="${GITHUB}" target="_blank" rel="noopener" aria-label="GitHub">${brand('github', { size: 22 })}</m3e-icon-button>
          <m3e-tooltip for="gh-btn">Source on GitHub</m3e-tooltip>
        </div>
      </header>
      <main id="outlet" tabindex="-1"></main>
      <footer class="footer">
        <div class="footer-crew" aria-hidden="true">
          ${sprite('ambient/nimbus', 1)}${sprite('claude/scribe', 1)}${sprite('grok/courier', 1)}${sprite('codex/sentinel', 1)}${sprite('ambient/brew', 1)}
        </div>
        <div class="footer-cols">
          <div>
            <p class="footer-brand">Ace <em>Studio</em></p>
            <p class="muted">${esc(SITE.tagline)}. An independent project, not affiliated with Modrinth, Mojang or the Pumpkin team.</p>
          </div>
          <div class="footer-brands" aria-label="Built with">
            <a href="https://modrinth.com/plugins" target="_blank" rel="noopener" class="fb">${brand('modrinth', { size: 18, tint: true })}<span>Ports from Modrinth</span></a>
            <a href="https://pumpkinmc.org" target="_blank" rel="noopener" class="fb">${logo()}<span>For PumpkinMC</span></a>
            <a href="https://webassembly.org" target="_blank" rel="noopener" class="fb">${brand('webassembly', { size: 18, tint: true })}<span>Runs as WebAssembly</span></a>
            <a href="${GITHUB}" target="_blank" rel="noopener" class="fb">${brand('github', { size: 18 })}<span>Open source on GitHub</span></a>
            <a href="https://discord.gg/aaNuD6rFEe" target="_blank" rel="noopener" class="fb">${brand('discord', { size: 18, tint: true })}<span>PumpkinMC Discord</span></a>
          </div>
          <nav aria-label="Footer">
            ${NAV.map((n) => `<a href="${n.href}">${esc(n.label)}</a>`).join('')}
            <a href="${GITHUB}" target="_blank" rel="noopener">GitHub</a>
          </nav>
        </div>
      </footer>
    </div>
    <m3e-nav-bar class="bottomnav" mode="compact" aria-label="Main">${NAV.filter((n) => n.mobile).map(navItem).join('')}</m3e-nav-bar>
  </div>
  ${appearanceDialog()}`;

  mountAppearance(root);
  wireSearch(root);
  const syncSaved = () => {
    const b = root.querySelector<HTMLElement>('.saved-badge')!;
    const n = prefs().favorites.length;
    b.textContent = String(n);
    b.toggleAttribute('hidden', n === 0);
  };
  onPrefs(syncSaved);
  syncSaved();
  return root.querySelector<HTMLElement>('#outlet')!;
}

function wireSearch(root: HTMLElement): void {
  const form = root.querySelector<HTMLFormElement>('.topsearch')!;
  const input = form.querySelector<HTMLInputElement>('input')!;
  form.addEventListener('submit', (e) => {
    e.preventDefault();
    const q = input.value.trim();
    navigate(`/plugins${q ? `?q=${encodeURIComponent(q)}` : ''}`);
    input.blur();
  });
  addEventListener('keydown', (e) => {
    const t = e.target as HTMLElement;
    const typing = t.closest('input, textarea, [contenteditable], m3e-select');
    if (e.key === '/' && !typing) {
      e.preventDefault();
      (document.querySelector<HTMLInputElement>('[data-page-search]') ?? input).focus();
    } else if (e.key === 'Escape' && t === input) input.blur();
  });
}

/** Highlight the nav item for the current route. */
export function syncNav(path: string): void {
  const base = '/' + (path.split('/')[1] ?? '');
  const match = base === '/plugin' ? '/plugins' : base;
  document.querySelectorAll<HTMLElement & { selected?: boolean }>('[data-nav]').forEach((n) => {
    const on = n.dataset.nav === match;
    n.selected = on;
    n.toggleAttribute('selected', on);
  });
  const top = document.querySelector<HTMLInputElement>('.topsearch input');
  if (top && match !== '/plugins') top.value = '';
}

export function logo(): string {
  return `<svg class="logo" viewBox="0 0 16 16" aria-hidden="true" shape-rendering="crispEdges">
    <path fill="#2b0e02" d="M7 1h2v1H7zM3 3h4v1H3zm6 0h4v1H9zM2 4h12v1H2zM1 5h14v8H1zm1 8h12v1H2zm1 1h10v1H3z"/>
    <path fill="var(--md-sys-color-primary, #f98317)" d="M3 4h10v1H3zM2 5h12v8H2zm1 8h10v1H3z"/>
    <path fill="#00000033" d="M4 5h1v8H4zm7 0h1v8h-1z"/>
    <path fill="#ffffff55" d="M2 5h2v1H2z"/>
    <path fill="#c47731" d="M7 2h1v2H7z"/><path fill="#8f4a0e" d="M8 2h1v2H8z"/>
    <path fill="var(--md-sys-color-on-primary-container, #140700)" d="M7 6h2v1H7zM6 7h4v1H6zM5 8h6v2H5zm2 2h2v1H7zm-1 1h4v1H6z"/>
  </svg>`;
}
