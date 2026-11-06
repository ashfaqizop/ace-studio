/** Site-wide configuration. */
export const SITE = {
  name: 'Ace Studio',
  tagline: 'Modrinth plugins, ported to PumpkinMC',
  /** GitHub repo that hosts the site, issues (port requests) and release assets (plugin downloads). */
  repo: 'ashfaqizop/ace-studio',
};

export const GITHUB = `https://github.com/${SITE.repo}`;
export const RELEASES = `${GITHUB}/releases`;

/** Release asset URL for a plugin build. Tags follow `<slug>-v<version>`. */
export function assetUrl(slug: string, version: string, file: string): string {
  return `${RELEASES}/download/${slug}-v${version}/${file}`;
}

export const ACCENTS = [
  { name: 'Pumpkin', color: '#ff6b2c' },
  { name: 'Creeper', color: '#3bb143' },
  { name: 'Diamond', color: '#2ec4d6' },
  { name: 'Amethyst', color: '#9a5cf0' },
  { name: 'Redstone', color: '#e5352b' },
  { name: 'Gold', color: '#f2b705' },
] as const;

export const VARIANTS = ['vibrant', 'tonal-spot', 'expressive', 'fidelity', 'content', 'rainbow', 'fruit-salad', 'neutral', 'monochrome'] as const;
