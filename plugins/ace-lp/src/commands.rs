//! `/lp` (aliases `/luckperms`, `/perms`, `/perm`, `/permissions`): the LuckPerms command tree, parsed from one
//! greedy argument. Replies use `&` colour codes and are sent after the state lock is released.
//! (LuckPerms: `commands/user`, `commands/group`, `commands/track`, `commands/generic/*`, `commands/misc`.)

use crate::model::{self, Contexts, Group, Holder, Kind, Node, Track, valid_name};
use crate::state::{Lp, now};
use std::fmt::Write;

pub type Reply = Vec<String>;

const HEAD: &str = "&3&lLP &8» &7";

fn line(s: impl AsRef<str>) -> String {
    format!("{HEAD}{}", s.as_ref())
}

/// Split like LuckPerms: whitespace, with "double quotes" grouping.
pub fn tokenize(s: &str) -> Vec<String> {
    let mut out = vec![];
    let mut cur = String::new();
    let mut quoted = false;
    let mut any = false;
    for c in s.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            c if c.is_whitespace() && !quoted => {
                if any || !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
                any = false;
            }
            c => cur.push(c),
        }
    }
    if any || !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Trailing `key=value` arguments become contexts (`ArgumentList.getContextOrDefault`); "global" means none.
fn contexts(args: &[String]) -> Result<Contexts, String> {
    let mut c = Contexts::new();
    for a in args {
        if a.eq_ignore_ascii_case("global") {
            continue;
        }
        let (k, v) = a.split_once('=').ok_or_else(|| format!("&c'{a}' isn't a context. Use key=value, like world=minecraft:the_nether"))?;
        if k.is_empty() || v.is_empty() {
            return Err(format!("&c'{a}' isn't a valid context"));
        }
        c.entry(k.to_ascii_lowercase()).or_default().insert(v.to_ascii_lowercase());
    }
    Ok(c)
}

fn show_contexts(c: &Contexts) -> String {
    if c.is_empty() {
        return "&8(global)".into();
    }
    let parts: Vec<String> = c.iter().flat_map(|(k, vs)| vs.iter().map(move |v| format!("{k}={v}"))).collect();
    format!("&8({})", parts.join(", "))
}

/// "1d12h", "30m", "90s", "2w", or plain seconds (`DurationParser`).
pub fn duration(s: &str) -> Option<i64> {
    if let Ok(n) = s.parse::<i64>() {
        return (n > 0).then_some(n);
    }
    let mut total = 0i64;
    let mut num = String::new();
    for c in s.to_ascii_lowercase().chars() {
        if c.is_ascii_digit() {
            num.push(c);
            continue;
        }
        let n: i64 = num.parse().ok()?;
        num.clear();
        total += n * match c {
            's' => 1,
            'm' => 60,
            'h' => 3600,
            'd' => 86_400,
            'w' => 604_800,
            'y' => 31_536_000,
            _ => return None,
        };
    }
    (num.is_empty() && total > 0).then_some(total)
}

fn left(expiry: i64) -> String {
    let mut s = (expiry - now()).max(0);
    let mut out = String::new();
    for (unit, n) in [("d", 86_400), ("h", 3600), ("m", 60), ("s", 1)] {
        if s >= n || (unit == "s" && out.is_empty()) {
            let _ = write!(out, "{}{unit}", s / n);
            s %= n;
        }
    }
    out
}

fn bool_arg(s: Option<&String>) -> Result<bool, String> {
    match s.map(|s| s.to_ascii_lowercase()) {
        None => Ok(true),
        Some(v) if v == "true" => Ok(true),
        Some(v) if v == "false" => Ok(false),
        Some(v) => Err(format!("&c'{v}' isn't true or false")),
    }
}

const HELP: &[&str] = &[
    "&b/lp user <user> &7info | permission | parent | meta | promote | demote",
    "&b/lp group <group> &7info | permission | parent | meta | setweight | setdisplayname | listmembers | rename | clone",
    "&b/lp track <track> &7info | append | insert | remove | clear | rename | clone",
    "&b/lp creategroup | deletegroup | listgroups &7<name>",
    "&b/lp createtrack | deletetrack | listtracks &7<name>",
    "&b/lp sync | info",
    "&7Contexts go at the end: &fworld=minecraft:the_nether gamemode=creative server=<name>",
];

/// Which holder a `permission`/`parent`/`meta` subcommand edits.
enum Target {
    User(String),
    Group(String),
}

impl Lp {
    fn holder(&mut self, t: &Target) -> Option<&mut Holder> {
        match t {
            Target::User(u) => self.users.get_mut(u).map(|u| &mut u.data),
            Target::Group(g) => self.groups.get_mut(g).map(|g| &mut g.data),
        }
    }

    fn label(&self, t: &Target) -> String {
        match t {
            Target::User(u) => self.users.get(u).map(|u| if u.name.is_empty() { u.uuid.clone() } else { u.name.clone() }).unwrap_or_default(),
            Target::Group(g) => g.clone(),
        }
    }

