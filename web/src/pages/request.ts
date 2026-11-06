import type { Page } from '../lib/router';
import { GITHUB } from '../config';
import { esc, icon, num } from '../lib/html';
import { brand } from '../lib/brands';
import { speaker } from '../components/cards';
import { ROADMAP } from '../data/roadmap';
import { PLUGINS } from '../data/plugins';
import { buddySay } from '../components/buddy';

type El = HTMLElement & { checked?: boolean; value?: string | null };

const API = 'https://api.modrinth.com/v2';

interface ModrinthProject {
  slug: string;
  title: string;
  description: string;
  icon_url: string | null;
  downloads: number;
  followers: number;
  project_type: string;
  loaders: string[];
  categories: string[];
  license: { id: string; name: string; url: string | null };
  source_url: string | null;
}

interface SearchHit {
  slug: string;
  title: string;
  author: string;
  description: string;
  icon_url: string | null;
  downloads: number;
  follows: number;
  license: string;
}

/** Licences that allow a port with attribution. */
const OPEN = /^(MIT|Apache-2\.0|BSD-[23]-Clause|ISC|MPL-2\.0|LGPL-[23]\.[01](-only|-or-later)?|GPL-[23]\.0(-only|-or-later)?|AGPL-3\.0(-only|-or-later)?|CC0-1\.0|Unlicense|Zlib|0BSD|EUPL-1\.2)$/i;

/** A Modrinth project URL, or a bare slug/ID with no spaces. */
export function parseModrinth(input: string): string | null {
  const s = input.trim();
  if (!s) return null;
  const m = s.match(/modrinth\.com\/(?:plugin|mod|datapack|project)\/([\w.-]+)/i);
  if (m) return m[1];
  return /^[\w.-]{2,64}$/.test(s) ? s : null;
}

const isLink = (s: string) => /modrinth\.com\//i.test(s);

function licenceVerdict(id: string, name = ''): { variant: string; icon: string; title: string; body: string } {
  if (OPEN.test(id))
    return { variant: 'primary', icon: 'verified', title: `${id}: portable with credit`, body: 'Open-source licence. We can port it as long as we credit the author and keep the licence.' };
  if (/^LicenseRef-All-Rights-Reserved$/i.test(id) || /all rights reserved/i.test(name))
    return { variant: 'error', icon: 'block', title: 'All rights reserved', body: 'We can only port this with the author’s written permission. Mention it in your request if you have it.' };
  return { variant: 'tertiary', icon: 'help', title: `Custom licence (${id || 'unknown'})`, body: 'We need to read the licence terms before porting. We’ll check and reply on the request.' };
}

function preview(p: ModrinthProject, author?: string): string {
  const v = licenceVerdict(p.license.id, p.license.name);
  const t = p.title.toLowerCase();
  const known = ROADMAP.some((r) => r.name.toLowerCase() === t) || PLUGINS.some((x) => x.original.name.toLowerCase() === t);
  const url = `https://modrinth.com/plugin/${p.slug}`;
  return `
  <m3e-card variant="outlined" class="mr-card">
    <div slot="content" class="mr-body">
      <div class="mr-top">
        ${p.icon_url ? `<img class="mr-icon" src="${esc(p.icon_url)}" alt="" width="64" height="64">` : `<m3e-shape name="pixel-circle" class="mr-icon">${icon('extension')}</m3e-shape>`}
        <div>
          <h3>${esc(p.title)}</h3>
          <p class="muted small">${author ? `by ${esc(author)} · ` : ''}${num(p.downloads)} downloads · ${num(p.followers)} followers</p>
        </div>
        <span class="mr-badge" title="From Modrinth">${brand('modrinth', { size: 22, tint: true })}</span>
      </div>
      <p>${esc(p.description)}</p>
      <m3e-chip-set>${p.loaders.slice(0, 6).map((l) => `<m3e-chip>${esc(l)}</m3e-chip>`).join('')}</m3e-chip-set>
      <m3e-callout variant="${v.variant}">${icon(v.icon, { slot: 'icon' })}<span slot="label">${esc(v.title)}</span>${esc(v.body)}</m3e-callout>
      ${known ? `<m3e-callout variant="secondary">${icon('info', { slot: 'icon' })}<span slot="label">Already on our radar</span>This one is already in the catalog or on the roadmap. Requesting adds your vote.</m3e-callout>` : ''}
      <div class="mr-links">
        <m3e-button variant="text" href="${esc(url)}" target="_blank" rel="noopener">${brand('modrinth', { slot: 'icon', size: 18 })}Open on Modrinth</m3e-button>
        ${p.source_url ? `<m3e-button variant="text" href="${esc(p.source_url)}" target="_blank" rel="noopener">${brand('github', { slot: 'icon', size: 18 })}Source code</m3e-button>` : ''}
      </div>
    </div>
  </m3e-card>`;
}

