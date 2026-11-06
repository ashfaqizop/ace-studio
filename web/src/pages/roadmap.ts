import type { Page } from '../lib/router';
import { ROADMAP, STAGES, type Stage, type RoadmapItem } from '../data/roadmap';
import { char, sprite, type CharRef, CREW } from '../data/crew';
import { ago, esc, icon } from '../lib/html';
import { brand } from '../lib/brands';
import { reveal } from '../lib/ui';
import { speaker } from '../components/cards';
import { portRequests, type PortRequest } from '../lib/github';
import { GITHUB } from '../config';

type Seg = HTMLElement & { checked?: boolean };
type Col = Stage | 'requested';

const REQUESTED = { label: 'Requested', icon: 'how_to_vote', lead: 'claude/courier' as CharRef, hint: 'Open requests waiting for a licence check. Upvote the ones you want.' };
const COLS: Col[] = ['requested', 'queued', 'translating', 'testing', 'shipped'];
const meta = (c: Col) => (c === 'requested' ? REQUESTED : STAGES[c]);

interface Card {
  name: string;
  summary: string;
  stage: Col;
  progress: number;
  assignee: CharRef;
  votes: number;
  href?: string;
  external?: boolean;
  foot?: string;
}

/** `crew:forge` or `crew:codex/forge` label → character; otherwise the stage lead. */
function crewFor(label: string | undefined, stage: Col): CharRef {
  if (label) {
    const ref = label.includes('/') ? label : `claude/${label}`;
    if (CREW.some((c) => `${c.set}/${c.id}` === ref)) return ref as CharRef;
  }
  return meta(stage).lead;
}

const fromIssue = (r: PortRequest): Card => ({
  name: r.name,
  summary: r.modrinth ? `modrinth.com/plugin/${r.modrinth}` : `Requested by ${r.author}`,
  stage: r.stage,
  progress: r.progress,
  assignee: crewFor(r.crew, r.stage),
  votes: r.votes,
  href: r.slug && r.stage === 'shipped' ? `/plugin/${r.slug}` : r.url,
  external: !(r.slug && r.stage === 'shipped'),
  foot: `#${r.number} · ${ago(r.updated)}`,
});

const fromStatic = (r: RoadmapItem): Card => ({
  name: r.name, summary: r.summary, stage: r.stage, progress: r.progress, assignee: r.assignee, votes: r.votes,
  href: r.slug ? `/plugin/${r.slug}` : undefined,
});

function card(c: Card): string {
  const s = c.stage;
  const link = c.href ? ` actionable href="${esc(c.href)}"${c.external ? ' target="_blank" rel="noopener"' : ''}` : '';
  return `
  <m3e-card class="rm-card" variant="${s === 'shipped' ? 'filled' : 'elevated'}"${link} data-reveal>
    <div slot="content" class="rm-body">
      <div class="rm-top"><h3>${esc(c.name)}</h3><span class="votes" title="${c.votes} upvotes">${icon('thumb_up')}${c.votes}</span></div>
      <p class="muted">${esc(c.summary)}</p>
      ${s === 'translating' || s === 'testing'
        ? `<m3e-linear-progress-indicator variant="wavy" value="${c.progress}" aria-label="${c.progress}% done"></m3e-linear-progress-indicator><span class="small muted">${c.progress}% done</span>`
        : ''}
      <div class="rm-foot">
        <span class="assignee">${sprite(c.assignee, 0.5)}${esc(char(c.assignee).name)}</span>
        ${c.external ? `<span class="rm-gh">${brand('github', { size: 14 })}${esc(c.foot ?? 'GitHub')}</span>` : c.href ? `<span class="tag tag-primary">${icon('open_in_new')}View port</span>` : ''}
      </div>
    </div>
  </m3e-card>`;
}

