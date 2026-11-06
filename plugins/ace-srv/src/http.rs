//! Non-blocking HTTPS client over `wasi:http` (TLS is done by the host). Requests start immediately and are polled
//! once per tick, so a slow Discord API never holds up the server. Honours Discord's 429 `retry_after`.

use crate::state::State;
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use wasi::http::outgoing_handler;
use wasi::http::types::{
    Fields, FutureIncomingResponse, IncomingBody, IncomingResponse, Method as WMethod, OutgoingBody, OutgoingRequest,
    Scheme,
};
use wasi::io::streams::{InputStream, StreamError};

const MAX_IN_FLIGHT: usize = 6;
const MAX_ATTEMPTS: u8 = 4;
const USER_AGENT: &str = concat!("DiscordBot (https://github.com/ashfaqizop/ace-studio, ", env!("CARGO_PKG_VERSION"), ") Ace-SRV");

#[derive(Clone, Copy, Debug)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

pub struct Response {
    /// HTTP status, or 0 when the request failed before a response arrived.
    pub status: u16,
    pub body: Vec<u8>,
}

impl Response {
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }

    pub fn json(&self) -> Option<serde_json::Value> {
        serde_json::from_slice(&self.body).ok()
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

pub type Callback = Box<dyn FnOnce(&mut State, Response) + Send>;

pub struct Request {
    pub method: Method,
    pub host: &'static str,
    pub path: String,
    /// `Bot <token>` authorisation header; None for webhook calls.
    pub auth: Option<String>,
    pub body: Option<Vec<u8>>,
    pub callback: Option<Callback>,
    attempts: u8,
    not_before: Option<Instant>,
}

impl Request {
    pub fn new(method: Method, path: impl Into<String>) -> Self {
        Request {
            method,
            host: "discord.com",
            path: path.into(),
            auth: None,
            body: None,
            callback: None,
            attempts: 0,
            not_before: None,
        }
    }

    pub fn json(mut self, body: &serde_json::Value) -> Self {
        self.body = Some(body.to_string().into_bytes());
        self
    }

    pub fn auth(mut self, token: &str) -> Self {
        self.auth = Some(format!("Bot {token}"));
        self
    }

    pub fn then(mut self, cb: impl FnOnce(&mut State, Response) + Send + 'static) -> Self {
        self.callback = Some(Box::new(cb));
        self
    }
}

/// One request on the wire. Field order is drop order: child resources go before their parents.
struct InFlight {
    stream: Option<InputStream>,
    body: Option<IncomingBody>,
    response: Option<IncomingResponse>,
    future: Option<FutureIncomingResponse>,
    status: u16,
    buf: Vec<u8>,
    req: Request,
}

#[derive(Default)]
pub struct Http {
    queue: VecDeque<Request>,
    in_flight: Vec<InFlight>,
    /// Global pause after a global rate limit.
    paused_until: Option<Instant>,
    /// Requests that failed to start; their callbacks run on the next pump.
    failed: Vec<(Callback, Response)>,
    pub debug: bool,
}

enum Poll {
    Pending,
    Done,
    Failed(String),
}

fn start(req: &Request) -> Result<FutureIncomingResponse, String> {
    let mut headers: Vec<(String, Vec<u8>)> = vec![("user-agent".into(), USER_AGENT.as_bytes().to_vec())];
    if let Some(auth) = &req.auth {
        headers.push(("authorization".into(), auth.as_bytes().to_vec()));
    }
    if let Some(body) = &req.body {
        headers.push(("content-type".into(), b"application/json".to_vec()));
        headers.push(("content-length".into(), body.len().to_string().into_bytes()));
    }
    let fields = Fields::from_list(&headers).map_err(|e| format!("bad header: {e:?}"))?;
    let out = OutgoingRequest::new(fields);
    let method = match req.method {
        Method::Get => WMethod::Get,
        Method::Post => WMethod::Post,
        Method::Put => WMethod::Put,
        Method::Patch => WMethod::Patch,
        Method::Delete => WMethod::Delete,
    };
    out.set_method(&method).map_err(|()| "bad method".to_string())?;
    out.set_scheme(Some(&Scheme::Https)).map_err(|()| "bad scheme".to_string())?;
    out.set_authority(Some(req.host)).map_err(|()| "bad host".to_string())?;
    out.set_path_with_query(Some(&req.path)).map_err(|()| "bad path".to_string())?;
    let body = out.body().map_err(|()| "request body unavailable".to_string())?;
    let future = outgoing_handler::handle(out, None).map_err(|e| format!("{e:?}"))?;
    {
        let stream = body.write().map_err(|()| "request stream unavailable".to_string())?;
        if let Some(bytes) = &req.body {
            for chunk in bytes.chunks(4096) {
                stream.blocking_write_and_flush(chunk).map_err(|e| format!("write failed: {e:?}"))?;
            }
        }
    }
    OutgoingBody::finish(body, None).map_err(|e| format!("{e:?}"))?;
    Ok(future)
}

impl InFlight {
    fn poll(&mut self) -> Poll {
        if let Some(future) = &self.future {
            match future.get() {
                None => return Poll::Pending,
                Some(Err(())) => return Poll::Failed("response already taken".into()),
                Some(Ok(Err(code))) => return Poll::Failed(format!("{code:?}")),
                Some(Ok(Ok(resp))) => {
                    self.status = resp.status();
                    let body = match resp.consume() {
                        Ok(b) => b,
                        Err(()) => return Poll::Failed("response body unavailable".into()),
                    };
                    match body.stream() {
                        Ok(s) => self.stream = Some(s),
                        Err(()) => return Poll::Failed("response stream unavailable".into()),
                    }
                    self.body = Some(body);
                    self.response = Some(resp);
                }
            }
            self.future = None;
        }
        let Some(stream) = &self.stream else { return Poll::Done };
        loop {
            match stream.read(64 * 1024) {
                Ok(bytes) if bytes.is_empty() => return Poll::Pending,
                Ok(bytes) => {
                    self.buf.extend_from_slice(&bytes);
                    if self.buf.len() > 8 * 1024 * 1024 {
                        return Poll::Failed("response too large".into());
                    }
                }
                Err(StreamError::Closed) => return Poll::Done,
                Err(StreamError::LastOperationFailed(e)) => return Poll::Failed(e.to_debug_string()),
            }
        }
    }

    fn pollable(&self) -> Option<wasi::io::poll::Pollable> {
        if let Some(f) = &self.future {
            return Some(f.subscribe());
        }
        self.stream.as_ref().map(InputStream::subscribe)
    }
}

impl Http {
    pub fn send(&mut self, req: Request) {
        self.queue.push_back(req);
    }

    pub fn is_idle(&self) -> bool {
        self.queue.is_empty() && self.in_flight.is_empty()
    }

    fn start_queued(&mut self) {
        let now = Instant::now();
        if self.paused_until.is_some_and(|t| t > now) {
            return;
        }
        let mut waiting = VecDeque::new();
        while self.in_flight.len() < MAX_IN_FLIGHT {
            let Some(mut req) = self.queue.pop_front() else { break };
            if req.not_before.is_some_and(|t| t > now) {
                waiting.push_back(req);
                continue;
            }
            req.attempts += 1;
            match start(&req) {
                Ok(future) => self.in_flight.push(InFlight {
                    stream: None,
                    body: None,
                    response: None,
                    future: Some(future),
                    status: 0,
                    buf: Vec::new(),
                    req,
                }),
                Err(e) => {
                    tracing::warn!("Discord request {} {} could not start: {e}", method_name(req.method), req.path);
                    if let Some(cb) = req.callback.take() {
                        // run failure callbacks on the next pump, with the lock held by the caller
                        self.failed.push((cb, Response { status: 0, body: e.into_bytes() }));
                    }
                }
            }
        }
        waiting.append(&mut self.queue);
        self.queue = waiting;
    }

    /// Advances every request; returns finished callbacks for the caller to run with `&mut State`.
    pub fn pump(&mut self) -> Vec<(Callback, Response)> {
        self.start_queued();
        let mut done = std::mem::take(&mut self.failed);
        let mut i = 0;
        while i < self.in_flight.len() {
            match self.in_flight[i].poll() {
                Poll::Pending => i += 1,
                outcome => {
                    let mut f = self.in_flight.swap_remove(i);
                    let failure = if let Poll::Failed(e) = outcome { Some(e) } else { None };
                    let status = if failure.is_some() { 0 } else { f.status };
                    let body = std::mem::take(&mut f.buf);
                    let mut req = std::mem::replace(&mut f.req, Request::new(Method::Get, ""));
                    drop(f);
                    if self.debug {
                        tracing::info!("[debug] {} {} -> {status}", method_name(req.method), req.path);
                    }
                    if status == 429 && req.attempts < MAX_ATTEMPTS {
                        let parsed: Option<serde_json::Value> = serde_json::from_slice(&body).ok();
                        let secs = parsed.as_ref().and_then(|v| v["retry_after"].as_f64()).unwrap_or(1.0);
                        let wait = Instant::now() + Duration::from_millis((secs * 1000.0) as u64 + 50);
                        if parsed.as_ref().and_then(|v| v["global"].as_bool()).unwrap_or(false) {
                            self.paused_until = Some(wait);
                        }
                        req.not_before = Some(wait);
                        self.queue.push_front(req);
                        continue;
                    }
                    if status >= 500 && req.attempts < MAX_ATTEMPTS {
                        req.not_before = Some(Instant::now() + Duration::from_secs(u64::from(req.attempts) * 2));
                        self.queue.push_back(req);
                        continue;
                    }
                    if let Some(e) = &failure {
                        tracing::warn!("Discord request {} {} failed: {e}", method_name(req.method), req.path);
                    } else if !(200..300).contains(&status) {
                        tracing::warn!(
                            "Discord answered {status} to {} {}: {}",
                            method_name(req.method),
                            req.path,
                            String::from_utf8_lossy(&body).chars().take(300).collect::<String>()
                        );
                    }
                    if let Some(cb) = req.callback.take() {
                        done.push((cb, Response { status, body }));
                    }
                }
            }
        }
        self.start_queued();
        done
    }

    /// Blocks until all requests finish or `timeout` passes (only used while the server shuts down).
    pub fn wait_blocking(&mut self, timeout: Duration) -> Vec<(Callback, Response)> {
        let deadline = Instant::now() + timeout;
        let mut all = Vec::new();
        loop {
            all.extend(self.pump());
            if self.is_idle() || Instant::now() >= deadline {
                break;
            }
            let left = deadline.saturating_duration_since(Instant::now());
            let timer = wasi::clocks::monotonic_clock::subscribe_duration(left.as_nanos().min(50_000_000) as u64);
            let mut pollables: Vec<wasi::io::poll::Pollable> =
                self.in_flight.iter().filter_map(InFlight::pollable).collect();
            pollables.push(timer);
            let refs: Vec<&wasi::io::poll::Pollable> = pollables.iter().collect();
            let _ = wasi::io::poll::poll(&refs);
        }
        all
    }
}

fn method_name(m: Method) -> &'static str {
    match m {
        Method::Get => "GET",
        Method::Post => "POST",
        Method::Put => "PUT",
        Method::Patch => "PATCH",
        Method::Delete => "DELETE",
    }
}
