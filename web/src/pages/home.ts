import type { Page } from '../lib/router';
import { PLUGINS, CATEGORIES, TOTAL_DOWNLOADS, updatedAt, latest } from '../data/plugins';
import { ROADMAP } from '../data/roadmap';
import { sprite, char, type CharRef } from '../data/crew';
import { pluginCard, pluginIcon } from '../components/cards';
import { ago, esc, icon } from '../lib/html';
import { countUp, reveal } from '../lib/ui';
import { prefs } from '../lib/prefs';
import { pluginBySlug } from '../data/plugins';

const STAGE_CREW: CharRef[] = ['grok/scout', 'claude/orchy', 'codex/forge', 'ambient/pixel', 'grok/probe', 'codex/deploy'];

const LINES = [
  'Welcome to Ace Studio! We port Modrinth favourites to Pumpkin.',
  'Every port credits the original author. Always.',
  'Psst. Press / to search.',
  'New ports land on the roadmap first.',
];

const PIPELINE: { ref: CharRef; title: string; body: string; icon: string }[] = [
  { ref: 'claude/oracle', icon: 'travel_explore', title: 'Pick', body: 'A Modrinth plugin is requested, its licence is checked and the original author is credited.' },
  { ref: 'codex/forge', icon: 'translate', title: 'Translate', body: 'The crew rewrites it against the Pumpkin plugin API, feature by feature.' },
  { ref: 'grok/probe', icon: 'science', title: 'Test', body: 'It runs on test servers, gets profiled for tick cost and reviewed.' },
  { ref: 'claude/deploy', icon: 'rocket_launch', title: 'Ship', body: 'A signed .wasm build is released on GitHub and listed here.' },
];

const SHAPES = ['sunny', 'flower', '8-leaf-clover', 'soft-burst', 'puffy-diamond', 'pixel-circle', 'gem', 'boom'];

