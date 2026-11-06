//! Discord → Minecraft: gateway dispatches, the chat relay, the console channel, `!c` commands, the player list,
//! canned responses, DM account linking, and channel topics.
//! (DiscordSRV: DiscordChatListener, DiscordConsoleListener, DiscordAccountLinkListener, ChannelTopicUpdater.)

use crate::discord;
use crate::linking::Link;
use crate::state::{Action, Role, SelfUser, State};
use crate::text::{self, Rich, Value as TV};
use crate::util;
use regex::Regex;
use serde_json::Value;

pub fn on_dispatch(s: &mut State, t: &str, d: Value) {
    match t {
        "READY" => on_ready(s, &d),
        "GUILD_CREATE" => on_guild(s, &d),
        "GUILD_ROLE_CREATE" | "GUILD_ROLE_UPDATE" => {
            if is_our_guild(s, &d["guild_id"]) {
                insert_role(s, &d["role"]);
            }
        }
        "GUILD_ROLE_DELETE" => {
            if let Some(id) = d["role_id"].as_str() {
                s.roles.remove(id);
            }
        }
        "CHANNEL_CREATE" | "CHANNEL_UPDATE" => {
            if let (Some(id), Some(name)) = (d["id"].as_str(), d["name"].as_str()) {
                s.channel_names.insert(id.into(), name.into());
            }
        }
        "MESSAGE_CREATE" => on_message(s, &d),
        _ => {}
    }
}

fn is_our_guild(s: &State, id: &Value) -> bool {
    id.as_str().is_some_and(|id| s.guild_id.as_deref() == Some(id))
}

fn insert_role(s: &mut State, r: &Value) {
    if let Some(id) = r["id"].as_str() {
        s.roles.insert(id.into(), Role {
            name: r["name"].as_str().unwrap_or_default().into(),
            color: r["color"].as_u64().unwrap_or(0) as u32,
            position: r["position"].as_i64().unwrap_or(0),
        });
    }
}

fn on_ready(s: &mut State, d: &Value) {
    let user = &d["user"];
    let id = user["id"].as_str().unwrap_or_default().to_string();
    let avatar_url = match user["avatar"].as_str() {
        Some(hash) => format!("https://cdn.discordapp.com/avatars/{id}/{hash}.png"),
        None => "https://cdn.discordapp.com/embed/avatars/0.png".into(),
    };
    let name = user["global_name"].as_str().or(user["username"].as_str()).unwrap_or("Ace SRV").to_string();
    tracing::info!("Ace SRV connected to Discord as {name}");
    s.me = Some(SelfUser { id, name, avatar_url });

    let configured = s.cfg.guild_id.trim().to_string();
    s.guild_id = if crate::config::is_snowflake(&configured) {
        Some(configured)
    } else {
        d["guilds"].as_array().and_then(|g| g.first()).and_then(|g| g["id"].as_str()).map(String::from)
    };
    if s.guild_id.is_none() {
        tracing::warn!("The Ace SRV bot is not in any Discord server yet. Invite it with the OAuth2 URL generator (scope: bot).");
    }
    if s.cfg.chat.webhook_delivery || any_event_webhook(s) {
        if let Some(ch) = s.cfg.channel("global") {
            discord::ensure_webhook(s, &ch);
        }
    }
}

fn any_event_webhook(s: &State) -> bool {
    let m = &s.msgs;
    [&m.join, &m.first_join, &m.leave, &m.death, &m.advancement].iter().any(|e| e.enabled && e.webhook.enabled)
}

fn on_guild(s: &mut State, d: &Value) {
    if !is_our_guild(s, &d["id"]) {
        return;
    }
    s.roles.clear();
    if let Some(roles) = d["roles"].as_array() {
        for r in roles {
            insert_role(s, r);
        }
    }
    for list in [&d["channels"], &d["threads"]] {
        if let Some(chs) = list.as_array() {
            for c in chs {
                if let (Some(id), Some(name)) = (c["id"].as_str(), c["name"].as_str()) {
                    s.channel_names.insert(id.into(), name.into());
                }
            }
        }
    }
}

fn display_name(message: &Value) -> String {
    let author = &message["author"];
    message["member"]["nick"]
        .as_str()
        .or(author["global_name"].as_str())
        .or(author["username"].as_str())
        .unwrap_or("Unknown")
        .to_string()
}

