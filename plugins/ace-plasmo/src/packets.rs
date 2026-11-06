//! Plasmo Voice v2 packets: the plugin-channel ("TCP") set on `plasmo:voice/v2` and the UDP set.
//! Ids and field order follow `PacketTcpCodec` / `PacketUdpCodec` and each packet's `write`/`read`.

use crate::proto::{Bad, Id, In, Out, R};

pub const CHANNEL: &str = "plasmo:voice/v2";
pub const UDP_MAGIC: i32 = 0x4e90_04e9;

pub const ADDON_ID: &str = "plasmovoice";
pub const PROXIMITY_NAME: &str = "proximity";
pub const PROXIMITY_TRANSLATION: &str = "pv.activation.proximity";
/// `UUID.nameUUIDFromBytes("proximity_activation")`
pub const PROXIMITY_ACTIVATION: Id = Id(0x4aec_07ba_d109_3345, 0x9a0a_9202_2a11_6cd0);
/// `UUID.nameUUIDFromBytes("proximity_line")`
pub const PROXIMITY_LINE: Id = Id(0xd939_2647_f6b2_332d, 0x9adf_30a6_4008_28d6);

// ---- plugin channel ids (1-based, registration order in PacketTcpCodec) ----
const C_CONNECTION: u8 = 1;
const C_PLAYER_INFO_REQUEST: u8 = 2;
const C_CONFIG: u8 = 3;
const S_LANGUAGE_REQUEST: u8 = 5;
const C_LANGUAGE: u8 = 6;
const C_PLAYER_LIST: u8 = 7;
const C_PLAYER_INFO_UPDATE: u8 = 8;
const C_PLAYER_DISCONNECT: u8 = 9;
const S_PLAYER_INFO: u8 = 10;
const S_PLAYER_STATE: u8 = 11;
const S_PLAYER_AUDIO_END: u8 = 12;
const S_ACTIVATION_DISTANCES: u8 = 13;
const C_DISTANCE_VISUALIZE: u8 = 14;
const S_SOURCE_INFO_REQUEST: u8 = 15;
const C_SOURCE_INFO: u8 = 16;
const C_SELF_SOURCE_INFO: u8 = 17;
const C_SOURCE_AUDIO_END: u8 = 18;

// ---- UDP ids ----
const U_PING: u8 = 1;
const U_PLAYER_AUDIO: u8 = 2;
const U_SOURCE_AUDIO: u8 = 3;
const U_SELF_AUDIO_INFO: u8 = 4;

/// `VoicePlayerInfo`
#[derive(Clone, Debug)]
pub struct PlayerInfo {
    pub id: Id,
    pub name: String,
    pub muted: bool,
    pub voice_disabled: bool,
    pub mic_muted: bool,
}

impl PlayerInfo {
    fn write(&self, o: &mut Out) {
        o.id(self.id).utf(&self.name).bool(self.muted).bool(self.voice_disabled).bool(self.mic_muted);
    }
}

/// The proximity activation and source line, the only ones Ace Plasmo registers.
pub struct Proximity<'a> {
    pub distances: &'a [i32],
    pub default_distance: i32,
}

/// `SourceInfo` for a player source (`PlayerSourceInfo`), always Opus, mono, icon visible.
pub struct PlayerSource<'a> {
    pub id: Id,
    pub state: i8,
    pub player: &'a PlayerInfo,
}

impl PlayerSource<'_> {
    fn write(&self, o: &mut Out) {
        o.utf("PLAYER").utf(ADDON_ID).id(self.id);
        o.bool(false); // name: null
        o.u8(self.state as u8);
        o.bool(true).utf("opus").i32(0); // decoder info: OpusDecoderInfo
        o.bool(false); // stereo
        o.id(PROXIMITY_LINE);
        o.bool(true); // icon visible
        o.i32(0); // angle
        self.player.write(o);
    }
}

fn tcp(id: u8) -> Out {
    let mut o = Out::new();
    o.u8(id);
    o
}

pub fn connection(secret: Id, ip: &str, port: i32) -> Vec<u8> {
    let mut o = tcp(C_CONNECTION);
    o.id(secret).utf(ip).i32(port);
    o.into_vec()
}

pub fn player_info_request() -> Vec<u8> {
    tcp(C_PLAYER_INFO_REQUEST).into_vec()
}

pub struct Config<'a> {
    pub server_id: Id,
    pub sample_rate: i32,
    pub mtu: i32,
    pub opus_mode: &'a str,
    pub opus_bitrate: i32,
    /// AES key, RSA-encrypted with the player's public key
    pub encrypted_key: &'a [u8],
    pub proximity: Proximity<'a>,
    /// Whether the player may use the proximity activation (`pv.activation.proximity`).
    pub can_speak: bool,
    pub allow_freecam: bool,
    pub icon_hidden: &'a [String],
    pub icon_offset: f64,
}