    fn changed(&mut self, t: &Target) {
        match t {
            Target::User(u) => self.user_changed(u),
            Target::Group(g) => self.group_changed(g),
        }
    }

    pub fn command(&mut self, raw: &str) -> Reply {
        let args = tokenize(raw);
        let a: Vec<&str> = args.iter().map(String::as_str).collect();
        let lower: Vec<String> = a.iter().map(|s| s.to_ascii_lowercase()).collect();
        let l: Vec<&str> = lower.iter().map(String::as_str).collect();
        match l.as_slice() {
            [] | ["help"] => {
                let mut r = vec![line(format!("&bAce LP &7{} &8(LuckPerms for Pumpkin)", env!("CARGO_PKG_VERSION")))];
                r.extend(HELP.iter().map(|s| s.to_string()));
                r
            }
            ["info"] => vec![
                line(format!("&bAce LP &f{}", env!("CARGO_PKG_VERSION"))),
                format!("&7Server context: &f{}", self.server_name),
                "&7Storage: &fJSON &8(json-storage/)".to_string(),
                format!("&7Groups: &f{} &7Tracks: &f{} &7Online users: &f{}", self.groups.len(), self.tracks.len(), self.online.len()),
            ],
            ["sync"] => {
                self.reload();
                vec![line("&aReloaded groups, tracks and online users from storage.")]
            }
            ["creategroup", name, ..] => self.create_group(name, &a[2..]),
            ["deletegroup", name] => self.delete_group(name),
            ["listgroups"] => {
                let mut gs: Vec<&Group> = self.groups.values().collect();
                gs.sort_by(|x, y| y.data.weight().cmp(&x.data.weight()).then(x.name.cmp(&y.name)));
                let mut r = vec![line(format!("&bGroups &7({})", gs.len()))];
                for g in gs {
                    let dn = g.data.display_name().map(|d| format!(" &8({d}&8)")).unwrap_or_default();
                    r.push(format!("&8- &f{}{dn} &7weight {}", g.name, g.data.weight()));
                }
                r
            }
            ["createtrack", name] => {
                if !valid_name(name) {
                    return vec![line("&cTrack names can only use letters, digits, - and _.")];
                }
                if self.tracks.contains_key(*name) {
                    return vec![line(format!("&c{name} already exists."))];
                }
                let t = Track { name: name.to_string(), groups: vec![] };
                self.storage.save_track(&t);
                self.tracks.insert(t.name.clone(), t);
                vec![line(format!("&aCreated track &f{name}&a."))]
            }
            ["deletetrack", name] => {
                if self.tracks.remove(*name).is_none() {
                    return vec![line(format!("&c{name} doesn't exist."))];
                }
                self.storage.delete_track(name);
                vec![line(format!("&aDeleted track &f{name}&a."))]
            }
            ["listtracks"] => {
                let mut r = vec![line(format!("&bTracks &7({})", self.tracks.len()))];
                let mut ts: Vec<&Track> = self.tracks.values().collect();
                ts.sort_by(|x, y| x.name.cmp(&y.name));
                for t in ts {
                    r.push(format!("&8- &f{} &7{}", t.name, t.groups.join(" &8→ &7")));
                }
                r
            }
            ["user", who, rest @ ..] => {
                let Some(uuid) = self.find_user(a[1]) else {
                    return vec![line(format!("&cUser &f{who}&c hasn't joined this server."))];
                };
                let r = self.user_cmd(&uuid, rest, &a[2..]);
                if !self.online.contains_key(&uuid) {
                    self.users.remove(&uuid);
                }
                r
            }
            ["group", name, rest @ ..] => {
                if !self.groups.contains_key(*name) {
                    return vec![line(format!("&cGroup &f{name}&c doesn't exist. Create it with /lp creategroup {name}"))];
                }
                self.group_cmd(name, rest, &a[2..])
            }
            ["track", name, rest @ ..] => {
                if !self.tracks.contains_key(*name) {
                    return vec![line(format!("&cTrack &f{name}&c doesn't exist."))];
                }
                self.track_cmd(name, rest, &a[2..])
            }
            ["editor", ..] | ["applyedits", ..] => vec![line("&eThe web editor arrives in a later Ace LP version. Use the commands for now.")],
            _ => vec![line(format!("&cUnknown command. &7Try &f/lp help"))],
        }
    }

    fn create_group(&mut self, name: &str, extra: &[&str]) -> Reply {
        let name = name.to_ascii_lowercase();
        if !valid_name(&name) {
            return vec![line("&cGroup names can only use letters, digits, - and _.")];
        }
        if self.groups.contains_key(&name) {
            return vec![line(format!("&c{name} already exists."))];
        }
        let mut g = Group { name: name.clone(), data: Holder::default() };
        if let Some(w) = extra.first().and_then(|w| w.parse::<i32>().ok()) {
            g.data.set(Node::new(format!("weight.{w}"), true));
        }
        if let Some(dn) = extra.get(1) {
            g.data.set(Node::new(format!("displayname.{dn}"), true));
        }
        self.storage.save_group(&g);
        self.groups.insert(name.clone(), g);
        vec![line(format!("&aCreated group &f{name}&a."))]
    }

