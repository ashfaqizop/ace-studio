# Ace plugins

Pumpkin ports of existing Modrinth plugins, translated one by one from Java (Bukkit/Spigot/Paper) to Rust and
compiled to WebAssembly components for [Pumpkin](https://github.com/Pumpkin-MC/Pumpkin).

## Rules

- **Naming:** every port is called `Ace <name>` (or `Ace <short name>`). DiscordSRV becomes **Ace SRV**.
  The crate / plugin id is the kebab-case form (`ace-srv`); that id is also the data folder and permission namespace.
- **No local Cargo.** Nothing here is built on a workstation. GitHub Actions builds, checks and releases every port
  (`.github/workflows/plugins.yml`). Push, then read the CI result.
- **Licence follows the original.** A translation is a derivative work, so each port keeps its upstream licence
  (DiscordSRV is GPL-3.0, so Ace SRV is GPL-3.0).
- **Reference sources live in `.cache/`** (gitignored): the upstream plugin and the Paper API, used to read
  behaviour, never copied wholesale.
- **Site listing:** each port has an `ace.json` (the Ace Studio catalog entry). Running `npm run dev` or
  `npm run build` in `web/` copies it to `web/src/data/ports/`, which is committed because Vercel only sees `web/`.
  CI fails if the two differ or if the manifest version doesn't match `Cargo.toml`.
- **Releases:** tag `<id>-v<version>` (for example `ace-srv-v0.1.0`). CI attaches `<id>-<version>.wasm` to the
  GitHub release, which is where the Ace Studio site downloads from.

## Layout

```
plugins/
  Cargo.toml            workspace (all ports share one lockfile and release profile)
  .cargo/config.toml    builds for wasm32-wasip2
  ace-srv/              Ace SRV (DiscordSRV)
  .cache/               upstream sources for reference (ignored)
```

## Targets

Rust plugins target Pumpkin plugin API **v0.1** (`pumpkin-plugin-api`, pinned by git rev in the workspace).
