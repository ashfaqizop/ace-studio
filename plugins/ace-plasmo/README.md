# Ace Plasmo

Proximity voice chat for [Pumpkin](https://github.com/Pumpkin-MC/Pumpkin), compatible with the
[Plasmo Voice](https://modrinth.com/plugin/plasmo-voice) 2.x client mod. A port of Plasmo Voice's server side
(LGPL-3.0-only), by Ace Studio.

## Install

1. Put `ace-plasmo-<version>.wasm` in your server's `plugins/` folder.
2. Approve its permissions when the console asks, or pre-approve them in `pumpkin.toml`. Leave `loopback_only` off
   for this plugin: players connect from outside.

   ```toml
   [plugins.overrides."ace-plasmo"]
   allowed_permissions = ["fs.read.data", "fs.write.data", "network.udp"]
   ```

3. Open **UDP 25565** in your firewall or router (the voice port, next to Minecraft's TCP 25565).
4. Players install Plasmo Voice 2.x for their Minecraft version and join.

On first start the plugin writes `config.toml` to its data folder. Change `host.port` to move the voice port, and set
`[host.public]` when players reach the server through NAT, Docker or a tunnel.

## Commands and permissions

| Command | Permission | Default |
|---|---|---|
| `/vlist` | `ace-plasmo:command.list` | everyone |
| `/vrc`, `/vreconnect` | `ace-plasmo:command.reconnect` | everyone |
| talk in proximity | `ace-plasmo:activation.proximity` | everyone |
| hear from the camera in freecam | `ace-plasmo:allow_freecam` | everyone |

## Not in 0.1

Server mutes, groups and other Plasmo add-ons, and proxy support. See [PORTING.md](PORTING.md).

## Licence

LGPL-3.0-only, like Plasmo Voice. See [LICENSE](LICENSE).
