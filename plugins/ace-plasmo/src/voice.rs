//! The voice server: the plugin-channel handshake, the UDP socket and proximity routing.
//!
//! Plasmo Voice equivalents: `PlayerChannelHandler` + `VoiceTcpServerConnectionManager` (handshake),
//! `NettyPacketHandler` + `NettyUdpServerConnection` + `NettyUdpKeepAlive` (UDP), `VoiceServerActivationManager` +
//! `ProximityServerActivationHelper` + `VoiceServerProximitySource` + `SelfActivationHelper` (routing).
//!
//! The server never decodes audio: clients encrypt Opus frames with the shared AES key we hand out, and we forward
//! the bytes as they are. Everything here is pure state; host calls are queued as [`Action`]s and run after the lock
//! is released (see `lib.rs`).

use crate::config::Config;
use crate::packets::{self, FromClient, PlayerInfo, PlayerSource, Proximity, UdpFromClient};
use crate::proto::Id;
use rsa::pkcs8::DecodePublicKey;
use rsa::{Pkcs1v15Encrypt, RsaPublicKey};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::{SystemTime, UNIX_EPOCH};

/// Version we present as. The client only checks the major version against ours.
pub const PLASMO_VERSION_MAJOR: u32 = 2;
/// Retry schedule for `PlayerInfoRequestPacket` (`PlayerInfoRequestScheduler`).
const INFO_RETRIES_MS: [u64; 5] = [1_000, 3_000, 5_000, 10_000, 15_000];
const STATE_BROADCAST_MS: u64 = 250;
const MAX_DATAGRAMS_PER_TICK: usize = 4096;

pub enum Action {
    /// Send a payload on `plasmo:voice/v2`.
    Tcp(Id, Vec<u8>),
    /// Plain chat message to a player.
    Message(Id, String),
}

/// Where a player is, refreshed every tick (host calls happen outside the lock).
#[derive(Clone)]
pub struct Pos {
    pub world: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

pub struct VoicePlayer {
    pub id: Id,
    pub name: String,
    key: Option<RsaPublicKey>,
    pub mod_version: Option<String>,
    voice_disabled: bool,
    mic_muted: bool,
    /// Granted `ace-plasmo:activation.proximity` when the handshake ran.
    can_speak: bool,
    allow_freecam: bool,
    /// Given out in `ConnectionPacket`, proves the UDP sender is this player.
    secret: Option<Id>,
    /// UDP address once the client has pinged us.
    addr: Option<SocketAddr>,
    last_received: u64,
    next_keep_alive: u64,
    pub pos: Option<Pos>,
    // the proximity source this player speaks through
    source: Id,
    source_state: i8,
    source_announced: bool,
    self_announced: bool,
    speaking: bool,
    last_sequence: i64,
    proximity_distance: Option<i32>,
    // PlayerInfoRequest retries until the client answers
    info_attempt: usize,
    info_due: Option<u64>,
    state_broadcast_due: Option<u64>,
    last_state_broadcast: u64,
}

impl VoicePlayer {
    fn new(id: Id, name: String) -> Self {
        VoicePlayer {
            id,
            name,
            key: None,
            mod_version: None,
            voice_disabled: false,
            mic_muted: false,
            can_speak: false,
            allow_freecam: true,
            secret: None,
            addr: None,
            last_received: 0,
            next_keep_alive: 0,
            pos: None,
            source: Id::random(),
            source_state: 1,
            source_announced: false,
            self_announced: false,
            speaking: false,
            last_sequence: 0,
            proximity_distance: None,
            info_attempt: 0,
            info_due: None,
            state_broadcast_due: None,
            last_state_broadcast: 0,
        }
    }

    pub fn connected(&self) -> bool {
        self.addr.is_some()
    }

    fn info(&self) -> PlayerInfo {
        PlayerInfo {
            id: self.id,
            name: self.name.clone(),
            muted: false,
            voice_disabled: self.voice_disabled,
            mic_muted: self.mic_muted,
        }
    }
}

/// Permissions read from the host before the lock is taken.
pub struct Grants {
    pub speak: bool,
    pub freecam: bool,
}

pub struct Voice {
    pub cfg: Config,
    aes_key: [u8; 16],
    socket: Option<UdpSocket>,
    pub players: HashMap<Id, VoicePlayer>,
    by_secret: HashMap<Id, Id>,
    pub actions: Vec<Action>,
    recv_buf: Vec<u8>,
}

pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

impl Voice {
    pub fn new(cfg: Config) -> Self {
        let mut aes_key = [0u8; 16];
        let a = Id::random();
        aes_key[..8].copy_from_slice(&a.0.to_be_bytes());
        aes_key[8..].copy_from_slice(&a.1.to_be_bytes());
        Voice {
            cfg,
            aes_key,
            socket: None,
            players: HashMap::new(),
            by_secret: HashMap::new(),
            actions: Vec::new(),
            recv_buf: vec![0u8; 8192],
        }
    }

