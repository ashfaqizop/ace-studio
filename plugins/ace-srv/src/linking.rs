//! Minecraft ⇄ Discord account links, stored as JSON in the data folder.
//! (DiscordSRV: AccountLinkManager with linkedaccounts.json; the JDBC backend is not ported.)

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Clone)]
pub struct Link {
    pub uuid: String,
    pub name: String,
    pub discord_name: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Links {
    /// Discord user id → link
    pub accounts: BTreeMap<String, Link>,
    #[serde(skip)]
    path: String,
}

impl Links {
    pub fn load(data: &str) -> Self {
        let path = format!("{data}/linked-accounts.json");
        let mut links: Links = crate::util::read_json(&path);
        links.path = path;
        links
    }

    pub fn save(&self) {
        crate::util::write_json(&self.path, self);
    }

    pub fn by_uuid(&self, uuid: &str) -> Option<(String, Link)> {
        self.accounts.iter().find(|(_, l)| l.uuid == uuid).map(|(id, l)| (id.clone(), l.clone()))
    }

    pub fn by_discord(&self, id: &str) -> Option<Link> {
        self.accounts.get(id).cloned()
    }

    pub fn insert(&mut self, discord_id: &str, link: Link) {
        self.accounts.retain(|_, l| l.uuid != link.uuid);
        self.accounts.insert(discord_id.to_string(), link);
        self.save();
    }

    pub fn remove_uuid(&mut self, uuid: &str) -> Option<(String, Link)> {
        let found = self.by_uuid(uuid)?;
        self.accounts.remove(&found.0);
        self.save();
        Some(found)
    }
}
