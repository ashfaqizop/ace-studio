//! # Ace Plasmo
//!
//! Proximity voice chat for [Pumpkin](https://github.com/Pumpkin-MC/Pumpkin), speaking the protocol of the
//! [Plasmo Voice](https://github.com/plasmoapp/plasmo-voice) 2.x client mod. Translated from Plasmo Voice's server
//! (LGPL-3.0-only). See PORTING.md for what maps to what.
//!
//! Copyright (C) Ace Studio. This library is free software: you can redistribute it and/or modify it under the terms
//! of the GNU Lesser General Public License version 3 as published by the Free Software Foundation.

mod config;
mod packets;
mod proto;
mod voice;

use proto::Id;
use pumpkin_plugin_api::command::{Command, CommandError, CommandSender, ConsumedArgs};
use pumpkin_plugin_api::commands::CommandHandler;
use pumpkin_plugin_api::events::{
    EventData, EventPriority, PlayerCustomPayloadEvent, PlayerJoinEvent, PlayerLeaveEvent,
};
use pumpkin_plugin_api::permission::{Permission, PermissionDefault};
use pumpkin_plugin_api::permissions;
use pumpkin_plugin_api::scheduler::SchedulerExt;
use pumpkin_plugin_api::text::TextComponent;
use pumpkin_plugin_api::{Context, EventHandler, Plugin, PluginMetadata, Server, register_plugin};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use voice::{Action, Grants, Pos, Voice};

const PERM_SPEAK: &str = "ace-plasmo:activation.proximity";
const PERM_FREECAM: &str = "ace-plasmo:allow_freecam";
const PERM_LIST: &str = "ace-plasmo:command.list";
const PERM_RECONNECT: &str = "ace-plasmo:command.reconnect";

static STATE: Mutex<Option<Voice>> = Mutex::new(None);
static TICK_TASK: AtomicU32 = AtomicU32::new(u32::MAX);

/// Run `f` on the state. Never call the host inside `f`: a host call can re-enter the plugin, and the lock
/// would already be held (a panic is fatal in WASM). Queue an [`Action`] instead.
fn with<T>(f: impl FnOnce(&mut Voice) -> T) -> Option<T> {
    STATE.lock().unwrap_or_else(|e| e.into_inner()).as_mut().map(f)
}

/// Run the queued host calls, with the lock released.
fn flush(server: &Server) {
    let actions = with(|v| std::mem::take(&mut v.actions)).unwrap_or_default();
    for action in actions {
        match action {
            Action::Tcp(to, data) => {
                if let Some(java) = server.get_player_by_uuid(to.to_api()).and_then(|p| p.as_java()) {
                    java.send_custom_payload(packets::CHANNEL, &data);
                }
            }
            Action::Message(to, text) => {
                if let Some(p) = server.get_player_by_uuid(to.to_api()) {
                    p.send_system_message(TextComponent::text(&text), false);
                }
            }
        }
    }
}

struct AcePlasmo;

impl Plugin for AcePlasmo {
    fn new() -> Self {
        AcePlasmo
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
            authors: vec!["Ace Studio".into(), "Plasmo Voice contributors (original)".into()],
            description: "Ace Plasmo: proximity voice chat for the Plasmo Voice client mod, ported from Plasmo Voice".into(),
            dependencies: vec![],
            permissions: vec![
                // config.toml in the data folder
                permissions::FS_READ_DATA.into(),
                permissions::FS_WRITE_DATA.into(),
                // the voice server: bind a UDP port, receive from and send to clients
                permissions::NETWORK_UDP.into(),
            ],
        }
    }

    fn on_load(&self, context: Context) -> pumpkin_plugin_api::Result<()> {
        let data = context.get_data_folder();
        let cfg = config::load(std::path::Path::new(&data))?;
        let mut voice = Voice::new(cfg);
        match voice.bind() {
            Ok(addr) => tracing::info!("Ace Plasmo voice server listening on UDP {addr}"),
            Err(e) => tracing::error!("Ace Plasmo: {e}. Voice chat is off until the port is free and the plugin reloads."),
        }

        for (node, description, default) in [
            (PERM_SPEAK, "Talk in proximity voice chat", PermissionDefault::Allow),
            (PERM_FREECAM, "Hear from the camera position in freecam", PermissionDefault::Allow),
            (PERM_LIST, "Use /vlist", PermissionDefault::Allow),
            (PERM_RECONNECT, "Use /vrc", PermissionDefault::Allow),
        ] {
            context.register_permission(&Permission { node: node.into(), description: description.into(), default, children: vec![] })?;
        }

        // players already online (plugin reload)
        let server = context.get_server();
        for p in server.get_all_players() {
            voice.join(Id::from_api(p.get_id()), p.get_name());
        }
        *STATE.lock().unwrap_or_else(|e| e.into_inner()) = Some(voice);

        context.register_event_handler(Join, EventPriority::Lowest, false)?;
        context.register_event_handler(Leave, EventPriority::Lowest, false)?;
        context.register_event_handler(Payload, EventPriority::Normal, false)?;
        context.register_command(Command::new(&["vlist".to_string()], "Players using voice chat").execute(Cmd::List), PERM_LIST);
        context.register_command(
            Command::new(&["vrc".to_string(), "vreconnect".to_string()], "Reconnect to voice chat").execute(Cmd::Reconnect),
            PERM_RECONNECT,
        );

        // the voice loop: drain UDP, keep-alives, handshake retries, every tick
        let id = context.schedule_repeating_task(1, 1, |server| {
            refresh_positions(&server);
            with(Voice::tick);
            flush(&server);
        });
        TICK_TASK.store(id, Ordering::Relaxed);
        tracing::info!("Ace Plasmo {} loaded", env!("CARGO_PKG_VERSION"));
        Ok(())
    }

    fn on_unload(&self, _context: Context) -> pumpkin_plugin_api::Result<()> {
        let id = TICK_TASK.swap(u32::MAX, Ordering::Relaxed);
        if id != u32::MAX {
            pumpkin_plugin_api::scheduler::cancel_task(id);
        }
        // dropping the state closes the UDP socket
        STATE.lock().unwrap_or_else(|e| e.into_inner()).take();
        tracing::info!("Ace Plasmo unloaded");
        Ok(())
    }
}