function board(cards: Card[]): string {
  return COLS.map((s) => {
    const items = cards.filter((c) => c.stage === s).sort((a, b) => b.votes - a.votes);
    const m = meta(s);
    return `
    <section class="col col-${s}" data-stage="${s}" aria-label="${m.label}">
      <header class="col-head">
        ${sprite(m.lead, 1)}
        <div><h2>${icon(m.icon)} ${m.label} <span class="col-count">${items.length}</span></h2>
        <p class="muted small">${esc(m.hint)}</p></div>
      </header>
      <div class="col-items">${items.map(card).join('') || `<p class="muted col-empty">Nothing here right now.</p>`}</div>
    </section>`;
  }).join('');
}

const skeleton = () =>
  COLS.map((s) => `<section class="col col-${s}"><header class="col-head">${sprite(meta(s).lead, 1)}<div><h2>${icon(meta(s).icon)} ${meta(s).label}</h2></div></header>
    <div class="col-items">${[0, 1].map(() => `<m3e-skeleton animation="wave"><m3e-card variant="elevated"><div slot="content" style="height:96px"></div></m3e-card></m3e-skeleton>`).join('')}</div></section>`).join('');

const roadmap: Page = {
  title: 'Porting roadmap',
  render() {
    return `
    <section class="page-head">
      <span class="eyebrow">${icon('view_kanban')} Roadmap</span>
      <h1 class="display-md">What the crew is <em>porting</em></h1>
      <p class="lead">Live from our GitHub issues. Every request moves through the stages below; 👍 on an issue is a vote, and the most wanted ports jump the queue.</p>
    </section>
    <div id="rm-note"></div>
    <div class="roadmap-filter">
      <m3e-segmented-button id="stage-filter" aria-label="Filter by stage">
        <m3e-button-segment value="all" checked>All</m3e-button-segment>
        ${COLS.map((s) => `<m3e-button-segment value="${s}">${icon(meta(s).icon, { slot: 'icon' })}${meta(s).label}</m3e-button-segment>`).join('')}
      </m3e-segmented-button>
    </div>
    <div class="board" id="board" aria-busy="true">${skeleton()}</div>
    <m3e-callout variant="tertiary" class="roadmap-cta">${icon('lightbulb', { slot: 'icon' })}<span slot="label">Want something on the board?</span>
      <a href="/request">Request a port</a> and we'll check its licence and queue it. Already requested? <a href="${GITHUB}/issues?q=is%3Aissue+label%3Aport-request" target="_blank" rel="noopener">Upvote it on GitHub</a>.
    </m3e-callout>`;
  },
  mount(root) {
    const boardEl = root.querySelector<HTMLElement>('#board')!;
    const note = root.querySelector<HTMLElement>('#rm-note')!;
    let stop = () => {};
    let filter = 'all';
    const applyFilter = () => {
      root.querySelectorAll<HTMLElement>('.col').forEach((col) => col.toggleAttribute('hidden', filter !== 'all' && col.dataset.stage !== filter));
      boardEl.classList.toggle('single', filter !== 'all');
    };
    const show = (cards: Card[], warn = '') => {
      boardEl.innerHTML = board(cards);
      boardEl.removeAttribute('aria-busy');
      applyFilter();
      stop();
      stop = reveal(boardEl);
      note.innerHTML = warn
        ? speaker('claude/patch', warn)
        : cards.length
          ? ''
          : `<div class="roadmap-empty">${speaker('codex/architect', 'The board is fresh. The first requests land in Requested, then move along as the crew picks them up.')}
             <m3e-button variant="filled" href="/request">${icon('add_circle', { slot: 'icon' })}Request a port</m3e-button></div>`;
    };
    let alive = true;
    portRequests()
      .then((reqs) => alive && show([...reqs.map(fromIssue), ...ROADMAP.map(fromStatic)]))
      .catch(() => alive && show(ROADMAP.map(fromStatic), 'GitHub isn’t answering right now (maybe a rate limit). Showing what we have locally.'));
    root.querySelector('#stage-filter')!.addEventListener('change', (e) => {
      const on = [...(e.currentTarget as HTMLElement).querySelectorAll<Seg>('m3e-button-segment')].find((x) => x.checked);
      filter = on?.getAttribute('value') ?? 'all';
      applyFilter();
    });
    return () => {
      alive = false;
      stop();
    };
  },
};

export default roadmap;
