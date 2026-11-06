import type { Page } from '../lib/router';
import { ROADMAP, STAGES, type Stage } from '../data/roadmap';
import { char, sprite } from '../data/crew';
import { esc, icon } from '../lib/html';
import { reveal } from '../lib/ui';
import { speaker } from '../components/cards';

type Seg = HTMLElement & { checked?: boolean };

const ORDER = Object.keys(STAGES) as Stage[];

const roadmap: Page = {
  title: 'Porting roadmap',
  render() {
    return `
    <section class="page-head">
      <span class="eyebrow">${icon('view_kanban')} Roadmap</span>
      <h1 class="display-md">What the crew is <em>porting</em></h1>
      <p class="lead">Every port moves through four stages. Vote by requesting a plugin; the most wanted ones jump the queue.</p>
    </section>
    ${ROADMAP.length ? '' : `<div class="roadmap-empty">${speaker('codex/architect', 'The board is fresh. The first requests that pass the licence check land in Queued.')}
      <m3e-button variant="filled" href="/request">${icon('add_circle', { slot: 'icon' })}Request a port</m3e-button></div>`}
    <div class="roadmap-filter">
      <m3e-segmented-button id="stage-filter" aria-label="Filter by stage">
        <m3e-button-segment value="all" checked>All</m3e-button-segment>
        ${ORDER.map((s) => `<m3e-button-segment value="${s}">${icon(STAGES[s].icon, { slot: 'icon' })}${STAGES[s].label}</m3e-button-segment>`).join('')}
      </m3e-segmented-button>
    </div>
    <div class="board" id="board">
      ${ORDER.map((s) => {
        const items = ROADMAP.filter((r) => r.stage === s).sort((a, b) => b.votes - a.votes);
        const lead = char(STAGES[s].lead);
        return `
        <section class="col col-${s}" data-stage="${s}" aria-label="${STAGES[s].label}">
          <header class="col-head">
            ${sprite(STAGES[s].lead, 1)}
            <div><h2>${icon(STAGES[s].icon)} ${STAGES[s].label} <span class="col-count">${items.length}</span></h2>
            <p class="muted small">${esc(STAGES[s].hint)} Lead: ${esc(lead.name)}.</p></div>
          </header>
          <div class="col-items">
            ${items.map((r) => `
            <m3e-card class="rm-card" variant="${s === 'shipped' ? 'filled' : 'elevated'}"${r.slug ? ` actionable href="/plugin/${esc(r.slug)}"` : ''} data-reveal>
              <div slot="content" class="rm-body">
                <div class="rm-top"><h3>${esc(r.name)}</h3><span class="votes" title="Votes">${icon('arrow_upward')}${r.votes}</span></div>
                <p class="muted">${esc(r.summary)}</p>
                ${s === 'translating' || s === 'testing'
                  ? `<m3e-linear-progress-indicator variant="wavy" value="${r.progress}" aria-label="${r.progress}% done"></m3e-linear-progress-indicator><span class="small muted">${r.progress}% done</span>`
                  : ''}
                <div class="rm-foot">
                  <span class="assignee">${sprite(r.assignee, 0.5)}${esc(char(r.assignee).name)}</span>
                  ${r.slug ? `<span class="tag tag-primary">${icon('open_in_new')}View port</span>` : ''}
                </div>
              </div>
            </m3e-card>`).join('') || `<p class="muted col-empty">Nothing here right now.</p>`}
          </div>
        </section>`;
      }).join('')}
    </div>
    <m3e-callout variant="tertiary" class="roadmap-cta">${icon('lightbulb', { slot: 'icon' })}<span slot="label">Want something on the board?</span>
      <a href="/request">Request a port</a> and we'll check its licence and queue it.
    </m3e-callout>`;
  },
  mount(root) {
    root.querySelector('#stage-filter')!.addEventListener('change', (e) => {
      const on = [...(e.currentTarget as HTMLElement).querySelectorAll<Seg>('m3e-button-segment')].find((x) => x.checked);
      const v = on?.getAttribute('value') ?? 'all';
      root.querySelectorAll<HTMLElement>('.col').forEach((col) => col.toggleAttribute('hidden', v !== 'all' && col.dataset.stage !== v));
      root.querySelector('#board')!.classList.toggle('single', v !== 'all');
    });
    return reveal(root);
  },
};

export default roadmap;
