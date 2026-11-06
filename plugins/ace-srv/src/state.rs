//! Shared plugin state behind one mutex.
//!
//! Re-entrancy rule (see the Pumpkin book, "Plugin state, handles and reentrancy"): the lock is never held across a
//! Pumpkin host call, because such a call can fire an event straight back into this plugin. Code under the lock only
//! touches plain data, WASI (sockets, HTTP, files) and pushes [`Action`]s; the caller runs those after unlocking.

use crate::config::{Config, Messages};
use crate::gateway::{self, Gateway, GwEvent};
use crate::http::{Http, Response};
use crate::linking::Links;
use crate::text::Rich;
use crate::{bridge, util};
use pumpkin_plugin_api::Server;
use std::collections::{BTreeSet, HashMap};
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub static STATE: Mutex<Option<State>> = Mutex::new(None);

/// Host calls queued while the lock was held.
pub enum Action {
    /// Show to every player (and optionally the console).
    Broadcast(Rich),
    /// Show to one player, by UUID string.
    Tell(String, Rich),
    /// Run as the console.
    Console(String),
}

pub struct Role {
    pub name: String,
    pub color: u32,
    pub position: i64,
}

pub struct SelfUser {
    pub id: String,
    pub name: String,
    pub avatar_url: String,
}

pub struct PendingCode {
    pub uuid: String,
    pub name: String,
    pub expires: Instant,
}

pub enum Timer {
    DeleteMessage { channel: String, message: String },
}

#[derive(Clone)]
pub struct OnlinePlayer {
    pub uuid: String,
    pub name: String,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct KnownPlayers {
    pub uuids: BTreeSet<String>,
}

pub struct State {
    pub cfg: Config,
    pub msgs: Messages,
    pub data: String,
    pub http: Http,
    pub gw: Gateway,
    pub me: Option<SelfUser>,
    pub guild_id: Option<String>,
    pub roles: HashMap<String, Role>,
    pub channel_names: HashMap<String, String>,
    /// lowercase display name or username → user id, learnt from messages, for @mentions from the game
    pub member_ids: HashMap<String, String>,
    /// channel id → (webhook id, token)
    pub webhooks: HashMap<String, (String, String)>,
    pub webhook_pending: BTreeSet<String>,
    pub links: Links,
    pub codes: HashMap<String, PendingCode>,
    pub online: Vec<OnlinePlayer>,
    pub max_players: u32,
    pub tps: f64,
    pub mspt: f64,
    pub server_version: String,
    pub known: KnownPlayers,
    pub known_dirty: bool,
    pub console_buf: Vec<String>,
    pub console_next_flush: Instant,
    pub timers: Vec<(Instant, Timer)>,
    pub actions: Vec<Action>,
    pub started: Instant,
    pub presence_index: usize,
    pub next_presence: Instant,
    pub next_topic: Instant,
    pub warned_no_token: bool,
}

impl State {
    pub fn new(data: String, cfg: Config, msgs: Messages) -> Self {
        let links = Links::load(&data);
        let known: KnownPlayers = util::read_json(&format!("{data}/players.json"));
        let mut http = Http::default();
        http.debug = cfg.debug;
        let mut gw = Gateway::default();
        gw.debug = cfg.debug;
        let now = Instant::now();
        State {
            cfg,
            msgs,
            data,
            http,
            gw,
            me: None,
            guild_id: None,
            roles: HashMap::new(),
            channel_names: HashMap::new(),
            member_ids: HashMap::new(),
            webhooks: HashMap::new(),
            webhook_pending: BTreeSet::new(),
            links,
            codes: HashMap::new(),
            online: Vec::new(),
            max_players: 0,
            tps: 20.0,
            mspt: 0.0,
            server_version: "Pumpkin".into(),
            known,
            known_dirty: false,
            console_buf: Vec::new(),
            console_next_flush: now,
            timers: Vec::new(),
            actions: Vec::new(),
            started: now,
            presence_index: 0,
            next_presence: now + Duration::from_secs(5),
            next_topic: now + Duration::from_secs(30),
            warned_no_token: false,
        }
    }

    pub fn token(&self) -> String {
        self.cfg.bot_token.trim().to_string()
    }

    pub fn path(&self, file: &str) -> String {
        format!("{}/{file}", self.data)
    }

    pub fn bot_name(&self) -> String {
        self.me.as_ref().map_or_else(|| "Ace SRV".into(), |m| m.name.clone())
    }

    pub fn bot_avatar(&self) -> String {
        self.me.as_ref().map_or_else(String::new, |m| m.avatar_url.clone())
    }

    pub fn avatar_for(&self, uuid: &str, name: &str, size: u32) -> String {
        self.cfg.avatar_url.replace("{uuid}", uuid).replace("{username}", name).replace("{size}", &size.to_string())
    }

    pub fn presence_payload(&self) -> serde_json::Value {
        let statuses = &self.cfg.presence.statuses;
        let line = if statuses.is_empty() {
            "playing Minecraft".to_string()
        } else {
            statuses[self.presence_index % statuses.len()].clone()
        };
        let line = util::fill(&line, &[
            ("online", self.online.len().to_string()),
            ("max", self.max_players.to_string()),
        ]);
        gateway::presence(&line, &self.cfg.presence.online_status)
    }

