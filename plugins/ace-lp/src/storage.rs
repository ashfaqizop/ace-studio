//! Flat-file storage in LuckPerms' own JSON layout (`SeparatedConfigurateStorage` with the JSON loader), so a
//! `plugins/LuckPerms/json-storage` folder can be copied in as is: `users/<uuid>.json`, `groups/<name>.json`,
//! `tracks/<name>.json`, plus `uuidcache.json` (name → uuid of players seen here).

use crate::model::{Contexts, Group, Holder, Node, Track, User};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct Storage {
    root: PathBuf,
}

fn read_json(path: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str(&text) {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!("Skipping unreadable {}: {e}", path.display());
            None
        }
    }
}

fn write_json(path: &Path, v: &Value) {
    let text = serde_json::to_string_pretty(v).unwrap_or_default();
    let tmp = path.with_extension("json.tmp");
    let ok = std::fs::write(&tmp, text).and_then(|_| std::fs::rename(&tmp, path));
    if let Err(e) = ok {
        tracing::warn!("Couldn't write {}: {e}", path.display());
    }
}

/// Files are named after the lowercase id; never let a name walk out of its folder.
fn safe(name: &str) -> Option<String> {
    let n = name.to_ascii_lowercase();
    (!n.is_empty() && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')).then_some(n)
}

impl Storage {
    pub fn open(data: &Path) -> Self {
        let root = data.join("json-storage");
        for d in ["users", "groups", "tracks"] {
            let _ = std::fs::create_dir_all(root.join(d));
        }
        Storage { root }
    }

    fn list(&self, dir: &str) -> Vec<PathBuf> {
        let Ok(rd) = std::fs::read_dir(self.root.join(dir)) else { return vec![] };
        let mut v: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")).collect();
        v.sort();
        v
    }

    pub fn load_groups(&self) -> Vec<Group> {
        self.list("groups")
            .iter()
            .filter_map(|p| {
                let v = read_json(p)?;
                let stem = p.file_stem()?.to_string_lossy().to_string();
                let name = v.get("name").and_then(Value::as_str).unwrap_or(&stem).to_ascii_lowercase();
                Some(Group { name, data: read_nodes(&v) })
            })
            .collect()
    }

    pub fn load_tracks(&self) -> Vec<Track> {
        self.list("tracks")
            .iter()
            .filter_map(|p| {
                let v = read_json(p)?;
                let stem = p.file_stem()?.to_string_lossy().to_string();
                let name = v.get("name").and_then(Value::as_str).unwrap_or(&stem).to_ascii_lowercase();
                let groups = v
                    .get("groups")
                    .and_then(Value::as_array)
                    .map(|a| a.iter().filter_map(Value::as_str).map(str::to_ascii_lowercase).collect())
                    .unwrap_or_default();
                Some(Track { name, groups })
            })
            .collect()
    }

    pub fn load_user(&self, uuid: &str) -> Option<User> {
        let v = read_json(&self.root.join("users").join(format!("{}.json", safe(uuid)?)))?;
        let mut u = User {
            uuid: uuid.to_string(),
            name: v.get("name").and_then(Value::as_str).unwrap_or("").to_string(),
            primary_group: v.get("primaryGroup").and_then(Value::as_str).unwrap_or(crate::model::DEFAULT_GROUP).to_ascii_lowercase(),
            data: read_nodes(&v),
        };
        u.give_default_if_needed();
        Some(u)
    }

    /// Every stored user (for `/lp group <g> listmembers` and offline lookups).
    pub fn all_users(&self) -> Vec<User> {
        self.list("users")
            .iter()
            .filter_map(|p| self.load_user(&p.file_stem()?.to_string_lossy()))
            .collect()
    }

    pub fn save_user(&self, u: &User) {
        let Some(id) = safe(&u.uuid) else { return };
        let path = self.root.join("users").join(format!("{id}.json"));
        if u.is_default() {
            // like LuckPerms: users with nothing but the default group don't need a file
            let _ = std::fs::remove_file(&path);
            return;
        }
        let mut v = Map::new();
        v.insert("uuid".into(), json!(u.uuid));
        v.insert("name".into(), json!(u.name));
        v.insert("primaryGroup".into(), json!(u.primary_group));
        write_nodes(&mut v, &u.data);
        write_json(&path, &Value::Object(v));
    }

    pub fn save_group(&self, g: &Group) {
        let Some(id) = safe(&g.name) else { return };
        let mut v = Map::new();
        v.insert("name".into(), json!(g.name));
        write_nodes(&mut v, &g.data);
        write_json(&self.root.join("groups").join(format!("{id}.json")), &Value::Object(v));
    }

    pub fn delete_group(&self, name: &str) {
        if let Some(id) = safe(name) {
            let _ = std::fs::remove_file(self.root.join("groups").join(format!("{id}.json")));
        }
    }

    pub fn save_track(&self, t: &Track) {
        let Some(id) = safe(&t.name) else { return };
        write_json(&self.root.join("tracks").join(format!("{id}.json")), &json!({ "name": t.name, "groups": t.groups }));
    }

    pub fn delete_track(&self, name: &str) {
        if let Some(id) = safe(name) {
            let _ = std::fs::remove_file(self.root.join("tracks").join(format!("{id}.json")));
        }
    }

    pub fn load_uuid_cache(&self) -> BTreeMap<String, String> {
        read_json(&self.root.join("uuidcache.json"))
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default()
    }

    pub fn save_uuid_cache(&self, cache: &BTreeMap<String, String>) {
        write_json(&self.root.join("uuidcache.json"), &json!(cache));
    }
}

fn read_contexts(attrs: &Value) -> Contexts {
    let mut c = Contexts::new();
    if let Some(map) = attrs.get("context").and_then(Value::as_object) {
        for (k, v) in map {
            let vals: Vec<String> = match v {
                Value::String(s) => vec![s.clone()],
                Value::Array(a) => a.iter().filter_map(Value::as_str).map(String::from).collect(),
                _ => vec![],
            };
            for val in vals {
                c.entry(k.to_ascii_lowercase()).or_default().insert(val.to_ascii_lowercase());
            }
        }
    }
    // legacy top-level "server"/"world" attributes; "global" means no context
    for key in ["server", "world"] {
        if let Some(s) = attrs.get(key).and_then(Value::as_str)
            && !s.eq_ignore_ascii_case("global")
        {
            c.entry(key.into()).or_default().insert(s.to_ascii_lowercase());
        }
    }
    c
}

fn attrs_node(key: String, value: bool, attrs: &Value) -> Node {
    let expiry = attrs.get("expiry").and_then(Value::as_i64).filter(|e| *e != 0);
    Node::new(key, value).with(expiry, read_contexts(attrs))
}

/// `parseNode`: either `{ "<field>": key, ...attrs }` or the YAML-style `{ key: { ...attrs } }`.
fn entry<'a>(v: &'a Value, field: &str) -> Option<(String, &'a Value)> {
    let obj = v.as_object()?;
    if obj.len() == 1
        && let Some((k, attrs)) = obj.iter().next()
        && k != field
        && !k.is_empty()
    {
        return Some((k.clone(), attrs));
    }
    obj.get(field).and_then(Value::as_str).map(|k| (k.to_string(), v))
}

