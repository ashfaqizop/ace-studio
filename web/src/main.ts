import './m3e';
import './styles/app.css';
import './styles/pmc-spinners.css';
import { applyTheme } from './lib/prefs';
import { renderShell, syncNav } from './components/shell';
import { mountBuddy } from './components/buddy';
import { route, startRouter } from './lib/router';
import { installGlobalActions } from './lib/ui';

applyTheme();

route('/', () => import('./pages/home'));
route('/plugins', () => import('./pages/browse'));
route('/plugin/:slug', () => import('./pages/plugin'));
route('/roadmap', () => import('./pages/roadmap'));
route('/request', () => import('./pages/request'));
route('/docs', () => import('./pages/docs'));
route('/crew', () => import('./pages/crew'));

const outlet = renderShell(document.getElementById('app')!);
installGlobalActions();
mountBuddy();
startRouter(outlet, (ctx) => syncNav(ctx.path), () => import('./pages/notfound'));

// Boot loader: hide once the first page has rendered.
requestAnimationFrame(() => document.documentElement.classList.add('ready'));
