//! # Ace LP
//!
//! A permissions manager for [Pumpkin](https://github.com/Pumpkin-MC/Pumpkin): groups, inheritance, weights,
//! tracks, contexts, temporary nodes, prefixes and meta. Translated from [LuckPerms](https://github.com/LuckPerms/LuckPerms)
//! (MIT). See PORTING.md for what maps to what.
//!
//! Copyright (c) lucko (Luck) and contributors; Pumpkin port (c) Ace Studio. MIT licensed, see LICENSE.

mod commands;
mod ipc;
mod model;
mod resolve;
mod state;
mod storage;

use model::Contexts;
use pumpkin_plugin_api::command::{Command, CommandError, CommandNode, CommandSender, ConsumedArgs};
use pumpkin_plugin_api::command_wit::{Arg, ArgumentType, StringType};
use pumpkin_plugin_api::commands::CommandHandler;
use pumpkin_plugin_api::common::{GameMode, NamedColor};
use pumpkin_plugin_api::events::{
    EventData, EventPriority, PlayerChangedWorldEvent, PlayerGamemodeChangeEvent, PlayerJoinEvent, PlayerLeaveEvent,
};
use pumpkin_plugin_api::permission::{Permission, PermissionDefault, PermissionLevel};
use pumpkin_plugin_api::permissions;
use pumpkin_plugin_api::scheduler::SchedulerExt;
use pumpkin_plugin_api::text::TextComponent;
use pumpkin_plugin_api::{Context, EventHandler, Player, Plugin, PluginMetadata, Server, register_plugin};
use serde::Deserialize;
use state::{Action, Lp, with};
use std::sync::atomic::{AtomicU32, Ordering};

const PERM_ADMIN: &str = "ace-lp:command";

static TICK_TASK: AtomicU32 = AtomicU32::new(u32::MAX);

#[derive(Deserialize)]
#[serde(default)]
struct Config {
    /// The `server` context (LuckPerms `server` setting). "global" adds no server context.
    server: String,
}

impl Default for Config {
    fn default() -> Self {
        Config { server: "global".into() }
    }
}

const DEFAULT_CONFIG: &str = "# Ace LP\n\n# The name of this server, used for the `server` context (server=<name>) in permissions.\n# \"global\" means this server doesn't set a server context.\nserver = \"global\"\n";

fn uuid_of(p: &Player) -> String {
    pumpkin_plugin_api::uuid::to_string(p.get_id()).to_ascii_lowercase()
}

fn gamemode_name(g: GameMode) -> &'static str {
    match g {
        GameMode::Survival => "survival",
        GameMode::Creative => "creative",
        GameMode::Adventure => "adventure",
        GameMode::Spectator => "spectator",
    }
}

/// The contexts Ace LP calculates for a player (LuckPerms `world`, `gamemode`, `dimension-type` calculators).
fn contexts_of(world: &str, gamemode: GameMode) -> Contexts {
    let mut c = Contexts::new();
    let world = world.to_ascii_lowercase();
    // the short name too, so "world=overworld" and "world=minecraft:overworld" both work
    let short = world.rsplit(':').next().unwrap_or(&world).to_string();
    let w = c.entry("world".into()).or_default();
    w.insert(world.clone());
    w.insert(short.clone());
    c.entry("dimension-type".into()).or_default().insert(short);
    c.entry("gamemode".into()).or_default().insert(gamemode_name(gamemode).into());
    c
}

fn player_contexts(p: &Player) -> Contexts {
    contexts_of(&p.get_world().get_id(), p.get_gamemode())
}

/// Run queued host calls with the lock released.
fn flush(server: &Server) {
    let actions = with(|lp| std::mem::take(&mut lp.actions)).unwrap_or_default();
    for action in actions {
        match action {
            Action::Apply { uuid, set, unset } => {
                let Some(id) = pumpkin_plugin_api::uuid::parse(&uuid) else { continue };
                let Some(p) = server.get_player_by_uuid(id) else { continue };
                apply(&p, &set, &unset);
            }
        }
    }
}

fn apply(p: &Player, set: &[(String, bool)], unset: &[String]) {
    for key in unset {
        p.unset_permission(key);
    }
    for (key, value) in set {
        p.set_permission(key, *value);
    }
    // Pumpkin only resends the command tree when the op level changes; setting the same level forces it, so
    // commands a player just gained or lost appear or disappear in their chat suggestions right away.
    p.set_permission_level(p.get_permission_level());
}

struct AceLp;

