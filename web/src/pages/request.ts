import type { Page } from '../lib/router';
import { GITHUB } from '../config';
import { esc, icon, num } from '../lib/html';
import { speaker } from '../components/cards';
import { ROADMAP } from '../data/roadmap';
import { PLUGINS } from '../data/plugins';
import { buddySay } from '../components/buddy';

type El = HTMLElement & { checked?: boolean };

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

/** Licences that allow a port with attribution. */
const OPEN = /^(MIT|Apache-2\.0|BSD-[23]-Clause|ISC|MPL-2\.0|LGPL-[23]\.[01](-only|-or-later)?|GPL-[23]\.0(-only|-or-later)?|AGPL-3\.0(-only|-or-later)?|CC0-1\.0|Unlicense|Zlib|0BSD|EUPL-1\.2)$/i;

/** Accepts a Modrinth URL, "modrinth.com/plugin/x" or a bare slug/ID. */
export function parseModrinth(input: string): string | null {
  const s = input.trim();
  if (!s) return null;
  const m = s.match(/modrinth\.com\/(?:plugin|mod|datapack|project)\/([\w.-]+)/i);
  if (m) return m[1];
  return /^[\w.-]{2,64}$/.test(s) ? s : null;
}

function licenceVerdict(p: ModrinthProject): { variant: string; icon: string; title: string; body: string } {
  const id = p.license.id;
  if (OPEN.test(id))
    return { variant: 'primary', icon: 'verified', title: `${id}: portable with credit`, body: 'Open-source licence. We can port it as long as we credit the author and keep the licence.' };
  if (/^LicenseRef-All-Rights-Reserved$/i.test(id) || /all rights reserved/i.test(p.license.name))
    return { variant: 'error', icon: 'block', title: 'All rights reserved', body: 'We can only port this with the author’s written permission. Mention it in your request if you have it.' };
  return { variant: 'tertiary', icon: 'help', title: `Custom licence (${id || 'unknown'})`, body: 'We need to read the licence terms before porting. We’ll check and reply on the request.' };
}

function preview(p: ModrinthProject): string {
  const v = licenceVerdict(p);
  const known = ROADMAP.find((r) => r.name.toLowerCase() === p.title.toLowerCase()) || PLUGINS.find((x) => x.original.name.toLowerCase() === p.title.toLowerCase());
  return `
  <m3e-card variant="outlined" class="mr-card">
    <div slot="content" class="mr-body">
      <div class="mr-top">
        ${p.icon_url ? `<img class="mr-icon" src="${esc(p.icon_url)}" alt="" width="64" height="64">` : `<m3e-shape name="pixel-circle" class="mr-icon">${icon('extension')}</m3e-shape>`}
        <div><h3>${esc(p.title)}</h3><p class="muted small">${esc(p.project_type)} · ${num(p.downloads)} downloads · ${num(p.followers)} followers</p></div>
      </div>
      <p>${esc(p.description)}</p>
      <m3e-chip-set>${p.loaders.slice(0, 6).map((l) => `<m3e-chip>${esc(l)}</m3e-chip>`).join('')}</m3e-chip-set>
      <m3e-callout variant="${v.variant}">${icon(v.icon, { slot: 'icon' })}<span slot="label">${esc(v.title)}</span>${esc(v.body)}</m3e-callout>
      ${known ? `<m3e-callout variant="secondary">${icon('info', { slot: 'icon' })}<span slot="label">Already on our radar</span>This one is already in the catalog or on the roadmap. Requesting adds your vote.</m3e-callout>` : ''}
      <a class="small" href="https://modrinth.com/${esc(p.project_type)}/${esc(p.slug)}" target="_blank" rel="noopener">${icon('open_in_new')}Open on Modrinth</a>
    </div>
  </m3e-card>`;
}