    /// Bind the UDP voice socket (`NettyUdpServer`).
    pub fn bind(&mut self) -> Result<SocketAddr, String> {
        let addr = format!("{}:{}", self.cfg.host.ip, self.cfg.host.port);
        let socket = UdpSocket::bind(&addr).map_err(|e| format!("couldn't bind UDP {addr}: {e}"))?;
        socket.set_nonblocking(true).map_err(|e| format!("UDP set_nonblocking: {e}"))?;
        let local = socket.local_addr().map_err(|e| e.to_string())?;
        self.socket = Some(socket);
        Ok(local)
    }

    pub fn is_bound(&self) -> bool {
        self.socket.is_some()
    }

    fn tcp(&mut self, to: Id, data: Vec<u8>) {
        self.actions.push(Action::Tcp(to, data));
    }

    fn send_udp(&self, to: SocketAddr, data: &[u8]) {
        if let Some(s) = &self.socket {
            // WouldBlock means the kernel buffer is full: drop the frame, like a lossy network would
            let _ = s.send_to(data, to);
        }
    }

    fn proximity(&self) -> Proximity<'_> {
        Proximity {
            distances: &self.cfg.voice.proximity.distances,
            default_distance: self.cfg.voice.proximity.default_distance,
        }
    }

    // ---- players ----

    pub fn join(&mut self, id: Id, name: String) {
        let mut p = VoicePlayer::new(id, name);
        p.info_due = Some(0); // request right away, then retry
        self.players.insert(id, p);
    }

    pub fn leave(&mut self, id: Id) {
        if let Some(p) = self.players.remove(&id) {
            if let Some(secret) = p.secret {
                self.by_secret.remove(&secret);
            }
            if p.connected() {
                self.broadcast_tcp(packets::player_disconnect(id), None);
            }
        }
    }

    /// Drop the UDP connection and start the handshake again (`/vrc`, timeouts).
    pub fn reconnect(&mut self, id: Id) {
        let Some(p) = self.players.get_mut(&id) else { return };
        let was = p.addr.take().is_some();
        if let Some(secret) = p.secret.take() {
            self.by_secret.remove(&secret);
        }
        p.key = None;
        p.speaking = false;
        p.source_announced = false;
        p.self_announced = false;
        p.info_attempt = 0;
        p.info_due = Some(0);
        if was {
            self.broadcast_tcp(packets::player_disconnect(id), None);
        }
    }

    /// Queue a payload for every connected player, optionally except one.
    fn broadcast_tcp(&mut self, data: Vec<u8>, except: Option<Id>) {
        let to: Vec<Id> = self.players.values().filter(|p| p.connected() && Some(p.id) != except).map(|p| p.id).collect();
        for id in to {
            self.tcp(id, data.clone());
        }
    }

    // ---- plugin channel ----

