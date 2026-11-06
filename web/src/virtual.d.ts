declare module 'virtual:demo-data' {
  import type { Plugin } from './data/plugins';
  import type { RoadmapItem } from './data/roadmap';
  /** Local-only demo catalog (src/data/demo.local.ts). Always empty in production builds. */
  export const plugins: Plugin[];
  export const roadmap: RoadmapItem[];
}