    fn delete_group(&mut self, name: &str) -> Reply {
        let name = name.to_ascii_lowercase();
        if name == model::DEFAULT_GROUP {
            return vec![line("&cThe default group can't be deleted.")];
        }
        if self.groups.remove(&name).is_none() {
            return vec![line(format!("&c{name} doesn't exist."))];
        }
        self.storage.delete_group(&name);
        self.recalculate_all();
        vec![line(format!("&aDeleted group &f{name}&a. Members keep the node but it no longer does anything."))]
    }

    fn user_cmd(&mut self, uuid: &str, l: &[&str], a: &[&str]) -> Reply {
        let t = Target::User(uuid.to_string());
        match l {
            [] | ["info"] => {
                let Some(u) = self.users.get(uuid) else { return vec![] };
                let online = self.online.contains_key(uuid);
                let q = self.online.get(uuid).map(|o| o.contexts.clone()).unwrap_or_default();
                let primary = self.groups().primary_group(u, &q, now());
                let meta = self.user_meta(uuid).unwrap_or_default();
                let mut r = vec![
                    line(format!("&bUser &f{} &8({})", u.name, u.uuid)),
                    format!("&7Status: {}", if online { "&aonline" } else { "&coffline" }),
                    format!("&7Primary group: &f{primary}"),
                    format!("&7Parents: &f{}", u.data.parents().map(|(g, _)| g.to_string()).collect::<Vec<_>>().join(", ")),
                    format!("&7Prefix: &f{} &7Suffix: &f{}", meta.prefix.unwrap_or_else(|| "none".into()), meta.suffix.unwrap_or_else(|| "none".into())),
                ];
                if online {
                    r.push(format!("&7Contexts: {}", show_contexts(&self.online[uuid].contexts)));
                }
                r
            }
            ["permission" | "perm", rest @ ..] => self.permission_cmd(&t, rest, &a[1..]),
            ["parent", rest @ ..] => self.parent_cmd(&t, rest, &a[1..]),
            ["meta", rest @ ..] => self.meta_cmd(&t, rest, &a[1..]),
            ["promote" | "demote", track, ..] => self.promote(uuid, track, l[0] == "promote", &a[2..]),
            ["clear", ..] => {
                let ctx = match contexts(&own(&a[1..])) {
                    Ok(c) => c,
                    Err(e) => return vec![line(e)],
                };
                let n = self.holder(&t).map(|h| h.remove_where(|n| n.contexts == ctx)).unwrap_or(0);
                self.changed(&t);
                vec![line(format!("&aCleared {n} nodes from &f{}&a {}", self.label(&t), show_contexts(&ctx)))]
            }
            _ => vec![line("&cUsage: &f/lp user <user> info | permission | parent | meta | promote <track> | demote <track> | clear")],
        }
    }

