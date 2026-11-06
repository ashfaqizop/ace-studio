import type { Page } from '../lib/router';
import { codeBlock, esc, icon } from '../lib/html';
import { speaker } from '../components/cards';
import { GITHUB } from '../config';
import { brand } from '../lib/brands';

const FAQ: [string, string][] = [
  ['What is a “port”?', 'A port is a rewrite of an existing Modrinth plugin (usually for Paper/Spigot/Bukkit) against the PumpkinMC plugin API. Same features, native to Pumpkin, built as a sandboxed WebAssembly component.'],
  ['Are these official plugins?', 'No. Ace Studio is an independent project. We are not affiliated with Modrinth, Mojang, the Pumpkin team or the original authors. Every port credits its original author and keeps a compatible licence.'],
  ['Which plugins can you port?', 'Plugins with an open-source licence (MIT, Apache, GPL and so on) can be ported with credit. “All rights reserved” plugins need the author’s permission first. The request form checks this for you.'],
  ['Will my old config work?', 'Usually yes. Ports migrate the original YAML config to TOML on first start where possible. Each plugin’s Port notes tab lists what changed.'],
  ['Why does Pumpkin ask about permissions?', 'Pumpkin plugins run in a capability sandbox. When a plugin needs, for example, to write to its data folder, Pumpkin asks in the console. You can pre-approve with allowed_permissions in pumpkin.toml.'],
  ['Where do I report a bug?', 'Open an issue on GitHub. The “Report a bug” link on every plugin page pre-fills the plugin name.'],
];

const docs: Page = {
  title: 'Install guide',
  render() {
    return `
    <section class="page-head">
      <span class="eyebrow">${icon('menu_book')} Install guide</span>
      <h1 class="display-md">Install a port in <em>four steps</em></h1>
      <p class="lead">Ace Studio ports are WebAssembly plugins. They drop straight into Pumpkin’s plugins folder.</p>
    </section>
    <div class="docs">
      <article id="guide" class="prose docs-body">
        ${speaker('claude/scribe', 'I wrote this guide. Follow along and you’ll be running a port in a couple of minutes.')}

        <h2 id="download">1. Download the plugin</h2>
        <p>Open a plugin from <a href="/plugins">Browse</a> and press <strong>Download</strong>. You get a single <code>.wasm</code> file. Every release is also listed on <a class="brand-link" href="${GITHUB}/releases" target="_blank" rel="noopener">${brand('github', { size: 16 })}GitHub Releases</a>.</p>

        <h2 id="drop-in">2. Drop it into plugins/</h2>
        <p>Put the file in the <code>plugins/</code> folder next to your Pumpkin server binary. Pumpkin scans this folder on start-up and loads every <code>.wasm</code> component it finds.</p>
        ${codeBlock('server/\n├── pumpkin\n├── pumpkin.toml\n└── plugins/\n    └── my-port-1.0.0.wasm', 'folder layout')}

        <h2 id="configure">3. Configure pumpkin.toml</h2>
        <p>Plugins are controlled by the <code>[plugins]</code> section. The defaults work for most servers:</p>
        ${codeBlock('[plugins]\nenabled = true\nhot_reload = false\nask_permission_confirmation = true\nverify_signatures = true\nallow_unsigned = true', 'pumpkin.toml')}
        <m3e-callout variant="secondary">${icon('verified_user', { slot: 'icon' })}<span slot="label">Permissions</span>
          The first time a plugin asks for a capability (such as <code>fs:write:data</code>), Pumpkin prompts in the console. Each plugin page has a <strong>Copy config</strong> button that gives you a ready-made <code>[plugins.overrides]</code> block to pre-approve exactly what it needs.
        </m3e-callout>

        <h2 id="start">4. Start the server</h2>
        <p>Restart Pumpkin. With <code>hot_reload = true</code>, Pumpkin watches the <code>plugins/</code> folder and reloads changed plugins without a restart.</p>
        ${codeBlock('./pumpkin', 'terminal')}

        <h2 id="faq">FAQ</h2>
        <m3e-accordion class="faq">
          ${FAQ.map(([q, a]) => `<m3e-expansion-panel><span slot="header">${esc(q)}</span><p>${esc(a)}</p></m3e-expansion-panel>`).join('')}
        </m3e-accordion>
        <div class="help-row">
          <m3e-button variant="tonal" href="https://discord.gg/aaNuD6rFEe" target="_blank" rel="noopener">${brand('discord', { slot: 'icon', size: 18, tint: true })}PumpkinMC Discord</m3e-button>
          <m3e-button variant="tonal" href="https://docs.pumpkinmc.org/config/plugins" target="_blank" rel="noopener">${icon('menu_book', { slot: 'icon' })}Pumpkin plugin docs</m3e-button>
          <m3e-button variant="tonal" href="${GITHUB}/issues" target="_blank" rel="noopener">${brand('github', { slot: 'icon', size: 18 })}Ace Studio issues</m3e-button>
        </div>
      </article>
      <m3e-toc for="guide" max-depth="2" class="toc"><span slot="overline">Guide</span></m3e-toc>
    </div>`;
  },
};

export default docs;
