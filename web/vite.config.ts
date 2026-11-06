import { defineConfig, type Plugin } from 'vite';
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

/**
 * Port manifests live next to each port (`../plugins/<id>/ace.json`, the source of truth), but Vercel only sees
 * `web/`. So every local dev/build copies them into `src/data/ports/<id>.json`, which is committed and is what the
 * site imports. Without `../plugins` (on Vercel) the committed copies are used as they are.
 */
function syncPorts(): Plugin {
  const src = resolve(__dirname, '../plugins');
  const out = resolve(__dirname, 'src/data/ports');
  const sync = () => {
    if (!existsSync(src)) return;
    mkdirSync(out, { recursive: true });
    for (const dir of readdirSync(src, { withFileTypes: true })) {
      if (!dir.isDirectory() || dir.name.startsWith('.')) continue;
      const manifest = resolve(src, dir.name, 'ace.json');
      if (!existsSync(manifest)) continue;
      const text = readFileSync(manifest, 'utf8');
      const target = resolve(out, `${dir.name}.json`);
      if (!existsSync(target) || readFileSync(target, 'utf8') !== text) writeFileSync(target, text);
    }
  };
  return {
    name: 'ace-sync-ports',
    buildStart: sync,
    configureServer(server) {
      server.watcher.add(resolve(src, '*/ace.json'));
      server.watcher.on('change', (file) => {
        if (file.replace(/\\/g, '/').endsWith('/ace.json')) sync();
      });
    },
  };
}

/**
 * `virtual:demo-data` resolves to src/data/demo.local.ts during `vite dev` only (if the file
 * exists). Production builds always get an empty stub, so demo content can never ship.
 * demo.local.ts is gitignored.
 */
function demoData(): Plugin {
  const id = 'virtual:demo-data';
  const resolved = '\0' + id;
  const file = resolve(__dirname, 'src/data/demo.local.ts');
  let serve = false;
  return {
    name: 'ace-demo-data',
    configResolved(c) {
      serve = c.command === 'serve';
    },
    resolveId(source) {
      if (source !== id) return null;
      return serve && existsSync(file) ? file : resolved;
    },
    load(source) {
      if (source === resolved) return 'export const plugins = []; export const roadmap = [];';
      return null;
    },
  };
}

export default defineConfig({
  plugins: [syncPorts(), demoData()],
});