const request: Page = {
  title: 'Request a port',
  render() {
    return `
    <section class="page-head">
      <span class="eyebrow">${icon('add_circle')} Request</span>
      <h1 class="display-md">Request a <em>port</em></h1>
      <p class="lead">Paste a Modrinth link. We look it up live, check whether its licence allows a port, and open a ready-to-send request on GitHub.</p>
    </section>
    <div class="request">
      <form class="req-form" id="req-form" novalidate>
        <m3e-form-field variant="outlined" id="mr-field">
          <label slot="label" for="mr">Modrinth plugin link or slug</label>
          ${icon('link', { slot: 'prefix' })}
          <input id="mr" name="mr" required placeholder="https://modrinth.com/plugin/…" autocomplete="off" />
          <span slot="hint">e.g. modrinth.com/plugin/your-plugin or just the slug</span>
          <span slot="error">That doesn’t look like a Modrinth project link.</span>
        </m3e-form-field>
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
          <m3e-button variant="filled" size="medium" type="submit" id="send">${icon('send', { slot: 'icon' })}Open request on GitHub</m3e-button>
          <span class="muted small">You’ll review it on GitHub before posting. Needs a GitHub account.</span>
        </div>
      </form>
      <div class="req-side">
        <div id="lookup" aria-live="polite">
          ${speaker('claude/oracle', 'Paste a link and I’ll read the Modrinth page for you.')}
        </div>
      </div>
    </div>`;
  },
  mount(root) {
    const input = root.querySelector<HTMLInputElement>('#mr')!;
    const field = root.querySelector<HTMLElement>('#mr-field')!;
    const out = root.querySelector<HTMLElement>('#lookup')!;
    let project: ModrinthProject | null = null;
    let ctrl: AbortController | null = null;
    let t = 0;

    const lookup = async (slug: string) => {
      ctrl?.abort();
      ctrl = new AbortController();
      out.innerHTML = `<m3e-skeleton animation="wave" class="mr-skel"><m3e-card variant="outlined"><div slot="content" style="height:260px"></div></m3e-card></m3e-skeleton>
        <div class="lookup-status"><m3e-loading-indicator variant="contained"></m3e-loading-indicator><span class="muted">Oracle is reading Modrinth…</span></div>`;
      try {
        const res = await fetch(`https://api.modrinth.com/v2/project/${encodeURIComponent(slug)}`, { signal: ctrl.signal });
        if (res.status === 404) {
          project = null;
          out.innerHTML = speaker('grok/scout', `I couldn’t find “${slug}” on Modrinth. Check the link?`);
          return;
        }
        if (!res.ok) throw new Error(String(res.status));
        project = (await res.json()) as ModrinthProject;
        out.innerHTML = preview(project);
        buddySay(`Found ${project.title}!`, 'happy');
      } catch (err) {
        if ((err as Error).name === 'AbortError') return;
        project = null;
        out.innerHTML = speaker('claude/patch', 'Modrinth didn’t answer. You can still send the request; we’ll look it up by hand.');
      }
    };

    input.addEventListener('input', () => {
      clearTimeout(t);
      const slug = parseModrinth(input.value);
      field.toggleAttribute('error', !!input.value.trim() && !slug);
      if (!slug) return;
      t = window.setTimeout(() => lookup(slug), 450);
    });

    root.querySelector<HTMLFormElement>('#req-form')!.addEventListener('submit', (e) => {
      e.preventDefault();
      const slug = parseModrinth(input.value);
      if (!slug) {
        field.setAttribute('error', '');
        input.focus();
        return;
      }
      const why = root.querySelector<HTMLTextAreaElement>('#why')!.value.trim();
      const prio = [...root.querySelectorAll<El>('#prio m3e-button-segment')].find((s) => s.checked)?.getAttribute('value') ?? 'Nice to have';
      const name = project?.title ?? slug;
      const body = [
        `### Plugin`,
        `${name}: https://modrinth.com/${project?.project_type ?? 'plugin'}/${project?.slug ?? slug}`,
        project ? `Licence: \`${project.license.id || 'unknown'}\` · ${project.downloads.toLocaleString()} downloads · loaders: ${project.loaders.join(', ')}` : '',
        ``,
        `### Why port it`,
        why || '_No reason given._',
        ``,
        `### Priority`,
        prio,
        ``,
        `_Sent from the Ace Studio request form._`,
      ].filter((l) => l !== undefined).join('\n');
      const url = `${GITHUB}/issues/new?${new URLSearchParams({ title: `Port request: ${name}`, body, labels: 'port-request' })}`;
      window.open(url, '_blank', 'noopener');
      buddySay('Request drafted! Courier is on it.', 'happy');
    });
    return () => ctrl?.abort();
  },
};

export default request;
