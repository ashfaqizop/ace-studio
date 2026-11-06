import { sprite } from '../data/crew';
import { prefs, setPrefs, onPrefs } from '../lib/prefs';
import { esc } from '../lib/html';

/**
 * Pixel the office cat sits in the corner and reacts to what you do: copying,
 * saving, landing on an empty page. Click for a tip. Can be turned off in Appearance.
 */
const TIPS = [
  'Press <kbd>/</kbd> anywhere to search.',
  'Save plugins with the ♥ and find them under Saved.',
  'Every port credits its original Modrinth author.',
  'Missing a plugin? Request a port and we\'ll check its licence.',
  'Try a different accent colour in Appearance.',
  'The Roadmap shows what the crew is porting right now.',
];

let el: HTMLElement | null = null;
let timer = 0;
let tip = 0;

export function mountBuddy(): void {
  el = document.createElement('div');
  el.className = 'buddy';
  el.innerHTML = `
    <div class="buddy-bubble" role="status" aria-live="polite"></div>
    <button class="buddy-body" type="button" aria-label="Pixel the office cat: show a tip">
      ${sprite('ambient/pixel', 1, 'buddy-sprite')}
    </button>
    <button class="buddy-hide" type="button" aria-label="Hide Pixel">×</button>`;
  document.body.append(el);
  el.querySelector('.buddy-body')!.addEventListener('click', () => {
    buddySay(TIPS[tip++ % TIPS.length], 'talk', true);
  });
  el.querySelector('.buddy-hide')!.addEventListener('click', () => setPrefs({ buddy: false }));
  const sync = () => el!.toggleAttribute('hidden', !prefs().buddy);
  onPrefs(sync);
  sync();
  if (!prefs().greeted) {
    setTimeout(() => buddySay('Mrrp! I\'m Pixel. Click me for tips.', 'happy', true), 1400);
    setPrefs({ greeted: true });
  }
}

export function buddySay(text: string, mood: 'happy' | 'talk' | 'sad' = 'talk', html = false): void {
  if (!el || !prefs().buddy) return;
  const bubble = el.querySelector<HTMLElement>('.buddy-bubble')!;
  bubble.innerHTML = html ? text : esc(text);
  el.dataset.mood = mood;
  el.classList.remove('talking');
  void el.offsetWidth; // restart the hop animation
  el.classList.add('talking');
  clearTimeout(timer);
  timer = window.setTimeout(() => el?.classList.remove('talking'), 3600);
}