    fn group_cmd(&mut self, name: &str, l: &[&str], a: &[&str]) -> Reply {
        let t = Target::Group(name.to_string());
        match l {
            [] | ["info"] => {
                let g = &self.groups[name];
                vec![
                    line(format!("&bGroup &f{}", g.name)),
                    format!("&7Display name: &f{}", g.data.display_name().unwrap_or("none")),
                    format!("&7Weight: &f{}", g.data.weight()),
                    format!("&7Parents: &f{}", g.data.parents().map(|(p, _)| p.to_string()).collect::<Vec<_>>().join(", ")),
                    format!("&7Nodes: &f{}", g.data.nodes.len()),
                ]
            }
            ["permission" | "perm", rest @ ..] => self.permission_cmd(&t, rest, &a[1..]),
            ["parent", rest @ ..] => self.parent_cmd(&t, rest, &a[1..]),
            ["meta", rest @ ..] => self.meta_cmd(&t, rest, &a[1..]),
            ["setweight", w] => {
                let Ok(w) = w.parse::<i32>() else { return vec![line("&cWeight must be a whole number.")] };
                let g = self.groups.get_mut(name).map(|g| &mut g.data);
                if let Some(h) = g {
                    h.remove_where(|n| matches!(n.kind(), Kind::Weight(_)));
                    h.set(Node::new(format!("weight.{w}"), true));
                }
                self.group_changed(name);
                vec![line(format!("&aSet the weight of &f{name}&a to &f{w}&a."))]
            }
            ["setdisplayname", ..] if a.len() >= 2 => {
                let dn = a[1];
                if let Some(g) = self.groups.get_mut(name) {
                    g.data.remove_where(|n| matches!(n.kind(), Kind::DisplayName(_)));
                    if !dn.eq_ignore_ascii_case(name) {
                        g.data.set(Node::new(format!("displayname.{dn}"), true));
                    }
                }
                self.group_changed(name);
                vec![line(format!("&aSet the display name of &f{name}&a to &f{dn}&a."))]
            }
            ["listmembers"] => {
                let m = self.members_of(name);
                let mut r = vec![line(format!("&bMembers of &f{name} &7({})", m.len()))];
                r.extend(m.into_iter().map(|n| format!("&8- &f{n}")));
                r
            }
            ["rename" | "clone", new] => {
                let new = new.to_ascii_lowercase();
                if !valid_name(&new) || self.groups.contains_key(&new) {
                    return vec![line(format!("&c{new} is taken or not a valid name."))];
                }
                if l[0] == "rename" && name == model::DEFAULT_GROUP {
                    return vec![line("&cThe default group can't be renamed. Clone it instead.")];
                }
                let mut g = self.groups[name].clone();
                g.name = new.clone();
                self.storage.save_group(&g);
                self.groups.insert(new.clone(), g);
                if l[0] == "rename" {
                    self.groups.remove(name);
                    self.storage.delete_group(name);
                    self.recalculate_all();
                    return vec![line(format!("&aRenamed &f{name}&a to &f{new}&a. Members still point at {name}; move them with /lp user <user> parent set {new}"))];
                }
                vec![line(format!("&aCloned &f{name}&a to &f{new}&a."))]
            }
            ["clear", ..] => {
                let ctx = match contexts(&own(&a[1..])) {
                    Ok(c) => c,
                    Err(e) => return vec![line(e)],
                };
                let n = self.holder(&t).map(|h| h.remove_where(|n| n.contexts == ctx)).unwrap_or(0);
                self.changed(&t);
                vec![line(format!("&aCleared {n} nodes from &f{name}&a {}", show_contexts(&ctx)))]
            }
            _ => vec![line("&cUsage: &f/lp group <group> info | permission | parent | meta | setweight | setdisplayname | listmembers | rename | clone | clear")],
        }
    }

    fn permission_cmd(&mut self, t: &Target, l: &[&str], a: &[&str]) -> Reply {
        let who = self.label(t);
        match l {
            [] | ["info"] => {
                let Some(h) = self.holder(t) else { return vec![] };
                let mut nodes: Vec<Node> = h.nodes.iter().filter(|n| n.kind() == Kind::Permission).cloned().collect();
                nodes.sort_by(|x, y| x.key.cmp(&y.key));
                let mut r = vec![line(format!("&b{who}&7's permissions &8({})", nodes.len()))];
                for n in nodes {
                    let v = if n.value { "&a" } else { "&c" };
                    let exp = n.expiry.map(|e| format!(" &8(expires in {})", left(e))).unwrap_or_default();
                    r.push(format!("&8- {v}{} {}{exp}", n.key, show_contexts(&n.contexts)));
                }
                r
            }
            ["set", _node, ..] => {
                let has_value = a.get(2).is_some_and(|s| !s.contains('='));
                let value = if has_value {
                    match bool_arg(Some(&a[2].to_string())) {
                        Ok(v) => v,
                        Err(e) => return vec![line(e)],
                    }
                } else {
                    true
                };
                let ctx = match contexts(&own(&a[if has_value { 3 } else { 2 }..])) {
                    Ok(c) => c,
                    Err(e) => return vec![line(e)],
                };
                let key = node_key(a[1]);
                let added = self.holder(t).map(|h| h.set(Node::new(key.clone(), value).with(None, ctx.clone()))).unwrap_or(false);
                if !added {
                    return vec![line(format!("&e{who} already has {key} set to {value} {}", show_contexts(&ctx)))];
                }
                self.changed(t);
                vec![line(format!("&aSet &f{key}&a to &f{value}&a for &f{who}&a {}", show_contexts(&ctx)))]
            }
            ["settemp", _node, val, dur, ..] => {
                let value = match bool_arg(Some(&val.to_string())) {
                    Ok(v) => v,
                    Err(e) => return vec![line(e)],
                };
                let Some(secs) = duration(dur) else { return vec![line(format!("&c'{dur}' isn't a duration. Try 1d, 2h30m or 600."))] };
                let ctx = match contexts(&own(&a[4..])) {
                    Ok(c) => c,
                    Err(e) => return vec![line(e)],
                };
                let key = node_key(a[1]);
                if let Some(h) = self.holder(t) {
                    h.set(Node::new(key.clone(), value).with(Some(now() + secs), ctx.clone()));
                }
                self.changed(t);
                vec![line(format!("&aSet &f{key}&a to &f{value}&a for &f{who}&a for &f{}&a {}", left(now() + secs), show_contexts(&ctx)))]
            }
            ["unset" | "unsettemp", _node, ..] => {
                let ctx = match contexts(&own(&a[2..])) {
                    Ok(c) => c,
                    Err(e) => return vec![line(e)],
                };
                let key = node_key(a[1]);
                let temp = l[0] == "unsettemp";
                let removed = self.holder(t).map(|h| h.unset(&key, &ctx, temp)).unwrap_or(false);
                if !removed {
                    return vec![line(format!("&e{who} doesn't have {key} {}", show_contexts(&ctx)))];
                }
                self.changed(t);
                vec![line(format!("&aUnset &f{key}&a for &f{who}&a {}", show_contexts(&ctx)))]
            }
            ["check", _node] => {
                let key = node_key(a[1]).to_ascii_lowercase();
                let (holder, query, primary) = match t {
                    Target::User(u) => {
                        let Some(user) = self.users.get(u) else { return vec![] };
                        (user.data.clone(), self.query(u), Some(user.primary_group.clone()))
                    }
                    Target::Group(g) => (self.groups[g].data.clone(), Contexts::new(), None),
                };
                let perms = self.groups().permissions(&holder, &query, now(), primary.as_deref());
                let result = match perms.get(&key) {
                    Some(true) => "&atrue".to_string(),
                    Some(false) => "&cfalse".to_string(),
                    None => "&7undefined &8(Pumpkin falls back to wildcards, then the node's default)".to_string(),
                };
                vec![line(format!("&f{key}&7 for &f{who}&7: {result}"))]
            }
            ["clear", ..] => {
                let ctx = match contexts(&own(&a[1..])) {
                    Ok(c) => c,
                    Err(e) => return vec![line(e)],
                };
                let n = self.holder(t).map(|h| h.remove_where(|n| n.kind() == Kind::Permission && n.contexts == ctx)).unwrap_or(0);
                self.changed(t);
                vec![line(format!("&aCleared {n} permissions from &f{who}&a {}", show_contexts(&ctx)))]
            }
            _ => vec![line("&cUsage: &fpermission info | set <node> [true|false] [context...] | settemp <node> <true|false> <duration> | unset <node> | unsettemp <node> | check <node> | clear")],
        }
    }

