//! Loaded data, online players and the permissions we've pushed to Pumpkin.
//!
//! LuckPerms on Bukkit answers every permission check itself. Pumpkin has no hook for that, so Ace LP does the
//! next best thing: it flattens each online player's permissions for their current contexts and writes them into
//! Pumpkin's permission attachment (`player.set-permission`). Every plugin's checks, and the command tree, then see
//! them. Whenever data or contexts change, only the difference is re-applied.

use crate::model::{Contexts, Group, Track, User};
use crate::resolve::{Groups, Meta};
use crate::storage::Storage;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub static STATE: Mutex<Option<Lp>> = Mutex::new(None);

/// Run `f` on the state. Never call the host inside: queue an [`Action`].
pub fn with<T>(f: impl FnOnce(&mut Lp) -> T) -> Option<T> {
    STATE.lock().unwrap_or_else(|e| e.into_inner()).as_mut().map(f)
}

pub fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

pub enum Action {
    /// Update a player's Pumpkin permission attachment, then resend their command tree.
    Apply { uuid: String, set: Vec<(String, bool)>, unset: Vec<String> },
}

pub struct Online {
    pub name: String,
    pub contexts: Contexts,
    /// What we last wrote into Pumpkin for this player.
    pub applied: BTreeMap<String, bool>,
}

pub struct Lp {
    pub server_name: String,
    pub storage: Storage,
    pub groups: HashMap<String, Group>,
    pub tracks: HashMap<String, Track>,
    /// Users of online players, plus any loaded by a command.
    pub users: HashMap<String, User>,
    /// lowercase name → uuid of everyone who has joined
    pub uuid_cache: BTreeMap<String, String>,
    pub online: HashMap<String, Online>,
    pub actions: Vec<Action>,
}

impl Lp {
    pub fn load(data: &Path, server_name: String) -> Self {
        let storage = Storage::open(data);
        let mut lp = Lp {
            server_name,
            groups: HashMap::new(),
            tracks: HashMap::new(),
            users: HashMap::new(),
            uuid_cache: storage.load_uuid_cache(),
            online: HashMap::new(),
            actions: vec![],
            storage,
        };
        lp.reload();
        lp
    }

    /// (Re)read groups and tracks from disk; `/lp sync`.
    pub fn reload(&mut self) {
        self.groups = self.storage.load_groups().into_iter().map(|g| (g.name.clone(), g)).collect();
        if !self.groups.contains_key(crate::model::DEFAULT_GROUP) {
            let g = Group { name: crate::model::DEFAULT_GROUP.into(), data: Default::default() };
            self.storage.save_group(&g);
            self.groups.insert(g.name.clone(), g);
        }
        self.tracks = self.storage.load_tracks().into_iter().map(|t| (t.name.clone(), t)).collect();
        let online: Vec<String> = self.online.keys().cloned().collect();
        for uuid in online {
            if let Some(u) = self.storage.load_user(&uuid) {
                self.users.insert(uuid, u);
            }
        }
        self.recalculate_all();
    }

