//! Nodes, holders and tracks (LuckPerms: `node/*`, `model/PermissionHolder`, `Group`, `User`, `Track`).
//!
//! A node is a key plus a value, optional expiry and contexts. Its kind comes from the key, like upstream:
//! `group.<name>` (inheritance), `prefix.<priority>.<text>`, `suffix.<priority>.<text>`, `meta.<key>.<value>`,
//! `weight.<n>`, `displayname.<name>`, anything else is a permission.

use std::collections::{BTreeMap, BTreeSet};

/// Context key → allowed values (`ImmutableContextSet`). Empty means "global".
pub type Contexts = BTreeMap<String, BTreeSet<String>>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub key: String,
    pub value: bool,
    /// Unix seconds; `None` = permanent.
    pub expiry: Option<i64>,
    pub contexts: Contexts,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind<'a> {
    Inheritance(&'a str),
    Prefix(i32, &'a str),
    Suffix(i32, &'a str),
    Meta(&'a str, &'a str),
    Weight(i32),
    DisplayName(&'a str),
    Permission,
}

impl Node {
    pub fn new(key: impl Into<String>, value: bool) -> Self {
        Node { key: key.into(), value, expiry: None, contexts: Contexts::new() }
    }

    pub fn with(mut self, expiry: Option<i64>, contexts: Contexts) -> Self {
        self.expiry = expiry;
        self.contexts = contexts;
        self
    }

    pub fn kind(&self) -> Kind<'_> {
        let k = self.key.as_str();
        let lower = k.to_ascii_lowercase();
        if let Some(g) = k.get(6..).filter(|_| lower.starts_with("group.")) {
            return Kind::Inheritance(g);
        }
        for (prefix, is_prefix) in [("prefix.", true), ("suffix.", false)] {
            if lower.starts_with(prefix)
                && let Some((p, text)) = k[prefix.len()..].split_once('.')
                && let Ok(p) = p.parse()
            {
                return if is_prefix { Kind::Prefix(p, text) } else { Kind::Suffix(p, text) };
            }
        }
        if lower.starts_with("meta.")
            && let Some((key, value)) = k[5..].split_once('.')
        {
            return Kind::Meta(key, value);
        }
        if lower.starts_with("weight.")
            && let Ok(w) = k[7..].parse()
        {
            return Kind::Weight(w);
        }
        if lower.starts_with("displayname.") {
            return Kind::DisplayName(&k[12..]);
        }
        Kind::Permission
    }

    pub fn expired(&self, now: i64) -> bool {
        self.expiry.is_some_and(|e| e <= now)
    }

    /// `ContextSet.isSatisfiedBy`: every key the node names must have one of its values in the query.
    pub fn applies(&self, query: &Contexts) -> bool {
        self.contexts.iter().all(|(k, vals)| query.get(k).is_some_and(|q| vals.iter().any(|v| q.contains(v))))
    }

    /// Same key, contexts and temporary-ness (`NodeEquality.IGNORE_EXPIRY_TIME_AND_VALUE`), used for set/unset.
    pub fn same_slot(&self, other: &Node) -> bool {
        self.key.eq_ignore_ascii_case(&other.key) && self.contexts == other.contexts && self.expiry.is_some() == other.expiry.is_some()
    }
}

/// `ContextSetComparator`, descending: non-empty, then has `server`, then has `world`, then more entries, wins.
pub fn context_rank(c: &Contexts) -> (bool, bool, bool, usize) {
    (!c.is_empty(), c.contains_key("server"), c.contains_key("world"), c.values().map(|v| v.len()).sum())
}

#[derive(Clone, Debug, Default)]
pub struct Holder {
    pub nodes: Vec<Node>,
}

impl Holder {
    /// Add, replacing a node in the same slot. Returns false if an identical node was already there.
    pub fn set(&mut self, node: Node) -> bool {
        if let Some(i) = self.nodes.iter().position(|n| n.same_slot(&node)) {
            if self.nodes[i] == node {
                return false;
            }
            self.nodes[i] = node;
        } else {
            self.nodes.push(node);
        }
        true
    }

    /// Remove nodes with this key and contexts (`temporary` picks timed or permanent ones).
    pub fn unset(&mut self, key: &str, contexts: &Contexts, temporary: bool) -> bool {
        let before = self.nodes.len();
        self.nodes.retain(|n| !(n.key.eq_ignore_ascii_case(key) && &n.contexts == contexts && n.expiry.is_some() == temporary));
        self.nodes.len() != before
    }

    pub fn remove_where(&mut self, f: impl Fn(&Node) -> bool) -> usize {
        let before = self.nodes.len();
        self.nodes.retain(|n| !f(n));
        before - self.nodes.len()
    }

    /// Drop expired nodes; true if anything changed.
    pub fn expire(&mut self, now: i64) -> bool {
        self.remove_where(|n| n.expired(now)) > 0
    }

    pub fn parents(&self) -> impl Iterator<Item = (&str, &Node)> {
        self.nodes.iter().filter_map(|n| match n.kind() {
            Kind::Inheritance(g) if n.value => Some((g, n)),
            _ => None,
        })
    }

    /// The group's own weight (`WeightCache`): the highest `weight.N`.
    pub fn weight(&self) -> i32 {
        self.nodes
            .iter()
            .filter(|n| n.value)
            .filter_map(|n| if let Kind::Weight(w) = n.kind() { Some(w) } else { None })
            .max()
            .unwrap_or(0)
    }

    pub fn display_name(&self) -> Option<&str> {
        self.nodes.iter().find_map(|n| if let Kind::DisplayName(d) = n.kind() { n.value.then_some(d) } else { None })
    }
}

#[derive(Clone, Debug)]
pub struct Group {
    pub name: String,
    pub data: Holder,
}

#[derive(Clone, Debug)]
pub struct User {
    pub uuid: String,
    pub name: String,
    pub primary_group: String,
    pub data: Holder,
}

impl User {
    pub fn new(uuid: &str, name: &str) -> Self {
        let mut u = User { uuid: uuid.into(), name: name.into(), primary_group: DEFAULT_GROUP.into(), data: Holder::default() };
        u.give_default_if_needed();
        u
    }

    /// `UserManager.giveDefaultIfNeeded`: everyone inherits `default` unless they inherit something else.
    pub fn give_default_if_needed(&mut self) -> bool {
        if self.data.parents().next().is_none() {
            self.data.set(Node::new(format!("group.{DEFAULT_GROUP}"), true));
            return true;
        }
        false
    }

    /// Only `group.default` and nothing else: no need to keep a file (`UserManager.isNonDefaultUser`).
    pub fn is_default(&self) -> bool {
        self.data.nodes.len() == 1
            && self.data.nodes[0].key.eq_ignore_ascii_case(&format!("group.{DEFAULT_GROUP}"))
            && self.data.nodes[0].contexts.is_empty()
            && self.data.nodes[0].expiry.is_none()
            && self.primary_group.eq_ignore_ascii_case(DEFAULT_GROUP)
    }
}

#[derive(Clone, Debug)]
pub struct Track {
    pub name: String,
    pub groups: Vec<String>,
}

pub const DEFAULT_GROUP: &str = "default";

/// Group, track and context names LuckPerms accepts (`DataConstraints`).
pub fn valid_name(s: &str) -> bool {
    !s.is_empty() && s.len() <= 36 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds() {
        assert_eq!(Node::new("group.admin", true).kind(), Kind::Inheritance("admin"));
        assert_eq!(Node::new("prefix.100.&c[Admin] ", true).kind(), Kind::Prefix(100, "&c[Admin] "));
        assert_eq!(Node::new("meta.home-limit.5", true).kind(), Kind::Meta("home-limit", "5"));
        assert_eq!(Node::new("weight.50", true).kind(), Kind::Weight(50));
        assert_eq!(Node::new("minecraft:command.gamemode", true).kind(), Kind::Permission);
        assert_eq!(Node::new("prefix.x.y", true).kind(), Kind::Permission);
    }

    #[test]
    fn contexts_apply() {
        let mut c = Contexts::new();
        c.entry("world".into()).or_default().insert("nether".into());
        let n = Node::new("a:b", true).with(None, c.clone());
        assert!(!n.applies(&Contexts::new()));
        assert!(n.applies(&c));
        assert!(Node::new("a:b", true).applies(&c));
    }

    #[test]
    fn default_group() {
        let mut u = User::new("u", "Steve");
        assert!(u.is_default());
        u.data.set(Node::new("group.vip", true));
        u.data.unset("group.default", &Contexts::new(), false);
        assert!(!u.give_default_if_needed());
        assert!(!u.is_default());
    }
}