    fn parent_cmd(&mut self, t: &Target, l: &[&str], a: &[&str]) -> Reply {
        let who = self.label(t);
        match l {
            [] | ["info"] => {
                let Some(h) = self.holder(t) else { return vec![] };
                let parents: Vec<String> = h
                    .parents()
                    .map(|(g, n)| {
                        let exp = n.expiry.map(|e| format!(" &8(expires in {})", left(e))).unwrap_or_default();
                        format!("&8- &f{g} {}{exp}", show_contexts(&n.contexts))
                    })
                    .collect();
                let mut r = vec![line(format!("&b{who}&7's parents"))];
                r.extend(parents);
                r
            }
            ["add" | "set" | "remove" | "addtemp" | "removetemp", group, ..] => {
                let group = group.to_ascii_lowercase();
                if !self.groups.contains_key(&group) && l[0] != "remove" && l[0] != "removetemp" {
                    return vec![line(format!("&cGroup &f{group}&c doesn't exist."))];
                }
                if let Target::Group(g) = t
                    && *g == group
                {
                    return vec![line("&cA group can't inherit from itself.")];
                }
                let key = format!("group.{group}");
                let (expiry, ctx_from) = if l[0] == "addtemp" {
                    let Some(secs) = a.get(2).and_then(|d| duration(d)) else { return vec![line("&cUsage: &fparent addtemp <group> <duration> [context...]")] };
                    (Some(now() + secs), 3)
                } else {
                    (None, 2)
                };
                let ctx = match contexts(&own(&a[ctx_from.min(a.len())..])) {
                    Ok(c) => c,
                    Err(e) => return vec![line(e)],
                };
                let verb = l[0];
                let Some(h) = self.holder(t) else { return vec![] };
                let msg = match verb {
                    "set" => {
                        h.remove_where(|n| matches!(n.kind(), Kind::Inheritance(_)) && n.contexts == ctx);
                        h.set(Node::new(key, true).with(None, ctx.clone()));
                        format!("&f{who}&a now only inherits &f{group}&a {}", show_contexts(&ctx))
                    }
                    "remove" | "removetemp" => {
                        if !h.unset(&key, &ctx, verb == "removetemp") {
                            return vec![line(format!("&e{who} doesn't inherit {group} {}", show_contexts(&ctx)))];
                        }
                        format!("&f{who}&a no longer inherits &f{group}&a {}", show_contexts(&ctx))
                    }
                    _ => {
                        if !h.set(Node::new(key, true).with(expiry, ctx.clone())) {
                            return vec![line(format!("&e{who} already inherits {group} {}", show_contexts(&ctx)))];
                        }
                        let dur = expiry.map(|e| format!(" for {}", left(e))).unwrap_or_default();
                        format!("&f{who}&a now inherits &f{group}&a{dur} {}", show_contexts(&ctx))
                    }
                };
                if let Target::User(u) = t
                    && verb == "set"
                    && let Some(user) = self.users.get_mut(u)
                {
                    user.primary_group = group.clone();
                }
                self.changed(t);
                vec![line(msg)]
            }
            ["switchprimarygroup", group] => {
                let Target::User(u) = t else { return vec![line("&cOnly users have a primary group.")] };
                let group = group.to_ascii_lowercase();
                let Some(user) = self.users.get_mut(u) else { return vec![] };
                if !user.data.parents().any(|(g, _)| g.eq_ignore_ascii_case(&group)) {
                    return vec![line(format!("&c{who} doesn't inherit {group}. Add it first."))];
                }
                user.primary_group = group.clone();
                self.changed(t);
                vec![line(format!("&aSet &f{who}&a's primary group to &f{group}&a."))]
            }
            ["clear", ..] => {
                let ctx = match contexts(&own(&a[1..])) {
                    Ok(c) => c,
                    Err(e) => return vec![line(e)],
                };
                let n = self.holder(t).map(|h| h.remove_where(|n| matches!(n.kind(), Kind::Inheritance(_)) && n.contexts == ctx)).unwrap_or(0);
                self.changed(t);
                vec![line(format!("&aCleared {n} parents from &f{who}&a {}", show_contexts(&ctx)))]
            }
            _ => vec![line("&cUsage: &fparent info | set <group> | add <group> | remove <group> | addtemp <group> <duration> | removetemp <group> | switchprimarygroup <group> | clear")],
        }
    }

