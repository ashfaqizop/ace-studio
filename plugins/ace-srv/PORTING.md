# Porting DiscordSRV → Ace SRV

Upstream: DiscordSRV (`DiscordSRV/DiscordSRV`, GPL-3.0, read at commit `da9e027`). Target: Pumpkin plugin API v0.1
(`pumpkin-plugin-api` at `6361843`), `wasm32-wasip2`.

## Architecture

| DiscordSRV (Java, Bukkit) | Ace SRV (Rust, Pumpkin WASM) |
| --- | --- |
| JDA (gateway + REST, its own threads) | `gateway.rs`: WebSocket over a WASI TCP socket with rustls + tungstenite, non-blocking. `http.rs`: REST over `wasi:http` (TLS by the host), non-blocking |
| Bukkit scheduler threads | One repeating task each tick pumps the gateway, HTTP, timers, presence and topics. Nothing blocks |
| `@EventHandler` listeners | `EventHandler<E>` registered non-blocking (read-only side effects) |
| `config.yml` / `messages.yml` (YAML, many locales) | `config.toml` / `messages.toml` (TOML, English), merged over built-in defaults |
| `linkedaccounts.json` / JDBC | `linked-accounts.json` |
| Legacy & codes and MiniMessage via Adventure | `text.rs`: MiniMessage subset + & codes → `TextComponent` children |

The state lives in one mutex that is never held across a Pumpkin host call. Host calls are queued as `Action`s and
run after unlocking, because Pumpkin can call straight back into the plugin from inside a host call.

## Feature map

| DiscordSRV | Ace SRV | Notes |
| --- | --- | --- |
| Minecraft → Discord chat (`PlayerChatListener`) | ✅ | `minecraft_to_discord.format`, game filters, @mention translation |
| Webhook chat delivery (`Experiment_WebhookChatMessageDelivery`) | ✅ | `chat.webhook_delivery`; the webhook is created automatically |
| Discord → Minecraft chat (`DiscordChatListener`) | ✅ | role colour/alias/selection, reply, attachments as links, filters, prefix, blocked users/roles, bots/webhooks |
| Colour codes for chosen roles | ✅ | `chat.color_code_roles` |
| Mentions / emotes translation | ✅ | `<@id>`, `<@&id>`, `<#id>`, `<:emote:id>`; unicode emoji show/hide (no emoji-to-name table) |
| Join / first join / leave / death / advancement messages | ✅ | Same embed + webhook schema as `messages.yml`, in TOML |
| Server startup / shutdown messages | ✅ | Shutdown is sent synchronously on unload with a 3 s deadline |
| Console channel commands (`DiscordConsoleListener`) | ✅ | blacklist / whitelist, usage log, ✅ / ⛔ reactions |
| Console log forwarding | ⚠️ partial | Pumpkin gives plugins no log stream, so the console channel mirrors chat, joins, leaves, deaths, advancements and console commands instead of the raw log |
| `!c` console commands in chat | ✅ | roles allowed, whitelist, bypass roles, expiration |
| `playerlist` command | ✅ | with auto-delete |
| Canned responses | ✅ | exact match, case-insensitive |
| Presence / game status (`PresenceUpdater`) | ✅ | rotating statuses with `%online%`/`%max%` |
| Channel topic updater | ✅ | chat and console topics, offline topics at shutdown |
| Account linking (`AccountLinkManager`, DM code) | ✅ | Linked role, link/unlink console commands, relink policy |
| `/discord` commands | ✅ | invite, link, unlink, linked, reload, broadcast |
| Require linked account to play (`RequireLinkModule`) | ❌ later | needs a member lookup during login |
| Group ↔ role synchronisation | ❌ later | Pumpkin has no permission groups yet |
| Voice module, alerts, watchdog | ❌ | watchdog needs threads; voice needs Discord voice + proximity |
| Plugin hooks (Vault, LuckPerms, chat plugins, Dynmap…) | ❌ | none of those exist on Pumpkin |
| Slash command API for other plugins | ❌ later | could be offered over Pumpkin IPC |
| Localised configs (14 languages) | ❌ later | English only for now |
| Message deletion / edit sync | ❌ later | |

## Placeholders kept

`%name% %username% %displayname% %message% %toprole% %toprolealias% %toprolecolor% %allroles% %reply%
%channelname% %deathmessage% %advancement% %embedavatarurl% %botname% %botavatarurl% %playercount% %playermax%
%totalplayers% %uptimemins% %tps% %date% %time%`. `%primarygroup%` is dropped (no permission groups on Pumpkin).
