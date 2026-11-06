//! `data/config.toml`, written from `defaults/config.toml` on first start (Plasmo Voice: `VoiceServerConfig`).

use crate::proto::Id;
use serde::Deserialize;
use std::path::Path;

const DEFAULTS: &str = include_str!("../defaults/config.toml");

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub server_id: String,
    pub host: Host,
    pub voice: Voice,
}

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct Host {
    pub ip: String,
    pub port: u16,
    pub public: Option<Public>,
}

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct Public {
    pub ip: String,
    pub port: u16,
}

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct Voice {
    pub sample_rate: i32,
    pub mtu_size: i32,
    pub keep_alive_timeout_ms: u64,
    pub max_extra_audio_broadcast_distance: i32,
    pub client_mod_min_version: String,
    pub proximity: Proximity,
    pub opus: Opus,
    pub player_icon: PlayerIcon,
}

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct Proximity {
    pub distances: Vec<i32>,
    pub default_distance: i32,
}

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct Opus {
    pub mode: String,
    pub bitrate: i32,
}

#[derive(Deserialize, Clone, Default)]
#[serde(default)]
pub struct PlayerIcon {
    pub visibility: Vec<String>,
    pub y_offset: f64,
}

impl Default for Config {
    fn default() -> Self {
        Config { server_id: String::new(), host: Host::default(), voice: Voice::default() }
    }
}
impl Default for Host {
    fn default() -> Self {
        Host { ip: "0.0.0.0".into(), port: 25565, public: None }
    }
}
impl Default for Public {
    fn default() -> Self {
        Public { ip: "0.0.0.0".into(), port: 0 }
    }
}
impl Default for Voice {
    fn default() -> Self {
        Voice {
            sample_rate: 48_000,
            mtu_size: 1024,
            keep_alive_timeout_ms: 15_000,
            max_extra_audio_broadcast_distance: 16,
            client_mod_min_version: "2.0.0".into(),
            proximity: Proximity::default(),
            opus: Opus::default(),
            player_icon: PlayerIcon::default(),
        }
    }
}
impl Default for Proximity {
    fn default() -> Self {
        Proximity { distances: vec![8, 16, 32], default_distance: 16 }
    }
}
impl Default for Opus {
    fn default() -> Self {
        Opus { mode: "VOIP".into(), bitrate: -1000 }
    }
}

const ICONS: [&str; 5] =
    ["HIDE_NOT_INSTALLED", "HIDE_VOICE_CHAT_DISABLED", "HIDE_SERVER_MUTED", "HIDE_CLIENT_MUTED", "HIDE_SOURCE_ICON"];

impl Config {
    /// Clamp everything to what the client accepts, the way Plasmo Voice's validators do.
    fn validate(&mut self) {
        let v = &mut self.voice;
        if ![8_000, 12_000, 24_000, 48_000].contains(&v.sample_rate) {
            tracing::warn!("voice.sample_rate {} isn't supported, using 48000", v.sample_rate);
            v.sample_rate = 48_000;
        }
        v.mtu_size = v.mtu_size.clamp(128, 5000);
        v.keep_alive_timeout_ms = v.keep_alive_timeout_ms.clamp(1_000, 120_000);
        v.max_extra_audio_broadcast_distance = v.max_extra_audio_broadcast_distance.max(0);
        if !["VOIP", "AUDIO", "RESTRICTED_LOWDELAY"].contains(&v.opus.mode.as_str()) {
            tracing::warn!("voice.opus.mode {:?} isn't supported, using VOIP", v.opus.mode);
            v.opus.mode = "VOIP".into();
        }
        let b = v.opus.bitrate;
        if !(b == -1 || b == -1000 || (500..=512_000).contains(&b)) {
            tracing::warn!("voice.opus.bitrate {b} isn't supported, using -1000 (auto)");
            v.opus.bitrate = -1000;
        }
        let p = &mut v.proximity;
        p.distances.sort_unstable();
        p.distances.dedup();
        p.distances.truncate(64);
        if p.distances.is_empty() {
            p.distances = vec![8, 16, 32];
        }
        p.default_distance = valid_default(&p.distances, p.default_distance);
        v.player_icon.visibility.retain(|n| {
            let ok = ICONS.contains(&n.as_str());
            if !ok {
                tracing::warn!("Unknown voice.player_icon.visibility entry {n:?}");
            }
            ok
        });
        if self.host.ip.trim().is_empty() {
            self.host.ip = "0.0.0.0".into();
        }
    }

    pub fn server_id(&self) -> Id {
        parse_uuid(&self.server_id).unwrap_or_default()
    }

    /// The address and port clients are told to connect to (`ConnectionPacket`).
    pub fn public_address(&self) -> (String, i32) {
        let (mut ip, mut port) = (self.host.ip.clone(), self.host.port);
        if let Some(p) = &self.host.public {
            if p.ip != "0.0.0.0" && !p.ip.is_empty() {
                ip = p.ip.clone();
            }
            if p.port != 0 {
                port = p.port;
            }
        }
        (ip, port as i32)
    }
}

/// `VoiceActivation.validateDefaultDistance`
fn valid_default(distances: &[i32], default: i32) -> i32 {
    if distances.len() == 2 && distances[0] == -1 {
        return if (1..=distances[1]).contains(&default) { default } else { distances[1] / 2 };
    }
    if distances.contains(&default) { default } else { distances[distances.len() / 2] }
}

fn parse_uuid(s: &str) -> Option<Id> {
    let hex: String = s.chars().filter(|c| *c != '-').collect();
    if hex.len() != 32 {
        return None;
    }
    let n = u128::from_str_radix(&hex, 16).ok()?;
    Some(Id((n >> 64) as u64, n as u64))
}

pub fn format_uuid(id: Id) -> String {
    let n = ((id.0 as u128) << 64) | id.1 as u128;
    let h = format!("{n:032x}");
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

/// Read `config.toml`, creating it (with a fresh server id) the first time.
pub fn load(data: &Path) -> Result<Config, String> {
    let path = data.join("config.toml");
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => {
            let t = DEFAULTS.replace("__SERVER_ID__", &format_uuid(crate::proto::Id::random()));
            if let Err(e) = std::fs::create_dir_all(data).and_then(|_| std::fs::write(&path, &t)) {
                tracing::warn!("Couldn't write {}: {e}", path.display());
            }
            t
        }
    };
    let mut cfg: Config = toml::from_str(&text).map_err(|e| format!("config.toml: {e}"))?;
    if parse_uuid(&cfg.server_id).is_none() {
        tracing::warn!("server_id {:?} isn't a UUID; using a temporary one", cfg.server_id);
        cfg.server_id = format_uuid(crate::proto::Id::random());
    }
    cfg.validate();
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_parse() {
        let mut cfg: Config = toml::from_str(&DEFAULTS.replace("__SERVER_ID__", "1b4e28ba-2fa1-11d2-883f-0016d3cca427")).unwrap();
        cfg.validate();
        assert_eq!(cfg.host.port, 25565);
        assert_eq!(cfg.voice.proximity.distances, [8, 16, 32]);
        assert_eq!(cfg.voice.proximity.default_distance, 16);
        assert_eq!(format_uuid(cfg.server_id()), "1b4e28ba-2fa1-11d2-883f-0016d3cca427");
        assert_eq!(cfg.public_address(), ("0.0.0.0".into(), 25565));
    }

    #[test]
    fn default_distance_is_validated() {
        assert_eq!(valid_default(&[8, 16, 32], 20), 16);
        assert_eq!(valid_default(&[-1, 64], 100), 32);
        assert_eq!(valid_default(&[-1, 64], 10), 10);
    }
}