    fn meta_cmd(&mut self, t: &Target, l: &[&str], a: &[&str]) -> Reply {
        let who = self.label(t);
        match l {
            [] | ["info"] => {
                let Some(h) = self.holder(t) else { return vec![] };
                let mut r = vec![line(format!("&b{who}&7's meta"))];
                for n in &h.nodes {
                    let ctx = show_contexts(&n.contexts);
                    match n.kind() {
                        Kind::Prefix(p, x) => r.push(format!("&8- &7prefix &f{p} &r{x}&r {ctx}")),
                        Kind::Suffix(p, x) => r.push(format!("&8- &7suffix &f{p} &r{x}&r {ctx}")),
                        Kind::Meta(k, v) => r.push(format!("&8- &7{k} = &f{v} {ctx}")),
                        _ => {}
                    }
                }
                if let Target::User(u) = t
                    && let Some(m) = self.user_meta(u)
                {
                    r.push(format!("&7Effective prefix: &r{}&r &7suffix: &r{}", m.prefix.unwrap_or_default(), m.suffix.unwrap_or_default()));
                }
                r
            }
            ["set", _key, _value, ..] => {
                let (k, v) = (a[1].to_ascii_lowercase(), a[2].to_string());
                let ctx = match contexts(&own(&a[3..])) {
                    Ok(c) => c,
                    Err(e) => return vec![line(e)],
                };
                if let Some(h) = self.holder(t) {
                    h.remove_where(|n| matches!(n.kind(), Kind::Meta(mk, _) if mk.eq_ignore_ascii_case(&k)) && n.contexts == ctx);
                    h.set(Node::new(format!("meta.{k}.{v}"), true).with(None, ctx.clone()));
                }
                self.changed(t);
                vec![line(format!("&aSet meta &f{k}&a to &f{v}&a for &f{who}&a {}", show_contexts(&ctx)))]
            }
            ["unset", _key, ..] => {
                let k = a[1].to_ascii_lowercase();
                let ctx = match contexts(&own(&a[2..])) {
                    Ok(c) => c,
                    Err(e) => return vec![line(e)],
                };
                let n = self.holder(t).map(|h| h.remove_where(|n| matches!(n.kind(), Kind::Meta(mk, _) if mk.eq_ignore_ascii_case(&k)) && n.contexts == ctx)).unwrap_or(0);
                self.changed(t);
                vec![line(format!("&aRemoved {n} &f{k}&a meta from &f{who}&a"))]
            }
            [verb @ ("setprefix" | "addprefix" | "setsuffix" | "addsuffix"), ..] if a.len() >= 2 => {
                let is_prefix = verb.ends_with("prefix");
                let kind = if is_prefix { "prefix" } else { "suffix" };
                // "[priority] <text> [context...]"
                let (priority, text_at) = match a[1].parse::<i32>() {
                    Ok(p) if a.len() >= 3 => (Some(p), 2),
                    _ => (None, 1),
                };
                if priority.is_none() && verb.starts_with("add") {
                    return vec![line(format!("&cUsage: &fmeta {verb} <priority> <{kind}> [context...]"))];
                }
                let text = a[text_at].to_string();
                let ctx = match contexts(&own(&a[text_at + 1..])) {
                    Ok(c) => c,
                    Err(e) => return vec![line(e)],
                };
                let Some(h) = self.holder(t) else { return vec![] };
                let own_max = h
                    .nodes
                    .iter()
                    .filter_map(|n| match n.kind() {
                        Kind::Prefix(p, _) if is_prefix => Some(p),
                        Kind::Suffix(p, _) if !is_prefix => Some(p),
                        _ => None,
                    })
                    .max()
                    .unwrap_or(0);
                if verb.starts_with("set") {
                    h.remove_where(|n| n.contexts == ctx && matches!((n.kind(), is_prefix), (Kind::Prefix(..), true) | (Kind::Suffix(..), false)));
                }
                let p = priority.unwrap_or(own_max + 1);
                h.set(Node::new(format!("{kind}.{p}.{text}"), true).with(None, ctx.clone()));
                self.changed(t);
                vec![line(format!("&aSet {kind} &r{text}&r&a (priority {p}) for &f{who}&a {}", show_contexts(&ctx)))]
            }
            [verb @ ("removeprefix" | "removesuffix"), _p, ..] => {
                let is_prefix = verb.ends_with("prefix");
                let Ok(p) = a[1].parse::<i32>() else { return vec![line(format!("&cUsage: &fmeta {verb} <priority> [text]"))] };
                let text = a.get(2).map(|s| s.to_string());
                let n = self
                    .holder(t)
                    .map(|h| {
                        h.remove_where(|n| match (n.kind(), is_prefix) {
                            (Kind::Prefix(np, nt), true) | (Kind::Suffix(np, nt), false) => np == p && text.as_deref().is_none_or(|x| x == nt),
                            _ => false,
                        })
                    })
                    .unwrap_or(0);
                self.changed(t);
                vec![line(format!("&aRemoved {n} {} from &f{who}&a", if is_prefix { "prefixes" } else { "suffixes" }))]
            }
            ["clear", ..] => {
                let n = self.holder(t).map(|h| h.remove_where(|n| matches!(n.kind(), Kind::Prefix(..) | Kind::Suffix(..) | Kind::Meta(..)))).unwrap_or(0);
                self.changed(t);
                vec![line(format!("&aCleared {n} meta nodes from &f{who}&a"))]
            }
            _ => vec![line("&cUsage: &fmeta info | set <key> <value> | unset <key> | setprefix [priority] <prefix> | addprefix <priority> <prefix> | removeprefix <priority> [prefix] | (same for suffix) | clear")],
        }
    }