    pub fn on_payload(&mut self, from: Id, data: &[u8], grants: Grants) {
        if !self.is_bound() {
            return;
        }
        let packet = match packets::decode_tcp(data) {
            Ok(p) => p,
            Err(_) => {
                tracing::debug!("Ignoring an unknown or malformed Plasmo Voice packet");
                return;
            }
        };
        if !self.players.contains_key(&from) {
            return;
        }
        match packet {
            FromClient::PlayerInfo { voice_disabled, mic_muted, mc_version, version, public_key } => {
                self.on_player_info(from, voice_disabled, mic_muted, &mc_version, &version, &public_key, grants)
            }
            FromClient::PlayerState { voice_disabled, mic_muted } => self.on_state(from, voice_disabled, mic_muted),
            FromClient::AudioEnd { sequence, activation, distance } => self.on_audio_end(from, sequence, activation, distance),
            FromClient::ActivationDistances(list) => self.on_distances(from, &list),
            FromClient::SourceInfoRequest(source) => self.on_source_info_request(from, source),
            FromClient::LanguageRequest(lang) => {
                let entries = [(packets::PROXIMITY_TRANSLATION, "Proximity")];
                self.tcp(from, packets::language(&lang, &entries));
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn on_player_info(&mut self, id: Id, voice_disabled: bool, mic_muted: bool, mc: &str, version: &str, key: &[u8], g: Grants) {
        let min = self.cfg.voice.client_mod_min_version.clone();
        let client = semver(version);
        if client.0 != PLASMO_VERSION_MAJOR || client < semver(&min) {
            tracing::info!("{} has Plasmo Voice {version} (Minecraft {mc}); this server needs {min} or newer 2.x", self.name(id));
            self.actions.push(Action::Message(
                id,
                format!("Your Plasmo Voice version ({version}) isn't supported on this server. Install Plasmo Voice 2.x, {min} or newer, for Minecraft {mc}: https://modrinth.com/plugin/plasmo-voice"),
            ));
            if let Some(p) = self.players.get_mut(&id) {
                p.info_due = None;
            }
            return;
        }
        let key = match RsaPublicKey::from_public_key_der(key) {
            Ok(k) => k,
            Err(e) => {
                tracing::warn!("{} sent an unreadable RSA public key: {e}", self.name(id));
                return;
            }
        };
        let fresh = Id::random();
        let (ip, port) = self.cfg.public_address();
        let Some(p) = self.players.get_mut(&id) else { return };
        p.key = Some(key);
        p.mod_version = Some(version.to_string());
        p.voice_disabled = voice_disabled;
        p.mic_muted = mic_muted;
        p.can_speak = g.speak;
        p.allow_freecam = g.freecam;
        p.info_due = None;
        let secret = *p.secret.get_or_insert(fresh);
        self.by_secret.insert(secret, id);
        self.tcp(id, packets::connection(secret, &ip, port));
    }

    fn on_state(&mut self, id: Id, voice_disabled: bool, mic_muted: bool) {
        let now = now_ms();
        let Some(p) = self.players.get_mut(&id) else { return };
        if !p.connected() || (p.voice_disabled == voice_disabled && p.mic_muted == mic_muted) {
            return;
        }
        p.voice_disabled = voice_disabled;
        p.mic_muted = mic_muted;
        // a state change is a "major" source change: listeners refresh the source info
        p.source_announced = false;
        if now.saturating_sub(p.last_state_broadcast) >= STATE_BROADCAST_MS {
            p.last_state_broadcast = now;
            let info = packets::player_info_update(&p.info());
            self.broadcast_tcp(info, None);
        } else if p.state_broadcast_due.is_none() {
            p.state_broadcast_due = Some(p.last_state_broadcast + STATE_BROADCAST_MS);
        }
    }

    fn on_distances(&mut self, id: Id, list: &[(Id, i32)]) {
        let Some(p) = self.players.get_mut(&id) else { return };
        for (activation, distance) in list {
            if *activation != packets::PROXIMITY_ACTIVATION {
                continue;
            }
            let old = p.proximity_distance.replace(*distance);
            if old.is_some() && old != Some(*distance) {
                self.actions.push(Action::Tcp(id, packets::distance_visualize(*distance, 0x00a000)));
            }
        }
    }

    fn on_source_info_request(&mut self, from: Id, source: Id) {
        let Some(owner) = self.players.values().find(|p| p.source == source) else { return };
        if owner.id == from {
            return;
        }
        if self.players.get(&from).is_some_and(|p| p.voice_disabled) {
            return;
        }
        let info = owner.info();
        let data = packets::source_info(&PlayerSource { id: owner.source, state: owner.source_state, player: &info });
        self.tcp(from, data);
    }

    // ---- UDP ----

    /// Drain the socket. Called once per server tick.
    pub fn poll_udp(&mut self) {
        let mut received = Vec::new();
        {
            let Some(socket) = &self.socket else { return };
            let buf = &mut self.recv_buf;
            for _ in 0..MAX_DATAGRAMS_PER_TICK {
                match socket.recv_from(buf) {
                    Ok((n, from)) => received.push((buf[..n].to_vec(), from)),
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    // a client vanished (ICMP port unreachable on some stacks): keep draining
                    Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => continue,
                    Err(e) => {
                        tracing::debug!("UDP receive: {e}");
                        break;
                    }
                }
            }
        }
        for (data, from) in received {
            self.on_datagram(&data, from);
        }
    }

    fn on_datagram(&mut self, data: &[u8], from: SocketAddr) {
        let Ok((header, packet)) = packets::decode_udp(data) else { return };
        let Some(&id) = self.by_secret.get(&header.secret) else { return };
        let now = now_ms();
        let first = {
            let Some(p) = self.players.get_mut(&id) else { return };
            p.last_received = now;
            let first = p.addr.is_none();
            if p.addr != Some(from) {
                p.addr = Some(from);
            }
            first
        };
        if first {
            self.on_connected(id);
        }
        if let UdpFromClient::Audio { sequence, data, activation, distance } = packet {
            self.on_audio(id, sequence, &data, activation, distance);
        }
    }

    /// First UDP packet with a valid secret: send the config and everyone's voice state.
    fn on_connected(&mut self, id: Id) {
        let Some(p) = self.players.get(&id) else { return };
        let Some(key) = &p.key else { return };
        let encrypted = match key.encrypt(&mut rsa::rand_core::OsRng, Pkcs1v15Encrypt, &self.aes_key) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("Couldn't encrypt the voice key for {}: {e}", p.name);
                return;
            }
        };
        let (can_speak, freecam, name, info) = (p.can_speak, p.allow_freecam, p.name.clone(), p.info());
        let v = &self.cfg.voice;
        let config = packets::config(&packets::Config {
            server_id: self.cfg.server_id(),
            sample_rate: v.sample_rate,
            mtu: v.mtu_size,
            opus_mode: &v.opus.mode,
            opus_bitrate: v.opus.bitrate,
            encrypted_key: &encrypted,
            proximity: self.proximity(),
            can_speak,
            allow_freecam: freecam,
            icon_hidden: &v.player_icon.visibility,
            icon_offset: v.player_icon.y_offset,
        });
        self.tcp(id, config);
        let list: Vec<PlayerInfo> = self.players.values().filter(|p| p.connected()).map(|p| p.info()).collect();
        let refs: Vec<&PlayerInfo> = list.iter().collect();
        self.tcp(id, packets::player_list(&refs));
        self.broadcast_tcp(packets::player_info_update(&info), None);
        tracing::info!("{name} connected to voice chat");
    }

    fn allowed_distance(&self, d: i32) -> i32 {
        let ds = &self.cfg.voice.proximity.distances;
        let ok = if ds.len() == 2 && ds[0] == -1 { (1..=ds[1]).contains(&d) } else { ds.contains(&d) };
        if ok { d } else { self.cfg.voice.proximity.default_distance }
    }

    /// Who hears a source at `distance` blocks (`VoiceServerProximitySource.getListeners`).
    fn listeners(&self, speaker: Id, distance: i32) -> Vec<(Id, SocketAddr, Id)> {
        let Some(sp) = self.players.get(&speaker).and_then(|p| p.pos.clone()) else { return vec![] };
        let reach = (distance + self.cfg.voice.max_extra_audio_broadcast_distance).min(distance * 2) as f64;
        let reach2 = reach * reach;
        self.players
            .values()
            .filter(|p| p.id != speaker && !p.voice_disabled)
            .filter_map(|p| {
                let addr = p.addr?;
                let secret = p.secret?;
                let pos = p.pos.as_ref()?;
                let (dx, dy, dz) = (pos.x - sp.x, pos.y - sp.y, pos.z - sp.z);
                (pos.world == sp.world && dx * dx + dy * dy + dz * dz <= reach2).then_some((p.id, addr, secret))
            })
            .collect()
    }

    fn on_audio(&mut self, id: Id, sequence: i64, data: &[u8], activation: Id, distance: i16) {
        let now = now_ms() as i64;
        let distance = self.allowed_distance(distance as i32);
        let Some(p) = self.players.get_mut(&id) else { return };
        if p.mic_muted || !p.can_speak || activation != packets::PROXIMITY_ACTIVATION {
            return;
        }
        if !p.speaking && (sequence > p.last_sequence || (sequence - p.last_sequence).abs() > 10) {
            p.speaking = true;
        }
        if !p.speaking {
            return;
        }
        let (source, state, announce, announce_self) = (p.source, p.source_state, !p.source_announced, !p.self_announced);
        p.source_announced = true;
        p.self_announced = true;
        let (self_addr, self_secret) = (p.addr, p.secret);
        let info = p.info();
        let src = PlayerSource { id: source, state, player: &info };

        let listeners = self.listeners(id, distance);
        if announce {
            let data = packets::source_info(&src);
            for (to, _, _) in &listeners {
                self.actions.push(Action::Tcp(*to, data.clone()));
            }
        }
        if announce_self {
            self.actions.push(Action::Tcp(id, packets::self_source_info(&src, id)));
        }
        for (_, addr, secret) in &listeners {
            let pkt = packets::udp_source_audio(*secret, now, sequence, data, source, state, distance as i16);
            self.send_udp(*addr, &pkt);
        }
        if let (Some(addr), Some(secret)) = (self_addr, self_secret) {
            let pkt = packets::udp_self_audio_info(secret, now, source, sequence, distance as i16);
            self.send_udp(addr, &pkt);
        }
    }

    fn on_audio_end(&mut self, id: Id, sequence: i64, activation: Id, distance: i16) {
        if activation != packets::PROXIMITY_ACTIVATION {
            return;
        }
        let ds = &self.cfg.voice.proximity.distances;
        let d = distance as i32;
        let valid = ds.is_empty() || if ds.len() == 2 && ds[0] == -1 { (1..=ds[1]).contains(&d) } else { ds.contains(&d) };
        let distance = self.allowed_distance(d);
        let Some(p) = self.players.get_mut(&id) else { return };
        if !p.speaking || !p.can_speak || !valid {
            return;
        }
        p.speaking = false;
        p.last_sequence = sequence;
        let source = p.source;
        let end = packets::source_audio_end(source, sequence);
        for (to, _, _) in self.listeners(id, distance) {
            self.actions.push(Action::Tcp(to, end.clone()));
        }
        self.tcp(id, end);
    }

    // ---- per tick ----

    pub fn tick(&mut self) {
        self.poll_udp();
        let now = now_ms();
        let timeout = self.cfg.voice.keep_alive_timeout_ms;
        let mut timed_out = vec![];
        let mut pings = vec![];
        let mut state_updates = vec![];
        let mut requests = vec![];
        for p in self.players.values_mut() {
            if let Some(due) = p.state_broadcast_due
                && now >= due
            {
                p.state_broadcast_due = None;
                p.last_state_broadcast = now;
                if p.connected() {
                    state_updates.push(packets::player_info_update(&p.info()));
                }
            }
            if let Some(due) = p.info_due
                && now >= due
            {
                if p.key.is_some() || p.info_attempt > INFO_RETRIES_MS.len() {
                    p.info_due = None;
                } else {
                    requests.push(p.id);
                    p.info_due = INFO_RETRIES_MS.get(p.info_attempt).map(|d| now + d);
                    p.info_attempt += 1;
                }
            }
            let (Some(addr), Some(secret)) = (p.addr, p.secret) else { continue };
            if now.saturating_sub(p.last_received) > timeout {
                timed_out.push(p.id);
            } else if now >= p.next_keep_alive {
                // jitter keeps a crowd of clients from pinging in lockstep (NettyUdpKeepAlive)
                let jitter = 1_500 + (secret.1 ^ now) % 1_500;
                p.next_keep_alive = now + 1_000 + jitter;
                pings.push((addr, secret));
            }
        }
        for (addr, secret) in pings {
            self.send_udp(addr, &packets::udp_ping(secret, now as i64));
        }
        for data in state_updates {
            self.broadcast_tcp(data, None);
        }
        for id in requests {
            self.tcp(id, packets::player_info_request());
        }
        for id in timed_out {
            tracing::info!("{} timed out of voice chat; reconnecting", self.name(id));
            self.reconnect(id);
        }
    }

    pub fn name(&self, id: Id) -> String {
        self.players.get(&id).map(|p| p.name.clone()).unwrap_or_else(|| "?".into())
    }

    /// Ids of players whose position we need this tick.
    pub fn tracked(&self) -> Vec<Id> {
        self.players.values().filter(|p| p.connected()).map(|p| p.id).collect()
    }
}

/// "2.1.3-beta" → (2, 1, 3); anything unreadable is 0.
fn semver(v: &str) -> (u32, u32, u32) {
    let core = v.split(['-', '+']).next().unwrap_or("");
    let mut it = core.split('.').map(|n| n.trim().parse::<u32>().unwrap_or(0));
    (it.next().unwrap_or(0), it.next().unwrap_or(0), it.next().unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::semver;

    #[test]
    fn versions_compare() {
        assert_eq!(semver("2.2.0-beta.2"), (2, 2, 0));
        assert!(semver("2.1.3") >= semver("2.0.0"));
        assert!(semver("1.9.9") < semver("2.0.0"));
        assert_eq!(semver("junk"), (0, 0, 0));
    }
}
