//! Turning a user's nodes into the effective permission map and meta, the way LuckPerms does:
//! depth-first pre-order traversal of the inheritance graph (`InheritanceGraph`), parents ordered by group weight
//! with the primary group winning ties (`InheritanceComparator`), and within each holder the more specific contexts
//! first, then temporary before permanent (`NodeMap` + `ContextSetComparator` + `NodeComparator`). The first node
//! seen for a key wins (`processExportedPermissions`).

use crate::model::{Contexts, Group, Holder, Kind, Node, User, context_rank};
use std::collections::{BTreeMap, HashMap, HashSet};

pub struct Groups<'a>(pub &'a HashMap<String, Group>);

/// A holder's applicable nodes in LuckPerms priority order.
fn ordered<'h>(h: &'h Holder, query: &Contexts, now: i64) -> Vec<&'h Node> {
    let mut v: Vec<&Node> = h.nodes.iter().filter(|n| !n.expired(now) && n.applies(query)).collect();
    // descending: specific contexts first, then temporary, then (for equal keys) false before true
    v.sort_by(|a, b| {
        context_rank(&b.contexts)
            .cmp(&context_rank(&a.contexts))
            .then(b.expiry.is_some().cmp(&a.expiry.is_some()))
            .then(a.value.cmp(&b.value))
    });
    v
}

impl Groups<'_> {
    /// Parents of a holder that apply in `query`, sorted strongest first.
    fn parents(&self, h: &Holder, query: &Contexts, now: i64, primary: Option<&str>) -> Vec<&Group> {
        let mut seen = HashSet::new();
        let mut out: Vec<&Group> = vec![];
        for n in ordered(h, query, now) {
            if let Kind::Inheritance(name) = n.kind()
                && n.value
                && let Some(g) = self.0.get(&name.to_ascii_lowercase())
                && seen.insert(g.name.clone())
            {
                out.push(g);
            }
        }
        out.sort_by(|a, b| {
            b.data.weight().cmp(&a.data.weight()).then_with(|| {
                let pa = primary.is_some_and(|p| p.eq_ignore_ascii_case(&a.name));
                let pb = primary.is_some_and(|p| p.eq_ignore_ascii_case(&b.name));
                pb.cmp(&pa)
            })
        });
        out
    }

    /// Every holder in traversal order: the start holder, then groups depth-first, each visited once.
    pub fn traverse<'a>(&'a self, start: &'a Holder, query: &Contexts, now: i64, primary: Option<&str>) -> Vec<(&'a Holder, Option<&'a str>)> {
        let mut out = vec![(start, None)];
        let mut visited = HashSet::new();
        let mut stack: Vec<&Group> = self.parents(start, query, now, primary).into_iter().rev().collect();
        while let Some(g) = stack.pop() {
            if !visited.insert(g.name.clone()) {
                continue;
            }
            out.push((&g.data, Some(g.name.as_str())));
            for p in self.parents(&g.data, query, now, primary).into_iter().rev() {
                if !visited.contains(&p.name) {
                    stack.push(p);
                }
            }
        }
        out
    }

    /// The flattened permissions: what Pumpkin should see for this player right now. Inheritance nodes are
    /// included (`group.<name>`), as LuckPerms exposes them to permission checks too. Meta nodes are not.
    pub fn permissions(&self, start: &Holder, query: &Contexts, now: i64, primary: Option<&str>) -> BTreeMap<String, bool> {
        let mut map: BTreeMap<String, bool> = BTreeMap::new();
        for (h, _) in self.traverse(start, query, now, primary) {
            for n in ordered(h, query, now) {
                match n.kind() {
                    Kind::Permission | Kind::Inheritance(_) => {
                        map.entry(n.key.to_ascii_lowercase()).or_insert(n.value);
                    }
                    _ => {}
                }
            }
        }
        map
    }

    /// `MetaAccumulator`: highest-priority prefix and suffix, first value per meta key, highest weight.
    pub fn meta(&self, start: &Holder, query: &Contexts, now: i64, primary: Option<&str>) -> Meta {
        let mut m = Meta::default();
        let mut prefixes: BTreeMap<i32, String> = BTreeMap::new();
        let mut suffixes: BTreeMap<i32, String> = BTreeMap::new();
        for (h, _) in self.traverse(start, query, now, primary) {
            for n in ordered(h, query, now) {
                if !n.value {
                    continue;
                }
                match n.kind() {
                    Kind::Prefix(p, t) => {
                        prefixes.entry(p).or_insert_with(|| t.to_string());
                    }
                    Kind::Suffix(p, t) => {
                        suffixes.entry(p).or_insert_with(|| t.to_string());
                    }
                    Kind::Meta(k, v) => {
                        m.meta.entry(k.to_ascii_lowercase()).or_insert_with(|| v.to_string());
                    }
                    _ => {}
                }
            }
        }
        m.prefix = prefixes.into_iter().next_back().map(|(_, t)| t);
        m.suffix = suffixes.into_iter().next_back().map(|(_, t)| t);
        m
    }

    /// Groups the holder inherits from, in traversal order (without the holder itself).
    pub fn inherited_groups(&self, start: &Holder, query: &Contexts, now: i64, primary: Option<&str>) -> Vec<String> {
        self.traverse(start, query, now, primary).into_iter().filter_map(|(_, n)| n.map(String::from)).collect()
    }

    /// `parents-by-weight`: the user's direct parent group with the highest weight.
    pub fn primary_group(&self, u: &User, query: &Contexts, now: i64) -> String {
        self.parents(&u.data, query, now, Some(&u.primary_group))
            .first()
            .map(|g| g.name.clone())
            .unwrap_or_else(|| u.primary_group.clone())
    }
}

