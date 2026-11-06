//! Minecraft → Discord: Pumpkin event handlers. Each handler copies what it needs out of the event (host calls),
//! then takes the state lock for pure work, then runs queued actions.
//! (DiscordSRV: PlayerChatListener, PlayerJoinLeaveListener, PlayerDeathListener, PlayerAdvancementDoneListener.)

use crate::config::EventSpec;
use crate::discord;
use crate::state::{self, OnlinePlayer, State};
use crate::text;
use crate::util;
use pumpkin_plugin_api::events::{
    EventData, EventPriority, PlayerAdvancementDoneEvent, PlayerChatEvent, PlayerDeathEvent, PlayerJoinEvent,
    PlayerLeaveEvent, ServerCommandEvent, ServerLoadEvent,
};
use pumpkin_plugin_api::{Context, EventHandler, Server};
use regex::Regex;

pub fn register(context: &Context) -> pumpkin_plugin_api::Result<()> {
    context.register_event_handler(Chat, EventPriority::Lowest, false)?;
    context.register_event_handler(Join, EventPriority::Lowest, false)?;
    context.register_event_handler(Leave, EventPriority::Lowest, false)?;
    context.register_event_handler(Death, EventPriority::Lowest, false)?;
    context.register_event_handler(Advancement, EventPriority::Lowest, false)?;
    context.register_event_handler(Load, EventPriority::Normal, false)?;
    context.register_event_handler(ConsoleCommand, EventPriority::Lowest, false)?;
    Ok(())
}

fn uuid_string(player: &pumpkin_plugin_api::Player) -> String {
    pumpkin_plugin_api::uuid::to_string(player.get_id())
}

struct Chat;
impl EventHandler<PlayerChatEvent> for Chat {
    fn handle(&self, server: Server, event: EventData<PlayerChatEvent>) -> EventData<PlayerChatEvent> {
        if !event.cancelled {
            let name = event.player.get_name();
            let uuid = uuid_string(&event.player);
            let message = event.message.clone();
            state::with(|s| on_chat(s, &name, &uuid, &message));
            state::flush_actions(&server);
        }
        event
    }
}

fn on_chat(s: &mut State, name: &str, uuid: &str, message: &str) {
    s.console_line("INFO", &format!("<{name}> {message}"));
    if !s.cfg.chat.minecraft_to_discord {
        return;
    }
    let Some(channel) = s.cfg.channel("global") else { return };
    let mut content = text::strip(message);
    for (pattern, replacement) in &s.cfg.chat.game_filters {
        match Regex::new(pattern) {
            Ok(re) => content = re.replace_all(&content, replacement.as_str()).into_owned(),
            Err(e) => tracing::warn!("Invalid game_filters regex {pattern:?}: {e}"),
        }
    }
    let content = content.trim().to_string();
    if content.is_empty() {
        return;
    }
    let (content, mentioned) = translate_mentions(s, &content);
    let vars = [
        ("displayname", crate::bridge::escape_markdown(name)),
        ("username", crate::bridge::escape_markdown(name)),
        ("uuid", uuid.to_string()),
        ("message", content),
        ("date", util::date()),
        ("time", util::time()),
    ];
    let m2d = s.msgs.minecraft_to_discord.clone();
    if s.cfg.chat.webhook_delivery {
        let username = util::fill(&m2d.webhook_username, &[("displayname", name.to_string()), ("username", name.to_string())]);
        let body = serde_json::json!({
            "content": util::truncate(&util::fill(&m2d.webhook_content, &vars), 2000),
            "allowed_mentions": discord::allowed_mentions(&mentioned),
        });
        let avatar = s.avatar_for(uuid, name, 128);
        if discord::webhook_send(s, &channel, &username, &avatar, body) {
            return;
        }
    }
    let body = serde_json::json!({
        "content": util::truncate(&util::fill(&m2d.format, &vars), 2000),
        "allowed_mentions": discord::allowed_mentions(&mentioned),
    });
    discord::send_message(s, &channel, body);
}

/// `@name` → `<@id>` for Discord members we have seen or linked.
fn translate_mentions(s: &State, content: &str) -> (String, Vec<String>) {
    if !s.cfg.chat.translate_mentions || !content.contains('@') {
        return (content.to_string(), Vec::new());
    }
    let mut ids = Vec::new();
    let words: Vec<String> = content
        .split(' ')
        .map(|w| {
            let Some(name) = w.strip_prefix('@') else { return w.to_string() };
            let clean: String = name.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '.').collect();
            let lower = clean.to_lowercase();
            let id = s.member_ids.get(&lower).cloned().or_else(|| {
                s.links.accounts.iter().find(|(_, l)| l.discord_name.to_lowercase() == lower || l.name.to_lowercase() == lower).map(|(id, _)| id.clone())
            });
            match id {
                Some(id) if !clean.is_empty() => {
                    ids.push(id.clone());
                    format!("<@{id}>{}", &name[clean.len()..])
                }
                _ => w.to_string(),
            }
        })
        .collect();
    ids.sort();
    ids.dedup();
    (words.join(" "), ids)
}