    pub fn groups(&self) -> Groups<'_> {
        Groups(&self.groups)
    }

    pub fn query(&self, uuid: &str) -> Contexts {
        let mut q = self.online.get(uuid).map(|o| o.contexts.clone()).unwrap_or_default();
        if self.server_name != "global" {
            q.entry("server".into()).or_default().insert(self.server_name.clone());
        }
        q
    }

    // ---- players ----

    pub fn join(&mut self, uuid: &str, name: &str, contexts: Contexts) {
        let mut user = self.storage.load_user(uuid).unwrap_or_else(|| User::new(uuid, name));
        if user.name != name {
            user.name = name.to_string();
            self.storage.save_user(&user);
        }
        let key = name.to_ascii_lowercase();
        if self.uuid_cache.get(&key).map(String::as_str) != Some(uuid) {
            self.uuid_cache.insert(key, uuid.to_string());
            self.storage.save_uuid_cache(&self.uuid_cache);
        }
        self.users.insert(uuid.to_string(), user);
        self.online.insert(uuid.to_string(), Online { name: name.to_string(), contexts, applied: BTreeMap::new() });
        self.recalculate(uuid);
    }

    /// Returns what we had set, so the caller can clear it from Pumpkin while the player handle is still valid.
    pub fn leave(&mut self, uuid: &str) -> Vec<String> {
        self.users.remove(uuid);
        self.online.remove(uuid).map(|o| o.applied.into_keys().collect()).unwrap_or_default()
    }

    pub fn set_contexts(&mut self, uuid: &str, contexts: Contexts) {
        if let Some(o) = self.online.get_mut(uuid)
            && o.contexts != contexts
        {
            o.contexts = contexts;
            self.recalculate(uuid);
        }
    }

    /// Recompute one online player's permissions and queue the difference.
    pub fn recalculate(&mut self, uuid: &str) {
        let Some(user) = self.users.get(uuid) else { return };
        let query = self.query(uuid);
        let want = self.groups().permissions(&user.data, &query, now(), Some(&user.primary_group));
        let Some(o) = self.online.get_mut(uuid) else { return };
        let set: Vec<(String, bool)> = want.iter().filter(|(k, v)| o.applied.get(*k) != Some(v)).map(|(k, v)| (k.clone(), *v)).collect();
        let unset: Vec<String> = o.applied.keys().filter(|k| !want.contains_key(*k)).cloned().collect();
        if set.is_empty() && unset.is_empty() {
            return;
        }
        o.applied = want;
        self.actions.push(Action::Apply { uuid: uuid.to_string(), set, unset });
    }

    pub fn recalculate_all(&mut self) {
        let ids: Vec<String> = self.online.keys().cloned().collect();
        for id in ids {
            self.recalculate(&id);
        }
    }

    /// Drop expired nodes everywhere (`ExpireTemporaryTask`), once a second.
    pub fn tick(&mut self) {
        let t = now();
        let mut changed_groups = false;
        for g in self.groups.values_mut() {
            if g.data.expire(t) {
                self.storage.save_group(g);
                changed_groups = true;
            }
        }
        let mut changed_users = vec![];
        for (id, u) in self.users.iter_mut() {
            if u.data.expire(t) {
                u.give_default_if_needed();
                self.storage.save_user(u);
                changed_users.push(id.clone());
            }
        }
        if changed_groups {
            self.recalculate_all();
        } else {
            for id in changed_users {
                self.recalculate(&id);
            }
        }
    }

    // ---- lookups for commands and IPC ----

    /// A user by online name, known name or UUID, loaded into `users` if needed.
    pub fn find_user(&mut self, who: &str) -> Option<String> {
        let lower = who.to_ascii_lowercase();
        let uuid = if let Some((id, _)) = self.online.iter().find(|(_, o)| o.name.eq_ignore_ascii_case(who)) {
            id.clone()
        } else if let Some(id) = self.uuid_cache.get(&lower) {
            id.clone()
        } else if lower.len() == 36 && lower.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
            lower
        } else {
            return None;
        };
        if !self.users.contains_key(&uuid) {
            let name = self.uuid_cache.iter().find(|(_, v)| **v == uuid).map(|(k, _)| k.clone()).unwrap_or_default();
            let user = self.storage.load_user(&uuid).unwrap_or_else(|| User::new(&uuid, &name));
            self.users.insert(uuid.clone(), user);
        }
        Some(uuid)
    }

    pub fn user_meta(&self, uuid: &str) -> Option<Meta> {
        let u = self.users.get(uuid)?;
        Some(self.groups().meta(&u.data, &self.query(uuid), now(), Some(&u.primary_group)))
    }

    /// Persist a user and refresh them if they're online.
    pub fn user_changed(&mut self, uuid: &str) {
        if let Some(u) = self.users.get_mut(uuid) {
            u.give_default_if_needed();
            self.storage.save_user(u);
        }
        if self.online.contains_key(uuid) {
            self.recalculate(uuid);
        } else {
            self.users.remove(uuid);
        }
    }

    pub fn group_changed(&mut self, name: &str) {
        if let Some(g) = self.groups.get(name) {
            self.storage.save_group(g);
        }
        self.recalculate_all();
    }

    /// Members stored on disk plus loaded users, for `listmembers`.
    pub fn members_of(&self, group: &str) -> BTreeSet<String> {
        let key = format!("group.{group}");
        let mut out = BTreeSet::new();
        for u in self.storage.all_users().iter().chain(self.users.values()) {
            if u.data.nodes.iter().any(|n| n.value && n.key.eq_ignore_ascii_case(&key)) {
                out.insert(if u.name.is_empty() { u.uuid.clone() } else { u.name.clone() });
            }
        }
        out
    }
}