#[derive(Default, Debug, Clone)]
pub struct Meta {
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub meta: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Node;

    fn group(name: &str, nodes: &[(&str, bool)]) -> Group {
        let mut data = Holder::default();
        for (k, v) in nodes {
            data.set(Node::new(*k, *v));
        }
        Group { name: name.into(), data }
    }

    fn world(w: &str) -> Contexts {
        let mut c = Contexts::new();
        c.entry("world".into()).or_default().insert(w.into());
        c
    }

    #[test]
    fn inheritance_and_weights() {
        let mut groups = HashMap::new();
        groups.insert("default".into(), group("default", &[("a:build", true), ("a:fly", false), ("prefix.10.[Member] ", true)]));
        groups.insert("vip".into(), group("vip", &[("weight.20", true), ("group.default", true), ("a:fly", true), ("prefix.20.[VIP] ", true)]));
        groups.insert("muted".into(), group("muted", &[("weight.5", true), ("a:fly", false)]));
        let g = Groups(&groups);
        let mut u = User::new("u", "Steve");
        u.data.set(Node::new("group.vip", true));
        u.data.set(Node::new("group.muted", true));
        let q = Contexts::new();
        let perms = g.permissions(&u.data, &q, 0, Some("default"));
        assert_eq!(perms.get("a:fly"), Some(&true), "vip (weight 20) beats muted (5) and default");
        assert_eq!(perms.get("a:build"), Some(&true));
        assert_eq!(perms.get("group.vip"), Some(&true));
        let meta = g.meta(&u.data, &q, 0, Some("default"));
        assert_eq!(meta.prefix.as_deref(), Some("[VIP] "));
        assert_eq!(g.primary_group(&u, &q, 0), "vip");
    }

    #[test]
    fn own_nodes_and_contexts_win() {
        let mut groups = HashMap::new();
        groups.insert("default".into(), group("default", &[("a:fly", true)]));
        let g = Groups(&groups);
        let mut u = User::new("u", "Steve");
        u.data.set(Node::new("a:fly", false).with(None, world("nether")));
        assert_eq!(g.permissions(&u.data, &world("overworld"), 0, None).get("a:fly"), Some(&true));
        assert_eq!(g.permissions(&u.data, &world("nether"), 0, None).get("a:fly"), Some(&false));
        // expired nodes are ignored
        u.data.set(Node::new("a:dig", true).with(Some(100), Contexts::new()));
        assert_eq!(g.permissions(&u.data, &Contexts::new(), 50, None).get("a:dig"), Some(&true));
        assert_eq!(g.permissions(&u.data, &Contexts::new(), 150, None).get("a:dig"), None);
    }

    #[test]
    fn cycles_terminate() {
        let mut groups = HashMap::new();
        groups.insert("a".into(), group("a", &[("group.b", true), ("x:1", true)]));
        groups.insert("b".into(), group("b", &[("group.a", true), ("x:2", true)]));
        let g = Groups(&groups);
        let mut h = Holder::default();
        h.set(Node::new("group.a", true));
        let p = g.permissions(&h, &Contexts::new(), 0, None);
        assert!(p.contains_key("x:1") && p.contains_key("x:2"));
    }
}