function option(h: SearchHit): string {
  const lic = licenceVerdict(h.license);
  return `<m3e-option value="${esc(h.slug)}">
    <span class="mr-opt">
      ${h.icon_url ? `<img src="${esc(h.icon_url)}" alt="" width="36" height="36" loading="lazy">` : `<span class="mr-opt-ph">${icon('extension')}</span>`}
      <span class="mr-opt-main"><span class="mr-opt-title">${esc(h.title)}</span><span class="mr-opt-sub">by ${esc(h.author)} · ${num(h.downloads)} downloads</span></span>
      <span class="mr-opt-lic lic-${lic.variant}">${icon(lic.icon)}${esc(h.license || '?')}</span>
    </span>
  </m3e-option>`;
}

const request: Page = {
  title: 'Request a port',
  render() {
    return `
    <section class="page-head">
      <span class="eyebrow">${brand('modrinth', { size: 16 })} Request</span>
      <h1 class="display-md">Request a <em>port</em></h1>
      <p class="lead">Start typing a Modrinth plugin’s name. Pick it from the live results and we’ll check whether its licence allows a port, then open a ready-to-send request on GitHub.</p>
    </section>
    <div class="request">
      <form class="req-form" id="req-form" novalidate>
        <m3e-form-field variant="outlined" id="mr-field" class="mr-field">
          <label slot="label" for="mr">Search Modrinth plugins or paste a link</label>
          <span slot="prefix" class="mr-prefix">${brand('modrinth', { size: 20, tint: true })}</span>
          <input id="mr" name="mr" required placeholder="e.g. EssentialsX, LuckPerms…" autocomplete="off" spellcheck="false" />
          <span slot="hint">Results come live from Modrinth. Links like modrinth.com/plugin/… work too.</span>
          <span slot="error">Pick a plugin from the list or paste its Modrinth link.</span>
        </m3e-form-field>
        <m3e-autocomplete for="mr" id="mr-ac" filter="none" auto-activate hide-selection-indicator
          loading-label="Searching Modrinth…" no-data-label="No Modrinth plugins match that"
          panel-class="mr-panel" results-label="Modrinth results updated"></m3e-autocomplete>
        <m3e-form-field variant="outlined">
          <label slot="label" for="why">Why should we port it?</label>
          <textarea id="why" name="why" rows="4" placeholder="What does your server use it for?"></textarea>
          <m3e-textarea-autosize for="why"></m3e-textarea-autosize>
        </m3e-form-field>
        <div class="field-group">
          <span class="label">How badly do you need it?</span>
          <m3e-segmented-button id="prio" aria-label="Priority">
            <m3e-button-segment value="Nice to have" checked>${icon('sentiment_satisfied', { slot: 'icon' })}Nice to have</m3e-button-segment>
            <m3e-button-segment value="Would use it">${icon('thumb_up', { slot: 'icon' })}Would use it</m3e-button-segment>
            <m3e-button-segment value="Blocking my server">${icon('priority_high', { slot: 'icon' })}Blocking</m3e-button-segment>
          </m3e-segmented-button>
        </div>
        <div class="req-actions">
          <m3e-button variant="filled" size="medium" type="submit" id="send">${brand('github', { slot: 'icon', size: 20 })}Open request on GitHub</m3e-button>
          <span class="muted small">You’ll review it on GitHub before posting. Needs a GitHub account.</span>
        </div>
      </form>
      <div class="req-side">
        <div id="lookup" aria-live="polite">
          ${speaker('claude/oracle', 'Type a plugin name and I’ll search Modrinth for you.')}
        </div>
        <p class="muted small powered">${brand('modrinth', { size: 14 })} Search powered by the public Modrinth API.</p>
      </div>
    </div>`;
  },
  mount(root) {
    const input = root.querySelector<HTMLInputElement>('#mr')!;
    const field = root.querySelector<HTMLElement>('#mr-field')!;
    const ac = root.querySelector<El>('#mr-ac')!;
    const out = root.querySelector<HTMLElement>('#lookup')!;
    const hits = new Map<string, SearchHit>();
    let project: ModrinthProject | null = null;
    let selected: string | null = null;
    let searchCtrl: AbortController | null = null;
    let lookupCtrl: AbortController | null = null;
    let st = 0;

    const lookup = async (slug: string) => {
      lookupCtrl?.abort();
      lookupCtrl = new AbortController();
      out.innerHTML = `<m3e-skeleton animation="wave" class="mr-skel"><m3e-card variant="outlined"><div slot="content" style="height:260px"></div></m3e-card></m3e-skeleton>
        <div class="lookup-status"><m3e-loading-indicator variant="contained"></m3e-loading-indicator><span class="muted">Oracle is reading Modrinth…</span></div>`;
      try {
        const res = await fetch(`${API}/project/${encodeURIComponent(slug)}`, { signal: lookupCtrl.signal });
        if (res.status === 404) {
          project = null;
          out.innerHTML = speaker('grok/scout', `I couldn’t find “${slug}” on Modrinth. Check the name or link?`);
          return;
        }
        if (!res.ok) throw new Error(String(res.status));
        project = (await res.json()) as ModrinthProject;
        selected = project.slug;
        out.innerHTML = preview(project, hits.get(project.slug)?.author);
        field.removeAttribute('error');
        buddySay(`Found ${project.title}!`, 'happy');
      } catch (err) {
        if ((err as Error).name === 'AbortError') return;
        project = null;
        out.innerHTML = speaker('claude/patch', 'Modrinth didn’t answer. You can still send the request; we’ll look it up by hand.');
      }
    };

    const search = async (term: string) => {
      searchCtrl?.abort();
      searchCtrl = new AbortController();
      ac.toggleAttribute('loading', true);
      try {
        const q = new URLSearchParams({ query: term, limit: '8', index: 'relevance', facets: JSON.stringify([['project_type:plugin']]) });
        const res = await fetch(`${API}/search?${q}`, { signal: searchCtrl.signal });
        if (!res.ok) throw new Error(String(res.status));
        const data = (await res.json()) as { hits: SearchHit[] };
        data.hits.forEach((h) => hits.set(h.slug, h));
        ac.innerHTML = data.hits.map(option).join('');
      } catch (err) {
        if ((err as Error).name !== 'AbortError') ac.innerHTML = '';
      } finally {
        ac.toggleAttribute('loading', false);
      }
    };

    const onTerm = (raw: string) => {
      const term = raw.trim();
      clearTimeout(st);
      if (selected && hits.get(selected)?.title === term) return; // the input now shows the picked title
      selected = null;
      field.removeAttribute('error');
      if (isLink(term)) {
        ac.innerHTML = '';
        const slug = parseModrinth(term);
        if (slug) st = window.setTimeout(() => lookup(slug), 300);
        else field.setAttribute('error', '');
        return;
      }
      if (term.length < 2) {
        ac.innerHTML = '';
        return;
      }
      st = window.setTimeout(() => search(term), 220);
    };

    ac.addEventListener('query', (e) => onTerm((e as CustomEvent<{ term?: string }>).detail?.term ?? input.value));
    input.addEventListener('input', () => onTerm(input.value));
    ac.addEventListener('change', () => {
      const slug = ac.value;
      if (!slug) return;
      selected = slug;
      const hit = hits.get(slug);
      // show the clean title in the box instead of the option's full text
      requestAnimationFrame(() => {
        if (hit) input.value = hit.title;
      });
      lookup(slug);
    });

    root.querySelector<HTMLFormElement>('#req-form')!.addEventListener('submit', (e) => {
      e.preventDefault();
      const slug = selected ?? parseModrinth(input.value);
      if (!slug) {
        field.setAttribute('error', '');
        input.focus();
        return;
      }
      const why = root.querySelector<HTMLTextAreaElement>('#why')!.value.trim();
      const prio = [...root.querySelectorAll<El>('#prio m3e-button-segment')].find((s) => s.checked)?.getAttribute('value') ?? 'Nice to have';
      const name = project?.title ?? hits.get(slug)?.title ?? slug;
      const author = hits.get(slug)?.author;
      const body = [
        '### Plugin',
        `${name}: https://modrinth.com/plugin/${project?.slug ?? slug}`,
        project ? `Licence: \`${project.license.id || 'unknown'}\` · ${project.downloads.toLocaleString()} downloads · loaders: ${project.loaders.join(', ')}` : '',
        author ? `Author: ${author}` : '',
        '',
        '### Why port it',
        why || '_No reason given._',
        '',
        '### Priority',
        prio,
        '',
        '_Sent from the Ace Studio request form._',
      ].join('\n');
      const url = `${GITHUB}/issues/new?${new URLSearchParams({ title: `Port request: ${name}`, body, labels: 'port-request' })}`;
      window.open(url, '_blank', 'noopener');
      buddySay('Request drafted! Courier is on it.', 'happy');
    });

    return () => {
      searchCtrl?.abort();
      lookupCtrl?.abort();
      clearTimeout(st);
    };
  },
};

export default request;
