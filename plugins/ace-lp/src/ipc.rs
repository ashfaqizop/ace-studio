//! The Ace LP API for other plugins, over Pumpkin IPC (`ipc.send-ipc-message("ace-lp", json)`), standing in for
//! LuckPerms' Java API / Vault for the most common reads. Request and response are JSON:
//!
//! ```json
//! { "op": "user", "uuid": "069a79f4-44e9-4726-a5be-fca90e38aaf5" }
//! → { "ok": true, "name": "Notch", "primary_group": "admin", "groups": ["admin", "default"],
//!     "prefix": "&c[Admin] ", "suffix": null, "meta": { "homes": "5" } }
//! { "op": "has_group", "uuid": "…", "group": "vip" }   → { "ok": true, "value": true }
//! { "op": "group", "group": "vip" }                      → { "ok": true, "weight": 20, "display_name": null, "parents": [...] }
//! ```
//! Contexts are the player's current ones when online, otherwise global.

use crate::state::{now, with};
use serde_json::{Value, json};

pub fn handle(_sender: &str, message: &[u8]) -> Result<Vec<u8>, String> {
    let req: Value = serde_json::from_slice(message).map_err(|e| format!("Ace LP expects JSON: {e}"))?;
    let op = req.get("op").and_then(Value::as_str).unwrap_or("");
    let res = with(|lp| match op {
        "user" | "has_group" => {
            let who = req.get("uuid").or_else(|| req.get("name")).and_then(Value::as_str).unwrap_or("");
            let Some(uuid) = lp.find_user(who) else { return json!({ "ok": false, "error": "unknown user" }) };
            let out = {
                let u = &lp.users[&uuid];
                let q = lp.query(&uuid);
                let g = lp.groups();
                let groups = g.inherited_groups(&u.data, &q, now(), Some(&u.primary_group));
                if op == "has_group" {
                    let want = req.get("group").and_then(Value::as_str).unwrap_or("").to_ascii_lowercase();
                    json!({ "ok": true, "value": groups.contains(&want) })
                } else {
                    let meta = g.meta(&u.data, &q, now(), Some(&u.primary_group));
                    json!({
                        "ok": true,
                        "name": u.name,
                        "primary_group": g.primary_group(u, &q, now()),
                        "groups": groups,
                        "prefix": meta.prefix,
                        "suffix": meta.suffix,
                        "meta": meta.meta,
                    })
                }
            };
            if !lp.online.contains_key(&uuid) {
                lp.users.remove(&uuid);
            }
            out
        }
        "group" => {
            let name = req.get("group").and_then(Value::as_str).unwrap_or("").to_ascii_lowercase();
            match lp.groups.get(&name) {
                Some(g) => json!({
                    "ok": true,
                    "weight": g.data.weight(),
                    "display_name": g.data.display_name(),
                    "parents": g.data.parents().map(|(p, _)| p.to_string()).collect::<Vec<_>>(),
                }),
                None => json!({ "ok": false, "error": "unknown group" }),
            }
        }
        _ => json!({ "ok": false, "error": "unknown op; use user, has_group or group" }),
    })
    .ok_or("Ace LP isn't loaded")?;
    Ok(serde_json::to_vec(&res).unwrap_or_default())
}
