# Ace Studio

Modrinth plugins, ported to [PumpkinMC](https://pumpkinmc.org).

This repo is the Ace Studio workspace: plugin ports are built here, and the website lives in [`web/`](web/).

## Website (`web/`)

Vite + TypeScript with [M3E](https://github.com/matraic/m3e) (Material 3 Expressive web components).

```sh
cd web
npm install
npm run dev      # local dev server
npm run build    # typecheck + production build to web/dist
```

- **Catalog:** `web/src/data/plugins.ts` — add one entry per shipped port.
- **Roadmap:** `web/src/data/roadmap.ts` — ports in progress.
- **Downloads:** GitHub release assets tagged `<slug>-v<version>` (see `web/src/config.ts`).
- **Port requests:** the Request page opens a pre-filled GitHub issue labelled `port-request`.
- **Characters:** `web/public/characters/`; animated sheets are generated with `python web/scripts/animate.py` (16 frames, Aseprite-compatible JSON next to each sheet).

Deployed on Vercel with `web` as the project root.

Ace Studio is an independent project, not affiliated with Modrinth, Mojang or the Pumpkin team.
