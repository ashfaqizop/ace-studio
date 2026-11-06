//! `/discord` (alias `/acesrv`): invite, link, unlink, linked, reload, broadcast.
//! (DiscordSRV: CommandManager with CommandHelp/Link/Unlink/Linked/Reload/Broadcast.)

use crate::state::{self, PendingCode, State};
use crate::text::{self, Rich, Value as TV};
use crate::{bridge, discord, util};
use pumpkin_plugin_api::command::{Command, CommandError, CommandNode, CommandSender, ConsumedArgs};
use pumpkin_plugin_api::command_wit::{Arg, ArgumentType, StringType};
use pumpkin_plugin_api::commands::CommandHandler;
use pumpkin_plugin_api::permission::{Permission, PermissionDefault, PermissionLevel};
use pumpkin_plugin_api::{Context, Server};
use std::time::{Duration, Instant};

pub const PERM_USE: &str = "ace-srv:command.discord";
pub const PERM_LINK: &str = "ace-srv:command.link";
pub const PERM_ADMIN: &str = "ace-srv:command.admin";

pub fn register(context: &Context) -> pumpkin_plugin_api::Result<()> {
    for (node, description, default) in [
        (PERM_USE, "Use /discord", PermissionDefault::Allow),
        (PERM_LINK, "Link a Discord account with /discord link", PermissionDefault::Allow),
        (PERM_ADMIN, "Reload Ace SRV and broadcast to Discord", PermissionDefault::Op(PermissionLevel::Two)),
    ] {
        context.register_permission(&Permission {
            node: node.into(),
            description: description.into(),
            default,
            children: vec![],
        })?;
    }

    let command = Command::new(&["discord".to_string(), "acesrv".to_string()], "Ace SRV: Discord bridge")
        .then(CommandNode::literal("link").execute(Sub::Link))
        .then(CommandNode::literal("unlink").execute(Sub::Unlink))
        .then(CommandNode::literal("linked").execute(Sub::Linked))
        .then(CommandNode::literal("reload").execute(Sub::Reload))
        .then(CommandNode::literal("help").execute(Sub::Help))
        .then(
            CommandNode::literal("broadcast")
                .then(CommandNode::argument("message", &ArgumentType::String(StringType::Greedy)).execute(Sub::Broadcast)),
        )
        .execute(Sub::Invite);
    context.register_command(command, PERM_USE);
    Ok(())
}

#[derive(Clone, Copy)]
enum Sub {
    Invite,
    Help,
    Link,
    Unlink,
    Linked,
    Reload,
    Broadcast,
}

fn say(sender: &CommandSender, rich: Rich) {
    sender.send_message(rich.to_component());
}

fn msg(template: &str, vars: &[(&str, String)]) -> Rich {
    let vars: Vec<(&str, TV)> = vars.iter().map(|(k, v)| (*k, TV::Plain(v.clone()))).collect();
    text::render(template, &vars)
}