pub fn config(c: &Config) -> Vec<u8> {
    let mut o = tcp(C_CONFIG);
    o.id(c.server_id);
    // CaptureInfo
    o.i32(c.sample_rate).i32(c.mtu).bool(true).utf("opus").i32(2);
    o.utf("mode").utf(c.opus_mode).utf("bitrate").utf(&c.opus_bitrate.to_string());
    // EncryptionInfo
    o.bool(true).utf("AES/CBC/PKCS5Padding").bytes(c.encrypted_key);
    // source lines
    o.i32(1);
    o.utf(PROXIMITY_NAME).utf(PROXIMITY_TRANSLATION).utf("plasmovoice:textures/icons/speaker.png");
    o.f64(1.0).i32(1).bool(false);
    // activations
    if c.can_speak {
        o.i32(1);
        activation(&mut o, &c.proximity);
    } else {
        o.i32(0);
    }
    // ConfigPlayerInfoPacket: synchronized permissions
    o.i32(1).utf("pv.allow_freecam").bool(c.allow_freecam);
    // PlayerIconConfig
    o.i32(c.icon_hidden.len() as i32);
    for name in c.icon_hidden {
        o.utf(name);
    }
    o.f64(0.0).f64(c.icon_offset).f64(0.0);
    o.into_vec()
}

fn activation(o: &mut Out, p: &Proximity) {
    o.utf(PROXIMITY_NAME).utf(PROXIMITY_TRANSLATION).utf("plasmovoice:textures/icons/microphone.png");
    o.i32(p.distances.len() as i32);
    for d in p.distances {
        o.i32(*d);
    }
    o.i32(p.default_distance);
    o.bool(true).bool(true).bool(false); // proximity, transitive, stereo supported
    o.bool(false); // encoder info: none (client default)
    o.i32(1); // weight
}

pub fn language(name: &str, entries: &[(&str, &str)]) -> Vec<u8> {
    let mut o = tcp(C_LANGUAGE);
    o.utf(name).i32(entries.len() as i32);
    for (k, v) in entries {
        o.utf(k).utf(v);
    }
    o.into_vec()
}

pub fn player_list(players: &[&PlayerInfo]) -> Vec<u8> {
    let mut o = tcp(C_PLAYER_LIST);
    o.i32(players.len() as i32);
    for p in players {
        p.write(&mut o);
    }
    o.into_vec()
}

pub fn player_info_update(p: &PlayerInfo) -> Vec<u8> {
    let mut o = tcp(C_PLAYER_INFO_UPDATE);
    p.write(&mut o);
    o.into_vec()
}

pub fn player_disconnect(id: Id) -> Vec<u8> {
    let mut o = tcp(C_PLAYER_DISCONNECT);
    o.id(id);
    o.into_vec()
}

pub fn source_info(s: &PlayerSource) -> Vec<u8> {
    let mut o = tcp(C_SOURCE_INFO);
    s.write(&mut o);
    o.into_vec()
}

pub fn self_source_info(s: &PlayerSource, player: Id) -> Vec<u8> {
    let mut o = tcp(C_SELF_SOURCE_INFO);
    s.write(&mut o);
    o.id(player).id(PROXIMITY_ACTIVATION).i64(-1);
    o.into_vec()
}

pub fn source_audio_end(source: Id, sequence: i64) -> Vec<u8> {
    let mut o = tcp(C_SOURCE_AUDIO_END);
    o.id(source).i64(sequence);
    o.into_vec()
}

/// Draws the distance sphere around the player when they change their proximity distance.
pub fn distance_visualize(radius: i32, color: i32) -> Vec<u8> {
    let mut o = tcp(C_DISTANCE_VISUALIZE);
    o.i32(radius).i32(color).bool(false);
    o.into_vec()
}

/// Packets the client sends on the plugin channel.
#[derive(Debug)]
pub enum FromClient {
    PlayerInfo { voice_disabled: bool, mic_muted: bool, mc_version: String, version: String, public_key: Vec<u8> },
    PlayerState { voice_disabled: bool, mic_muted: bool },
    AudioEnd { sequence: i64, activation: Id, distance: i16 },
    ActivationDistances(Vec<(Id, i32)>),
    SourceInfoRequest(Id),
    LanguageRequest(String),
}