    /// `/lp user <u> promote|demote <track>` (`TrackPromote`/`TrackDemote`).
    fn promote(&mut self, uuid: &str, track: &str, up: bool, ctx_args: &[&str]) -> Reply {
        let Some(tr) = self.tracks.get(track).cloned() else { return vec![line(format!("&cTrack &f{track}&c doesn't exist."))] };
        if tr.groups.is_empty() {
            return vec![line(format!("&cTrack &f{track}&c has no groups."))];
        }
        let ctx = match contexts(&own(ctx_args)) {
            Ok(c) => c,
            Err(e) => return vec![line(e)],
        };
        let Some(user) = self.users.get_mut(uuid) else { return vec![] };
        let name = user.name.clone();
        let on: Vec<String> = user
            .data
            .parents()
            .filter(|(g, n)| n.contexts == ctx && tr.groups.iter().any(|t| t.eq_ignore_ascii_case(g)))
            .map(|(g, _)| g.to_ascii_lowercase())
            .collect();
        if on.len() > 1 {
            return vec![line(format!("&c{name} is in more than one group on {track}: {}. Fix that first.", on.join(", ")))];
        }
        let current = on.first().and_then(|g| tr.groups.iter().position(|t| t == g));
        let next = match (current, up) {
            (None, true) => Some(0),
            (None, false) => return vec![line(format!("&c{name} isn't on track {track}."))],
            (Some(i), true) if i + 1 >= tr.groups.len() => return vec![line(format!("&e{name} is already at the top of {track}."))],
            (Some(i), true) => Some(i + 1),
            (Some(0), false) => None,
            (Some(i), false) => Some(i - 1),
        };
        if let Some(i) = current {
            user.data.unset(&format!("group.{}", tr.groups[i]), &ctx, false);
        }
        let msg = match next {
            Some(i) => {
                let g = tr.groups[i].clone();
                user.data.set(Node::new(format!("group.{g}"), true).with(None, ctx.clone()));
                if ctx.is_empty() {
                    user.primary_group = g.clone();
                }
                let from = current.map(|c| tr.groups[c].clone()).unwrap_or_else(|| "nothing".into());
                format!("&a{} &f{name}&a on &f{track}&a: &f{from}&a → &f{g}&a {}", if up { "Promoted" } else { "Demoted" }, show_contexts(&ctx))
            }
            None => format!("&aDemoted &f{name}&a off the bottom of &f{track}&a {}", show_contexts(&ctx)),
        };
        self.user_changed(uuid);
        vec![line(msg)]
    }

