import { defineConfig, type Plugin } from 'vite';
import { existsSync } from 'node:fs';
import { resolve } from 'node:path';

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
  plugins: [demoData()],
});
