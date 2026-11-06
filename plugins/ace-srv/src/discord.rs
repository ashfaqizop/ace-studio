//! Discord REST calls (API v10) built on the non-blocking HTTP client, plus embed building.
//! (DiscordSRV: DiscordUtil, WebhookUtil and the JDA REST layer.)

use crate::config::EventSpec;
use crate::http::{Method, Request};
use crate::state::State;
use crate::util;
use serde_json::{Value, json};

const API: &str = "/api/v10";

fn bot(state: &State, method: Method, path: String) -> Request {
    Request::new(method, format!("{API}{path}")).auth(&state.token())
}

/// Never ping @everyone/@here or roles from relayed text; users only when explicitly listed.
pub fn allowed_mentions(users: &[String]) -> Value {
    json!({ "parse": [], "users": users })
}

pub fn send_message(state: &mut State, channel: &str, body: Value) {
    let req = bot(state, Method::Post, format!("/channels/{channel}/messages")).json(&body);
    state.http.send(req);
}

pub fn send_message_then(
    state: &mut State,
    channel: &str,
    body: Value,
    then: impl FnOnce(&mut State, Option<String>) + Send + 'static,
) {
    let req = bot(state, Method::Post, format!("/channels/{channel}/messages"))
        .json(&body)
        .then(move |s, resp| then(s, resp.json().and_then(|v| v["id"].as_str().map(String::from))));
    state.http.send(req);
}

pub fn send_text(state: &mut State, channel: &str, content: &str) {
    if content.trim().is_empty() {
        return;
    }
    let body = json!({ "content": util::truncate(content, 2000), "allowed_mentions": allowed_mentions(&[]) });
    send_message(state, channel, body);
}

pub fn reply(state: &mut State, channel: &str, message_id: &str, content: &str) {
    let body = json!({
        "content": util::truncate(content, 2000),
        "allowed_mentions": allowed_mentions(&[]),
        "message_reference": { "message_id": message_id, "fail_if_not_exists": false },
    });
    send_message(state, channel, body);
}

pub fn delete_message(state: &mut State, channel: &str, message: &str) {
    let req = bot(state, Method::Delete, format!("/channels/{channel}/messages/{message}"));
    state.http.send(req);
}

pub fn react(state: &mut State, channel: &str, message: &str, emoji: &str) {
    let req = bot(state, Method::Put, format!("/channels/{channel}/messages/{message}/reactions/{}/@me", util::url_encode(emoji)));
    state.http.send(req);
}

pub fn set_topic(state: &mut State, channel: &str, topic: &str) {
    let req = bot(state, Method::Patch, format!("/channels/{channel}")).json(&json!({ "topic": util::truncate(topic, 1024) }));
    state.http.send(req);
}

pub fn set_role(state: &mut State, user: &str, role_id: &str, add: bool) {
    let Some(guild) = state.guild_id.clone() else { return };
    let method = if add { Method::Put } else { Method::Delete };
    let req = bot(state, method, format!("/guilds/{guild}/members/{user}/roles/{role_id}"));
    state.http.send(req);
}

pub fn role_id_by_name(state: &State, name: &str) -> Option<String> {
    state.roles.iter().find(|(_, r)| r.name.eq_ignore_ascii_case(name)).map(|(id, _)| id.clone())
}