fn announce(s: &mut State, spec: EventSpec, vars: Vec<(&str, String)>) {
    if !spec.enabled {
        return;
    }
    let Some(channel) = s.cfg.channel("global") else { return };
    let Some(body) = discord::event_body(&spec, &vars) else { return };
    if spec.webhook.enabled {
        let name = util::fill(&spec.webhook.name, &vars);
        let avatar = util::fill(&spec.webhook.avatar_url, &vars);
        if discord::webhook_send(s, &channel, &name, &avatar, body.clone()) {
            return;
        }
    }
    discord::send_message(s, &channel, body);
}

fn player_vars(s: &State, name: &str, uuid: &str) -> Vec<(&'static str, String)> {
    vec![
        ("username", crate::bridge::escape_markdown(name)),
        ("displayname", crate::bridge::escape_markdown(name)),
        ("uuid", uuid.to_string()),
        ("embedavatarurl", s.avatar_for(uuid, name, 128)),
        ("botname", s.bot_name()),
        ("botavatarurl", s.bot_avatar()),
        ("date", util::date()),
        ("time", util::time()),
    ]
}

struct Join;
impl EventHandler<PlayerJoinEvent> for Join {
    fn handle(&self, server: Server, event: EventData<PlayerJoinEvent>) -> EventData<PlayerJoinEvent> {
        let name = event.player.get_name();
        let uuid = uuid_string(&event.player);
        let max = server.get_max_players();
        state::with(|s| {
            s.max_players = max;
            s.online.retain(|p| p.uuid != uuid);
            s.online.push(OnlinePlayer { uuid: uuid.clone(), name: name.clone() });
            s.console_line("INFO", &format!("{name} joined the game"));
            let first = s.known.uuids.insert(uuid.clone());
            s.known_dirty |= first;
            let spec = if first { s.msgs.first_join.clone() } else { s.msgs.join.clone() };
            let vars = player_vars(s, &name, &uuid);
            announce(s, spec, vars);
        });
        state::flush_actions(&server);
        event
    }
}

struct Leave;
impl EventHandler<PlayerLeaveEvent> for Leave {
    fn handle(&self, server: Server, event: EventData<PlayerLeaveEvent>) -> EventData<PlayerLeaveEvent> {
        let name = event.player.get_name();
        let uuid = uuid_string(&event.player);
        state::with(|s| {
            s.online.retain(|p| p.uuid != uuid);
            s.console_line("INFO", &format!("{name} left the game"));
            let spec = s.msgs.leave.clone();
            let vars = player_vars(s, &name, &uuid);
            announce(s, spec, vars);
        });
        state::flush_actions(&server);
        event
    }
}

struct Death;
impl EventHandler<PlayerDeathEvent> for Death {
    fn handle(&self, server: Server, event: EventData<PlayerDeathEvent>) -> EventData<PlayerDeathEvent> {
        let name = event.player.get_name();
        let uuid = uuid_string(&event.player);
        let mut message = util::strip_ansi(&event.death_message.to_pretty_console()).trim().to_string();
        if message.is_empty() {
            message = format!("{name} died");
        }
        state::with(|s| {
            s.console_line("INFO", &message);
            let spec = s.msgs.death.clone();
            let mut vars = player_vars(s, &name, &uuid);
            vars.push(("deathmessage", crate::bridge::escape_markdown(&message)));
            announce(s, spec, vars);
        });
        state::flush_actions(&server);
        event
    }
}

struct Advancement;
impl EventHandler<PlayerAdvancementDoneEvent> for Advancement {
    fn handle(&self, server: Server, event: EventData<PlayerAdvancementDoneEvent>) -> EventData<PlayerAdvancementDoneEvent> {
        // recipe unlocks and hidden advancements have no display or are not announced in chat; skip those like vanilla
        let title = server.get_advancement(&event.advancement_id).and_then(|info| info.display).and_then(|d| {
            d.announce_to_chat.then(|| util::strip_ansi(&d.title.to_pretty_console()).trim().to_string())
        });
        let Some(title) = title.filter(|t| !t.is_empty()) else { return event };
        let name = event.player.get_name();
        let uuid = uuid_string(&event.player);
        state::with(|s| {
            s.console_line("INFO", &format!("{name} has made the advancement [{title}]"));
            let spec = s.msgs.advancement.clone();
            let mut vars = player_vars(s, &name, &uuid);
            vars.push(("advancement", crate::bridge::escape_markdown(&title)));
            announce(s, spec, vars);
        });
        state::flush_actions(&server);
        event
    }
}

struct Load;
impl EventHandler<ServerLoadEvent> for Load {
    fn handle(&self, server: Server, event: EventData<ServerLoadEvent>) -> EventData<ServerLoadEvent> {
        // (get_sys_info would give the Pumpkin version but needs the sys.info permission; not worth the prompt)
        let max = server.get_max_players();
        state::with(|s| {
            s.max_players = max;
            startup_message(s);
        });
        event
    }
}

pub fn startup_message(s: &mut State) {
    let Some(channel) = s.cfg.channel("global") else { return };
    let text = s.msgs.server.startup.clone();
    discord::send_text(s, &channel, &text);
}

struct ConsoleCommand;
impl EventHandler<ServerCommandEvent> for ConsoleCommand {
    fn handle(&self, _server: Server, event: EventData<ServerCommandEvent>) -> EventData<ServerCommandEvent> {
        let command = event.command.clone();
        state::with(|s| s.console_line("INFO", &format!("Console issued server command: /{command}")));
        event
    }
}
