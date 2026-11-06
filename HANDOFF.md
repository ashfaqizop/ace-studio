# Ace Studio — session handoff

Read this first. It is everything learnt so far that the code and git history don't say on their own.
Last updated 2026-11-06 (after Ace SRV v0.1.0 beta shipped).

## What this is

**Ace Studio** ports popular Modrinth plugins (Bukkit/Spigot/Paper/datapacks) to **Pumpkin** (Rust Minecraft server)
as WebAssembly plugins, and lists them on its own site. Not affiliated with the Pumpkin Market, but releases are also
published there.

- Repo: `ashfaqizop/ace-studio` (public), workspace `G:\1AA\ace-plugin`
- Site: https://aceplugins.dpdns.org (Vercel project `ace-studio`, team silicon-studios, **root directory `web/`**)
- Shipped: **Ace SRV** (DiscordSRV port) v0.1.0 beta — GitHub release `ace-srv-v0.1.0`, Pumpkin Market listing id **180**
- Next: **VeinMiner** (`modrinth.com/datapack/veinminer`, source `MiraculixxT/Veinminer`, AGPL-3.0-only) → "Ace VeinMiner"

## The user's rules (do not break)

- **Naming:** `Ace <name>` / `Ace <short name>`. Crate/plugin id is kebab-case (`ace-srv`) — it is also the data folder
  (`plugins/data/<id>`) and the permission namespace (`<id>:...`; must equal `PluginMetadata.name` exactly).
- **No local Cargo.** Never run cargo on the workstation. GitHub Actions builds, tests and releases. Push → read CI.
  (Local `npm`/`npx` for `web/` is fine.)
- **Licence follows the original** (translation = derivative). DiscordSRV GPL-3.0 → Ace SRV GPL-3.0;
  VeinMiner AGPL-3.0-only → Ace VeinMiner AGPL-3.0-only. Copy the upstream LICENSE into the port.
- **Reference sources go in `plugins/.cache/`** (any `.cache/` is gitignored). Read upstream there; never copy wholesale.
- **No Claude/AI attribution** in commits or PRs (no Co-Authored-By, no "Generated with").
- **Don't touch `G:\1AA\Market-Frontend`** (the user's own work). `Market-Frontend\latest` is the user's local Pumpkin
  test server; `Market-Frontend\testp` holds test builds. Only edit there when explicitly asked (e.g. `pumpkin.toml`).
- No screenshots unless asked. Don't poll in loops — use background commands that notify on exit.
- Commit/push when asked ("push", "commit and push"). The user likes web + plugin changes in one push.
- For the separate Pumpkin Market-Frontend PR work: UI/UX-only changes.
- **If a port can't be built because the Pumpkin API lacks something: don't emulate, don't build — list it.**
  `plugins/<id>/ace.json` with `status: "waiting-api"`, `versions: []`, `waiting.{since, needs[{api, why}], tracking?}`
  and the upstream LICENSE, no crate. The site shows it as "Waiting on Pumpkin Plugin API" (own Browse section, own page
  with a "What's missing" tab). First case: Ace VeinMiner.
- Original-plugin icons on the site come from Modrinth automatically (`web/src/lib/modrinth.ts`, slug parsed from
  `original.url`); `original.icon` in ace.json is an optional override.

## Repo layout

```text
web/                 Vite + TS + @m3e/web site (see "Site" below)
plugins/             Cargo workspace, target wasm32-wasip2 (.cargo/config.toml)
  Cargo.toml         workspace deps; pumpkin-plugin-api pinned by git rev
  README.md          porting rules
  ace-srv/           Ace SRV: src/, defaults/*.toml, ace.json, icon.png, PORTING.md, CHANGELOG.md, LICENSE
  .cache/            upstream sources (ignored): discordsrv/, paper/ (paper-api only)
.github/workflows/   plugins-build.yml (reusable), plugins.yml (push/PR), plugins-release.yml (tags)
pumpkin-docs/        (ignored) official docs as .md + book/ = vyPal's Pumpkin Plugin Book (MIT) — READ THESE
characters/ spinners/  (ignored) assets/experiments
```

## Pumpkin plugin facts (learnt the hard way)

Docs: `pumpkin-docs/plugin-dev/rust/*`, `pumpkin-docs/book/**` (book is the most accurate: verified against source).

- Rust SDK `pumpkin-plugin-api` from git, **pinned** `rev = 6361843f5d9029b0cf7ecd6b1f2e86ace1256934` (server
  `0.2.1+26.3-26.60`, Minecraft 26.3). It implements **plugin API v0.1** (frozen 2026-09-29). Don't add `version=`.
