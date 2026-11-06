//! Discord gateway (WebSocket) client. Runs inside the WASM sandbox: a WASI TCP socket, rustls for TLS and tungstenite
//! for WebSocket framing. The socket is non-blocking and `tick` is called once per server tick, so it never stalls the
//! game. Handles hello/identify/resume, heartbeats, zombie detection, reconnects with backoff and fatal close codes.
//! (DiscordSRV uses JDA for this.)

use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use serde_json::{Value, json};
use std::io::ErrorKind;
use std::net::TcpStream;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tungstenite::handshake::client::ClientHandshake;
use tungstenite::handshake::{HandshakeError, MidHandshake};
use tungstenite::{Message, WebSocket};

type Tls = StreamOwned<ClientConnection, TcpStream>;

const DEFAULT_HOST: &str = "gateway.discord.gg";

/// GUILDS | GUILD_MESSAGES | DIRECT_MESSAGES | MESSAGE_CONTENT
pub const INTENTS: u64 = (1 << 0) | (1 << 9) | (1 << 12) | (1 << 15);

pub enum GwEvent {
    Dispatch(String, Value),
    /// The connection will not be retried (bad token, missing intents).
    Fatal(String),
}

pub struct Gateway {
    ws: Option<WebSocket<Tls>>,
    mid: Option<MidHandshake<ClientHandshake<Tls>>>,
    session_id: Option<String>,
    resume_host: Option<String>,
    seq: Option<u64>,
    heartbeat: Option<Duration>,
    next_heartbeat: Option<Instant>,
    acked: bool,
    next_connect: Instant,
    backoff: Duration,
    pub ready: bool,
    pub stopped: bool,
    pub debug: bool,
    identify_sent: bool,
}

impl Default for Gateway {
    fn default() -> Self {
        Gateway {
            ws: None,
            mid: None,
            session_id: None,
            resume_host: None,
            seq: None,
            heartbeat: None,
            next_heartbeat: None,
            acked: true,
            next_connect: Instant::now(),
            backoff: Duration::from_secs(1),
            ready: false,
            stopped: false,
            debug: false,
            identify_sent: false,
        }
    }
}

fn tls_config() -> Result<Arc<ClientConfig>, String> {
    static CONFIG: OnceLock<Arc<ClientConfig>> = OnceLock::new();
    if let Some(c) = CONFIG.get() {
        return Ok(c.clone());
    }
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .with_root_certificates(roots)
        .with_no_client_auth();
    let config = Arc::new(config);
    let _ = CONFIG.set(config.clone());
    Ok(config)
}

fn host_of(url: &str) -> String {
    url.trim_start_matches("wss://").trim_start_matches("ws://").split(['/', '?']).next().unwrap_or(DEFAULT_HOST).to_string()
}

impl Gateway {
    /// Drops the connection; reconnects (resuming if `resume`) after the backoff.
    fn disconnect(&mut self, resume: bool, why: &str) {
        if let Some(ws) = self.ws.as_mut() {
            let _ = ws.close(None);
            let _ = ws.flush();
        }
        self.ws = None;
        self.mid = None;
        self.ready = false;
        self.identify_sent = false;
        self.next_heartbeat = None;
        if !resume {
            self.session_id = None;
            self.seq = None;
            self.resume_host = None;
        }
        self.next_connect = Instant::now() + self.backoff;
        tracing::warn!("Discord gateway disconnected ({why}); reconnecting in {}s", self.backoff.as_secs());
        self.backoff = (self.backoff * 2).min(Duration::from_secs(60));
    }

    /// Closes for good (plugin unload or reload).
    pub fn shutdown(&mut self) {
        if let Some(ws) = self.ws.as_mut() {
            let _ = ws.close(None);
            let _ = ws.flush();
        }
        *self = Gateway { stopped: true, ..Gateway::default() };
    }

    pub fn reset(&mut self) {
        self.shutdown();
        self.stopped = false;
    }

