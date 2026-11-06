# Ace LP

Permissions for [Pumpkin](https://github.com/Pumpkin-MC/Pumpkin): groups, inheritance, weights, tracks, contexts,
temporary permissions, prefixes, suffixes and meta. A port of [LuckPerms](https://luckperms.net) (MIT) by Ace Studio.

## Install

1. Put `ace-lp-<version>.wasm` in `plugins/` and approve `fs.read.data` and `fs.write.data` when the console asks.
2. Join once, then give yourself access from the console: `lp user <you> permission set ace-lp:command true`
   (op level 4 works too).
3. Build groups:

   ```
   /lp creategroup admin 100
   /lp group admin permission set minecraft:command.gamemode true
   /lp group admin meta setprefix 100 "&c[Admin] "
   /lp user Steve parent add admin
   ```

Everyone starts in `default`. Permissions apply the moment you change them, no relog needed.

## Pumpkin nodes

Pumpkin permission nodes are `namespace:path`, such as `minecraft:command.gamemode` or `ace-srv:command.link`.
Wildcards are `namespace:*` and `namespace:path.*`. A bare `*` isn't supported by Pumpkin, so use
`minecraft:*` and one `<plugin>:*` per plugin instead.

## Contexts

Add contexts at the end of a command: `/lp group builder permission set minecraft:command.fill true world=overworld`.

| Context | Values |
|---|---|
| `world` | the world id (`minecraft:overworld`) or its short name (`overworld`) |
| `dimension-type` | `overworld`, `the_nether`, `the_end`, ... |
| `gamemode` | `survival`, `creative`, `adventure`, `spectator` |
| `server` | the `server` name in `config.toml` (not set when it's `global`) |

## Storage

`plugins/data/ace-lp/json-storage/` uses LuckPerms' JSON layout (`users/`, `groups/`, `tracks/`). You can copy a
LuckPerms `json-storage` folder in. Node names will usually need changing to Pumpkin's `namespace:path` form.

## For other plugins

Send JSON over Pumpkin IPC to `ace-lp`: `{"op":"user","uuid":"..."}` returns primary group, groups, prefix, suffix
and meta. See `src/ipc.rs`.

## Not in 0.1

The web editor (`/lp editor`), verbose, tree, bulk updates, the action log, import/export, SQL/MongoDB storage and
cross-server sync. See [PORTING.md](PORTING.md).
