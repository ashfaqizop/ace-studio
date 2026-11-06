export interface Ctx {
  path: string;
  params: Record<string, string>;
  query: URLSearchParams;
}

export interface Page {
  title: string | ((ctx: Ctx) => string);
  render(ctx: Ctx): string;
  /** Wire up events after render. May return a cleanup function. */
  mount?(root: HTMLElement, ctx: Ctx): void | (() => void);
}

type Loader = () => Promise<{ default: Page }>;

const routes: [RegExp, string[], Loader][] = [];
let outlet: HTMLElement;
let cleanup: (() => void) | void;
let afterNavigate: (ctx: Ctx) => void = () => {};
let current = '';

export function route(pattern: string, load: Loader): void {
  const keys: string[] = [];
  const re = new RegExp(
    '^' + pattern.replace(/\/:([a-z]+)/gi, (_, k) => (keys.push(k), '/([^/]+)')) + '/?$',
    'i',
  );
  routes.push([re, keys, load]);
}

export function startRouter(el: HTMLElement, onNavigate: (ctx: Ctx) => void, notFound: Loader): void {
  outlet = el;
  afterNavigate = onNavigate;
  routes.push([/.*/, [], notFound]);
  addEventListener('popstate', () => render(false));
  document.addEventListener('click', intercept);
  render(false);
}

export function navigate(to: string, { replace = false } = {}): void {
  const url = new URL(to, location.href);
  if (url.origin !== location.origin) {
    location.href = url.href;
    return;
  }
  const next = url.pathname + url.search + url.hash;
  if (next === location.pathname + location.search + location.hash) return;
  history[replace ? 'replaceState' : 'pushState'](null, '', next);
  render(true);
}

/** Update the query string without re-rendering (for in-page filters). */
export function setQuery(q: URLSearchParams): void {
  const s = q.toString();
  history.replaceState(null, '', location.pathname + (s ? `?${s}` : ''));
}

function intercept(e: MouseEvent): void {
  if (e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
  for (const n of e.composedPath()) {
    if (!(n instanceof Element)) continue;
    if (n.hasAttribute('data-noroute')) return;
    const href = n.getAttribute('href');
    if (href == null) continue;
    if (n.getAttribute('target') === '_blank' || n.hasAttribute('download')) return;
    if (href.startsWith('#') || /^(mailto|tel):/.test(href)) return;
    const url = new URL(href, location.href);
    if (url.origin !== location.origin) return;
    e.preventDefault();
    navigate(url.pathname + url.search + url.hash);
    return;
  }
}

async function render(animate: boolean): Promise<void> {
  const path = location.pathname;
  const key = path + location.search;
  const [re, keys, load] = routes.find(([r]) => r.test(path))!;
  const m = path.match(re)!;
  const params: Record<string, string> = {};
  keys.forEach((k, i) => (params[k] = decodeURIComponent(m[i + 1])));
  const ctx: Ctx = { path, params, query: new URLSearchParams(location.search) };

  const samePage = current.split('?')[0] === path;
  current = key;
  const slow = setTimeout(() => outlet.classList.add('is-loading'), 120);
  let page: Page;
  try {
    page = (await load()).default;
  } catch (err) {
    clearTimeout(slow);
    outlet.classList.remove('is-loading');
    outlet.innerHTML = `<div class="page-error">Couldn't load this page. <a href="${path}">Retry</a></div>`;
    console.error(err);
    return;
  }
  clearTimeout(slow);
  if (current !== key) return; // a newer navigation won

  const swap = () => {
    if (typeof cleanup === 'function') cleanup();
    outlet.classList.remove('is-loading');
    outlet.innerHTML = page.render(ctx);
    document.title = `${typeof page.title === 'function' ? page.title(ctx) : page.title} · Ace Studio`;
    cleanup = page.mount?.(outlet, ctx);
    if (location.hash) document.getElementById(location.hash.slice(1))?.scrollIntoView();
    else if (!samePage) scrollTo({ top: 0 });
    afterNavigate(ctx);
  };

  const vt = (document as Document & { startViewTransition?: (cb: () => void) => unknown }).startViewTransition;
  if (animate && vt && !matchMedia('(prefers-reduced-motion: reduce)').matches) vt.call(document, swap);
  else swap();
}