impl CommandHandler for Sub {
    fn handle(&self, sender: CommandSender, server: Server, args: ConsumedArgs) -> Result<i32, CommandError> {
        let allowed = |node: &str| sender.is_console() || sender.has_permission(&server, node);
        let Some(m) = state::with(|s| s.msgs.commands.clone()) else { return Ok(0) };
        match self {
            Sub::Invite => {
                let invite = state::with(|s| s.cfg.invite_link.clone()).unwrap_or_default();
                let mut line = msg(&m.invite, &[("invite", invite.clone())]);
                for seg in &mut line.0 {
                    if seg.text.contains(&invite) && !invite.is_empty() {
                        seg.style.link = Some(invite.clone());
                    }
                }
                say(&sender, line);
                say(&sender, msg(&m.help, &[]));
            }
            Sub::Help => say(&sender, msg(&m.help, &[])),
            Sub::Link => {
                if !allowed(PERM_LINK) {
                    say(&sender, msg(&m.no_permission, &[]));
                    return Ok(0);
                }
                let Some(player) = sender.as_player() else {
                    say(&sender, msg(&m.players_only, &[]));
                    return Ok(0);
                };
                let uuid = pumpkin_plugin_api::uuid::to_string(player.get_id());
                let name = player.get_name();
                if let Some(reply) = state::with(|s| link_code(s, &uuid, &name)) {
                    say(&sender, reply);
                }
            }
            Sub::Unlink => {
                let Some(player) = sender.as_player() else {
                    say(&sender, msg(&m.players_only, &[]));
                    return Ok(0);
                };
                let uuid = pumpkin_plugin_api::uuid::to_string(player.get_id());
                let reply = state::with(|s| match s.links.remove_uuid(&uuid) {
                    Some((discord_id, link)) => {
                        bridge::unlinked(s, &discord_id, &link);
                        msg(&s.msgs.linking.unlinked, &[])
                    }
                    None => msg(&s.msgs.linking.not_linked, &[]),
                });
                if let Some(reply) = reply {
                    say(&sender, reply);
                }
                state::flush_actions(&server);
            }
            Sub::Linked => {
                let Some(player) = sender.as_player() else {
                    say(&sender, msg(&m.players_only, &[]));
                    return Ok(0);
                };
                let uuid = pumpkin_plugin_api::uuid::to_string(player.get_id());
                let reply = state::with(|s| match s.links.by_uuid(&uuid) {
                    Some((id, link)) => {
                        msg(&s.msgs.linking.linked_info, &[("discordname", link.discord_name), ("discordid", id)])
                    }
                    None => msg(&s.msgs.linking.not_linked, &[]),
                });
                if let Some(reply) = reply {
                    say(&sender, reply);
                }
            }
            Sub::Reload => {
                if !allowed(PERM_ADMIN) {
                    say(&sender, msg(&m.no_permission, &[]));
                    return Ok(0);
                }
                match state::with(reload).unwrap_or_else(|| Err("not loaded".into())) {
                    Ok(()) => say(&sender, msg(&m.reloaded, &[])),
                    Err(e) => say(&sender, Rich::plain_text(format!("Ace SRV reload failed: {e}"))),
                }
            }
            Sub::Broadcast => {
                if !allowed(PERM_ADMIN) {
                    say(&sender, msg(&m.no_permission, &[]));
                    return Ok(0);
                }
                let text = match args.get_value("message") {
                    Arg::Msg(t) | Arg::Simple(t) => t,
                    _ => String::new(),
                };
                if text.trim().is_empty() {
                    return Ok(0);
                }
                state::with(|s| {
                    if let Some(ch) = s.cfg.channel("global") {
                        discord::send_text(s, &ch, &text);
                    }
                });
                say(&sender, msg(&m.broadcast_sent, &[]));
            }
        }
        Ok(1)
    }
}

fn link_code(s: &mut State, uuid: &str, name: &str) -> Rich {
    if let Some((_, link)) = s.links.by_uuid(uuid) {
        return msg(&s.msgs.linking.already_linked, &[("discordname", link.discord_name)]);
    }
    if !s.gw.ready {
        return msg(&s.msgs.linking.not_connected, &[]);
    }
    // reuse a live code for the same player
    let existing = s.codes.iter().find(|(_, c)| c.uuid == uuid).map(|(code, _)| code.clone());
    let code = existing.unwrap_or_else(|| loop {
        let code = format!("{:04}", util::random_u64() % 10_000);
        if !s.codes.contains_key(&code) {
            break code;
        }
    });
    let minutes = s.cfg.linking.code_expiry_minutes.max(1);
    s.codes.insert(code.clone(), PendingCode {
        uuid: uuid.into(),
        name: name.into(),
        expires: Instant::now() + Duration::from_secs(minutes * 60),
    });
    msg(&s.msgs.linking.code, &[("code", code), ("botname", s.bot_name()), ("minutes", minutes.to_string())])
}

/// Re-reads config.toml and messages.toml; reconnects to Discord if the token changed.
pub fn reload(s: &mut State) -> Result<(), String> {
    let (cfg, msgs) = crate::config::load_all(&s.data)?;
    let token_changed = cfg.bot_token != s.cfg.bot_token;
    s.http.debug = cfg.debug;
    s.cfg = cfg;
    s.msgs = msgs;
    s.warned_no_token = false;
    if token_changed {
        s.gw.reset();
        s.me = None;
    }
    s.gw.debug = s.cfg.debug;
    Ok(())
}