    fn connect(&mut self) -> Result<(), String> {
        let host = self.resume_host.clone().filter(|_| self.session_id.is_some()).unwrap_or_else(|| DEFAULT_HOST.into());
        let tcp = TcpStream::connect((host.as_str(), 443)).map_err(|e| format!("connect {host}: {e}"))?;
        let _ = tcp.set_nodelay(true);
        tcp.set_nonblocking(true).map_err(|e| e.to_string())?;
        let name = ServerName::try_from(host.clone()).map_err(|e| e.to_string())?;
        let conn = ClientConnection::new(tls_config()?, name).map_err(|e| e.to_string())?;
        let url = format!("wss://{host}/?v=10&encoding=json");
        match tungstenite::client::client(url.as_str(), StreamOwned::new(conn, tcp)) {
            Ok((ws, _)) => self.ws = Some(ws),
            Err(HandshakeError::Interrupted(mid)) => self.mid = Some(mid),
            Err(HandshakeError::Failure(e)) => return Err(e.to_string()),
        }
        Ok(())
    }

    pub fn send(&mut self, op: u8, d: Value) {
        let Some(ws) = self.ws.as_mut() else { return };
        let payload = json!({ "op": op, "d": d }).to_string();
        match ws.send(Message::text(payload)) {
            Ok(()) => {}
            Err(tungstenite::Error::Io(e)) if e.kind() == ErrorKind::WouldBlock => {}
            Err(e) => tracing::warn!("Discord gateway send failed: {e}"),
        }
    }

    pub fn tick(&mut self, token: &str, presence: &Value) -> Vec<GwEvent> {
        let mut events = Vec::new();
        if self.stopped {
            return events;
        }
        let now = Instant::now();

        if self.ws.is_none() && self.mid.is_none() {
            if now < self.next_connect {
                return events;
            }
            if let Err(e) = self.connect() {
                self.disconnect(true, &e);
                return events;
            }
        }

        if let Some(mid) = self.mid.take() {
            match mid.handshake() {
                Ok((ws, _)) => self.ws = Some(ws),
                Err(HandshakeError::Interrupted(mid)) => {
                    self.mid = Some(mid);
                    return events;
                }
                Err(HandshakeError::Failure(e)) => {
                    self.disconnect(true, &format!("handshake: {e}"));
                    return events;
                }
            }
        }

        for _ in 0..64 {
            let Some(ws) = self.ws.as_mut() else { break };
            match ws.read() {
                Ok(Message::Text(text)) => self.on_payload(text.as_str(), token, presence, &mut events),
                Ok(Message::Close(frame)) => {
                    let code = frame.map(|f| u16::from(f.code)).unwrap_or(1000);
                    self.on_close(code, &mut events);
                    return events;
                }
                Ok(_) => {}
                Err(tungstenite::Error::Io(e)) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) => {
                    self.disconnect(true, &e.to_string());
                    return events;
                }
            }
        }

        if let (Some(at), Some(interval)) = (self.next_heartbeat, self.heartbeat) {
            if now >= at {
                if !self.acked {
                    self.disconnect(true, "heartbeat not acknowledged");
                    return events;
                }
                self.acked = false;
                let seq = self.seq.map_or(Value::Null, Value::from);
                self.send(1, seq);
                self.next_heartbeat = Some(now + interval);
            }
        }