pub fn decode_tcp(data: &[u8]) -> R<FromClient> {
    let mut i = In::new(data);
    Ok(match i.u8()? {
        S_PLAYER_INFO => FromClient::PlayerInfo {
            voice_disabled: i.bool()?,
            mic_muted: i.bool()?,
            mc_version: i.utf(64)?,
            version: i.utf(64)?,
            public_key: i.bytes(1, 2048)?,
        },
        S_PLAYER_STATE => FromClient::PlayerState { voice_disabled: i.bool()?, mic_muted: i.bool()? },
        S_PLAYER_AUDIO_END => FromClient::AudioEnd { sequence: i.i64()?, activation: i.id()?, distance: i.i16()? },
        S_ACTIVATION_DISTANCES => {
            let n = i.int_in(0, i8::MAX as i32)?;
            let mut v = Vec::with_capacity(n as usize);
            for _ in 0..n {
                v.push((i.id()?, i.i32()?));
            }
            FromClient::ActivationDistances(v)
        }
        S_SOURCE_INFO_REQUEST => FromClient::SourceInfoRequest(i.id()?),
        S_LANGUAGE_REQUEST => FromClient::LanguageRequest(i.utf(32)?),
        _ => return Err(Bad),
    })
}

// ---- UDP ----

pub struct UdpHeader {
    pub secret: Id,
}

#[derive(Debug)]
pub enum UdpFromClient {
    Ping,
    Audio { sequence: i64, data: Vec<u8>, activation: Id, distance: i16 },
}

pub fn decode_udp(buf: &[u8]) -> R<(UdpHeader, UdpFromClient)> {
    let mut i = In::new(buf);
    if i.i32()? != UDP_MAGIC {
        return Err(Bad);
    }
    let kind = i.u8()?;
    let secret = i.id()?;
    let _timestamp = i.i64()?;
    let packet = match kind {
        U_PING => UdpFromClient::Ping,
        U_PLAYER_AUDIO => UdpFromClient::Audio {
            sequence: i.i64()?,
            data: i.bytes(1, 2048)?,
            activation: i.id()?,
            distance: i.i16()?,
        },
        _ => return Err(Bad),
    };
    Ok((UdpHeader { secret }, packet))
}

fn udp(kind: u8, secret: Id, now_ms: i64) -> Out {
    let mut o = Out::new();
    o.i32(UDP_MAGIC).u8(kind).id(secret).i64(now_ms);
    o
}

pub fn udp_ping(secret: Id, now_ms: i64) -> Vec<u8> {
    let mut o = udp(U_PING, secret, now_ms);
    o.i64(now_ms);
    o.into_vec()
}

pub fn udp_source_audio(secret: Id, now_ms: i64, sequence: i64, data: &[u8], source: Id, state: i8, distance: i16) -> Vec<u8> {
    let mut o = udp(U_SOURCE_AUDIO, secret, now_ms);
    o.i64(sequence).bytes(data).id(source).u8(state as u8).i16(distance);
    o.into_vec()
}

pub fn udp_self_audio_info(secret: Id, now_ms: i64, source: Id, sequence: i64, distance: i16) -> Vec<u8> {
    let mut o = udp(U_SELF_AUDIO_INFO, secret, now_ms);
    o.id(source).i64(sequence).bool(false).i16(distance);
    o.into_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use md5::{Digest, Md5};

    /// Java's `UUID.nameUUIDFromBytes`: MD5, then version 3 and the IETF variant.
    fn name_uuid(name: &str) -> Id {
        let mut h: [u8; 16] = Md5::digest(name.as_bytes()).into();
        h[6] = (h[6] & 0x0f) | 0x30;
        h[8] = (h[8] & 0x3f) | 0x80;
        Id(u64::from_be_bytes(h[..8].try_into().unwrap()), u64::from_be_bytes(h[8..].try_into().unwrap()))
    }

    #[test]
    fn proximity_ids_match_plasmo() {
        assert_eq!(name_uuid("proximity_activation"), PROXIMITY_ACTIVATION);
        assert_eq!(name_uuid("proximity_line"), PROXIMITY_LINE);
    }

    #[test]
    fn player_audio_decodes() {
        let secret = Id(7, 9);
        let mut o = Out::new();
        o.i32(UDP_MAGIC).u8(U_PLAYER_AUDIO).id(secret).i64(123);
        o.i64(42).bytes(&[0x60, 1, 2, 3]).id(PROXIMITY_ACTIVATION).i16(16).bool(false);
        let (h, p) = decode_udp(&o.into_vec()).unwrap();
        assert_eq!(h.secret, secret);
        match p {
            UdpFromClient::Audio { sequence, data, activation, distance } => {
                assert_eq!((sequence, data.len(), activation, distance), (42, 4, PROXIMITY_ACTIVATION, 16));
            }
            _ => panic!("not audio"),
        }
    }

    #[test]
    fn player_info_decodes() {
        let mut o = Out::new();
        o.u8(S_PLAYER_INFO).bool(false).bool(true).utf("26.3").utf("2.2.0").bytes(&[1, 2, 3]);
        match decode_tcp(&o.into_vec()).unwrap() {
            FromClient::PlayerInfo { mic_muted, version, public_key, .. } => {
                assert!(mic_muted);
                assert_eq!(version, "2.2.0");
                assert_eq!(public_key, [1, 2, 3]);
            }
            other => panic!("{other:?}"),
        }
    }
}