impl Plugin for AceLp {
    fn new() -> Self {
        AceLp
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
            authors: vec!["Ace Studio".into(), "lucko and LuckPerms contributors (original)".into()],
            description: "Ace LP: groups, inheritance, tracks and contexts for Pumpkin permissions, ported from LuckPerms".into(),
            dependencies: vec![],
            permissions: vec![
                // config.toml and json-storage/ (users, groups, tracks)
                permissions::FS_READ_DATA.into(),
                permissions::FS_WRITE_DATA.into(),
            ],
        }
    }

    fn on_load(&self, context: Context) -> pumpkin_plugin_api::Result<()> {
        let data = std::path::PathBuf::from(context.get_data_folder());
        let cfg_path = data.join("config.toml");
        let cfg: Config = match std::fs::read_to_string(&cfg_path) {
            Ok(t) => toml::from_str(&t).map_err(|e| format!("config.toml: {e}"))?,
            Err(_) => {
                let _ = std::fs::create_dir_all(&data).and_then(|_| std::fs::write(&cfg_path, DEFAULT_CONFIG));
                Config::default()
            }
        };
        let mut lp = Lp::load(&data, cfg.server.to_ascii_lowercase());

        let server = context.get_server();
        for p in server.get_all_players() {
            lp.join(&uuid_of(&p), &p.get_name(), player_contexts(&p));
        }
        tracing::info!("Ace LP {} loaded: {} groups, {} tracks", env!("CARGO_PKG_VERSION"), lp.groups.len(), lp.tracks.len());
        *state::STATE.lock().unwrap_or_else(|e| e.into_inner()) = Some(lp);
        flush(&server);

        context.register_permission(&Permission {
            node: PERM_ADMIN.into(),
            description: "Use /lp to manage permissions".into(),
            default: PermissionDefault::Op(PermissionLevel::Four),
            children: vec![],
        })?;
        let names: Vec<String> = ["lp", "luckperms", "perms", "perm", "permissions"].iter().map(|s| s.to_string()).collect();
        let command = Command::new(&names, "Ace LP: manage permissions, groups and tracks")
            .then(CommandNode::argument("args", &ArgumentType::String(StringType::Greedy)).execute(LpCommand))
            .execute(LpCommand);
        context.register_command(command, PERM_ADMIN);

        // apply permissions before anything else reacts to the join
        context.register_event_handler(Join, EventPriority::Highest, true)?;
        context.register_event_handler(Leave, EventPriority::Lowest, false)?;
        context.register_event_handler(WorldChange, EventPriority::Highest, false)?;
        context.register_event_handler(GamemodeChange, EventPriority::Highest, false)?;

        // expire temporary nodes every second
        let id = context.schedule_repeating_task(20, 20, |server| {
            with(Lp::tick);
            flush(&server);
        });
        TICK_TASK.store(id, Ordering::Relaxed);
        Ok(())
    }

    fn on_unload(&self, context: Context) -> pumpkin_plugin_api::Result<()> {
        let id = TICK_TASK.swap(u32::MAX, Ordering::Relaxed);
        if id != u32::MAX {
            pumpkin_plugin_api::scheduler::cancel_task(id);
        }
        // take back everything we granted, so unloading Ace LP leaves Pumpkin's defaults
        let applied: Vec<(String, Vec<String>)> = with(|lp| {
            lp.online.iter().map(|(id, o)| (id.clone(), o.applied.keys().cloned().collect())).collect()
        })
        .unwrap_or_default();
        state::STATE.lock().unwrap_or_else(|e| e.into_inner()).take();
        let server = context.get_server();
        for (uuid, keys) in applied {
            if let Some(p) = pumpkin_plugin_api::uuid::parse(&uuid).and_then(|id| server.get_player_by_uuid(id)) {
                apply(&p, &[], &keys);
            }
        }
        tracing::info!("Ace LP unloaded");
        Ok(())
    }

    fn handle_ipc_message(&self, sender: String, message: Vec<u8>) -> Result<Vec<u8>, String> {
        ipc::handle(&sender, &message)
    }
}

struct Join;
impl EventHandler<PlayerJoinEvent> for Join {
    fn handle(&self, server: Server, event: EventData<PlayerJoinEvent>) -> EventData<PlayerJoinEvent> {
        let (uuid, name, ctx) = (uuid_of(&event.player), event.player.get_name(), player_contexts(&event.player));
        with(|lp| lp.join(&uuid, &name, ctx));
        flush(&server);
        event
    }
}

