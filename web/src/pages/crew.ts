import type { Page } from '../lib/router';
import { CREW, SETS, sprite, type CharSet } from '../data/crew';
import { esc, icon } from '../lib/html';
import { reveal } from '../lib/ui';

type Chip = HTMLElement & { selected?: boolean };

const crew: Page = {
  title: 'Meet the crew',
  render() {
    const sets = Object.keys(SETS) as CharSet[];
    return `
    <section class="page-head">
      <span class="eyebrow">${icon('groups')} The crew</span>
      <h1 class="display-md">Meet the <em>crew</em></h1>
      <p class="lead">${CREW.length} tiny pixel colleagues work on every port: scouting the original source, forging the Pumpkin version, testing it and shipping it. Click anyone to say hi.</p>
    </section>
    <m3e-filter-chip-set id="set-filter" aria-label="Filter by set">
      <m3e-filter-chip value="all" selected>${icon('apps', { slot: 'icon' })}Everyone</m3e-filter-chip>
      ${sets.map((s) => `<m3e-filter-chip value="${s}">${esc(SETS[s].label)}</m3e-filter-chip>`).join('')}
    </m3e-filter-chip-set>
    ${sets.map((s) => `
    <section class="crew-set" data-set="${s}">
      <div class="band-head"><div><h2 class="headline">${esc(SETS[s].label)}</h2><p class="muted">${esc(SETS[s].blurb)}</p></div></div>
      <div class="crew-grid">
        ${CREW.filter((c) => c.set === s).map((c) => `
        <button type="button" class="crew-card" id="crew-${c.set}-${c.id}" data-reveal data-line="${esc(c.line)}">
          <span class="crew-art">${sprite(`${c.set}/${c.id}`, 2)}</span>
          <span class="crew-name">${esc(c.name)}</span>
          <span class="crew-role muted">${esc(c.role)}</span>
          <span class="crew-line bubble" aria-live="polite"></span>
        </button>`).join('')}
      </div>
    </section>`).join('')}`;
  },
  mount(root) {
    root.querySelector('#set-filter')!.addEventListener('change', (e) => {
      const chips = [...(e.currentTarget as HTMLElement).querySelectorAll<Chip>('m3e-filter-chip')];
      const v = chips.find((c) => c.selected)?.getAttribute('value') ?? 'all';
      root.querySelectorAll<HTMLElement>('.crew-set').forEach((sec) => sec.toggleAttribute('hidden', v !== 'all' && sec.dataset.set !== v));
    });
    root.querySelectorAll<HTMLButtonElement>('.crew-card').forEach((b) =>
      b.addEventListener('click', () => {
        root.querySelectorAll('.crew-card.talking').forEach((o) => o !== b && o.classList.remove('talking'));
        b.querySelector('.crew-line')!.textContent = b.dataset.line!;
        b.classList.toggle('talking');
      }),
    );
    // arriving from the command palette (/crew#crew-set-id): wave hello
    const target = location.hash && root.querySelector<HTMLButtonElement>(location.hash);
    if (target) setTimeout(() => target.click(), 400);
    return reveal(root);
  },
};

export default crew;