fn member_role_ids(message: &Value) -> Vec<String> {
    message["member"]["roles"].as_array().map(|r| r.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default()
}

fn on_message(s: &mut State, m: &Value) {
    let author_id = m["author"]["id"].as_str().unwrap_or_default().to_string();
    if s.me.as_ref().is_some_and(|me| me.id == author_id) {
        return;
    }
    // remember names for @mentions typed in game
    if s.cfg.chat.translate_mentions && !m["author"]["bot"].as_bool().unwrap_or(false) {
        let name = display_name(m).to_lowercase();
        s.member_ids.insert(name, author_id.clone());
        if let Some(u) = m["author"]["username"].as_str() {
            s.member_ids.insert(u.to_lowercase(), author_id.clone());
        }
    }
    let channel = m["channel_id"].as_str().unwrap_or_default().to_string();
    if m["guild_id"].is_null() {
        on_direct_message(s, m, &author_id, &channel);
        return;
    }
    if !is_our_guild(s, &m["guild_id"]) {
        return;
    }
    if s.cfg.console_channel().as_deref() == Some(channel.as_str()) {
        on_console_message(s, m, &author_id, &channel);
        return;
    }
    if s.cfg.channels.values().any(|id| *id == channel) {
        on_chat_message(s, m, &author_id, &channel);
    }
}

fn role_names(s: &State, ids: &[String]) -> Vec<String> {
    ids.iter().filter_map(|id| s.roles.get(id).map(|r| r.name.clone())).collect()
}

fn has_any_role(names: &[String], wanted: &[String]) -> bool {
    names.iter().any(|n| wanted.iter().any(|w| w.eq_ignore_ascii_case(n)))
}

fn on_chat_message(s: &mut State, m: &Value, author_id: &str, channel: &str) {
    let chat = s.cfg.chat.clone();
    let message_id = m["id"].as_str().unwrap_or_default().to_string();
    let is_bot = m["author"]["bot"].as_bool().unwrap_or(false);
    if (chat.block_bots && is_bot) || (chat.block_webhooks && !m["webhook_id"].is_null()) {
        return;
    }
    if chat.blocked_ids.iter().any(|id| id == author_id) {
        return;
    }
    let role_ids = member_role_ids(m);
    let roles = role_names(s, &role_ids);
    if !chat.blocked_roles.is_empty() {
        let hit = role_ids.iter().any(|id| chat.blocked_roles.contains(id)) || has_any_role(&roles, &chat.blocked_roles);
        if hit != chat.blocked_roles_whitelist {
            return;
        }
    }
    let content = m["content"].as_str().unwrap_or_default().to_string();
    let trimmed = content.trim();

    // canned responses
    if let Some((_, answer)) = chat.canned_responses.iter().find(|(k, _)| k.eq_ignore_ascii_case(trimmed)) {
        discord::reply(s, channel, &message_id, answer);
        return;
    }

    // player list
    let pl = &chat.player_list;
    if pl.enabled && trimmed.eq_ignore_ascii_case(&pl.trigger) {
        let reply = player_list(s);
        let (expire, delete_request) = (pl.expiration, pl.delete_request);
        let ch = channel.to_string();
        let body = serde_json::json!({ "content": reply, "allowed_mentions": discord::allowed_mentions(&[]) });
        discord::send_message_then(s, channel, body, move |s, id| {
            if let Some(id) = id {
                s.delete_later(&ch, &id, expire);
            }
        });
        if delete_request {
            s.delete_later(channel, &message_id, expire);
        }
        return;
    }

    // !c console commands
    let cc = &chat.console_command;
    if cc.enabled && !cc.prefix.is_empty() {
        if let Some(rest) = trimmed.strip_prefix(&cc.prefix) {
            if rest.starts_with(' ') || rest.is_empty() {
                let command = rest.trim().trim_start_matches('/').to_string();
                if command.is_empty() {
                    return;
                }
                if !has_any_role(&roles, &cc.roles_allowed) {
                    discord::react(s, channel, &message_id, "⛔");
                    return;
                }
                let first = command.split_whitespace().next().unwrap_or_default().to_lowercase();
                let listed = cc.whitelist.iter().any(|c| c.eq_ignore_ascii_case(&first));
                let bypass = has_any_role(&roles, &cc.whitelist_bypass_roles);
                if !bypass && (listed == cc.whitelist_is_blacklist) {
                    discord::react(s, channel, &message_id, "⛔");
                    return;
                }
                run_console(s, &command, &display_name(m), author_id);
                discord::react(s, channel, &message_id, "✅");
                s.delete_later(channel, &message_id, cc.expiration);
                return;
            }
        }
    }

    if !chat.discord_to_minecraft {
        return;
    }
    if chat.require_linked && s.links.by_discord(author_id).is_none() {
        return;
    }
    let mut body = content.clone();
    if !chat.prefix_required.is_empty() {
        let starts = body.starts_with(&chat.prefix_required);
        if chat.prefix_blacklist {
            if starts {
                return;
            }
        } else if !starts {
            return;
        } else {
            body = body[chat.prefix_required.len()..].trim_start().to_string();
        }
    }

    body = translate_discord_markup(s, m, &body);
    for (pattern, replacement) in &chat.discord_filters {
        match Regex::new(pattern) {
            Ok(re) => body = re.replace_all(&body, replacement.as_str()).into_owned(),
            Err(e) => tracing::warn!("Invalid discord_filters regex {pattern:?}: {e}"),
        }
    }
    let attachments: Vec<String> = m["attachments"]
        .as_array()
        .map(|a| a.iter().filter_map(|x| x["url"].as_str().map(String::from)).collect())
        .unwrap_or_default();
    body = util::truncate(body.trim(), chat.truncate_length);
    if body.is_empty() && attachments.is_empty() {
        return;
    }

    // roles shown in game
    let mut shown: Vec<&Role> = role_ids
        .iter()
        .filter_map(|id| s.roles.get(id))
        .filter(|r| {
            if chat.role_selection.is_empty() {
                return true;
            }
            let listed = chat.role_selection.iter().any(|n| n.eq_ignore_ascii_case(&r.name));
            listed == chat.role_selection_whitelist
        })
        .collect();
    shown.sort_by(|a, b| b.position.cmp(&a.position));
    let alias = |name: &str| chat.role_aliases.get(name).cloned().unwrap_or_else(|| name.to_string());
    let top = shown.first();
    let top_color = shown.iter().find(|r| r.color != 0).map_or(0, |r| r.color);
    let all_roles = shown.iter().map(|r| alias(&r.name)).collect::<Vec<_>>().join(&s.msgs.discord_to_minecraft.all_roles_separator);

    let reply = match m["referenced_message"]["author"].as_object() {
        Some(_) => {
            let name = display_name(&m["referenced_message"]);
            text::render(&s.msgs.discord_to_minecraft.reply_format, &[("name", TV::Plain(name))])
        }
        None => Rich::default(),
    };

    let mut message = if has_any_role(&roles, &chat.color_code_roles) { text::legacy(&body) } else { Rich::plain_text(body.clone()) };
    for url in &attachments {
        if !message.0.is_empty() {
            message.0.push(text::Seg { text: " ".into(), style: text::Style::default() });
        }
        message.0.push(text::Seg {
            text: url.clone(),
            style: text::Style { underlined: true, link: Some(url.clone()), ..text::Style::default() },
        });
    }

    let d2m = &s.msgs.discord_to_minecraft;
    let format = if top.is_some() { &d2m.format } else { &d2m.format_no_role };
    // the role colour is trusted (set by the guild), so it becomes a real tag before parsing
    let format = format.replace("%toprolecolor%", &text::role_color_tag(top_color));
    let name = display_name(m);
    let line = text::render(&format, &[
        ("name", TV::Plain(name.clone())),
        ("username", TV::Plain(m["author"]["username"].as_str().unwrap_or_default().into())),
        ("toprole", TV::Plain(top.map(|r| r.name.clone()).unwrap_or_default())),
        ("toprolealias", TV::Plain(top.map(|r| alias(&r.name)).unwrap_or_default())),
        ("allroles", TV::Plain(all_roles)),
        ("channelname", TV::Plain(s.channel_names.get(channel).cloned().unwrap_or_default())),
        ("reply", TV::Rich(reply)),
        ("message", TV::Rich(message)),
    ]);
    s.actions.push(Action::Broadcast(line));
}

/// <@id>, <@&id>, <#id> and custom emotes → readable text; unicode emoji optionally removed.
fn translate_discord_markup(s: &State, m: &Value, input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        let Some(end) = after.find('>') else {
            out.push_str(after);
            rest = "";
            break;
        };
        let inner = &after[1..end];
        let replaced = if !s.cfg.chat.translate_mentions {
            None
        } else if let Some(id) = inner.strip_prefix("@&") {
            s.roles.get(id).map(|r| format!("@{}", r.name))
        } else if let Some(id) = inner.strip_prefix('@') {
            let id = id.trim_start_matches('!');
            m["mentions"].as_array().and_then(|list| {
                list.iter().find(|u| u["id"].as_str() == Some(id)).map(|u| {
                    let name = u["member"]["nick"].as_str().or(u["global_name"].as_str()).or(u["username"].as_str()).unwrap_or(id);
                    format!("@{name}")
                })
            })
        } else if let Some(id) = inner.strip_prefix('#') {
            s.channel_names.get(id).map(|n| format!("#{n}"))
        } else {
            None
        };
        let replaced = replaced.or_else(|| {
            // <:name:id> and <a:name:id>
            let parts: Vec<&str> = inner.split(':').collect();
            (parts.len() == 3 && (parts[0].is_empty() || parts[0] == "a") && parts[2].bytes().all(|b| b.is_ascii_digit()))
                .then(|| format!(":{}:", parts[1]))
        });
        match replaced {
            Some(r) => out.push_str(&r),
            None => out.push_str(&after[..=end]),
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    if s.cfg.chat.emoji.eq_ignore_ascii_case("hide") {
        out.retain(|c| !is_emoji(c));
    }
    out
}

fn is_emoji(c: char) -> bool {
    matches!(c as u32, 0x1F000..=0x1FAFF | 0x2600..=0x27BF | 0xFE0F | 0x200D)
}

pub fn player_list(s: &State) -> String {
    let pl = &s.msgs.player_list;
    let vars = [("playercount", s.online.len().to_string()), ("playermax", s.max_players.to_string())];
    if s.online.is_empty() {
        return util::fill(&pl.empty, &vars);
    }
    let mut names: Vec<String> = s
        .online
        .iter()
        .map(|p| util::fill(&pl.player, &[("displayname", escape_markdown(&p.name)), ("username", escape_markdown(&p.name))]))
        .collect();
    names.sort_by_key(|n| n.to_lowercase());
    util::truncate(&format!("{}\n{}", util::fill(&pl.header, &vars), names.join(&pl.separator)), 2000)
}

pub fn escape_markdown(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if "*_~`|>\\".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn run_console(s: &mut State, command: &str, who: &str, who_id: &str) {
    tracing::info!("Discord user {who} ({who_id}) ran console command: {command}");
    if !s.cfg.console.usage_log.trim().is_empty() {
        let file = util::fill(&s.cfg.console.usage_log, &[("date", util::date())]);
        util::append_line(&s.path(&file), &format!("[{} {}] {who} ({who_id}): {command}", util::date(), util::time()));
    }
    s.actions.push(Action::Console(command.to_string()));
}

fn on_console_message(s: &mut State, m: &Value, author_id: &str, channel: &str) {
    if s.cfg.console.block_bots && (m["author"]["bot"].as_bool().unwrap_or(false) || !m["webhook_id"].is_null()) {
        return;
    }
    let message_id = m["id"].as_str().unwrap_or_default().to_string();
    let command = m["content"].as_str().unwrap_or_default().trim().trim_start_matches('/').to_string();
    if command.is_empty() {
        return;
    }
    let first = command.split_whitespace().next().unwrap_or_default().to_lowercase();
    let listed = s.cfg.console.blacklist.iter().any(|c| c.eq_ignore_ascii_case(&first));
    if listed != s.cfg.console.blacklist_is_whitelist {
        discord::react(s, channel, &message_id, "⛔");
        return;
    }
    run_console(s, &command, &display_name(m), author_id);
    discord::react(s, channel, &message_id, "✅");
}

fn on_direct_message(s: &mut State, m: &Value, author_id: &str, channel: &str) {
    if m["author"]["bot"].as_bool().unwrap_or(false) {
        return;
    }
    let content = m["content"].as_str().unwrap_or_default().trim().to_string();
    let discord_name = m["author"]["global_name"].as_str().or(m["author"]["username"].as_str()).unwrap_or("Unknown").to_string();
    let msgs = s.msgs.linking.clone();

    if !(content.len() == 4 && content.bytes().all(|b| b.is_ascii_digit())) {
        discord::send_text(s, channel, &msgs.dm_help);
        return;
    }
    let Some(pending) = s.codes.remove(&content) else {
        discord::send_text(s, channel, &msgs.invalid_code);
        return;
    };
    if let Some(existing) = s.links.by_discord(author_id) {
        if !s.cfg.linking.allow_relink {
            discord::send_text(s, channel, &util::fill(&msgs.already_linked_discord, &[("name", existing.name)]));
            return;
        }
    }
    s.links.insert(author_id, Link { uuid: pending.uuid.clone(), name: pending.name.clone(), discord_name: discord_name.clone() });
    let vars = [
        ("name", pending.name.clone()),
        ("uuid", pending.uuid.clone()),
        ("discordname", discord_name.clone()),
        ("discordid", author_id.to_string()),
    ];
    discord::send_text(s, channel, &util::fill(&msgs.linked_discord, &vars));
    s.actions.push(Action::Tell(
        pending.uuid.clone(),
        text::render(&msgs.linked_game, &[("discordname", TV::Plain(discord_name.clone()))]),
    ));
    let role = s.cfg.linking.linked_role.clone();
    if !role.trim().is_empty() {
        match discord::role_id_by_name(s, &role) {
            Some(id) => discord::set_role(s, author_id, &id, true),
            None => tracing::warn!("linking.linked_role {role:?} does not exist in the Discord server"),
        }
    }
    for cmd in s.cfg.linking.linked_commands.clone() {
        let cmd = util::fill(&cmd, &vars);
        if !cmd.trim().is_empty() {
            s.actions.push(Action::Console(cmd));
        }
    }
    tracing::info!("Linked {} ({}) to Discord user {discord_name} ({author_id})", pending.name, pending.uuid);
}

/// Unlink from the game side: role removal and unlink commands.
pub fn unlinked(s: &mut State, discord_id: &str, link: &Link) {
    let role = s.cfg.linking.linked_role.clone();
    if !role.trim().is_empty() {
        if let Some(id) = discord::role_id_by_name(s, &role) {
            discord::set_role(s, discord_id, &id, false);
        }
    }
    let vars = [
        ("name", link.name.clone()),
        ("uuid", link.uuid.clone()),
        ("discordname", link.discord_name.clone()),
        ("discordid", discord_id.to_string()),
    ];
    for cmd in s.cfg.linking.unlinked_commands.clone() {
        let cmd = util::fill(&cmd, &vars);
        if !cmd.trim().is_empty() {
            s.actions.push(Action::Console(cmd));
        }
    }
}

pub fn update_topics(s: &mut State, shutdown: bool) {
    let t = s.msgs.topic.clone();
    let vars = [
        ("playercount", s.online.len().to_string()),
        ("playermax", s.max_players.to_string()),
        ("totalplayers", s.known.uuids.len().to_string()),
        ("uptimemins", (s.started.elapsed().as_secs() / 60).to_string()),
        ("uptimehours", (s.started.elapsed().as_secs() / 3600).to_string()),
        ("tps", format!("{:.1}", s.tps)),
        ("mspt", format!("{:.1}", s.mspt)),
        ("serverversion", s.server_version.clone()),
        ("date", util::date()),
        ("time", util::time()),
    ];
    if let Some(ch) = s.cfg.channel("global") {
        let topic = util::fill(if shutdown { &t.chat_shutdown } else { &t.chat }, &vars);
        if !topic.trim().is_empty() {
            discord::set_topic(s, &ch, &topic);
        }
    }
    if let Some(ch) = s.cfg.console_channel() {
        let topic = util::fill(if shutdown { &t.console_shutdown } else { &t.console }, &vars);
        if !topic.trim().is_empty() {
            discord::set_topic(s, &ch, &topic);
        }
    }
}