- Build **release only** (debug components fail to load). Target `wasm32-wasip2`; output is a component.
- `PluginMetadata { name, version, authors, description, dependencies, permissions }`; `on_load(&self, Context)`.
- **Re-entrancy:** host calls can synchronously call back into the plugin (events, commands). Never hold a Mutex
  across a Pumpkin host call — in WASM that is a panic = dead plugin. Ace SRV pattern: one `static STATE: Mutex<Option<State>>`,
  pure work under the lock, queue `Action`s, run them after unlocking (`state.rs`).
- Any panic / trap kills the plugin for good → no `unwrap` on host data.
- `TextComponent` is a resource; styling methods take `self` and return it (chain them). `add_child` consumes the child.
- `register_permission` node prefix must equal metadata `name`; unregistered nodes deny everyone but console.
- Unwired command argument types trap the plugin at registration (see book commands/argument-types.md).
- Events: see `book/advanced/event-reference.md` for which actually fire (`AsyncPlayerChatEvent` never fires;
  `PlayerChatEvent`, join/leave/death/advancement, `ServerCommandEvent`, `ServerLoadEvent` do). Non-blocking handlers
  can't cancel/modify; blocking ones hold the game.
- `server.get_sys_info()` needs the `sys.info` permission — avoided.
- Scheduler: `context.schedule_repeating_task(delay, period, |server| ...)`; one call into plugins at a time globally.
- **Sandbox permissions** (`PluginMetadata.permissions`; host mapping in `pumpkin-wasm-host/src/runtime.rs`):
  `fs.read.data`/`fs.write.data` (data folder mounted as `data`, use `context.get_data_folder()`), `http.outbound`
  (wasi:http, host does TLS), `network.dns`, `network.tcp.connect` (or `network.outbound`), `sys.env*`.
  `[plugins.overrides."<id>".environment]` in pumpkin.toml injects env vars **without** any permission (used for secrets).
- Networking that works: `wasi` crate 0.14 for non-blocking `wasi:http` (poll `FutureIncomingResponse::get()` each tick;
  `blocking_write_and_flush` ≤4096 B chunks; call `handle()` before writing the body); `std::net::TcpStream` +
  `set_nonblocking` over wasi:sockets; rustls 0.23 (`ring`) + webpki-roots + tungstenite 0.27 for WebSockets.
  **ring needs a C toolchain**: CI installs wasi-sdk 25 and sets `CC/AR/CFLAGS_wasm32_wasip2`.
- **Signing:** `pumpkin.metadata` + `wasm_signature` custom sections (Ed25519 over clean wasm + metadata JSON). The host
  verifies with the key embedded in the plugin (no trusted-key config exists despite the docs). Unsigned only warns
  (`allow_unsigned = true` default). Market-uploaded binaries come back signed.

## CI / release flow

- `plugins.yml`: every push touching `plugins/**` builds all ports, runs `cargo test --lib --target x86_64-unknown-linux-gnu`
  (non-fatal), uploads artifact `plugins` (`<id>-<version>.wasm`). Job `manifests` fails if `plugins/<id>/ace.json`
  ≠ `web/src/data/ports/<id>.json`, or `versions[0].version` ≠ Cargo.toml version.
- **Release:** bump version in `Cargo.toml` + `ace.json` (+ CHANGELOG.md), push, then `git tag <id>-v<version>` and
  push the tag. `plugins-release.yml` builds → if `ace.json.market.id` and secret `PUMPKIN_MARKET_TOKEN` exist:
  uploads to the Market, publishes, downloads the signed copy → GitHub release with that `.wasm`.
- Get a CI build locally: `gh run download <id> -n plugins -D <dir>`.

## Pumpkin Market API (market.pumpkinmc.org)

