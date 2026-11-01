//! Recording flows as they pass through the proxy.
//!
//! Bodies are never buffered: each one is wrapped in a [`TapBody`] that
//! forwards frames as they arrive and keeps a copy of the first `log_max`
//! bytes, past which the whole body goes to a file instead. A flow is submitted to the [`Recorder`] when the last thing holding
//! it (the handler, the request body, the response body) is dropped.
//!
//! A WebSocket or server-sent events can last for hours, so those get a row as they
//! open and their messages as they come: see [`StreamHandle`].

use crate::ca::CRL_PATH;
use crate::rules::HostRules;
use bytes::Bytes;
use http::{Method, Request, Response, StatusCode, header};
use http_body::{Frame, SizeHint};
use http_body_util::combinators::BoxBody;
use hudsucker::{
    Body, HttpContext, HttpHandler, RequestOrResponse, WebSocketContext, WebSocketHandler,
    futures::{Sink, SinkExt, Stream, StreamExt},
    tokio_tungstenite::tungstenite::{self, Message as WsMessage},
};
use std::{
    collections::HashMap,
    error::Error as StdError,
    net::SocketAddr,
    pin::Pin,
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tracewing_store::spool::{Spool, SpoolFile};
use tracewing_store::store::{BodyRecord, FlowRecord, Recorder, StreamHandle};

/// The CRL the proxy answers for itself, and the address it answers on.
#[derive(Clone)]
pub struct Crl {
    pub host: String,
    pub der: Bytes,
}

/// A host that exists only here: the proxy answers for it. A browser that gets the
/// page over HTTPS went through the proxy and trusts its certificate, which no other
/// check can tell, as browsers keep trust stores of their own.
pub const CHECK_HOST: &str = "tracewing.test";

const CHECK_PAGE: &str = r#"<!doctype html>
<html><head><meta charset="utf-8"><title>tracewing</title>
<style>
body { font: 16px system-ui, sans-serif; margin: 15vh auto; max-width: 30em; padding: 0 1em;
       color: #161b26; background: #f4f6fb; }
@media (prefers-color-scheme: dark) { body { color: #e4e5e8; background: #141517; } }
</style></head>
<body><h1>It works</h1>
<p>This browser's traffic goes through tracewing. You can close this page.</p></body></html>
"#;

/// The last browser to reach the test page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckVisit {
    pub user_agent: String,
    /// Over HTTPS, so the browser trusts the certificate.
    pub secure: bool,
}

#[derive(Debug, Default)]
pub struct Check(Mutex<Option<CheckVisit>>);

impl Check {
    pub fn last(&self) -> Option<CheckVisit> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn saw(&self, visit: CheckVisit) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = Some(visit);
    }
}

#[derive(Clone)]
pub struct FlowHandler {
    recorder: Recorder,
    log_max: usize,
    crl: Crl,
    spool: Spool,
    recording: Arc<AtomicBool>,
    rules: Arc<RwLock<HostRules>>,
    check: Arc<Check>,
    /// WebSockets from their upgrade until they end. hudsucker tells the handler of
    /// messages only by client and URL, and this is what ties those to a flow.
    websockets: Arc<Mutex<HashMap<WsKey, Arc<Flow>>>>,
    /// The flow of the request this clone is handling. hudsucker clones the
    /// handler per request, so request and response see the same value.
    flow: Option<Arc<Flow>>,
}

impl FlowHandler {
    pub fn new(
        recorder: Recorder,
        log_max: usize,
        crl: Crl,
        spool: Spool,
        recording: Arc<AtomicBool>,
        rules: Arc<RwLock<HostRules>>,
        check: Arc<Check>,
    ) -> Self {
        Self {
            recorder,
            log_max,
            crl,
            spool,
            recording,
            rules,
            check,
            websockets: Arc::default(),
            flow: None,
        }
    }
}

impl HttpHandler for FlowHandler {
    async fn handle_request(&mut self, ctx: &HttpContext, req: Request<Body>) -> RequestOrResponse {
        // The tunnel itself is not a flow; the requests inside it are.
        if req.method() == Method::CONNECT {
            return req.into();
        }
        // Revocation checks ask for this while the handshake waits, directly or
        // through the system proxy. They are not traffic the user sent.
        if req.uri().path() == CRL_PATH && request_host(&req) == Some(self.crl.host.as_str()) {
            return Response::builder()
                .header(header::CONTENT_TYPE, "application/pkix-crl")
                .body(Body::from(http_body_util::Full::new(self.crl.der.clone())))
                .expect("valid response")
                .into();
        }

        let scheme = req.uri().scheme_str().unwrap_or("http");
        if request_host(&req).map(|host| without_default_port(host, scheme)) == Some(CHECK_HOST) {
            self.check.saw(CheckVisit {
                user_agent: header_text(req.headers(), header::USER_AGENT).unwrap_or_default(),
                secure: scheme == "https",
            });
            return Response::builder()
                .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                .body(Body::from(http_body_util::Full::new(Bytes::from_static(
                    CHECK_PAGE.as_bytes(),
                ))))
                .expect("valid response")
                .into();
        }
        // Paused: traffic goes through as if nothing were watching.
        if !self.recording.load(Ordering::Relaxed) {
            return req.into();
        }

        let flow = Arc::new(Flow::start(self.recorder.clone(), ctx.client_addr, &req));
        if is_websocket(&req) {
            // hudsucker answers an upgrade itself, and handle_response never hears.
            flow.opened_as("websocket", Some(101));
            lock(&self.websockets)
                .insert(WsKey::new(ctx.client_addr, req.uri()), Arc::clone(&flow));
        }
        let (cap, tapped, spool) = (self.log_max, Arc::clone(&flow), self.spool.clone());
        let req = req.map(move |body| TapBody::wrap(body, tapped, Side::Request, cap, spool, None));
        self.flow = Some(flow);
        req.into()
    }

    /// Left alone, a connection is a row and nothing more: all that can be known is
    /// where it went.
    async fn should_intercept_connect(&mut self, ctx: &HttpContext, req: &Request<Body>) -> bool {
        let Some(host) = req.uri().authority().map(|a| a.as_str()) else {
            return true;
        };
        let leave = self
            .rules
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .leaves_alone(host);
        if leave && self.recording.load(Ordering::Relaxed) {
            self.recorder.submit(FlowRecord {
                started_us: unix_micros(),
                client: ctx.client_addr.to_string(),
                kind: "tunnel",
                method: "CONNECT".to_owned(),
                scheme: "https".to_owned(),
                host: without_default_port(host, "https").to_owned(),
                client_version: format!("{:?}", req.version()),
                status: Some(200),
                request: BodyRecord {
                    complete: true,
                    ..BodyRecord::default()
                },
                response: BodyRecord {
                    complete: true,
                    ..BodyRecord::default()
                },
                ..FlowRecord::default()
            });
        }
        !leave
    }

    async fn handle_response(&mut self, _ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        let Some(flow) = self.flow.take() else {
            return res;
        };
        flow.on_response(&res);
        // Text only: events that come compressed cannot be told apart as they come.
        let events = is_event_stream(&res) && !res.headers().contains_key(header::CONTENT_ENCODING);
        if events {
            flow.opened_as("sse", None);
        }
        let stream = events.then(|| flow.stream()).flatten();
        let (cap, spool) = (self.log_max, self.spool.clone());
        res.map(move |body| TapBody::wrap(body, flow, Side::Response, cap, spool, stream))
    }

    async fn handle_error(
        &mut self,
        _ctx: &HttpContext,
        err: hyper_util::client::legacy::Error,
    ) -> Response<Body> {
        let message = error_chain(&err);
        if let Some(flow) = self.flow.take() {
            flow.fail(message.clone());
        }
        Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(Body::from(format!("tracewing: {message}\n")))
            .expect("valid response")
    }
}

/// hudsucker's own loop, and each message written down on its way.
impl WebSocketHandler for FlowHandler {
    async fn handle_websocket(
        self,
        ctx: WebSocketContext,
        mut stream: impl Stream<Item = Result<WsMessage, tungstenite::Error>> + Unpin + Send + 'static,
        mut sink: impl Sink<WsMessage, Error = tungstenite::Error> + Unpin + Send + 'static,
    ) {
        let (key, from_client) = match &ctx {
            WebSocketContext::ClientToServer { src, dst, .. } => (WsKey::new(*src, dst), true),
            WebSocketContext::ServerToClient { src, dst, .. } => (WsKey::new(*dst, src), false),
        };
        let flow = lock(&self.websockets).get(&key).cloned();
        let recorded = flow.as_ref().and_then(|flow| flow.stream());
        while let Some(message) = stream.next().await {
            match message {
                Ok(message) => {
                    if let Some(recorded) = &recorded {
                        note_message(recorded, from_client, &message);
                    }
                    match sink.send(message).await {
                        Err(tungstenite::Error::ConnectionClosed) | Ok(()) => {}
                        Err(e) => tracing::debug!("WebSocket send: {e}"),
                    }
                }
                Err(e) => {
                    tracing::debug!("WebSocket message: {e}");
                    let _ = sink.send(WsMessage::Close(None)).await;
                    break;
                }
            }
        }
        // Either side ending ends it; the row is written when the other lets go too.
        lock(&self.websockets).remove(&key);
    }
}

fn note_message(stream: &StreamHandle, from_client: bool, message: &WsMessage) {
    let (kind, data): (&'static str, &[u8]) = match message {
        WsMessage::Text(text) => ("text", text.as_bytes()),
        WsMessage::Binary(data) => ("binary", data),
        WsMessage::Ping(data) => ("ping", data),
        WsMessage::Pong(data) => ("pong", data),
        WsMessage::Close(_) => ("close", &[]),
        WsMessage::Frame(_) => return,
    };
    stream.message(from_client, kind, data);
}

/// A WebSocket's client and where it went. hudsucker changes the scheme on the way,
/// so that is left out.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct WsKey {
    client: SocketAddr,
    target: String,
}

impl WsKey {
    fn new(client: SocketAddr, uri: &http::Uri) -> Self {
        Self {
            client,
            target: format!(
                "{}{}",
                uri.authority().map_or("", |a| a.as_str()),
                uri.path_and_query().map_or("/", |p| p.as_str())
            ),
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

fn is_websocket<T>(req: &Request<T>) -> bool {
    req.headers()
        .get(header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.eq_ignore_ascii_case("websocket"))
}

fn is_event_stream<T>(res: &Response<T>) -> bool {
    res.headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.to_ascii_lowercase().starts_with("text/event-stream"))
}

/// The most of one event waited for before it is written as it stands.
const EVENT_MAX: usize = 1024 * 1024;

/// Cuts server-sent events apart: they end in a blank line, and frames need not
/// fall on the ends of events.
#[derive(Debug, Default)]
struct EventSplitter {
    buffer: Vec<u8>,
}

impl EventSplitter {
    fn feed(&mut self, data: &[u8], mut event: impl FnMut(&[u8])) {
        self.buffer.extend_from_slice(data);
        let mut from = 0;
        while let Some((end, next)) = event_end(&self.buffer[from..]) {
            event(&self.buffer[from..from + end]);
            from += next;
        }
        self.buffer.drain(..from);
        if self.buffer.len() > EVENT_MAX {
            event(&self.buffer);
            self.buffer.clear();
        }
    }

    fn finish(&mut self, mut event: impl FnMut(&[u8])) {
        if !self.buffer.is_empty() {
            event(&self.buffer);
            self.buffer.clear();
        }
    }
}

/// Where the first event ends, and where the next begins.
fn event_end(data: &[u8]) -> Option<(usize, usize)> {
    let newlines = data
        .windows(2)
        .position(|w| w == b"\n\n")
        .map(|i| (i, i + 2));
    let returns = data
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|i| (i, i + 4));
    match (newlines, returns) {
        (Some(a), Some(b)) => Some(if a.0 < b.0 { a } else { b }),
        (a, b) => a.or(b),
    }
}

fn request_host<T>(req: &Request<T>) -> Option<&str> {
    req.uri().authority().map(|a| a.as_str()).or_else(|| {
        req.headers()
            .get(header::HOST)
            .and_then(|h| h.to_str().ok())
    })
}

/// HTTP/1.1 inside a tunnel says `example.com:443` where HTTP/2 says
/// `example.com`; a list of hosts should show one of them.
fn without_default_port<'a>(host: &'a str, scheme: &str) -> &'a str {
    let port = if scheme == "https" { ":443" } else { ":80" };
    host.strip_suffix(port).unwrap_or(host)
}