        if let Some(ws) = self.ws.as_mut() {
            match ws.flush() {
                Ok(()) => {}
                Err(tungstenite::Error::Io(e)) if e.kind() == ErrorKind::WouldBlock => {}
                Err(e) => self.disconnect(true, &e.to_string()),
            }
        }
        events
    }

    fn on_close(&mut self, code: u16, events: &mut Vec<GwEvent>) {
        let fatal = match code {
            4004 => Some("the bot token is invalid; check bot_token / ACE_SRV_BOT_TOKEN"),
            4010 | 4011 => Some("Discord rejected the shard configuration"),
            4012 => Some("unsupported gateway version"),
            4013 => Some("invalid intents"),
            4014 => Some(
                "the bot is missing the MESSAGE CONTENT intent; enable it under Bot → Privileged Gateway Intents in the Discord developer portal",
            ),
            _ => None,
        };
        if let Some(why) = fatal {
            self.shutdown();
            events.push(GwEvent::Fatal(format!("Discord closed the connection ({code}): {why}")));
            return;
        }
        let resume = !matches!(code, 1000 | 1001 | 4007 | 4009);
        self.disconnect(resume, &format!("close code {code}"));
    }

    fn on_payload(&mut self, text: &str, token: &str, presence: &Value, events: &mut Vec<GwEvent>) {
        let Ok(v) = serde_json::from_str::<Value>(text) else { return };
        if let Some(s) = v["s"].as_u64() {
            self.seq = Some(s);
        }
        match v["op"].as_u64().unwrap_or(99) {
            // Hello
            10 => {
                let ms = v["d"]["heartbeat_interval"].as_u64().unwrap_or(41_250);
                let interval = Duration::from_millis(ms);
                self.heartbeat = Some(interval);
                let jitter = (crate::util::random_u64() % 1000) as f64 / 1000.0;
                self.next_heartbeat = Some(Instant::now() + interval.mul_f64(jitter));
                self.acked = true;
                if let (Some(session), Some(seq)) = (self.session_id.clone(), self.seq) {
                    self.send(6, json!({ "token": token, "session_id": session, "seq": seq }));
                } else {
                    self.send(
                        2,
                        json!({
                            "token": token,
                            "intents": INTENTS,
                            "properties": { "os": "pumpkin", "browser": "ace-srv", "device": "ace-srv" },
                            "presence": presence,
                        }),
                    );
                }
                self.identify_sent = true;
            }
            // Dispatch
            0 => {
                let t = v["t"].as_str().unwrap_or_default().to_string();
                if self.debug && t != "PRESENCE_UPDATE" && t != "TYPING_START" {
                    tracing::info!("[debug] gateway dispatch {t}");
                }
                match t.as_str() {
                    "READY" => {
                        self.session_id = v["d"]["session_id"].as_str().map(String::from);
                        self.resume_host = v["d"]["resume_gateway_url"].as_str().map(host_of);
                        self.ready = true;
                        self.backoff = Duration::from_secs(1);
                    }
                    "RESUMED" => {
                        self.ready = true;
                        self.backoff = Duration::from_secs(1);
                        tracing::info!("Discord gateway session resumed");
                    }
                    _ => {}
                }
                let mut v = v;
                events.push(GwEvent::Dispatch(t, v["d"].take()));
            }
            // Heartbeat request
            1 => {
                let seq = self.seq.map_or(Value::Null, Value::from);
                self.send(1, seq);
            }
            // Reconnect
            7 => self.disconnect(true, "Discord asked to reconnect"),
            // Invalid session
            9 => {
                let resumable = v["d"].as_bool().unwrap_or(false);
                self.backoff = Duration::from_millis(1000 + crate::util::random_u64() % 4000);
                self.disconnect(resumable, "invalid session");
            }
            // Heartbeat ACK
            11 => self.acked = true,
            _ => {}
        }
    }

    /// Gateway op 3: update the bot's status.
    pub fn update_presence(&mut self, presence: Value) {
        if self.ready {
            self.send(3, presence);
        }
    }
}

/// Builds a presence payload from a status line like "watching 3/20 players".
pub fn presence(status_line: &str, online_status: &str) -> Value {
    let (kind, text) = status_line.split_once(' ').unwrap_or(("playing", status_line));
    let (activity_type, name) = match kind.to_ascii_lowercase().as_str() {
        "playing" => (0, text),
        "streaming" => (1, text),
        "listening" => (2, text),
        "watching" => (3, text),
        "custom" => (4, text),
        "competing" => (5, text),
        _ => (0, status_line),
    };
    let activity = if activity_type == 4 {
        json!({ "type": 4, "name": "Custom Status", "state": name })
    } else {
        json!({ "type": activity_type, "name": name })
    };
    let status = match online_status.to_ascii_lowercase().as_str() {
        "idle" => "idle",
        "dnd" | "do_not_disturb" => "dnd",
        "invisible" => "invisible",
        _ => "online",
    };
    json!({ "since": Value::Null, "activities": [activity], "status": status, "afk": false })
}