- Public spec `/api/openapi.json` only covers `/api/v1/rest/*` (read: plugins, download, metadata, public-key, licences).
- Developer endpoints (from the site's JS; base `/api`, send `X-Requested-With: XMLHttpRequest`):
  `POST /plugins/draft {name, category, type}` → `{id}`; `PUT /plugins/{id}` multipart (`metadata` JSON:
  `name, category, sourceLink, keywords, translatedDescriptions:{en:md}, isEarlyAccess, commands[]` + optional
  `preview_image`) — the same PUT with `wasm` file + `metadata {version, track: stable|beta|alpha, releaseNotes}` uploads
  a version; `GET /plugins/{id}/readiness`; `POST /plugins/{id}/publish`; `DELETE /plugins/{id}` needs body `{code: <2FA>}`.
  Categories: Admin Tools, Economy, Fun, World Management, Utilities, Chat, Other. Binary max 5 MB. Icon 32–512 px.
- Auth: `Authorization: Bearer pmk_...` works on these routes. API keys **cannot** manage API keys (web session only).
- Cloudflare blocks Python's default User-Agent (error 1010) — send a UA (e.g. `curl/8.x`).
- Market web bug: creating an API key never shows the token (page expects `rawKey`). Workaround: create via the
  DevTools console `fetch('/api/user/api-keys', {method:'POST', credentials:'include', ...})` and print the response.
- The CI token lives only in the GitHub secret `PUMPKIN_MARKET_TOKEN` (all scopes, no expiry). It was once pasted in chat
  → the user should rotate it. Never write tokens into files or commits.

## Site (web/)

- Vite 8 + TS 7 + `@m3e/web` 2.11 (use M3E components wherever possible), fonts Instrument Serif + Bricolage Grotesque,
  pixel crew sprites (`web/public/characters`, 16-frame sheets), custom History router with View Transitions.
- Catalog: `web/src/data/plugins.ts` imports `./ports/*.json` (glob). `vite.config.ts` plugin `ace-sync-ports` copies
  `../plugins/*/ace.json` there on dev/build — **commit the copies** (Vercel can't see outside `web/`).
- Demo data: `src/data/demo.local.ts` (gitignored) via `virtual:demo-data`, dev only, never in production.
- Live data: GitHub releases → download counts and file sizes (`[data-dl]`, `[data-size-file]`); roadmap and port
  requests from GitHub issues (labels `port-request`, `stage:*`), cached 10 min in sessionStorage (60 req/h limit).
- `vercel.json`: plain catch-all rewrite `/(.*) → /index.html` (regex lookaheads were not applied).
- `m3e-shape` lays slot content in a shadow wrapper → centre icons with absolute positioning (`.plugin-icon > m3e-icon`).

## Ace SRV specifics

`plugins/ace-srv/PORTING.md` maps DiscordSRV features. Config/messages are TOML merged over embedded defaults
(`defaults/*.toml`, unknown keys warned, "free maps" replaced wholesale). Not ported: raw console log (no log API),
group sync, require-link, voice, watchdog, Bukkit hooks. Gateway intents need **Message Content** enabled.
Token via env `ACE_SRV_BOT_TOKEN`.

## Windows / tooling gotchas

- Git Bash converts `/x` args to Windows paths → prefix `MSYS_NO_PATHCONV=1` for `gh api` paths etc.
- Backslashes in heredocs / inline Python get mangled. For Windows paths in generated files, write a script file with
  the Write tool and build backslashes with `chr(92)`.
- `gh` is authenticated as the repo owner. Vercel CLI token is expired (re-login needed for API calls).
- Waiting on CI: `gh run watch <id> --exit-status` in a background command, then `gh run view <id> --log`.

## Ace VeinMiner — listed as waiting-api (`plugins/ace-veinminer/ace.json`), BLOCKED on Pumpkin API (user: wait, don't emulate)

Upstream cloned to `plugins/.cache/veinminer` (MiraculixxT/Veinminer @ b3bfeb3, Kotlin; AGPL-3.0-only per Modrinth, no
LICENSE file in repo). Core algorithm: `core/.../pattern/Veinmining.kt` (layered BFS over a `ShapeStrategy`: normal,
flat, tunnel WxH, stairs; `searchRadius`, `maxChain`, per-layer `delay`), settings in `data/VeinminerSettings.kt`
(cooldown, mustSneak, needCorrectTool, decreaseDurability, hungerPerBlock, mergeItemDrops, permissionRestricted…),
groups `veinminer/assets/default_groups.json` (`#c:ores` + `#minecraft:pickaxes`), Paper impl `veinminer/paper/.../VeinMinerEvent.kt`.

Available in API v0.1: `BlockBreakEvent` (player, block, pos, should_drop, cancellable), world get/set block (+ `skip-drops`
flag, but `set_block_state` never drops loot), `is_sneaking`, held item + enchantments, food/exhaustion, inventory slots.
Missing (server has them internally): `World::break_block(pos, Some(player), flags)` (fires BlockBreakEvent, loot with the
held tool → Fortune/Silk Touch, break effects — `crates/pumpkin-core/src/world/mod.rs`), `World::drop_stack(pos, stack)`,
tool damage respecting Unbreaking, block/item tag lookups. Rust SDK only targets v0.1 (frozen), so this needs a new API
version with SDK support. Resume when Pumpkin ships it; the request is in the Discord ideas post.

## Open items

- Rotate `PUMPKIN_MARKET_TOKEN` (exposed in chat). Do it in the browser console, pipe straight to `gh secret set`.
- Post the Discord ideas-forum message to Pumpkin (draft was given to the user): log stream API, `execute_command`
  output, deferrable pre-login, permission groups, trusted signing keys, `get_version()`, Market developer API docs.
- Market-Frontend: the API-key token display bug is worth a PR by the user.