const home: Page = {
  title: 'Modrinth plugins, ported to PumpkinMC',
  render() {
    const featured = PLUGINS.filter((p) => p.featured);
    const recent = [...PLUGINS].sort((a, b) => updatedAt(b).localeCompare(updatedAt(a))).slice(0, 5);
    const inProgress = ROADMAP.filter((r) => r.stage === 'translating' || r.stage === 'testing').length;
    const viewed = prefs().recent.map(pluginBySlug).filter(Boolean).slice(0, 6);

    return `
    <section class="hero">
      <div class="hero-shapes" aria-hidden="true">
        <m3e-shape class="hs hs1" name="sunny"></m3e-shape>
        <m3e-shape class="hs hs2" name="flower"></m3e-shape>
        <m3e-shape class="hs hs3" name="pixel-circle"></m3e-shape>
        <m3e-shape class="hs hs4" name="soft-burst"></m3e-shape>
      </div>
      <div class="hero-copy">
        <span class="eyebrow">${icon('swap_horiz')} Modrinth → PumpkinMC</span>
        <h1 class="display">Your favourite plugins, <em>reborn</em> for Pumpkin.</h1>
        <p class="lead">Ace Studio translates great Modrinth plugins into native, sandboxed PumpkinMC plugins, one careful port at a time, always crediting the original authors.</p>
        <div class="hero-actions">
          <m3e-button variant="filled" size="large" shape="square" href="/plugins">${icon('extension', { slot: 'icon' })}Browse ports</m3e-button>
          <m3e-button variant="tonal" size="large" href="/request">${icon('add_circle', { slot: 'icon' })}Request a port</m3e-button>
        </div>
        <m3e-chip-set class="hero-cats" aria-label="Popular categories">
          ${Object.entries(CATEGORIES).slice(0, 6).map(([k, c]) => `<m3e-suggestion-chip href="/plugins?cat=${k}">${icon(c.icon, { slot: 'icon' })}${esc(c.label)}</m3e-suggestion-chip>`).join('')}
        </m3e-chip-set>
      </div>
      <div class="stage" aria-label="The Ace Studio crew">
        <div class="stage-bubble bubble" id="stage-line" aria-live="polite"><strong>${esc(char('claude/orchy').name)}</strong>${esc(LINES[0])}</div>
        <div class="stage-floor">
          ${STAGE_CREW.map((r, i) => `<button type="button" class="stage-actor" style="--i:${i}" data-line="${esc(char(r).line)}" data-name="${esc(char(r).name)}" aria-label="${esc(char(r).name)}">${sprite(r, 2)}</button>`).join('')}
        </div>
      </div>
    </section>

    ${PLUGINS.length ? `<section class="stats" aria-label="Ace Studio in numbers">
      <m3e-card variant="filled" class="stat" data-reveal><span class="stat-n" data-count="${PLUGINS.length}">0</span><span class="stat-l">ports shipped</span>${icon('inventory_2', { cls: 'stat-i' })}</m3e-card>
      <m3e-card variant="filled" class="stat" data-reveal><span class="stat-n" data-count="${inProgress}">0</span><span class="stat-l">in the workshop</span>${icon('construction', { cls: 'stat-i' })}</m3e-card>
      <m3e-card variant="filled" class="stat" data-reveal><span class="stat-n" data-count="${TOTAL_DOWNLOADS}">0</span><span class="stat-l">downloads</span>${icon('download', { cls: 'stat-i' })}</m3e-card>
      <m3e-card variant="filled" class="stat" data-reveal><span class="stat-n" data-count="${ROADMAP.reduce((n, r) => n + r.votes, 0)}">0</span><span class="stat-l">community votes</span>${icon('how_to_vote', { cls: 'stat-i' })}</m3e-card>
    </section>` : ''}

    ${viewed.length ? `
    <section class="band">
      <div class="band-head"><h2 class="headline">Continue browsing</h2></div>
      <div class="chips-row">
        ${viewed.map((p) => `<a class="mini" href="/plugin/${esc(p!.slug)}">${pluginIcon(p!, 32)}<span>${esc(p!.name)}</span></a>`).join('')}
      </div>
    </section>` : ''}

    ${featured.length ? `    <section class="band">
      <div class="band-head">
        <div><span class="eyebrow">${icon('star')} Featured</span><h2 class="headline">Fresh out of the forge</h2></div>
        <m3e-button variant="text" href="/plugins">See all ${icon('arrow_forward', { slot: 'trailing-icon' })}</m3e-button>
      </div>
      <m3e-slide-group class="featured" aria-label="Featured ports">
        ${featured.map((p) => `<div class="featured-item">${pluginCard(p)}</div>`).join('')}
      </m3e-slide-group>
    </section>` : `
    <section class="band launch" data-reveal>
      <div class="launch-art" aria-hidden="true">
        <m3e-shape name="very-sunny" class="launch-shape"></m3e-shape>
        ${sprite('claude/forge', 3)}${sprite('grok/probe', 2)}${sprite('codex/deploy', 2)}
      </div>
      <div class="launch-copy">
        <span class="eyebrow">${icon('construction')} In the workshop</span>
        <h2 class="display-sm">The first ports are being <em>forged</em>.</h2>
        <p class="lead">No plugins have shipped yet. Follow the roadmap, or tell us which Modrinth plugin you want first and it goes straight into the queue.</p>
        <div class="hero-actions">
          <m3e-button variant="filled" href="/request">${icon('add_circle', { slot: 'icon' })}Request the first port</m3e-button>
          <m3e-button variant="tonal" href="/roadmap">${icon('view_kanban', { slot: 'icon' })}Watch the roadmap</m3e-button>
        </div>
      </div>
    </section>`}

    <section class="band pipeline-band">
      <div class="band-head"><div><span class="eyebrow">${icon('route')} How it works</span><h2 class="headline">How a port ships</h2></div>
        <m3e-button variant="text" href="/roadmap">Open the roadmap ${icon('arrow_forward', { slot: 'trailing-icon' })}</m3e-button></div>
      <ol class="pipeline">
        ${PIPELINE.map((s, i) => `
        <li class="pipe-step" data-reveal>
          <m3e-card variant="outlined">
            <div slot="content" class="pipe-body">
              <div class="pipe-art"><m3e-shape name="${SHAPES[i * 2]}" class="pipe-shape"></m3e-shape>${sprite(s.ref, 2)}</div>
              <span class="pipe-n">0${i + 1}</span>
              <h3>${icon(s.icon)} ${esc(s.title)}</h3>
              <p class="muted">${esc(s.body)}</p>
            </div>
          </m3e-card>
        </li>`).join('')}
      </ol>
    </section>

    <section class="band ${recent.length ? 'split' : 'solo'}">
      ${recent.length ? `      <div>
        <div class="band-head"><div><span class="eyebrow">${icon('schedule')} Updated</span><h2 class="headline">Recently updated</h2></div></div>
        <m3e-list class="recent-list" variant="segmented">
          ${recent.map((p) => `
          <m3e-list-item>
            <a slot="leading" href="/plugin/${esc(p.slug)}" tabindex="-1" aria-hidden="true">${pluginIcon(p, 40)}</a>
            <a class="li-link" href="/plugin/${esc(p.slug)}">${esc(p.name)}</a>
            <span slot="supporting-text">v${esc(latest(p).version)} · ${ago(updatedAt(p))}</span>
            <span slot="trailing" class="tag">${esc(p.license)}</span>
          </m3e-list-item>`).join('')}
        </m3e-list>
      </div>` : ''}
      <m3e-card variant="elevated" class="cta-card">
        <div slot="content" class="cta-body">
          <div class="cta-art"><m3e-shape name="very-sunny" class="cta-shape"></m3e-shape>${sprite('codex/courier', 3)}</div>
          <h2 class="display-sm">Miss a plugin from Modrinth?</h2>
          <p>Tell us which one. We check its licence on the spot and put it on the roadmap.</p>
          <m3e-button variant="filled" size="medium" href="/request">${icon('send', { slot: 'icon' })}Request a port</m3e-button>
        </div>
      </m3e-card>
    </section>`;
  },
  mount(root) {
    const stops = [countUp(root), reveal(root)];
    // morph the hero shapes through M3E's built-in shape morphing
    const hs = [...root.querySelectorAll<HTMLElement>('.hs')];
    let k = 0;
    const morph = setInterval(() => {
      k++;
      hs.forEach((s, i) => s.setAttribute('name', SHAPES[(k + i * 2) % SHAPES.length]));
    }, 2600);
    // crew speech bubble: rotate lines, or say a character's own line on click
    const bubble = root.querySelector<HTMLElement>('#stage-line')!;
    let line = 0;
    const say = (name: string, text: string) => {
      bubble.classList.remove('pop');
      void bubble.offsetWidth;
      bubble.innerHTML = `<strong>${esc(name)}</strong>${esc(text)}`;
      bubble.classList.add('pop');
    };
    const rotate = setInterval(() => say('Orchy', LINES[++line % LINES.length]), 5200);
    root.querySelectorAll<HTMLElement>('.stage-actor').forEach((a) =>
      a.addEventListener('click', () => {
        say(a.dataset.name!, a.dataset.line!);
        a.classList.remove('jump');
        void a.offsetWidth;
        a.classList.add('jump');
      }),
    );
    return () => {
      stops.forEach((s) => s());
      clearInterval(morph);
      clearInterval(rotate);
    };
  },
};

export default home;
