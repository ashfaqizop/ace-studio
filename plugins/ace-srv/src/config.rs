//! Config and messages: TOML files in the data folder, merged over the commented defaults shipped in the binary,
//! so a key missing from an admin's file always falls back to its default.
//! (DiscordSRV: config.yml / messages.yml, see PORTING.md for the key mapping.)

use serde::Deserialize;
use std::collections::BTreeMap;
use toml::{Table, Value};

pub const DEFAULT_CONFIG: &str = include_str!("../defaults/config.toml");
pub const DEFAULT_MESSAGES: &str = include_str!("../defaults/messages.toml");

/// Tables whose entries are user data: the admin's table replaces the default one instead of being merged into it.
const FREE_MAPS: &[&str] = &[
    "channels",
    "chat.role_aliases",
    "chat.game_filters",
    "chat.discord_filters",
    "chat.canned_responses",
];

#[derive(Deserialize, Clone)]
pub struct Config {
    pub bot_token: String,
    pub guild_id: String,
    pub invite_link: String,
    pub avatar_url: String,
    pub debug: bool,
    pub channels: BTreeMap<String, String>,
    pub console: ConsoleCfg,
    pub presence: PresenceCfg,
    pub chat: ChatCfg,
    pub topic: TopicCfg,
    pub linking: LinkingCfg,
}

#[derive(Deserialize, Clone)]
pub struct ConsoleCfg {
    pub channel_id: String,
    pub refresh_seconds: u64,
    pub use_code_blocks: bool,
    pub block_bots: bool,
    pub blacklist: Vec<String>,
    pub blacklist_is_whitelist: bool,
    pub usage_log: String,
}

#[derive(Deserialize, Clone)]
pub struct PresenceCfg {
    pub enabled: bool,
    pub statuses: Vec<String>,
    pub online_status: String,
    pub rotate_minutes: u64,
}

#[derive(Deserialize, Clone)]
pub struct ChatCfg {
    pub discord_to_minecraft: bool,
    pub minecraft_to_discord: bool,
    pub truncate_length: usize,
    pub translate_mentions: bool,
    pub emoji: String,
    pub prefix_required: String,
    pub prefix_blacklist: bool,
    pub color_code_roles: Vec<String>,
    pub broadcast_to_console: bool,
    pub require_linked: bool,
    pub block_bots: bool,
    pub block_webhooks: bool,
    pub blocked_ids: Vec<String>,
    pub blocked_roles: Vec<String>,
    pub blocked_roles_whitelist: bool,
    pub role_selection: Vec<String>,
    pub role_selection_whitelist: bool,
    pub webhook_delivery: bool,
    pub role_aliases: BTreeMap<String, String>,
    pub game_filters: BTreeMap<String, String>,
    pub discord_filters: BTreeMap<String, String>,
    pub canned_responses: BTreeMap<String, String>,
    pub console_command: ConsoleCommandCfg,
    pub player_list: PlayerListCfg,
}

#[derive(Deserialize, Clone)]
pub struct ConsoleCommandCfg {
    pub enabled: bool,
    pub prefix: String,
    pub roles_allowed: Vec<String>,
    pub whitelist: Vec<String>,
    pub whitelist_bypass_roles: Vec<String>,
    pub whitelist_is_blacklist: bool,
    pub expiration: u64,
}

#[derive(Deserialize, Clone)]
pub struct PlayerListCfg {
    pub enabled: bool,
    pub trigger: String,
    pub expiration: u64,
    pub delete_request: bool,
}

#[derive(Deserialize, Clone)]
pub struct TopicCfg {
    pub enabled: bool,
    pub rate_minutes: u64,
    pub at_shutdown: bool,
}

#[derive(Deserialize, Clone)]
pub struct LinkingCfg {
    pub linked_role: String,
    pub allow_relink: bool,
    pub code_expiry_minutes: u64,
    pub linked_commands: Vec<String>,
    pub unlinked_commands: Vec<String>,
}

#[derive(Deserialize, Clone)]
pub struct Messages {
    pub discord_to_minecraft: DiscordToMinecraft,
    pub minecraft_to_discord: MinecraftToDiscord,
    pub server: ServerMsgs,
    pub player_list: PlayerListMsgs,
    pub topic: TopicMsgs,
    pub console: ConsoleMsgs,
    pub linking: LinkingMsgs,
    pub commands: CommandMsgs,
    pub join: EventSpec,
    pub first_join: EventSpec,
    pub leave: EventSpec,
    pub death: EventSpec,
    pub advancement: EventSpec,
}

#[derive(Deserialize, Clone)]
pub struct DiscordToMinecraft {
    pub format: String,
    pub format_no_role: String,
    pub reply_format: String,
    pub all_roles_separator: String,
}

#[derive(Deserialize, Clone)]
pub struct MinecraftToDiscord {
    pub format: String,
    pub webhook_username: String,
    pub webhook_content: String,
}