fn read_nodes(v: &Value) -> Holder {
    let mut h = Holder::default();
    let list = |name: &str| v.get(name).and_then(Value::as_array).cloned().unwrap_or_default();
    for item in list("permissions") {
        if let Value::String(s) = &item {
            h.set(Node::new(s.clone(), true));
        } else if let Some((key, attrs)) = entry(&item, "permission") {
            let value = attrs.get("value").and_then(Value::as_bool).unwrap_or(true);
            h.set(attrs_node(key, value, attrs));
        }
    }
    for item in list("parents") {
        if let Value::String(s) = &item {
            h.set(Node::new(format!("group.{}", s.to_ascii_lowercase()), true));
        } else if let Some((group, attrs)) = entry(&item, "group") {
            h.set(attrs_node(format!("group.{}", group.to_ascii_lowercase()), true, attrs));
        }
    }
    for (section, kind) in [("prefixes", "prefix"), ("suffixes", "suffix")] {
        for item in list(section) {
            if let Some((text, attrs)) = entry(&item, kind) {
                let priority = attrs.get("priority").and_then(Value::as_i64).unwrap_or(0);
                h.set(attrs_node(format!("{kind}.{priority}.{text}"), true, attrs));
            }
        }
    }
    for item in list("meta") {
        if let Some((key, attrs)) = entry(&item, "key") {
            let value = match attrs.get("value") {
                Some(Value::String(s)) => s.clone(),
                Some(other) => other.to_string(),
                None => "null".into(),
            };
            h.set(attrs_node(format!("meta.{key}.{value}"), true, attrs));
        }
    }
    h
}

