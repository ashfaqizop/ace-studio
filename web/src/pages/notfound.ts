import type { Page } from '../lib/router';
import { emptyState } from '../components/cards';
import { esc, icon } from '../lib/html';
import { sprite } from '../data/crew';

const notFound: Page = {
  title: 'Chunk not found',
  render(c) {
    return `
    <div class="nf">
      <span class="nf-code serif" aria-hidden="true">4${sprite('ambient/pixel', 2, 'nf-cat')}4</span>
      ${emptyState('claude/echo', 'This chunk never generated', `Nothing lives at <code>${esc(c.path)}</code>. Echo looked everywhere, then vanished too.`,
        `<m3e-button variant="filled" href="/">${icon('home', { slot: 'icon' })}Take me home</m3e-button>
         <m3e-button variant="tonal" href="/plugins">${icon('extension', { slot: 'icon' })}Browse ports</m3e-button>`)}
    </div>`;
  },
};

export default notFound;
