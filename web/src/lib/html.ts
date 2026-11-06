import { marked } from 'marked';
import DOMPurify from 'dompurify';

const ESC: Record<string, string> = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' };

/** Escape any value for safe use in HTML text or attribute values. */
export const esc = (v: unknown): string => String(v ?? '').replace(/[&<>"']/g, (c) => ESC[c]);

/** Material Symbols (Rounded) icon. */
export function icon(name: string, opts: { slot?: string; filled?: boolean; cls?: string } = {}): string {
  return `<m3e-icon name="${esc(name)}" variant="rounded"${opts.filled ? ' filled' : ''}${opts.slot ? ` slot="${esc(opts.slot)}"` : ''}${opts.cls ? ` class="${esc(opts.cls)}"` : ''}></m3e-icon>`;
}

const compact = new Intl.NumberFormat('en', { notation: 'compact', maximumFractionDigits: 1 });
export const num = (n: number): string => compact.format(n);

const rtf = new Intl.RelativeTimeFormat('en', { numeric: 'auto' });
export function ago(iso: string): string {
  const diff = (new Date(iso).getTime() - Date.now()) / 1000;
  const steps: [Intl.RelativeTimeFormatUnit, number][] = [
    ['year', 31536000], ['month', 2592000], ['week', 604800], ['day', 86400], ['hour', 3600], ['minute', 60],
  ];
  for (const [unit, s] of steps) if (Math.abs(diff) >= s) return rtf.format(Math.round(diff / s), unit);
  return 'just now';
}

export const date = (iso: string): string =>
  new Date(iso).toLocaleDateString('en', { year: 'numeric', month: 'short', day: 'numeric' });

export function bytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(2)} MB`;
}

marked.setOptions({ gfm: true, breaks: false });

/** Render trusted-but-sanitised Markdown. Headings get ids so m3e-toc can link to them. */
export function md(src: string): string {
  const raw = marked.parse(src, { async: false }) as string;
  const clean = DOMPurify.sanitize(raw, { USE_PROFILES: { html: true } });
  const tpl = document.createElement('template');
  tpl.innerHTML = clean;
  const seen = new Map<string, number>();
  tpl.content.querySelectorAll('h1,h2,h3,h4').forEach((h) => {
    let id = (h.textContent || 'section').toLowerCase().trim().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
    const n = seen.get(id) ?? 0;
    seen.set(id, n + 1);
    if (n) id += `-${n}`;
    h.id = id;
  });
  tpl.content.querySelectorAll('pre').forEach((pre) => pre.classList.add('code'));
  tpl.content.querySelectorAll('a[href^="http"]').forEach((a) => {
    a.setAttribute('target', '_blank');
    a.setAttribute('rel', 'noopener noreferrer');
  });
  return tpl.innerHTML;
}

/** A code block with a copy button (wired globally via [data-copy]). */
export function codeBlock(code: string, label = ''): string {
  return `<div class="codeblock">
    ${label ? `<span class="codeblock-label">${esc(label)}</span>` : ''}
    <pre class="code"><code>${esc(code)}</code></pre>
    <m3e-icon-button class="codeblock-copy" variant="tonal" size="small" data-copy="${esc(code)}" aria-label="Copy">
      ${icon('content_copy')}
    </m3e-icon-button>
  </div>`;
}
