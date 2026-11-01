//! Recording flows as they pass through the proxy.
//!
//! Bodies are never buffered: each one is wrapped in a [`TapBody`] that
//! forwards frames as they arrive and keeps a copy of the first `log_max`
//! bytes, past which the whole body goes to a file instead. A flow is submitted to the [`Recorder`] when the last thing holding
//! it (the handler, the request body, the response body) is dropped.

use crate::ca::CRL_PATH;
use crate::rules::HostRules;
use bytes::Bytes;
use http::{Method, Request, Response, StatusCode, header};
use http_body::{Frame, SizeHint};
use http_body_util::combinators::BoxBody;
use hudsucker::{Body, HttpContext, HttpHandler, RequestOrResponse, WebSocketHandler};
use std::{
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
use tracewing_store::store::{BodyRecord, FlowRecord, Recorder};

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
        let (cap, tapped, spool) = (self.log_max, Arc::clone(&flow), self.spool.clone());
        let req = req.map(move |body| TapBody::wrap(body, tapped, Side::Request, cap, spool));
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
        let (cap, spool) = (self.log_max, self.spool.clone());
        res.map(move |body| TapBody::wrap(body, flow, Side::Response, cap, spool))
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

/// WebSocket messages are forwarded as-is; only the upgrade request is
/// recorded for now.
impl WebSocketHandler for FlowHandler {}

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
        self.recorder.submit(std::mem::take(record));
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
}

impl TapBody {
    fn wrap(inner: Body, flow: Arc<Flow>, side: Side, cap: usize, spool: Spool) -> Body {
        Body::from(BoxBody::new(Self {
            inner,
            flow,
            side,
            cap,
            body: BodyRecord::default(),
            done: false,
            spool,
            file: None,
        }))
    }

    fn observe(&mut self, data: &Bytes) {
        self.body.bytes += data.len() as u64;
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