fn attrs_of(n: &Node, write_value: bool) -> Map<String, Value> {
    let mut a = Map::new();
    if write_value {
        a.insert("value".into(), json!(n.value));
    }
    if let Some(e) = n.expiry {
        a.insert("expiry".into(), json!(e));
    }
    if !n.contexts.is_empty() {
        let mut c = Map::new();
        for (k, vals) in &n.contexts {
            let vals: Vec<&String> = vals.iter().collect();
            c.insert(k.clone(), if vals.len() == 1 { json!(vals[0]) } else { json!(vals) });
        }
        a.insert("context".into(), Value::Object(c));
    }
    a
}

fn write_nodes(to: &mut Map<String, Value>, h: &Holder) {
    use crate::model::Kind;
    let (mut perms, mut parents, mut prefixes, mut suffixes, mut meta) = (vec![], vec![], vec![], vec![], vec![]);
    for n in &h.nodes {
        let push = |list: &mut Vec<Value>, field: &str, key: &str, mut attrs: Map<String, Value>| {
            attrs.insert(field.into(), json!(key));
            list.push(Value::Object(attrs));
        };
        match n.kind() {
            Kind::Prefix(p, text) if n.value => {
                let mut a = attrs_of(n, false);
                a.insert("priority".into(), json!(p));
                push(&mut prefixes, "prefix", text, a);
            }
            Kind::Suffix(p, text) if n.value => {
                let mut a = attrs_of(n, false);
                a.insert("priority".into(), json!(p));
                push(&mut suffixes, "suffix", text, a);
            }
            Kind::Meta(k, v) if n.value => {
                let mut a = attrs_of(n, false);
                a.insert("value".into(), json!(v));
                push(&mut meta, "key", k, a);
            }
            Kind::Inheritance(g) if n.value => push(&mut parents, "group", g, attrs_of(n, false)),
            _ => push(&mut perms, "permission", &n.key, attrs_of(n, true)),
        }
    }
    to.insert("permissions".into(), Value::Array(perms));
    for (name, list) in [("parents", parents), ("prefixes", prefixes), ("suffixes", suffixes), ("meta", meta)] {
        if !list.is_empty() {
            to.insert(name.into(), Value::Array(list));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_luckperms_json() {
        let v: Value = serde_json::from_str(
            r#"{
              "name": "admin",
              "permissions": [
                { "permission": "minecraft:command.gamemode", "value": true },
                { "permission": "essentials:fly", "value": false, "context": { "world": "nether" } },
                { "permission": "weight.100", "value": true }
              ],
              "parents": [ { "group": "mod", "expiry": 1999999999 } ],
              "prefixes": [ { "prefix": "&c[Admin] ", "priority": 100 } ],
              "meta": [ { "key": "homes", "value": "5" } ]
            }"#,
        )
        .unwrap();
        let h = read_nodes(&v);
        assert_eq!(h.nodes.len(), 6);
        assert_eq!(h.weight(), 100);
        assert!(h.nodes.iter().any(|n| n.key == "prefix.100.&c[Admin] "));
        assert!(h.nodes.iter().any(|n| n.key == "group.mod" && n.expiry == Some(1_999_999_999)));
        let fly = h.nodes.iter().find(|n| n.key == "essentials:fly").unwrap();
        assert!(!fly.value && fly.contexts["world"].contains("nether"));

        // and back
        let mut out = Map::new();
        write_nodes(&mut out, &h);
        let again = read_nodes(&Value::Object(out));
        assert_eq!(again.nodes.len(), h.nodes.len());
        for n in &h.nodes {
            assert!(again.nodes.contains(n), "{n:?}");
        }
    }
}