struct Leave;
impl EventHandler<PlayerLeaveEvent> for Leave {
    fn handle(&self, _server: Server, event: EventData<PlayerLeaveEvent>) -> EventData<PlayerLeaveEvent> {
        let uuid = uuid_of(&event.player);
        let keys = with(|lp| lp.leave(&uuid)).unwrap_or_default();
        // Pumpkin keeps attachments by UUID; clear ours so nothing stale is left for the next join
        for key in keys {
            event.player.unset_permission(&key);
        }
        event
    }
}

struct WorldChange;
impl EventHandler<PlayerChangedWorldEvent> for WorldChange {
    fn handle(&self, server: Server, event: EventData<PlayerChangedWorldEvent>) -> EventData<PlayerChangedWorldEvent> {
        if !event.cancelled {
            let uuid = uuid_of(&event.player);
            let ctx = contexts_of(&event.to_world.get_id(), event.player.get_gamemode());
            with(|lp| lp.set_contexts(&uuid, ctx));
            flush(&server);
        }
        event
    }
}

struct GamemodeChange;
impl EventHandler<PlayerGamemodeChangeEvent> for GamemodeChange {
    fn handle(&self, server: Server, event: EventData<PlayerGamemodeChangeEvent>) -> EventData<PlayerGamemodeChangeEvent> {
        if !event.cancelled {
            let uuid = uuid_of(&event.player);
            let ctx = contexts_of(&event.player.get_world().get_id(), event.new_gamemode);
            with(|lp| lp.set_contexts(&uuid, ctx));
            flush(&server);
        }
        event
    }
}

struct LpCommand;
impl CommandHandler for LpCommand {
    fn handle(&self, sender: CommandSender, server: Server, args: ConsumedArgs) -> Result<i32, CommandError> {
        if !sender.is_console() && !sender.has_permission(&server, PERM_ADMIN) {
            sender.send_message(colored("&3&lLP &8» &cYou don't have permission to use /lp."));
            return Ok(0);
        }
        let raw = match args.get_value("args") {
            Arg::Simple(s) | Arg::Msg(s) => s,
            _ => String::new(),
        };
        let reply = with(|lp| lp.command(&raw)).unwrap_or_else(|| vec!["Ace LP isn't loaded".into()]);
        flush(&server);
        for l in &reply {
            sender.send_message(colored(l));
        }
        Ok(1)
    }
}

/// `&` colour and format codes to a text component (LuckPerms messages and prefixes use them).
fn colored(s: &str) -> TextComponent {
    let mut root = TextComponent::text("");
    let (mut color, mut bold, mut italic, mut under, mut strike) = (None::<NamedColor>, false, false, false, false);
    let mut buf = String::new();
    let flush_seg = |root: TextComponent, buf: &mut String, color: Option<NamedColor>, b: bool, i: bool, u: bool, s: bool| {
        if buf.is_empty() {
            return root;
        }
        let mut c = TextComponent::text(buf);
        if let Some(col) = color {
            c = c.color_named(col);
        }
        if b {
            c = c.bold(true);
        }
        if i {
            c = c.italic(true);
        }
        if u {
            c = c.underlined(true);
        }
        if s {
            c = c.strikethrough(true);
        }
        buf.clear();
        root.add_child(c)
    };
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if (ch == '&' || ch == '§')
            && let Some(code) = chars.peek().map(|c| c.to_ascii_lowercase())
            && "0123456789abcdefklmnor".contains(code)
        {
            chars.next();
            root = flush_seg(root, &mut buf, color, bold, italic, under, strike);
            match code {
                'l' => bold = true,
                'o' => italic = true,
                'n' => under = true,
                'm' => strike = true,
                'k' => {}
                'r' => (color, bold, italic, under, strike) = (None, false, false, false, false),
                c => {
                    color = Some(named(c));
                    (bold, italic, under, strike) = (false, false, false, false);
                }
            }
            continue;
        }
        buf.push(ch);
    }
    flush_seg(root, &mut buf, color, bold, italic, under, strike)
}

fn named(c: char) -> NamedColor {
    match c {
        '0' => NamedColor::Black,
        '1' => NamedColor::DarkBlue,
        '2' => NamedColor::DarkGreen,
        '3' => NamedColor::DarkAqua,
        '4' => NamedColor::DarkRed,
        '5' => NamedColor::DarkPurple,
        '6' => NamedColor::Gold,
        '7' => NamedColor::Gray,
        '8' => NamedColor::DarkGray,
        '9' => NamedColor::Blue,
        'a' => NamedColor::Green,
        'b' => NamedColor::Aqua,
        'c' => NamedColor::Red,
        'd' => NamedColor::LightPurple,
        'e' => NamedColor::Yellow,
        _ => NamedColor::White,
    }
}

register_plugin!(AceLp);
