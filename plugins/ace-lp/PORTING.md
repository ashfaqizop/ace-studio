# Porting notes: LuckPerms → Ace LP

Upstream: [LuckPerms/LuckPerms](https://github.com/LuckPerms/LuckPerms) `master` @ `b878ae9`, MIT.

## How permissions reach Pumpkin

LuckPerms on Bukkit replaces each player's `Permissible`, so it answers every permission check itself. Pumpkin has
no such hook, but its permission manager keeps a per-player **attachment** (node → true/false) that every check reads
first, with `namespace:*` / `prefix.*` wildcards, children and registered defaults behind it
(`pumpkin-util/src/permission.rs`).

So Ace LP flattens each online player's nodes for their current contexts, exactly as LuckPerms resolves them, and
writes the result into that attachment with `player.set-permission`. On every change (command, temporary node
expiring, world or gamemode change) only the difference is written. Afterwards it sets the player's op level to
itself, which makes Pumpkin resend the command tree so tab-complete matches. When a player leaves, or the plugin
unloads, everything Ace LP set is removed.

## Resolution

| LuckPerms | Here |
|---|---|
| `InheritanceGraph` (depth-first pre-order) | `resolve::Groups::traverse` |
| `InheritanceComparator` (weight, then primary group) | `resolve::Groups::parents` |
| `NodeMap` + `ContextSetComparator` + `NodeComparator` | `resolve::ordered`: specific contexts, temporary, deny first |
| `PermissionHolder.exportPermissions` (first key wins) | `resolve::Groups::permissions` |
| `MetaAccumulator` (highest prefix/suffix, first meta) | `resolve::Groups::meta` |
| `primary-group-calculation: parents-by-weight` | `resolve::Groups::primary_group` |
| `UserManager.giveDefaultIfNeeded` | `model::User::give_default_if_needed` |
| `ExpireTemporaryTask` | `state::Lp::tick` (every second) |

## Storage

`SeparatedConfigurateStorage` with the JSON loader: same file layout and field names (`permissions`, `parents`,
`prefixes`, `suffixes`, `meta`, `primaryGroup`, `context`, `expiry`, `priority`). Users with nothing but `default`
have no file, like upstream.

## Commands

`/lp user|group … info | permission | parent | meta`, `promote`/`demote`, group `setweight`/`setdisplayname`/
`listmembers`/`rename`/`clone`, tracks, `creategroup`/`deletegroup`/`listgroups`, `createtrack`/`deletetrack`/
`listtracks`, `sync`, `info`. Arguments are parsed from one greedy argument with LuckPerms' quoting rules.
One permission node (`ace-lp:command`, op level 4 by default) guards `/lp`; upstream has one per subcommand.

## Differences

- **No global `*`.** Pumpkin wildcards are per namespace (`minecraft:*`). Ace LP can't expand `*` because the plugin
  API can't list registered permissions.
- **No verbose or `permission check` tracing of other plugins' checks**: checks never pass through Ace LP.
- **Contexts**: `world` (full id and short name), `dimension-type`, `gamemode`, and `server` from `config.toml`.
- **Temporary nodes** replace an existing one in the same slot (upstream's default modifier is `deny`).

## Later

- **Web editor**: `/lp editor` uploads a gzipped payload to bytebin and `/lp applyedits <code>` downloads the
  changes (`http.outbound`); the live mode adds a bytesocks WebSocket with signed messages (`/lp trusteditor`).
- Action log, verbose (limited to Ace LP's own view), tree, bulk updates, import/export, SQL storage and
  cross-server messaging.
