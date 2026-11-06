//! # Ace SRV
//!
//! A Discord ⇄ Minecraft bridge for [Pumpkin](https://github.com/Pumpkin-MC/Pumpkin), translated from
//! [DiscordSRV](https://github.com/DiscordSRV/DiscordSRV) (GPL-3.0). See PORTING.md for what maps to what.
//!
//! Copyright (C) Ace Studio. This program is free software: you can redistribute it and/or modify it under the terms
//! of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or
//! (at your option) any later version.

mod bridge;
mod commands;
mod config;
mod discord;
mod game;
mod gateway;
mod http;
mod linking;
mod state;
mod text;
mod util;

use pumpkin_plugin_api::permissions;
use pumpkin_plugin_api::scheduler::SchedulerExt;
use pumpkin_plugin_api::{Context, Plugin, PluginMetadata, register_plugin};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Duration;

static PUMP_TASK: AtomicU32 = AtomicU32::new(u32::MAX);
static TICKS: AtomicU64 = AtomicU64::new(0);

struct AceSrv;

impl Plugin for AceSrv {
    fn new() -> Self {
        AceSrv
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
            authors: vec!["Ace Studio".into(), "DiscordSRV contributors (original)".into()],
            description: "Ace SRV: Discord ⇄ Minecraft chat bridge, ported from DiscordSRV".into(),
            dependencies: vec![],
            permissions: vec![
                // config, messages, linked accounts
                permissions::FS_WRITE_DATA.into(),
                permissions::FS_READ_DATA.into(),
                // Discord REST API and webhooks
                permissions::HTTP_OUTBOUND.into(),
                // Discord gateway WebSocket
                permissions::NETWORK_DNS.into(),
                permissions::NETWORK_TCP_CONNECT.into(),
            ],
        }
    }

    fn on_load(&self, context: Context) -> pumpkin_plugin_api::Result<()> {
        let data = context.get_data_folder();
        let (cfg, msgs) = config::load_all(&data)?;
        let has_token = cfg.has_token();
        *state::STATE.lock().unwrap_or_else(|e| e.into_inner()) = Some(state::State::new(data, cfg, msgs));

        game::register(&context)?;
        commands::register(&context)?;

        let server = context.get_server();
        let max = server.get_max_players();
        let online: Vec<state::OnlinePlayer> = server
            .get_all_players()
            .iter()
            .map(|p| state::OnlinePlayer { uuid: pumpkin_plugin_api::uuid::to_string(p.get_id()), name: p.get_name() })
            .collect();
        state::with(|s| {
            s.max_players = max;
            s.online = online;
        });

        // the bridge runs on a per-tick pump; everything inside is non-blocking
        let id = context.schedule_repeating_task(1, 1, |server| {
            let n = TICKS.fetch_add(1, Ordering::Relaxed);
            if n % 100 == 0 {
                let (tps, mspt) = (server.get_tps(), server.get_mspt());
                state::with(|s| {
                    s.tps = tps;
                    s.mspt = mspt;
                });
            }
            state::with(state::State::tick);
            state::flush_actions(&server);
        });
        PUMP_TASK.store(id, Ordering::Relaxed);

        if has_token {
            tracing::info!("Ace SRV {} loaded; connecting to Discord", env!("CARGO_PKG_VERSION"));
        }
        Ok(())
    }

    fn on_unload(&self, _context: Context) -> pumpkin_plugin_api::Result<()> {
        let id = PUMP_TASK.swap(u32::MAX, Ordering::Relaxed);
        if id != u32::MAX {
            pumpkin_plugin_api::scheduler::cancel_task(id);
        }
        // last words: shutdown message and offline topics, sent synchronously with a short deadline
        let state = state::STATE.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(mut s) = state {
            if s.cfg.has_token() {
                if let Some(ch) = s.cfg.channel("global") {
                    let text = s.msgs.server.shutdown.clone();
                    discord::send_text(&mut s, &ch, &text);
                }
                if s.cfg.topic.enabled && s.cfg.topic.at_shutdown {
                    bridge::update_topics(&mut s, true);
                }
                let done = s.http.wait_blocking(Duration::from_secs(3));
                s.run_callbacks(done);
            }
            s.gw.shutdown();
            if s.known_dirty {
                util::write_json(&s.path("players.json"), &s.known);
            }
        }
        tracing::info!("Ace SRV unloaded");
        Ok(())
    }
}

register_plugin!(AceSrv);
