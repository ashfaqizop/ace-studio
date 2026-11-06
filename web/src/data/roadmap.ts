import type { CharRef } from './crew';
import { roadmap as demoRoadmap } from 'virtual:demo-data';

export type Stage = 'queued' | 'translating' | 'testing' | 'shipped';

export const STAGES: Record<Stage, { label: string; icon: string; lead: CharRef; hint: string }> = {
  queued: { label: 'Queued', icon: 'inbox', lead: 'codex/architect', hint: 'Requested and licence-checked, waiting for a builder.' },
  translating: { label: 'Translating', icon: 'translate', lead: 'claude/forge', hint: 'Being rewritten against the Pumpkin plugin API.' },
  testing: { label: 'Testing', icon: 'science', lead: 'grok/probe', hint: 'Running on test servers and getting reviewed.' },
  shipped: { label: 'Shipped', icon: 'rocket_launch', lead: 'codex/deploy', hint: 'Released. Download it from the catalog.' },
};

export interface RoadmapItem {
  name: string;
  summary: string;
  stage: Stage;
  progress: number;
  assignee: CharRef;
  votes: number;
  slug?: string;
}

/** Ports in progress. Add an item when a request is accepted; move it along as it progresses. */
export const ROADMAP: RoadmapItem[] = [
  ...demoRoadmap, // dev-only demo entries (empty in production)
];