    fn track_cmd(&mut self, name: &str, l: &[&str], a: &[&str]) -> Reply {
        let save = |lp: &mut Lp| {
            if let Some(t) = lp.tracks.get(name) {
                lp.storage.save_track(t);
            }
        };
        match l {
            [] | ["info"] => {
                let t = &self.tracks[name];
                vec![line(format!("&bTrack &f{}", t.name)), format!("&7{}", if t.groups.is_empty() { "empty".to_string() } else { t.groups.join(" &8→ &7") })]
            }
            ["append" | "insert", group, ..] => {
                let group = group.to_ascii_lowercase();
                if !self.groups.contains_key(&group) {
                    return vec![line(format!("&cGroup &f{group}&c doesn't exist."))];
                }
                let Some(t) = self.tracks.get_mut(name) else { return vec![] };
                if t.groups.contains(&group) {
                    return vec![line(format!("&e{group} is already on {name}."))];
                }
                if l[0] == "insert" {
                    let Some(pos) = a.get(2).and_then(|p| p.parse::<usize>().ok()).filter(|p| *p >= 1 && *p <= t.groups.len() + 1) else {
                        return vec![line("&cUsage: &f/lp track <track> insert <group> <position> &7(1 = first)")];
                    };
                    t.groups.insert(pos - 1, group.clone());
                } else {
                    t.groups.push(group.clone());
                }
                save(self);
                vec![line(format!("&aAdded &f{group}&a to &f{name}&a."))]
            }
            ["remove", group] => {
                let group = group.to_ascii_lowercase();
                let Some(t) = self.tracks.get_mut(name) else { return vec![] };
                let before = t.groups.len();
                t.groups.retain(|g| *g != group);
                if t.groups.len() == before {
                    return vec![line(format!("&e{group} isn't on {name}."))];
                }
                save(self);
                vec![line(format!("&aRemoved &f{group}&a from &f{name}&a."))]
            }
            ["clear"] => {
                if let Some(t) = self.tracks.get_mut(name) {
                    t.groups.clear();
                }
                save(self);
                vec![line(format!("&aCleared &f{name}&a."))]
            }
            ["rename" | "clone", new] => {
                let new = new.to_ascii_lowercase();
                if !valid_name(&new) || self.tracks.contains_key(&new) {
                    return vec![line(format!("&c{new} is taken or not a valid name."))];
                }
                let mut t = self.tracks[name].clone();
                t.name = new.clone();
                self.storage.save_track(&t);
                self.tracks.insert(new.clone(), t);
                if l[0] == "rename" {
                    self.tracks.remove(name);
                    self.storage.delete_track(name);
                }
                vec![line(format!("&a{} &f{name}&a to &f{new}&a.", if l[0] == "rename" { "Renamed" } else { "Cloned" }))]
            }
            _ => vec![line("&cUsage: &f/lp track <track> info | append <group> | insert <group> <position> | remove <group> | clear | rename | clone")],
        }
    }
}

fn own(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

/// Pumpkin nodes are `namespace:path`. Keep what the admin typed; LuckPerms keys are case-insensitive.
fn node_key(s: &str) -> String {
    s.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_quotes() {
        assert_eq!(tokenize(r#"group admin meta setprefix 100 "&c[Admin] " world=nether"#), [
            "group", "admin", "meta", "setprefix", "100", "&c[Admin] ", "world=nether"
        ]);
    }

    #[test]
    fn durations() {
        assert_eq!(duration("1d2h"), Some(93_600));
        assert_eq!(duration("30m"), Some(1800));
        assert_eq!(duration("90"), Some(90));
        assert_eq!(duration("abc"), None);
        assert_eq!(duration("5x"), None);
    }

    #[test]
    fn end_to_end() {
        let dir = std::env::temp_dir().join(format!("ace-lp-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut lp = Lp::load(&dir, "lobby".into());
        lp.join("00000000-0000-0000-0000-000000000001", "Steve", Contexts::new());
        lp.actions.clear();
        lp.command("creategroup admin 100");
        lp.command("group admin permission set minecraft:command.gamemode");
        lp.command("user Steve parent add admin");
        // Steve is online, so the change is queued for Pumpkin
        let applied = &lp.online["00000000-0000-0000-0000-000000000001"].applied;
        assert_eq!(applied.get("minecraft:command.gamemode"), Some(&true));
        assert_eq!(applied.get("group.admin"), Some(&true));
        lp.command("createtrack staff");
        lp.command("creategroup mod 50");
        lp.command("track staff append mod");
        lp.command("track staff append admin");
        let r = lp.command("user Steve demote staff");
        assert!(r[0].contains("admin") && r[0].contains("mod"), "{r:?}");
        // reloading from disk keeps everything
        let mut again = Lp::load(&dir, "lobby".into());
        assert!(again.groups.contains_key("admin") && again.tracks["staff"].groups == ["mod", "admin"]);
        assert!(again.find_user("steve").is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
