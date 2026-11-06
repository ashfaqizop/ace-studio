import { NAV } from './shell';
import { PLUGINS } from '../data/plugins';
import { CREW, SETS, sprite } from '../data/crew';
import { esc, icon } from '../lib/html';
import { fuzzy, highlight } from '../lib/fuzzy';
import { navigate } from '../lib/router';
import { prefs, setPrefs } from '../lib/prefs';
import { copy, toast } from '../lib/ui';
import { pluginIcon } from './cards';
import { brand } from '../lib/brands';
import { GITHUB } from '../config';

type Dialog = HTMLElement & { show(): Promise<void>; hide(v?: string): Promise<void>; open?: boolean };

interface Item {
  group: 'Actions' | 'Pages' | 'Ports' | 'Crew';
  label: string;
  sub?: string;
  art: string;
  keywords?: string;
  run: () => void;
}

function items(): Item[] {
  const go = (to: string) => () => navigate(to);
  const scheme = (s: 'light' | 'dark' | 'auto', label: string, ic: string): Item => ({
    group: 'Actions', label, art: icon(ic), keywords: 'theme colour mode appearance', run: () => {
      setPrefs({ scheme: s });
      toast(`${label} on`);
    },
  });
  return [
    { group: 'Actions', label: 'Search ports', art: icon('search'), keywords: 'find browse', run: () => document.querySelector<HTMLInputElement>('[data-page-search]')?.focus() ?? navigate('/plugins') },
    { group: 'Actions', label: 'Request a port', art: icon('add_circle'), keywords: 'modrinth new', run: go('/request') },
    scheme('dark', 'Dark mode', 'dark_mode'),
    scheme('light', 'Light mode', 'light_mode'),
    scheme('auto', 'Match system theme', 'brightness_auto'),
    { group: 'Actions', label: 'Appearance settings', art: icon('palette'), keywords: 'accent colour theme', run: () => (document.getElementById('appearance') as Dialog | null)?.show() },
    { group: 'Actions', label: prefs().buddy ? 'Hide Pixel the cat' : 'Show Pixel the cat', art: sprite('ambient/pixel', 0.5), keywords: 'buddy mascot', run: () => setPrefs({ buddy: !prefs().buddy }) },
    { group: 'Actions', label: 'Copy link to this page', art: icon('link'), keywords: 'share url', run: () => copy(location.href, 'Link copied') },
    { group: 'Actions', label: 'Compare saved selection', sub: prefs().compare.length ? `${prefs().compare.length} selected` : 'Nothing selected yet', art: icon('compare_arrows'), run: go('/compare') },
    { group: 'Actions', label: 'What’s new', art: icon('new_releases'), keywords: 'updates releases changelog', run: () => (document.getElementById('whatsnew') as Dialog | null)?.show() },
    ...(import.meta.env.DEV
      ? [{ group: 'Actions' as const, label: 'Dev: pretend last visit was 2 weeks ago', sub: 'Shows “New” badges for recent demo releases', art: icon('bug_report'), run: () => {
          setPrefs({ prevVisit: new Date(Date.now() - 14 * 86_400_000).toISOString() });
          location.reload();
        } }]
      : []),
    { group: 'Actions', label: 'Open GitHub repository', art: brand('github', { size: 20 }), keywords: 'source code issues', run: () => open(GITHUB, '_blank', 'noopener') },
    ...NAV.map((n): Item => ({ group: 'Pages', label: n.label, sub: n.href, art: icon(n.icon), run: go(n.href) })),
    { group: 'Pages', label: 'Saved plugins', sub: '/plugins?saved=1', art: icon('favorite'), run: go('/plugins?saved=1') },
    { group: 'Pages', label: 'Compare ports', sub: '/compare', art: icon('compare_arrows'), run: go('/compare') },
    ...PLUGINS.map((p): Item => ({ group: 'Ports', label: p.name, sub: p.summary, art: pluginIcon(p, 28), keywords: `${p.original.name} ${p.categories.join(' ')}`, run: go(`/plugin/${p.slug}`) })),
    ...CREW.map((c): Item => ({ group: 'Crew', label: c.name, sub: `${c.role} · ${SETS[c.set].label}`, art: sprite(`${c.set}/${c.id}`, 0.5), keywords: c.role, run: go(`/crew#crew-${c.set}-${c.id}`) })),
  ];
}

const ORDER: Item['group'][] = ['Actions', 'Pages', 'Ports', 'Crew'];