/// Copy each connected player's position into the state (host calls, outside the lock).
fn refresh_positions(server: &Server) {
    let ids = with(|v| v.tracked()).unwrap_or_default();
    let mut found = Vec::with_capacity(ids.len());
    for id in ids {
        let pos = server.get_player_by_uuid(id.to_api()).map(|p| {
            let (x, y, z) = p.get_position();
            Pos { world: p.get_world().get_id(), x, y, z }
        });
        found.push((id, pos));
    }
    with(|v| {
        for (id, pos) in found {
            if let Some(p) = v.players.get_mut(&id) {
                p.pos = pos;
            }
        }
    });
}

struct Join;
impl EventHandler<PlayerJoinEvent> for Join {
    fn handle(&self, server: Server, event: EventData<PlayerJoinEvent>) -> EventData<PlayerJoinEvent> {
        let (id, name) = (Id::from_api(event.player.get_id()), event.player.get_name());
        with(|v| v.join(id, name));
        flush(&server);
        event
    }
}

struct Leave;
impl EventHandler<PlayerLeaveEvent> for Leave {
    fn handle(&self, server: Server, event: EventData<PlayerLeaveEvent>) -> EventData<PlayerLeaveEvent> {
        let id = Id::from_api(event.player.get_id());
        with(|v| v.leave(id));
        flush(&server);
        event
    }
}

struct Payload;
impl EventHandler<PlayerCustomPayloadEvent> for Payload {
    fn handle(&self, server: Server, event: EventData<PlayerCustomPayloadEvent>) -> EventData<PlayerCustomPayloadEvent> {
        if event.channel == packets::CHANNEL {
            let id = Id::from_api(event.player.get_id());
            // permissions are host calls: read them before taking the lock
            let grants = Grants { speak: event.player.has_permission(PERM_SPEAK), freecam: event.player.has_permission(PERM_FREECAM) };
            with(|v| v.on_payload(id, &event.data, grants));
            flush(&server);
        }
        event
    }
}

#[derive(Clone, Copy)]
enum Cmd {
    List,
    Reconnect,
}

impl CommandHandler for Cmd {
    fn handle(&self, sender: CommandSender, server: Server, _args: ConsumedArgs) -> Result<i32, CommandError> {
        match self {
            Cmd::List => {
                let (total, mut names) = with(|v| {
                    let names: Vec<String> = v.players.values().filter(|p| p.connected()).map(|p| p.name.clone()).collect();
                    (v.players.len(), names)
                })
                .unwrap_or_default();
                names.sort_unstable_by_key(|n| n.to_lowercase());
                let list = if names.is_empty() { "no players with Plasmo Voice installed".to_string() } else { names.join(", ") };
                sender.send_message(TextComponent::text(&format!("Clients ({}/{total}): {list}", names.len())));
                Ok(names.len() as i32)
            }
            Cmd::Reconnect => {
                let Some(player) = sender.as_player() else {
                    sender.send_message(TextComponent::text("Only players can use this command"));
                    return Ok(0);
                };
                let id = Id::from_api(player.get_id());
                with(|v| v.reconnect(id));
                sender.send_message(TextComponent::text("Reconnect packet sent"));
                flush(&server);
                Ok(1)
            }
        }
    }
}

register_plugin!(AcePlasmo);
