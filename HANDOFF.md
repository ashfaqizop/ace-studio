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

## Ace VeinMiner Enchantment: waiting-api, add-on to Ace VeinMiner

`plugins/ace-veinminer-enchantment/ace.json`, LGPL-3.0-only (Modrinth). Upstream is in the same repo:
`veinminer-enchant/` and `datapacks/enchantment/` (enchantment JSON + tags `non_treasure`/`tradeable`). It's blocked by
Ace VeinMiner first. `enchantment-manager.register-enchantment` exists in v0.1, but `custom-enchantment` has no
min/max cost, and plugins can't add enchantments to tags (in_enchanting_table/tradeable/on_random_loot/treasure).
Upstream asks for no reuploads or forks on Modrinth/CurseForge; we publish only to GitHub, our site and the Pumpkin Market.

## Ace Plasmo (Plasmo Voice): built, v0.1.0 not released yet

`plugins/ace-plasmo/`, LGPL-3.0-only, upstream `plugins/.cache/plasmo` (plasmoapp/plasmo-voice @ b93ea39, 2.2.0-beta.2).
Server side only; players use the official Plasmo Voice 2.x mod. Protocol notes are in `ace-plasmo/PORTING.md`.
- Handshake runs on plugin channel `plasmo:voice/v2` (`PlayerCustomPayloadEvent` in, `java_player.send_custom_payload`
  out; Pumpkin fires the event for every play-phase payload, `crates/pumpkin-core/src/net/java/mod.rs`). Audio goes
  over a non-blocking `std::net::UdpSocket` (permission `network.udp`: `network.udp.bind` alone can't send), drained
  every tick.
- The server never decodes audio. It RSA-encrypts (PKCS#1 v1.5, `rsa` crate) a 16-byte AES key per client and forwards
  the encrypted Opus bytes as-is.
- Wire format: Java DataOutput (big-endian, modified UTF-8). Fixed ids `nameUUIDFromBytes("proximity_activation" /
  "proximity_line")` are hardcoded and checked by tests (md-5 dev-dep).
- Not yet: mutes, `/vreload`, mod-required kick, proxy service channel, add-ons, vanish.
- Testing needs a client with Plasmo Voice 2.x for MC 26.3, and UDP 25565 open.

Pumpkin source at the pinned rev is cloned in `plugins/.cache/pumpkin` (it includes `pumpkin-plugin-api` and the WIT).
It also has an in-progress **API v0.2** (`pumpkin-plugin-wit/v0.2`, `pumpkin-wasm-host-v0_2`). v0.2 adds
`world.break-block(pos, drop-items, flags)`, but with no player or tool, so it doesn't unblock VeinMiner (no Fortune or
Silk Touch, no tool wear).

## Ace LP (LuckPerms): built, 0.1.0 not released yet

`plugins/ace-lp/`, MIT; upstream is cloned to `plugins/.cache/luckperms` @ b878ae9. Pumpkin has no hook to answer
permission checks, but `player.set-permission` writes a per-player attachment that every check reads first
(`pumpkin-util/src/permission.rs`: exact node, then `ns:*` and `prefix.*` wildcards, children, then defaults). Ace LP
flattens LuckPerms resolution per player and context and writes only the difference. Calling
`set_permission_level(same)` forces a command-tree resend. Attachments are cleared on leave and unload.
Storage is LuckPerms' JSON layout. IPC exposes `user` / `has_group` / `group`.
Next: the web editor. Upload mode first (gzip → bytebin `usercontent.luckperms.net/post`, open
`luckperms.net/editor/<code>`, `/lp applyedits <code>`), then the live bytesocks socket (ECDSA P-256 signed,
`/lp trusteditor`). Payload: `metadata`, `permissionHolders`, `tracks`, `knownPermissions`, `potentialContexts`
(`webeditor/WebEditorRequest.java`); response: `changes`, `groupDeletions`, … (`WebEditorResponse.java`).

## Ace Chunky: waiting-api (`plugins/ace-chunky/ace.json`)

Chunky (pop4959, GPL-3.0-only; upstream is cloned to `plugins/.cache/chunky` @ c4274bd). Its platform `World` interface
needs `getChunkAtAsync` (load or generate) and `isChunkGenerated`, plus region-file access for trim. Plugin API v0.1 and
v0.2 `get-chunk` only returns already-loaded chunks (`pumpkin-wasm-host-v0_1/src/world.rs`), and block reads on
unloaded chunks return air. The server has `Level::get_or_fetch_chunk` (`pumpkin-world/src/level.rs`, force ticket).
Resume when the API exposes a chunk load/generate call.

## Ace ViaVersion: waiting-api (`plugins/ace-viaversion/ace.json`)

ViaVersion is GPL-3.0-or-later; upstream wasn't cloned (not needed yet). Pumpkin only accepts 26.3
(`LOWEST_SUPPORTED_MC_VERSION == CURRENT_MC_VERSION`) and kicks other versions in `net/java/handshake.rs` before any
plugin sees them. The core has `ConnectionPacketReceivedEvent`/`ConnectionPacketSentEvent` (`net/java/pending.rs`) for
login/config translation, but they're not in WIT v0.1/v0.2. Play-phase `PacketReceived/SentEvent` are in WIT. Comments
mention a native `pumpkin-java-multiversion`, so Pumpkin may do this itself.

## Ace SVC (Simple Voice Chat): ON HOLD for licence, listed as `waiting-permission` (`plugins/ace-svc/ace.json`)

Simple Voice Chat (henkelmax) is **All Rights Reserved** (licence file + Modrinth `LicenseRef-All-Rights-Reserved`).
Don't translate or ship it without Max Henkel's written permission. The user is contacting him (SVC Discord
<https://discord.gg/4dH2zwTmyX>, X @henkelmax; no public email). Upstream is cloned to `plugins/.cache/svc`, but none of
its code has been read. Feasibility in v0.1 looks good: `network.udp.bind`, `java_player.send_custom_payload`,
`PlayerCustomPayloadEvent` and `PlayerChannelEvent` (both fire). WIT v0.1 files are in `plugins/.cache/wit/`. The
legal alternative is Plasmo Voice (LGPL-3.0-only).

## Open items

- Rotate `PUMPKIN_MARKET_TOKEN` (exposed in chat). Do it in the browser console, pipe straight to `gh secret set`.
- Post the Discord ideas-forum message to Pumpkin (draft was given to the user): log stream API, `execute_command`
  output, deferrable pre-login, permission groups, trusted signing keys, `get_version()`, Market developer API docs.
- Market-Frontend: the API-key token display bug is worth a PR by the user.
