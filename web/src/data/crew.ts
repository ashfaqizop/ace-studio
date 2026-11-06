import { esc } from '../lib/html';

export type CharSet = 'claude' | 'ambient' | 'codex' | 'grok';

export interface Character {
  set: CharSet;
  id: string;
  name: string;
  role: string;
  line: string;
}

export const SETS: Record<CharSet, { label: string; blurb: string }> = {
  claude: { label: 'Clawd Office', blurb: 'Clawd and the original agent crew: square, tiny legs, big jobs.' },
  ambient: { label: 'Ambient', blurb: 'The office regulars who keep the vibes up between ports.' },
  codex: { label: 'Pixel Agents', blurb: 'Terminal-faced bots who speak fluent prompt.' },
  grok: { label: 'Grok Bots', blurb: 'Round, bold and slightly chaotic. They ship anyway.' },
};

const ROLES: [string, string, string, string][] = [
  ['orchy', 'Orchy', 'Orchestrator', 'I pick the next port and keep everyone moving.'],
  ['clawd', 'Clawd', 'Main agent', 'Talk to me. I run the session.'],
  ['scout', 'Scout', 'Explorer', 'I dig through the original source to map every feature.'],
  ['architect', 'Architect', 'Planner', 'Every port starts as a plan on my blueprint.'],
  ['forge', 'Forge', 'Builder', 'Bukkit in, Pumpkin out. Hammer goes clang.'],
  ['patch', 'Patch', 'Bug fixer', 'Found a bug? I already have the bandage ready.'],
  ['inspector', 'Inspector', 'Code reviewer', 'Nothing ships without my checkmark.'],
  ['probe', 'Probe', 'Tester', 'I break it on purpose so you never do.'],
  ['shell', 'Shell', 'Terminal operator', 'cargo build --release. Again. And again.'],
  ['scribe', 'Scribe', 'Documenter', 'I write the guides so you can skip the guessing.'],
  ['tidy', 'Tidy', 'Refactorer', 'Clean code, happy servers.'],
  ['sentinel', 'Sentinel', 'Security auditor', 'Every permission a plugin asks for goes past me.'],
  ['courier', 'Courier', 'Git handler', 'Releases, tags and requests: I deliver.'],
  ['oracle', 'Oracle', 'Researcher', 'I read the Modrinth page so the team doesn\'t guess.'],
  ['pipe', 'Pipe', 'Connector', 'I plug the plugin into the Pumpkin API.'],
  ['lens', 'Lens', 'Profiler', 'If it costs a tick, I will find it.'],
  ['stylist', 'Stylist', 'Frontend designer', 'I made this website pretty. You\'re welcome.'],
  ['schema', 'Schema', 'Data specialist', 'Configs, data files, migrations: all mine.'],
  ['deploy', 'Deploy', 'Release engineer', 'When it\'s green, I launch it.'],
  ['pocket', 'Pocket', 'Memory keeper', 'I remember what every port changed.'],
  ['echo', 'Echo', 'Sub-agent', 'I was here a second ago...'],
];

const AMBIENT: [string, string, string, string][] = [
  ['pixel', 'Pixel', 'Office cat', 'Mrrp. I sit on whichever keyboard is busiest.'],
  ['brew', 'Brew', 'Barista', 'Coffee\'s on. Long compile ahead?'],
  ['tick', 'Tick', 'Alarms & reminders', 'Ding! Release window in five.'],
  ['sprout', 'Sprout', 'Gardener', 'Every saved plugin is a seed.'],
  ['spin', 'Spin', 'DJ', 'Lo-fi beats to port plugins to.'],
  ['mop', 'Mop', 'Janitor', 'Cleaning up after Forge. Again.'],
  ['stretch', 'Stretch', 'Wellness coach', 'Stand up, stretch, then ship.'],
  ['nimbus', 'Nimbus', 'Weather reporter', 'Forecast: 100% chance of new ports.'],
  ['bard', 'Bard', 'Jokes & quotes', 'Why did the creeper port its plugin? It had a blast.'],
  ['snap', 'Snap', 'Photographer', 'Say cheese for the screenshots!'],
];

export const CREW: Character[] = [
  ...ROLES.map(([id, name, role, line]) => ({ set: 'claude' as const, id, name, role, line })),
  ...AMBIENT.map(([id, name, role, line]) => ({ set: 'ambient' as const, id, name, role, line })),
  ...(['codex', 'grok'] as const).flatMap((set) =>
    ROLES.map(([id, name, role, line]) =>
      id === 'clawd'
        ? { set, id: 'cowdex', name: 'Cowdex', role: 'Main agent', line: 'Moo-ving fast. Ask me anything.' }
        : { set, id, name, role, line },
    ),
  ),
];

export type CharRef = `${CharSet}/${string}`;

export function char(ref: CharRef): Character {
  const [set, id] = ref.split('/') as [CharSet, string];
  return CREW.find((c) => c.set === set && c.id === id) ?? CREW[0];
}

/** Codex and Grok sprites are 48x64; Claude and ambient are 48x48. */
export const isTall = (set: CharSet) => set === 'codex' || set === 'grok';

/** Frames per animation sheet (see scripts/animate.py). */
export const SPRITE_FRAMES = 16;

/**
 * Animated pixel sprite: a 16-frame sheet stepped with CSS. `scale` multiplies the
 * native 48px width. Each character starts at a different point in its loop.
 */
export function sprite(ref: CharRef, scale = 2, cls = ''): string {
  const c = char(ref);
  const h = isTall(c.set) ? 64 : 48;
  let hash = 0;
  for (const ch of ref) hash = (hash * 31 + ch.charCodeAt(0)) >>> 0;
  const delay = -((hash % SPRITE_FRAMES) * 100);
  return `<i class="sprite ${esc(cls)}" role="img" aria-label="${esc(`${c.name}, ${c.role}`)}" style="--sw:${48 * scale}px;--ar:48/${h};--sheet:url('/characters/anim/${c.set}/${c.id}.png');--still:url('/characters/${c.set}/${c.id}.png');animation-delay:${delay}ms"></i>`;
}