/// Makes sure we have a webhook named "Ace SRV" in `channel` (needs Manage Webhooks), creating it if missing.
pub fn ensure_webhook(state: &mut State, channel: &str) {
    if state.webhooks.contains_key(channel) || !state.webhook_pending.insert(channel.to_string()) {
        return;
    }
    let ch = channel.to_string();
    let req = bot(state, Method::Get, format!("/channels/{channel}/webhooks")).then(move |s, resp| {
        let me = s.me.as_ref().map(|m| m.id.clone()).unwrap_or_default();
        let existing = resp.json().and_then(|v| {
            v.as_array()?.iter().find_map(|w| {
                let ours = w["name"].as_str() == Some("Ace SRV") && w["user"]["id"].as_str() == Some(me.as_str());
                let token = w["token"].as_str()?;
                ours.then(|| (w["id"].as_str().unwrap_or_default().to_string(), token.to_string()))
            })
        });
        if let Some(hook) = existing {
            s.webhook_pending.remove(&ch);
            s.webhooks.insert(ch, hook);
            return;
        }
        if !resp.ok() {
            s.webhook_pending.remove(&ch);
            tracing::warn!("Ace SRV cannot read webhooks in channel {ch}; give the bot Manage Webhooks or turn webhook delivery off");
            return;
        }
        let ch2 = ch.clone();
        let create = bot(s, Method::Post, format!("/channels/{ch}/webhooks")).json(&json!({ "name": "Ace SRV" })).then(move |s, resp| {
            s.webhook_pending.remove(&ch2);
            if let Some(v) = resp.json() {
                if let (Some(id), Some(token)) = (v["id"].as_str(), v["token"].as_str()) {
                    s.webhooks.insert(ch2, (id.to_string(), token.to_string()));
                }
            }
        });
        s.http.send(create);
    });
    state.http.send(req);
}

/// Posts through the channel's webhook if there is one; returns false when the caller should fall back to the bot.
pub fn webhook_send(state: &mut State, channel: &str, username: &str, avatar: &str, mut body: Value) -> bool {
    let Some((id, token)) = state.webhooks.get(channel).cloned() else {
        ensure_webhook(state, channel);
        return false;
    };
    // Discord rejects webhook names containing "discord" and names over 80 characters
    let mut name = util::truncate(username, 80).replace("discord", "dіscord").replace("Discord", "Dіscord");
    if name.trim().is_empty() {
        name = "Player".into();
    }
    body["username"] = Value::from(name);
    if !avatar.is_empty() {
        body["avatar_url"] = Value::from(avatar);
    }
    let req = Request::new(Method::Post, format!("{API}/webhooks/{id}/{token}")).json(&body);
    state.http.send(req);
    true
}

fn hex_color(s: &str) -> Option<u32> {
    u32::from_str_radix(s.trim().trim_start_matches('#'), 16).ok()
}

fn set_if(obj: &mut Value, key: &str, value: String) {
    if !value.trim().is_empty() {
        obj[key] = Value::from(value);
    }
}

/// Builds the message body for an announcement, or None if it would be empty.
pub fn event_body(spec: &EventSpec, vars: &[(&str, String)]) -> Option<Value> {
    let f = |s: &str| util::fill(s, vars);
    let mut body = json!({ "allowed_mentions": allowed_mentions(&[]) });
    let content = f(&spec.content);
    let mut has = false;
    if !content.trim().is_empty() {
        body["content"] = Value::from(util::truncate(&content, 2000));
        has = true;
    }
    let e = &spec.embed;
    if e.enabled {
        let mut embed = json!({});
        if let Some(c) = hex_color(&e.color) {
            embed["color"] = Value::from(c);
        }
        let author_name = f(&e.author_name);
        if !author_name.trim().is_empty() {
            let mut author = json!({ "name": util::truncate(&author_name, 256) });
            set_if(&mut author, "icon_url", f(&e.author_image));
            set_if(&mut author, "url", f(&e.author_url));
            embed["author"] = author;
        }
        set_if(&mut embed, "title", util::truncate(&f(&e.title), 256));
        set_if(&mut embed, "url", f(&e.title_url));
        set_if(&mut embed, "description", util::truncate(&f(&e.description), 4096));
        let thumb = f(&e.thumbnail);
        if !thumb.trim().is_empty() {
            embed["thumbnail"] = json!({ "url": thumb });
        }
        let image = f(&e.image);
        if !image.trim().is_empty() {
            embed["image"] = json!({ "url": image });
        }
        let footer = f(&e.footer_text);
        if !footer.trim().is_empty() {
            let mut foot = json!({ "text": util::truncate(&footer, 2048) });
            set_if(&mut foot, "icon_url", f(&e.footer_icon));
            embed["footer"] = foot;
        }
        if e.timestamp {
            embed["timestamp"] = Value::from(util::iso8601());
        }
        if embed.as_object().is_some_and(|o| o.keys().any(|k| k != "color")) {
            body["embeds"] = json!([embed]);
            has = true;
        }
    }
    has.then_some(body)
}