#[derive(Deserialize, Clone)]
pub struct ServerMsgs {
    pub startup: String,
    pub shutdown: String,
}

#[derive(Deserialize, Clone)]
pub struct PlayerListMsgs {
    pub header: String,
    pub empty: String,
    pub player: String,
    pub separator: String,
}

#[derive(Deserialize, Clone)]
pub struct TopicMsgs {
    pub chat: String,
    pub console: String,
    pub chat_shutdown: String,
    pub console_shutdown: String,
}

#[derive(Deserialize, Clone)]
pub struct ConsoleMsgs {
    pub prefix: String,
}

#[derive(Deserialize, Clone)]
pub struct LinkingMsgs {
    pub code: String,
    pub already_linked: String,
    pub not_linked: String,
    pub linked_info: String,
    pub unlinked: String,
    pub linked_game: String,
    pub linked_discord: String,
    pub already_linked_discord: String,
    pub invalid_code: String,
    pub dm_help: String,
    pub not_connected: String,
}

#[derive(Deserialize, Clone)]
pub struct CommandMsgs {
    pub invite: String,
    pub reloaded: String,
    pub broadcast_sent: String,
    pub no_permission: String,
    pub players_only: String,
    pub help: String,
}

/// One Discord announcement (DiscordSRV's MinecraftPlayerJoinMessage and friends).
#[derive(Deserialize, Clone)]
pub struct EventSpec {
    pub enabled: bool,
    pub content: String,
    pub webhook: WebhookSpec,
    pub embed: EmbedSpec,
}

#[derive(Deserialize, Clone)]
pub struct WebhookSpec {
    pub enabled: bool,
    pub name: String,
    pub avatar_url: String,
}

#[derive(Deserialize, Clone)]
pub struct EmbedSpec {
    pub enabled: bool,
    pub color: String,
    pub author_name: String,
    pub author_image: String,
    pub author_url: String,
    pub title: String,
    pub title_url: String,
    pub description: String,
    pub thumbnail: String,
    pub image: String,
    pub footer_text: String,
    pub footer_icon: String,
    pub timestamp: bool,
}

fn merge(base: &mut Table, user: Table, path: &str, warnings: &mut Vec<String>) {
    for (key, value) in user {
        let full = if path.is_empty() { key.clone() } else { format!("{path}.{key}") };
        if !base.contains_key(&key) {
            if !FREE_MAPS.contains(&path) {
                warnings.push(full);
            }
            base.insert(key, value);
            continue;
        }
        let Some(slot) = base.get_mut(&key) else { continue };
        match (slot, value) {
            (Value::Table(b), Value::Table(u)) if !FREE_MAPS.contains(&full.as_str()) => {
                merge(b, u, &full, warnings);
            }
            (slot, value) => *slot = value,
        }
    }
}

/// Reads `file` from the data folder (writing the defaults the first time), merges it over the defaults and parses it.
fn load<T: for<'de> Deserialize<'de>>(data: &str, file: &str, defaults: &str) -> Result<T, String> {
    let path = format!("{data}/{file}");
    let user_text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(_) => {
            if let Err(e) = std::fs::write(&path, defaults) {
                tracing::warn!("Could not write default {file}: {e}");
            }
            defaults.to_string()
        }
    };
    let mut base: Table = defaults.parse().map_err(|e| format!("built-in {file} is invalid: {e}"))?;
    let user: Table = user_text.parse().map_err(|e| format!("{file}: {e}"))?;
    let mut warnings = Vec::new();
    merge(&mut base, user, "", &mut warnings);
    for w in warnings {
        tracing::warn!("{file}: unknown setting `{w}` is ignored");
    }
    T::deserialize(Value::Table(base)).map_err(|e| format!("{file}: {e}"))
}

pub fn load_all(data: &str) -> Result<(Config, Messages), String> {
    let mut cfg: Config = load(data, "config.toml", DEFAULT_CONFIG)?;
    let msgs: Messages = load(data, "messages.toml", DEFAULT_MESSAGES)?;
    if let Ok(token) = std::env::var("ACE_SRV_BOT_TOKEN") {
        if !token.trim().is_empty() {
            cfg.bot_token = token.trim().to_string();
        }
    }
    Ok((cfg, msgs))
}

impl Config {
    pub fn has_token(&self) -> bool {
        let t = self.bot_token.trim();
        !t.is_empty() && t != "BOTTOKEN"
    }

    /// The channel id for game channel `name`, if configured.
    pub fn channel(&self, name: &str) -> Option<String> {
        self.channels.get(name).filter(|id| is_snowflake(id)).cloned()
    }

    pub fn console_channel(&self) -> Option<String> {
        Some(self.console.channel_id.clone()).filter(|id| is_snowflake(id))
    }
}

/// A real Discord id: digits, and not the all-zero placeholder.
pub fn is_snowflake(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()) && id.bytes().any(|b| b != b'0')
}