export function paletteDialog(): string {
  return `
  <m3e-dialog id="palette" class="palette" dismissible close-label="Close command palette">
    <div class="pal">
      <m3e-search-bar class="pal-search">
        ${icon('keyboard_command_key', { slot: 'leading' })}
        <input slot="input" id="pal-q" type="text" placeholder="Jump to a page, port or crew member, or run an action…" autocomplete="off" spellcheck="false" aria-label="Command palette" aria-controls="pal-list" />
      </m3e-search-bar>
      <div class="pal-list" id="pal-list" role="listbox" aria-label="Results"></div>
      <div class="pal-foot muted small"><kbd>↑</kbd><kbd>↓</kbd> move <kbd>Enter</kbd> open <kbd>Esc</kbd> close</div>
    </div>
  </m3e-dialog>`;
}

export function mountPalette(root: HTMLElement): void {
  const dlg = root.querySelector<Dialog>('#palette')!;
  const input = dlg.querySelector<HTMLInputElement>('#pal-q')!;
  const list = dlg.querySelector<HTMLElement>('#pal-list')!;
  let shown: Item[] = [];
  let active = 0;

  const render = () => {
    const q = input.value.trim();
    const all = items();
    let ranked: { it: Item; score: number; html: string }[];
    if (!q) {
      ranked = all.filter((it) => it.group !== 'Crew').map((it) => ({ it, score: 0, html: esc(it.label) }));
    } else {
      ranked = [];
      for (const it of all) {
        const m = fuzzy(q, it.label);
        const k = m ? null : fuzzy(q, `${it.sub ?? ''} ${it.keywords ?? ''}`);
        if (m) ranked.push({ it, score: m.score * 2, html: highlight(it.label, m.hits) });
        else if (k) ranked.push({ it, score: k.score * 0.6, html: esc(it.label) });
      }
      ranked.sort((a, b) => b.score - a.score);
      ranked = ranked.slice(0, 24);
    }
    // keep group order stable, best items first within each group
    const groups = ORDER.map((g) => ranked.filter((r) => r.it.group === g)).filter((g) => g.length);
    shown = groups.flat().map((r) => r.it);
    active = Math.min(active, Math.max(0, shown.length - 1));
    let i = 0;
    list.innerHTML = shown.length
      ? groups.map((g) => `<div class="pal-group"><span class="label">${g[0].it.group}</span>${g.map((r) => {
          const idx = i++;
          return `<div class="pal-item" role="option" id="pal-${idx}" data-i="${idx}" aria-selected="${idx === active}">
            <span class="pal-art">${r.it.art}</span>
            <span class="pal-text"><span class="pal-label">${r.html}</span>${r.it.sub ? `<span class="pal-sub">${esc(r.it.sub)}</span>` : ''}</span>
            ${idx === active ? `<span class="pal-enter">${icon('keyboard_return')}</span>` : ''}
          </div>`;
        }).join('')}</div>`).join('')
      : `<div class="pal-empty">${sprite('grok/scout', 1)}<span>Nothing matches “${esc(q)}”.</span></div>`;
    input.setAttribute('aria-activedescendant', shown.length ? `pal-${active}` : '');
    list.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: 'nearest' });
  };

  const run = (i: number) => {
    const it = shown[i];
    if (!it) return;
    dlg.hide().then(() => it.run());
  };

  input.addEventListener('input', () => {
    active = 0;
    render();
  });
  input.addEventListener('keydown', (e) => {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      if (!shown.length) return;
      active = (active + (e.key === 'ArrowDown' ? 1 : -1) + shown.length) % shown.length;
      render();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      run(active);
    }
  });
  list.addEventListener('click', (e) => {
    const el = (e.target as HTMLElement).closest<HTMLElement>('.pal-item');
    if (el) run(Number(el.dataset.i));
  });
  list.addEventListener('mousemove', (e) => {
    const el = (e.target as HTMLElement).closest<HTMLElement>('.pal-item');
    if (el && Number(el.dataset.i) !== active) {
      active = Number(el.dataset.i);
      list.querySelectorAll('.pal-item').forEach((x) => x.setAttribute('aria-selected', String(x === el)));
    }
  });
  dlg.addEventListener('opened', () => {
    input.focus();
    input.select();
  });

  const openPalette = () => {
    input.value = '';
    active = 0;
    render();
    dlg.show();
  };
  addEventListener('keydown', (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      if (dlg.open) dlg.hide();
      else openPalette();
    }
  });
  root.querySelector('#palette-btn')?.addEventListener('click', openPalette);
}