fn header_text(headers: &http::HeaderMap, name: header::HeaderName) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

fn header_lines(headers: &http::HeaderMap) -> String {
    let mut lines = String::new();
    for (name, value) in headers {
        lines.push_str(name.as_str());
        lines.push_str(": ");
        lines.push_str(&String::from_utf8_lossy(value.as_bytes()));
        lines.push('\n');
    }
    lines
}

pub struct Flow {
    recorder: Recorder,
    started: Instant,
    record: Mutex<FlowRecord>,
    stream: Mutex<Option<StreamHandle>>,
}

impl Flow {
    fn start(recorder: Recorder, client: SocketAddr, req: &Request<Body>) -> Self {
        let uri = req.uri();
        let is_websocket = req
            .headers()
            .get(header::UPGRADE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.eq_ignore_ascii_case("websocket"));

        let record = FlowRecord {
            started_us: unix_micros(),
            client: client.to_string(),
            kind: if is_websocket { "websocket" } else { "http" },
            method: req.method().to_string(),
            scheme: uri.scheme_str().unwrap_or("http").to_owned(),
            host: without_default_port(
                request_host(req).unwrap_or_default(),
                uri.scheme_str().unwrap_or("http"),
            )
            .to_owned(),
            path: uri.path_and_query().map_or("/", |p| p.as_str()).to_owned(),
            client_version: format!("{:?}", req.version()),
            req_content_type: header_text(req.headers(), header::CONTENT_TYPE),
            req_encoding: header_text(req.headers(), header::CONTENT_ENCODING),
            req_headers: header_lines(req.headers()),
            ..FlowRecord::default()
        };

        Self {
            recorder,
            started: Instant::now(),
            record: Mutex::new(record),
            stream: Mutex::new(None),
        }
    }

