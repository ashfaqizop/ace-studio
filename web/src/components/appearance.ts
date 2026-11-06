import { ACCENTS, VARIANTS } from '../config';
import { icon, esc } from '../lib/html';
import { prefs, setPrefs, type Scheme } from '../lib/prefs';
import { sprite } from '../data/crew';

type Checkable = HTMLElement & { checked?: boolean; selected?: boolean; value?: string };

export function appearanceDialog(): string {
  const p = prefs();
  const seg = (v: Scheme, label: string, ic: string) =>
    `<m3e-button-segment value="${v}"${p.scheme === v ? ' checked' : ''}>${icon(ic, { slot: 'icon' })}${label}</m3e-button-segment>`;
  return `
  <m3e-dialog id="appearance" dismissible class="appearance">
    <span slot="header" class="appearance-head">${sprite('claude/stylist', 1)} Appearance</span>
    <div class="appearance-body">
      <section>
        <h3 class="label">Colour scheme</h3>
        <m3e-segmented-button id="ap-scheme" aria-label="Colour scheme">
          ${seg('auto', 'Auto', 'brightness_auto')}${seg('light', 'Light', 'light_mode')}${seg('dark', 'Dark', 'dark_mode')}
        </m3e-segmented-button>
      </section>
      <section>
        <h3 class="label">Accent</h3>
        <div class="swatches" role="radiogroup" aria-label="Accent colour">
          ${ACCENTS.map(
            (a) => `<button type="button" class="swatch" role="radio" aria-checked="${p.accent === a.color}" data-accent="${a.color}" style="--sw:${a.color}" aria-label="${esc(a.name)}">
              <m3e-shape name="${p.accent === a.color ? 'sunny' : 'circle'}"></m3e-shape><span>${esc(a.name)}</span></button>`,
          ).join('')}
        </div>
      </section>
      <section>
        <m3e-form-field variant="outlined">
          <label slot="label" for="ap-variant">Palette style</label>
          <m3e-select id="ap-variant">
            ${VARIANTS.map((v) => `<m3e-option value="${v}"${p.variant === v ? ' selected' : ''}>${v.replace('-', ' ')}</m3e-option>`).join('')}
          </m3e-select>
        </m3e-form-field>
      </section>
      <section class="switch-row">
        <label for="ap-motion"><strong>Expressive motion</strong><span class="muted">Springy, bouncy transitions.</span></label>
        <m3e-switch id="ap-motion" icons="both"${p.expressive ? ' checked' : ''}></m3e-switch>
      </section>
      <section class="switch-row">
        <label for="ap-buddy"><strong>Pixel the cat</strong><span class="muted">The little helper in the corner.</span></label>
        <m3e-switch id="ap-buddy" icons="both"${p.buddy ? ' checked' : ''}></m3e-switch>
      </section>
    </div>
    <div slot="actions" end>
      <m3e-button variant="text" id="ap-reset">Reset</m3e-button>
      <m3e-button variant="filled"><m3e-dialog-action return-value="done">Done</m3e-dialog-action></m3e-button>
    </div>
  </m3e-dialog>`;
}

export function mountAppearance(root: HTMLElement): void {
  const dlg = root.querySelector<HTMLElement>('#appearance')!;
  dlg.querySelector('#ap-scheme')!.addEventListener('change', (e) => {
    const segs = [...(e.currentTarget as HTMLElement).querySelectorAll<Checkable>('m3e-button-segment')];
    const on = segs.find((s) => s.checked);
    if (on) setPrefs({ scheme: on.getAttribute('value') as Scheme });
  });
  dlg.querySelectorAll<HTMLButtonElement>('[data-accent]').forEach((b) =>
    b.addEventListener('click', () => {
      setPrefs({ accent: b.dataset.accent! });
      dlg.querySelectorAll<HTMLButtonElement>('[data-accent]').forEach((o) => {
        const on = o === b;
        o.setAttribute('aria-checked', String(on));
        o.querySelector('m3e-shape')!.setAttribute('name', on ? 'sunny' : 'circle');
      });
    }),
  );
  dlg.querySelector('#ap-variant')!.addEventListener('change', (e) => {
    const v = (e.currentTarget as Checkable).value;
    if (typeof v === 'string') setPrefs({ variant: v });
  });
  dlg.querySelector('#ap-motion')!.addEventListener('change', (e) => setPrefs({ expressive: !!(e.currentTarget as Checkable).checked }));
  dlg.querySelector('#ap-buddy')!.addEventListener('change', (e) => setPrefs({ buddy: !!(e.currentTarget as Checkable).checked }));
  dlg.querySelector('#ap-reset')!.addEventListener('click', () => {
    setPrefs({ scheme: 'auto', accent: ACCENTS[0].color, variant: 'vibrant', expressive: true, buddy: true });
    syncControls(dlg);
  });
  dlg.addEventListener('opening', () => {
    syncControls(dlg);
    dlg.querySelector('.appearance-head .sprite')?.classList.add('bounce-once');
  });
  dlg.addEventListener('closed', () => dlg.querySelector('.appearance-head .sprite')?.classList.remove('bounce-once'));
}

/** Reflect the current prefs into the dialog's controls. */
function syncControls(dlg: HTMLElement): void {
  const p = prefs();
  dlg.querySelectorAll<Checkable>('#ap-scheme m3e-button-segment').forEach((s) => {
    s.checked = s.getAttribute('value') === p.scheme;
  });
  dlg.querySelectorAll<HTMLButtonElement>('[data-accent]').forEach((o) => {
    const on = o.dataset.accent === p.accent;
    o.setAttribute('aria-checked', String(on));
    o.querySelector('m3e-shape')!.setAttribute('name', on ? 'sunny' : 'circle');
  });
  const sel = dlg.querySelector<Checkable>('#ap-variant');
  if (sel) sel.value = p.variant;
  const motion = dlg.querySelector<Checkable>('#ap-motion');
  if (motion) motion.checked = p.expressive;
  const buddy = dlg.querySelector<Checkable>('#ap-buddy');
  if (buddy) buddy.checked = p.buddy;
}