    /// Adds a line to the console channel mirror (sent in batches).
    pub fn console_line(&mut self, level: &str, line: &str) {
        if self.cfg.console_channel().is_none() {
            return;
        }
        let prefix = util::fill(&self.msgs.console.prefix, &[("time", util::time()), ("level", level.into())]);
        self.console_buf.push(format!("{prefix}{line}"));
    }

    fn flush_console(&mut self) {
        let Some(channel) = self.cfg.console_channel() else {
            self.console_buf.clear();
            return;
        };
        if self.console_buf.is_empty() {
            return;
        }
        let lines = std::mem::take(&mut self.console_buf);
        let fence = self.cfg.console.use_code_blocks;
        let limit = if fence { 1990 } else { 2000 };
        let mut chunk = String::new();
        let mut chunks = Vec::new();
        for line in lines {
            let line = util::truncate(&line.replace("```", "`\u{200b}``"), limit - 2);
            if chunk.len() + line.len() + 1 > limit {
                chunks.push(std::mem::take(&mut chunk));
            }
            chunk.push_str(&line);
            chunk.push('\n');
        }
        if !chunk.is_empty() {
            chunks.push(chunk);
        }
        for c in chunks {
            let content = if fence { format!("```\n{c}```") } else { c };
            crate::discord::send_text(self, &channel, &content);
        }
    }

    /// One pump step, called every tick with the lock held.
    pub fn tick(&mut self) {
        let now = Instant::now();
        if !self.cfg.has_token() {
            if !self.warned_no_token {
                tracing::warn!(
                    "Ace SRV has no bot token. Set bot_token in plugins/data/ace-srv/config.toml or ACE_SRV_BOT_TOKEN, then /discord reload."
                );
                self.warned_no_token = true;
            }
            return;
        }

        let done = self.http.pump();
        self.run_callbacks(done);

        let presence = self.presence_payload();
        let token = self.token();
        for event in self.gw.tick(&token, &presence) {
            match event {
                GwEvent::Dispatch(t, d) => bridge::on_dispatch(self, &t, d),
                GwEvent::Fatal(why) => tracing::error!("{why}"),
            }
        }

        // timers
        if !self.timers.is_empty() {
            let (due, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.timers).into_iter().partition(|(at, _)| *at <= now);
            self.timers = later;
            for (_, timer) in due {
                match timer {
                    Timer::DeleteMessage { channel, message } => crate::discord::delete_message(self, &channel, &message),
                }
            }
        }

        if now >= self.console_next_flush {
            self.console_next_flush = now + Duration::from_secs(self.cfg.console.refresh_seconds.max(1));
            self.flush_console();
        }

        if self.cfg.presence.enabled && self.gw.ready && now >= self.next_presence {
            self.next_presence = now + Duration::from_secs(self.cfg.presence.rotate_minutes.max(1) * 60);
            self.presence_index = self.presence_index.wrapping_add(1);
            let p = self.presence_payload();
            self.gw.update_presence(p);
        }

        if self.cfg.topic.enabled && now >= self.next_topic {
            self.next_topic = now + Duration::from_secs(self.cfg.topic.rate_minutes.max(5) * 60);
            bridge::update_topics(self, false);
        }

        if self.known_dirty {
            self.known_dirty = false;
            util::write_json(&self.path("players.json"), &self.known);
        }

        self.codes.retain(|_, c| c.expires > now);
    }

    pub fn run_callbacks(&mut self, done: Vec<(crate::http::Callback, Response)>) {
        for (cb, resp) in done {
            cb(self, resp);
        }
    }

    pub fn delete_later(&mut self, channel: &str, message: &str, secs: u64) {
        if secs > 0 {
            self.timers.push((Instant::now() + Duration::from_secs(secs), Timer::DeleteMessage {
                channel: channel.into(),
                message: message.into(),
            }));
        }
    }
}

/// Runs `f` with the state locked; None if the plugin is not loaded.
pub fn with<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    let mut guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_mut().map(f)
}

/// Runs the queued host calls (call with the lock released).
pub fn flush_actions(server: &Server) {
    let (actions, to_console) = with(|s| (std::mem::take(&mut s.actions), s.cfg.chat.broadcast_to_console)).unwrap_or_default();
    for action in actions {
        match action {
            Action::Broadcast(rich) => {
                for player in server.get_all_players() {
                    player.send_system_message(rich.to_component(), false);
                }
                if to_console {
                    tracing::info!("{}", rich.plain());
                }
            }
            Action::Tell(uuid, rich) => {
                if let Some(id) = pumpkin_plugin_api::uuid::parse(&uuid) {
                    if let Some(player) = server.get_player_by_uuid(id) {
                        player.send_system_message(rich.to_component(), false);
                    }
                }
            }
            Action::Console(command) => {
                server.execute_command(&command, pumpkin_plugin_api::server::CommandSender::Console);
            }
        }
    }
}