    fn on_response(&self, res: &Response<Body>) {
        let mut record = self.lock();
        record.status = Some(res.status().as_u16());
        record.server_version = Some(format!("{:?}", res.version()));
        record.content_type = header_text(res.headers(), header::CONTENT_TYPE);
        record.resp_encoding = header_text(res.headers(), header::CONTENT_ENCODING);
        record.resp_headers = header_lines(res.headers());
        record.ttfb_us = Some(self.elapsed_us());
    }

    /// Writes the row now, for a stream that may go on for hours.
    fn opened_as(&self, kind: &'static str, status: Option<u16>) {
        let snapshot = {
            let mut record = self.lock();
            record.kind = kind;
            if status.is_some() {
                record.status = status;
            }
            record.clone()
        };
        *lock(&self.stream) = Some(self.recorder.open_stream(snapshot));
    }

    fn stream(&self) -> Option<StreamHandle> {
        lock(&self.stream).clone()
    }

    fn fail(&self, message: String) {
        self.lock().error.get_or_insert(message);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, FlowRecord> {
        self.record.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn elapsed_us(&self) -> i64 {
        self.started.elapsed().as_micros() as i64
    }
}

impl Drop for Flow {
    fn drop(&mut self) {
        let elapsed = self.elapsed_us();
        let record = self.record.get_mut().unwrap_or_else(|e| e.into_inner());
        record.duration_us.get_or_insert(elapsed);
        let record = std::mem::take(record);
        match lock(&self.stream).take() {
            Some(stream) => stream.finish(record),
            None => self.recorder.submit(record),
        }
    }
}

#[derive(Clone, Copy)]
enum Side {
    Request,
    Response,
}

struct TapBody {
    inner: Body,
    flow: Arc<Flow>,
    side: Side,
    cap: usize,
    body: BodyRecord,
    done: bool,
    spool: Spool,
    file: Option<SpoolFile>,
    /// Server-sent events, to cut apart as they pass.
    events: Option<(StreamHandle, EventSplitter)>,
}

impl TapBody {
    fn wrap(
        inner: Body,
        flow: Arc<Flow>,
        side: Side,
        cap: usize,
        spool: Spool,
        events: Option<StreamHandle>,
    ) -> Body {
        Body::from(BoxBody::new(Self {
            inner,
            flow,
            side,
            cap,
            body: BodyRecord::default(),
            done: false,
            spool,
            file: None,
            events: events.map(|stream| (stream, EventSplitter::default())),
        }))
    }

    fn observe(&mut self, data: &Bytes) {
        self.body.bytes += data.len() as u64;
        if let Some((stream, splitter)) = &mut self.events {
            splitter.feed(data, |event| stream.message(false, "event", event));
        }
        let room = self.cap.saturating_sub(self.body.captured.len());
        if data.len() > room {
            self.body.truncated = true;
            // The first frame past the cap: what was kept goes first.
            if self.file.is_none() {
                let mut file = self.spool.begin();
                self.body.spool_file = Some(file.name());
                file.write(Bytes::copy_from_slice(&self.body.captured));
                self.file = Some(file);
            }
        }
        self.body
            .captured
            .extend_from_slice(&data[..data.len().min(room)]);
        if let Some(file) = &mut self.file {
            file.write(data.clone());
        }
    }

    fn finish(&mut self, complete: bool, error: Option<String>) {
        if std::mem::replace(&mut self.done, true) {
            return;
        }
        self.body.complete = complete;
        if let Some((stream, splitter)) = &mut self.events {
            splitter.finish(|event| stream.message(false, "event", event));
        }
        if let Some(file) = self.file.take() {
            self.body.spool_lost = file.is_lost();
            if file.kept() as usize >= self.body.captured.len() && file.kept() > 0 {
                // The file holds all of it, or at least as much as memory does.
                self.body.captured = Vec::new();
                self.body.truncated = file.is_lost();
            } else {
                self.body.spool_file = None;
            }
        }
        let elapsed = self.flow.elapsed_us();
        let mut record = self.flow.lock();
        if let Some(error) = error {
            record.error.get_or_insert(error);
        }
        let body = std::mem::take(&mut self.body);
        match self.side {
            Side::Request => record.request = body,
            Side::Response => {
                record.response = body;
                record.duration_us = Some(elapsed);
            }
        }
    }
}

impl http_body::Body for TapBody {
    type Data = Bytes;
    type Error = hudsucker::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Self::Error>>> {
        let this = self.get_mut();
        let polled = Pin::new(&mut this.inner).poll_frame(cx);
        match &polled {
            Poll::Ready(Some(Ok(frame))) => {
                if let Some(data) = frame.data_ref() {
                    this.observe(data);
                }
            }
            Poll::Ready(Some(Err(e))) => this.finish(false, Some(error_chain(e))),
            Poll::Ready(None) => this.finish(true, None),
            Poll::Pending => {}
        }
        polled
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}

impl Drop for TapBody {
    fn drop(&mut self) {
        // hyper does not poll a body it already knows is empty, so an
        // end-of-stream body that is dropped still counts as complete.
        let complete = http_body::Body::is_end_stream(&self.inner);
        self.finish(complete, None);
    }
}

pub fn error_chain(err: &dyn StdError) -> String {
    let mut message = err.to_string();
    let mut source = err.source();
    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    message
}

fn unix_micros() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn events_of(frames: &[&[u8]]) -> Vec<Vec<u8>> {
        let mut splitter = EventSplitter::default();
        let mut out = Vec::new();
        for frame in frames {
            splitter.feed(frame, |event| out.push(event.to_vec()));
        }
        splitter.finish(|event| out.push(event.to_vec()));
        out
    }

    #[test]
    fn events_are_cut_wherever_frames_fall() {
        let want: Vec<Vec<u8>> = vec![b"data: a".to_vec(), b"data: b".to_vec()];
        assert_eq!(events_of(&[b"data: a\n\ndata: b\n\n"]), want);
        assert_eq!(events_of(&[b"data: ", b"a\n", b"\ndata: b\n\n"]), want);
        assert_eq!(events_of(&[b"data: a\r\n\r\ndata: b\r\n\r\n"]), want);
    }

    #[test]
    fn what_never_ended_still_counts() {
        assert_eq!(events_of(&[b"data: cut off"]), [b"data: cut off".to_vec()]);
        assert!(events_of(&[]).is_empty());
    }

    #[test]
    fn an_event_with_no_end_does_not_grow_forever() {
        let mut splitter = EventSplitter::default();
        let mut out = 0;
        splitter.feed(&vec![b'x'; EVENT_MAX + 1], |event| out += event.len());
        assert_eq!(out, EVENT_MAX + 1);
        assert!(splitter.buffer.is_empty());
    }

    #[test]
    fn a_websocket_is_known_whatever_its_scheme() {
        let client: SocketAddr = "127.0.0.1:5000".parse().unwrap();
        let https: http::Uri = "https://a.dev:443/ws?x=1".parse().unwrap();
        let wss: http::Uri = "wss://a.dev:443/ws?x=1".parse().unwrap();
        assert_eq!(WsKey::new(client, &https), WsKey::new(client, &wss));
    }
}
