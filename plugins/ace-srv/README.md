# Ace SRV

A Discord ⇄ Minecraft bridge for [Pumpkin](https://github.com/Pumpkin-MC/Pumpkin) servers, ported from
[DiscordSRV](https://github.com/DiscordSRV/DiscordSRV). GPL-3.0.

- Game chat in Discord, Discord chat in game, with role colours, role aliases, replies, mentions and attachments
- Join, first join, leave, death and advancement announcements as embeds (or webhooks with player heads)
- Server start and stop messages, live channel topics, rotating bot status
- A console channel: run console commands from Discord, see chat, joins and commands mirrored back
- `!c <command>` in the chat channel for chosen roles, `playerlist`, canned replies like `!ip`
- Account linking with a 4-digit code sent to the bot by DM, with a Linked role and link/unlink commands

## Install

1. Create a bot at <https://discord.com/developers/applications>. Under **Bot**, enable the
   **Message Content** privileged intent. Invite it with the `bot` scope and these permissions: View Channels,
   Send Messages, Embed Links, Read Message History, Add Reactions, Manage Messages (for auto-deleting the player
   list), Manage Channels (for topics), Manage Webhooks (only if you use webhooks), Manage Roles (only for the Linked
   role).
2. Drop `ace-srv-<version>.wasm` into the server's `plugins/` folder and start the server once. Approve the
   permissions it asks for in the console (file access to its data folder, outbound HTTP and a TCP connection to
   Discord).
3. Put the token in `pumpkin.toml` so it stays out of the plugin's files:

   ```toml
   [plugins.overrides."ace-srv".environment]
   ACE_SRV_BOT_TOKEN = "your-bot-token"
   ```

   (or set `bot_token` in `plugins/data/ace-srv/config.toml`).
4. Set `channels.global` (and optionally `console.channel_id`) in `config.toml` to your channel ids
   (Discord: Settings → Advanced → Developer Mode, then right-click a channel → Copy Channel ID).
5. Run `/discord reload` or restart.

## Commands

| Command | Permission | Default |
| --- | --- | --- |
| `/discord` | `ace-srv:command.discord` | everyone |
| `/discord link`, `unlink`, `linked` | `ace-srv:command.link` | everyone |
| `/discord reload`, `broadcast <message>` | `ace-srv:command.admin` | op level 2 |

`/acesrv` is an alias.

## Files

All in `plugins/data/ace-srv/`: `config.toml`, `messages.toml` (every text, with MiniMessage-style tags for the
game side), `linked-accounts.json`, `players.json` (for first-join and unique player counts) and the console usage
log.

## Building

There is no local build. GitHub Actions builds every push (`.github/workflows/plugins.yml`), and a tag
`ace-srv-v<version>` publishes the `.wasm` as a release.
