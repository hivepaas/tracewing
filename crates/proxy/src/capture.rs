//! Recording flows as they pass through the proxy.
//!
//! Bodies are never buffered: each one is wrapped in a [`TapBody`] that
//! forwards frames as they arrive and keeps a copy of the first `log_max`
//! bytes, past which the whole body goes to a file instead. A flow is submitted to the [`Recorder`] when the last thing holding
//! it (the handler, the request body, the response body) is dropped.
//!
//! A WebSocket or server-sent events can last for hours, so those get a row as they
//! open and their messages as they come: see [`StreamHandle`].

use crate::apps::Apps;
use crate::ca::CRL_PATH;
use crate::pause::{self, Pauses, Read, Stage, Verdict};
use crate::rules::HostRules;
use crate::throttle::{Conditions, Direction, Network};
use crate::traffic_rules::{End, Plan, TrafficRules, content_type_of, edit_body, edit_query};
use crate::upstream::{BoxError, TimedConnector, TlsFailure, Upstream};
use bytes::Bytes;
use http::{HeaderName, HeaderValue, Method, Request, Response, StatusCode, header};
use http_body::{Frame, SizeHint};
use http_body_util::combinators::BoxBody;
use hudsucker::{
    Body, HttpContext, HttpHandler, RequestOrResponse,
    futures::{Sink, SinkExt, Stream, StreamExt},
    tokio_tungstenite::{
        self, WebSocketStream,
        tungstenite::{self, Message as WsMessage, protocol::Role},
    },
};
use hyper_util::rt::TokioIo;
use std::{
    error::Error as StdError,
    net::SocketAddr,
    path::PathBuf,
    pin::Pin,
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tracewing_store::app::AppAnswer;
use tracewing_store::decode::{Encoding, Unpacker};
use tracewing_store::settings::Live;
use tracewing_store::spool::{Spool, SpoolFile};
use tracewing_store::store::{BodyRecord, FlowRecord, Recorder, StreamHandle};
use tracewing_store::{binjson, graphql, jsonrpc, protobuf};

/// How many of a socket's messages are looked at for what it is. One that
/// speaks JSON-RPC subscribes in its first message or two; one that does not
/// should stop being asked.
const RPC_LOOK_AT: usize = 20;

/// The CRL the proxy answers for itself, and the address it answers on.
#[derive(Clone)]
pub struct Crl {
    pub host: String,
    pub der: Bytes,
    /// The CA's certificate, which the check page gives to a device to trust.
    pub ca_der: Bytes,
}

/// Where on the check host the CA's certificate is.
pub const CA_PATH: &str = "/ca.crt";

/// A host that exists only here: the proxy answers for it. A browser that gets the
/// page over HTTPS went through the proxy and trusts its certificate, which no other
/// check can tell, as browsers keep trust stores of their own.
pub const CHECK_HOST: &str = "tracewing.test";

const PAGE_HEAD: &str = r#"<!doctype html>
<html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>tracewing</title>
<style>
body { font: 16px/1.5 system-ui, sans-serif; margin: 10vh auto; max-width: 32em; padding: 0 1em;
       color: #161b26; background: #f4f6fb; }
a { color: #2f55d4; }
li { margin: 0 0 .8em; }
details { margin: .4em 0; }
summary { cursor: pointer; font-weight: 600; }
@media (prefers-color-scheme: dark) {
  body { color: #e4e5e8; background: #141517; }
  a { color: #8fa8ff; }
}
</style></head>
<body>
"#;

/// How to trust the certificate once it is on the device.
const TRUST_STEPS: &str = r#"<details><summary>iPhone and iPad</summary>
<p>Allow the download. Then in Settings, General, VPN &amp; Device Management, choose the
tracewing CA profile and install it. Last, in Settings, General, About, Certificate Trust
Settings, turn on full trust for tracewing CA.</p></details>
<details><summary>Android</summary>
<p>In Settings, Security, Encryption &amp; credentials (the names vary by maker), choose Install a
certificate, then CA certificate, and pick tracewing.crt. Apps trust it only if they allow
certificates the user added: tracewing's Set up page shows how, for an app of your own.</p></details>
"#;

/// The longest an answer waits for the program that asked to be found.
const APP_HOLD: std::time::Duration = std::time::Duration::from_millis(20);

/// A program that asks once and is gone as soon as it is answered, as curl
/// is, can be found only while it waits. Looking takes about a millisecond,
/// and a quick server answers sooner: so an answer that would beat the look
/// waits for it, a little. Only a client on this machine has a program to
/// find, and a connection is looked for once, so this is a first answer's.
async fn let_its_program_be_found(flow: &Flow) {
    let Some(answer) = flow.lock().app.clone() else {
        return;
    };
    if answer.now().is_some() {
        return;
    }
    // Woken the moment it is found, on a thread that may wait.
    let deadline = std::time::Instant::now() + APP_HOLD;
    let _ = tokio::task::spawn_blocking(move || answer.wait_until(deadline)).await;
}

/// A WebSocket's URL written whole, `ws://` or `wss://`, as curl sends one to
/// a proxy, as the `http://` or `https://` its upgrade starts as. hudsucker
/// takes any scheme but `http` for one with TLS, and would reach a server
/// with none by TLS.
fn with_http_scheme<B>(req: Request<B>) -> Request<B> {
    let scheme = match req.uri().scheme_str() {
        Some(s) if s.eq_ignore_ascii_case("ws") => http::uri::Scheme::HTTP,
        Some(s) if s.eq_ignore_ascii_case("wss") => http::uri::Scheme::HTTPS,
        _ => return req,
    };
    let (mut parts, body) = req.into_parts();
    let mut uri = parts.uri.clone().into_parts();
    uri.scheme = Some(scheme);
    if let Ok(uri) = http::Uri::from_parts(uri) {
        parts.uri = uri;
    }
    Request::from_parts(parts, body)
}

/// The page a browser gets from the proxy itself. `proxy` is the address it
/// was opened at, when it was opened directly rather than through the proxy:
/// a device that has not been set up yet.
fn check_page(proxy: Option<&str>, trusted: bool) -> String {
    let body = match proxy {
        Some(proxy) => {
            let (host, port) = match proxy.rsplit_once(':') {
                Some((host, port)) if !host.is_empty() => (host, port),
                _ => (proxy, "80"),
            };
            format!(
                r#"<h1>Set up this device</h1>
<ol>
<li>In this Wi-Fi network's settings, set the proxy to Manual: server <b>{host}</b>, port <b>{port}</b>.</li>
<li><a href="{CA_PATH}">Get the certificate</a>, and trust it:
{TRUST_STEPS}</li>
<li>Open <a href="https://{CHECK_HOST}/">https://{CHECK_HOST}</a>. If it says it works, this device is set up.</li>
</ol>
"#,
                host = escape_html(host),
                port = escape_html(port),
            )
        }
        None if trusted => "<h1>It works</h1>\n<p>This browser's traffic goes through tracewing, \
                            and it trusts tracewing's certificate.</p>\n"
            .to_owned(),
        None => format!(
            r#"<h1>It works</h1>
<p>This browser's traffic goes through tracewing. <a href="{CA_PATH}">Get the certificate</a> and
trust it, then open this page again over <a href="https://{CHECK_HOST}/">HTTPS</a>.</p>
{TRUST_STEPS}"#
        ),
    };
    format!("{PAGE_HEAD}{body}</body></html>\n")
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// A browser that reached the test page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckVisit {
    pub user_agent: String,
    /// `Sec-CH-UA`, which tells Chromium's family apart where the user agent
    /// does not. Browsers send it over HTTPS only.
    pub brands: String,
    /// Over HTTPS, so the browser trusts the certificate.
    pub secure: bool,
}

/// How many browsers the test page remembers.
const VISITS_KEPT: usize = 16;

/// The browsers that reached the test page, the latest visit of each, oldest
/// first.
#[derive(Debug, Default)]
pub struct Check(Mutex<Vec<CheckVisit>>);

impl Check {
    pub fn last(&self) -> Option<CheckVisit> {
        self.visits().pop()
    }

    pub fn visits(&self) -> Vec<CheckVisit> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn saw(&self, visit: CheckVisit) {
        let mut visits = self.0.lock().unwrap_or_else(|e| e.into_inner());
        visits.retain(|v| (&v.user_agent, &v.brands) != (&visit.user_agent, &visit.brands));
        visits.push(visit);
        if visits.len() > VISITS_KEPT {
            visits.remove(0);
        }
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
    traffic_rules: Arc<RwLock<TrafficRules>>,
    check: Arc<Check>,
    pauses: Arc<Pauses>,
    /// Which program each connection is from, when that is looked for.
    apps: Option<Apps>,
    /// See `StorageSettings::hide_http3`.
    live: Arc<Live>,
    /// The network traffic goes through, slow on purpose or not.
    network: Arc<Network>,
    relay: Arc<crate::relay::Relay>,
    connector: TimedConnector,
    /// What slowed the request this clone is handling, to slow its response too.
    slowed: Option<Conditions>,
    /// Headers rules set, or take out, on the response of the request this
    /// clone is handling.
    response_headers: Vec<(HeaderName, Option<HeaderValue>)>,
    /// The status rules give that response in place of the server's.
    response_status: Option<u16>,
    /// What rules change in that response's body.
    response_body: Vec<(String, String)>,
    /// The rule that holds its response, with the request's method and URL.
    pause_response: Option<(String, String, String)>,
    /// The flow of the request this clone is handling. hudsucker clones the
    /// handler per request, so request and response see the same value.
    flow: Option<Arc<Flow>>,
}

/// What can be changed, or seen, while the proxy runs.
pub struct Controls {
    pub recording: Arc<AtomicBool>,
    pub rules: Arc<RwLock<HostRules>>,
    pub traffic_rules: Arc<RwLock<TrafficRules>>,
    pub check: Arc<Check>,
    pub pauses: Arc<Pauses>,
    pub apps: Option<Apps>,
    /// The settings that apply as they change.
    pub live: Arc<Live>,
    /// Changes apply to the next request.
    pub network: Arc<Network>,
    /// The proxy that connections to servers go through, if any.
    pub relay: Arc<crate::relay::Relay>,
    /// What connects to servers, for the WebSockets the proxy opens itself.
    pub connector: TimedConnector,
}

impl FlowHandler {
    pub fn new(
        recorder: Recorder,
        log_max: usize,
        crl: Crl,
        spool: Spool,
        controls: Controls,
    ) -> Self {
        Self {
            recorder,
            log_max,
            crl,
            spool,
            recording: controls.recording,
            rules: controls.rules,
            traffic_rules: controls.traffic_rules,
            check: controls.check,
            pauses: controls.pauses,
            apps: controls.apps,
            live: controls.live,
            network: controls.network,
            relay: controls.relay,
            connector: controls.connector,
            slowed: None,
            response_headers: Vec::new(),
            response_status: None,
            response_body: Vec::new(),
            pause_response: None,
            flow: None,
        }
    }
}

impl FlowHandler {
    /// What the rules make of a request. With no rules that is one lock and one look,
    /// which is all most traffic ever pays.
    fn plan_for<T>(&self, req: &Request<T>) -> Plan {
        let rules = self.traffic_rules.read().unwrap_or_else(|e| e.into_inner());
        if rules.is_empty() {
            return Plan::default();
        }
        let uri = req.uri();
        let scheme = uri.scheme_str().unwrap_or("http");
        let host = without_default_port(request_host(req).unwrap_or_default(), scheme);
        let path = uri.path_and_query().map_or("/", |p| p.as_str());
        rules.plan(req.method().as_str(), &format!("{host}{path}"))
    }

    /// Holds a request for the window. What goes on is what it says, and true
    /// if it says to send nothing.
    async fn hold_request(
        &self,
        req: Request<Body>,
        rule: String,
        url: String,
    ) -> (Request<Body>, bool) {
        let held = match pause::request_message(req, url).await {
            Read::Held(held) => held,
            Read::Passed(req) => return (req, false),
        };
        match self
            .pauses
            .hold(Stage::Request, rule, held.message.clone())
            .await
        {
            Verdict::Send(message) if message == held.message => (held.unchanged(), false),
            Verdict::Send(message) => match pause::request_from(message) {
                Ok(req) => (req, false),
                // The window checks before it lets go, so this is a bug.
                Err(problem) => {
                    tracing::warn!("a held request cannot go on as edited: {problem}");
                    (held.unchanged(), false)
                }
            },
            Verdict::Drop => (held.unchanged(), true),
        }
    }

    async fn hold_response(
        &self,
        res: Response<Body>,
        (rule, method, url): (String, String, String),
    ) -> Response<Body> {
        let held = match pause::response_message(res, &method, &url).await {
            Read::Held(held) => held,
            Read::Passed(res) => return res,
        };
        match self
            .pauses
            .hold(Stage::Response, rule, held.message.clone())
            .await
        {
            Verdict::Send(message) if message == held.message => held.unchanged(),
            Verdict::Send(message) => pause::response_from(message).unwrap_or_else(|problem| {
                tracing::warn!("a held response cannot go on as edited: {problem}");
                held.unchanged()
            }),
            Verdict::Drop => pause::dropped(),
        }
    }

    fn app_of(&self, client: SocketAddr) -> Option<Arc<AppAnswer>> {
        self.apps.as_ref()?.of(client, false)
    }

    /// A rule's answer: no server was asked, so none is on record.
    fn answer_as_rule(&mut self, flow: Option<Arc<Flow>>, res: Response<Body>) -> Response<Body> {
        let kept = flow.clone();
        let res = self.answer(flow, res);
        if let Some(flow) = kept {
            flow.lock().server_version = None;
        }
        res
    }

    /// A response on its way to the client, from the server or from a rule.
    fn answer(&mut self, flow: Option<Arc<Flow>>, mut res: Response<Body>) -> Response<Body> {
        let set = std::mem::take(&mut self.response_headers);
        // A rule that says what `Alt-Svc` is means it.
        let hide_http3 =
            self.live.hide_http3() && !set.iter().any(|(name, _)| name == header::ALT_SVC);
        for (name, value) in set {
            match value {
                Some(value) => res.headers_mut().insert(name, value),
                None => res.headers_mut().remove(name),
            };
        }
        // Written down as the server sent it.
        if let Some(flow) = &flow {
            flow.on_response(&res);
        }
        let upgrade = res.status() == StatusCode::SWITCHING_PROTOCOLS;
        drop_hop_by_hop(res.headers_mut(), upgrade);
        if hide_http3 {
            hide_http3_offers(res.headers_mut());
        }
        let Some(flow) = flow else {
            return res;
        };
        // Packed, it is unpacked as it comes, if it is packed in a way that can be.
        let encoding = Encoding::from_header(
            res.headers()
                .get(header::CONTENT_ENCODING)
                .and_then(|v| v.to_str().ok()),
        );
        let cut = stream_cut(&res).filter(|_| encoding != Encoding::Unknown);
        if let Some(cut) = cut {
            flow.opened_as(cut.kind(), None);
        }
        let stream = cut.and_then(|cut| Some((flow.stream()?, cut, encoding)));
        // Any other response still going after a while is listed then: a long
        // poll, a log as it is written, a big download.
        if cut.is_none() && may_take_a_while(&res) {
            let flow = Arc::downgrade(&flow);
            tokio::spawn(async move {
                tokio::time::sleep(LISTED_AFTER).await;
                if let Some(flow) = flow.upgrade() {
                    flow.open_if_going();
                }
            });
        }
        let (cap, spool) = (self.log_max, self.spool.clone());
        res.map(move |body| TapBody::wrap(body, flow, Side::Response, cap, spool, stream))
    }
}

impl FlowHandler {
    /// The server's response with the status and body rules give it. A body
    /// is changed whole, unpacked, and only up to `HOLD_MAX`, as at a
    /// breakpoint; events and upgrades go on as they come.
    async fn rewrite_response(&mut self, mut res: Response<Body>) -> Response<Body> {
        if let Some(status) = self.response_status.take()
            && let Ok(status) = StatusCode::from_u16(status)
            && res.status() != StatusCode::SWITCHING_PROTOCOLS
        {
            *res.status_mut() = status;
        }
        let edits = std::mem::take(&mut self.response_body);
        if edits.is_empty()
            || stream_cut(&res).is_some()
            || res.status() == StatusCode::SWITCHING_PROTOCOLS
        {
            return res;
        }
        let held = match pause::response_message(res, "", "").await {
            Read::Held(held) => held,
            Read::Passed(res) => return res,
        };
        let mut message = held.message.clone();
        message.body = edit_body(message.body, &edits);
        pause::response_from(message).unwrap_or_else(|problem| {
            tracing::warn!("a response cannot go on as a rule changed it: {problem}");
            held.unchanged()
        })
    }

    /// The request's body, through the slow network going up, if there is one.
    fn slow_request(&self, req: Request<Body>) -> Request<Body> {
        match &self.slowed {
            Some(conditions) => {
                let (network, conditions) = (Arc::clone(&self.network), conditions.clone());
                req.map(move |body| network.wrap(body, Direction::Up, conditions))
            }
            None => req,
        }
    }
}

/// A request with its query and body as rules say. A body is changed whole,
/// unpacked, and only up to `HOLD_MAX`, as at a breakpoint.
async fn rewrite_request(
    mut req: Request<Body>,
    query: &[(String, Option<String>)],
    body: &[(String, String)],
    streams: bool,
) -> Request<Body> {
    if !query.is_empty() {
        let uri = req.uri().clone();
        let path = uri.path().to_owned();
        let edited = edit_query(uri.query(), query);
        let mut parts = uri.into_parts();
        let path_and_query = match edited {
            Some(query) => format!("{path}?{query}"),
            None => path,
        };
        match path_and_query.parse() {
            Ok(path_and_query) => {
                parts.path_and_query = Some(path_and_query);
                if let Ok(uri) = http::Uri::from_parts(parts) {
                    *req.uri_mut() = uri;
                }
            }
            Err(e) => tracing::warn!("a rule's query cannot be sent: {e}"),
        }
    }
    if body.is_empty() || streams {
        return req;
    }
    let url = full_url(&req);
    let held = match pause::request_message(req, url).await {
        Read::Held(held) => held,
        Read::Passed(req) => return req,
    };
    let mut message = held.message.clone();
    message.body = edit_body(message.body, body);
    pause::request_from(message).unwrap_or_else(|problem| {
        tracing::warn!("a request cannot go on as a rule changed it: {problem}");
        held.unchanged()
    })
}

/// Takes the offers of HTTP/3 out of a response's `Alt-Svc`. With none left, it
/// says `clear`, which also makes the client forget those it was given before.
fn hide_http3_offers(headers: &mut http::HeaderMap) {
    if !headers.contains_key(header::ALT_SVC) {
        return;
    }
    let values: Vec<&str> = headers
        .get_all(header::ALT_SVC)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .collect();
    let Some(kept) = without_http3(&values) else {
        return;
    };
    match HeaderValue::from_str(&kept) {
        Ok(value) => {
            headers.insert(header::ALT_SVC, value);
        }
        Err(_) => {
            headers.remove(header::ALT_SVC);
        }
    }
}

/// `Alt-Svc` without its HTTP/3 offers, or `None` if it makes none.
fn without_http3(values: &[&str]) -> Option<String> {
    let entries: Vec<&str> = values.iter().flat_map(|v| alternatives(v)).collect();
    let offers_http3 = |entry: &&str| {
        let protocol = entry.split('=').next().unwrap_or_default().trim();
        let protocol = protocol.to_ascii_lowercase();
        // `h3`, the drafts' `h3-29`, and Google's own before them.
        protocol.starts_with("h3") || protocol == "quic"
    };
    if !entries.iter().any(offers_http3) {
        return None;
    }
    let kept: Vec<&str> = entries.into_iter().filter(|e| !offers_http3(e)).collect();
    Some(if kept.is_empty() {
        "clear".to_owned()
    } else {
        kept.join(", ")
    })
}

/// The alternatives in one `Alt-Svc` value. A comma inside quotes is not one.
fn alternatives(value: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut quoted = false;
    for (at, c) in value.char_indices() {
        match c {
            '"' => quoted = !quoted,
            ',' if !quoted => {
                out.push(value[start..at].trim());
                start = at + 1;
            }
            _ => {}
        }
    }
    out.push(value[start..].trim());
    out.retain(|e| !e.is_empty());
    out
}

const LOCAL_FILE_MAX: u64 = 1 << 30;

async fn answer_from_file(file: Option<PathBuf>) -> Response<Body> {
    let plain = |status: StatusCode, text: String| {
        Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(Body::from(http_body_util::Full::new(Bytes::from(text))))
            .expect("valid response")
    };
    let Some(path) = file else {
        return plain(
            StatusCode::NOT_FOUND,
            "tracewing: a rule maps this URL to a file that is not there\n".to_owned(),
        );
    };
    let too_big = tokio::fs::metadata(&path)
        .await
        .is_ok_and(|meta| meta.len() > LOCAL_FILE_MAX);
    if too_big {
        return plain(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "tracewing: {} is too large to answer with\n",
                path.display()
            ),
        );
    }
    match tokio::fs::read(&path).await {
        Ok(data) => Response::builder()
            .header(header::CONTENT_TYPE, content_type_of(&path))
            .body(Body::from(http_body_util::Full::new(Bytes::from(data))))
            .expect("valid response"),
        Err(e) => plain(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("tracewing: cannot read {}: {e}\n", path.display()),
        ),
    }
}

/// Keeps the path and query, and changes where it goes.
fn send_elsewhere(req: &mut Request<Body>, to: &http::Uri) {
    let mut parts = to.clone().into_parts();
    parts.path_and_query = req.uri().path_and_query().cloned();
    let Ok(uri) = http::Uri::from_parts(parts) else {
        return;
    };
    if let Some(Ok(host)) = uri.authority().map(|a| HeaderValue::from_str(a.as_str())) {
        req.headers_mut().insert(header::HOST, host);
    }
    *req.uri_mut() = uri;
}

impl HttpHandler for FlowHandler {
    async fn handle_request(&mut self, ctx: &HttpContext, req: Request<Body>) -> RequestOrResponse {
        // The tunnel itself is not a flow; the requests inside it are.
        if req.method() == Method::CONNECT {
            // A new connection, and the first the proxy hears of it.
            if let Some(apps) = &self.apps
                && self.recording.load(Ordering::Relaxed)
            {
                apps.of(ctx.client_addr, true);
            }
            return req.into();
        }
        let req = with_http_scheme(req);
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
        // Sent to the proxy as to a server, not through it: a device opening the
        // proxy's own address, as the Set up page's code says to.
        let here = req.uri().authority().is_none();
        let check =
            request_host(&req).map(|host| without_default_port(host, scheme)) == Some(CHECK_HOST);
        if here || check {
            if req.uri().path() == CA_PATH {
                return Response::builder()
                    .header(header::CONTENT_TYPE, "application/x-x509-ca-cert")
                    .header(
                        header::CONTENT_DISPOSITION,
                        "attachment; filename=\"tracewing.crt\"",
                    )
                    .body(Body::from(http_body_util::Full::new(
                        self.crl.ca_der.clone(),
                    )))
                    .expect("valid response")
                    .into();
            }
            let proxy = here
                .then(|| header_text(req.headers(), header::HOST))
                .flatten();
            if !here {
                self.check.saw(CheckVisit {
                    user_agent: header_text(req.headers(), header::USER_AGENT).unwrap_or_default(),
                    brands: req
                        .headers()
                        .get("sec-ch-ua")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or_default()
                        .to_owned(),
                    secure: scheme == "https",
                });
            }
            let page = check_page(proxy.as_deref(), !here && scheme == "https");
            return Response::builder()
                .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                .body(Body::from(http_body_util::Full::new(Bytes::from(page))))
                .expect("valid response")
                .into();
        }
        let mut req = req;
        let mut plan = self.plan_for(&req);
        // Set now, so that nothing is left over from the request before.
        self.response_headers = std::mem::take(&mut plan.response_headers);
        self.response_status = plan.status;
        self.response_body = std::mem::take(&mut plan.response_body);
        let url = full_url(&req);
        // A stream's body never ends, so it could not be held whole.
        let streams = is_websocket(&req);
        self.pause_response = plan
            .pause_response
            .clone()
            .filter(|_| !streams)
            .map(|rule| (rule, req.method().to_string(), url.clone()));
        let mut dropped = false;
        if let Some(rule) = plan
            .pause_request
            .clone()
            .filter(|_| !streams && plan.end.is_none())
        {
            (req, dropped) = self.hold_request(req, rule, url).await;
        }
        // Before the flow starts, so that it shows what was sent, as an edit at a
        // breakpoint does.
        if !dropped && plan.end.is_none() {
            req = rewrite_request(req, &plan.query, &plan.request_body, streams).await;
        }
        // Paused: traffic goes through as if nothing were watching, rules and all.
        let recording = self.recording.load(Ordering::Relaxed);
        let flow =
            recording.then(|| Arc::new(Flow::start(self.recorder.clone(), ctx.client_addr, &req)));
        if let Some(flow) = &flow {
            let mut record = flow.lock();
            record.app = self.app_of(ctx.client_addr);
            if !plan.names.is_empty() {
                record.rule = Some(plan.names.join(", "));
            }
        }
        if dropped {
            return self.answer_as_rule(flow, pause::dropped()).into();
        }
        for (name, value) in plan.request_headers {
            match value {
                Some(value) => req.headers_mut().insert(name, value),
                None => req.headers_mut().remove(name),
            };
        }
        // After the flow has them, so that it shows what the client sent.
        let upgrade = is_websocket(&req);
        drop_hop_by_hop(req.headers_mut(), upgrade);
        // Plain HTTP goes to an HTTP relay whole, and says who is asking.
        if req.uri().scheme_str() == Some("http")
            && let Some(basic) = req
                .uri()
                .host()
                .and_then(|host| self.relay.route_for(host))
                .filter(|route| route.kind == crate::relay::Kind::Http)
                .and_then(|route| route.basic())
            && let Ok(value) = HeaderValue::from_str(&basic)
        {
            req.headers_mut().insert(header::PROXY_AUTHORIZATION, value);
        }
        match plan.end {
            Some(End::Block(status)) => {
                let res = Response::builder()
                    .status(status)
                    .body(Body::empty())
                    .unwrap_or_else(|_| Response::new(Body::empty()));
                return self.answer_as_rule(flow, res).into();
            }
            Some(End::Local(file)) => {
                let res = answer_from_file(file).await;
                return self.answer_as_rule(flow, res).into();
            }
            // A WebSocket too: the proxy opens it to wherever the request points.
            Some(End::Remote(to)) => send_elsewhere(&mut req, &to),
            _ => {}
        }
        // A slow network, for what goes on to a server, a WebSocket's
        // messages too.
        self.slowed = self
            .network
            .for_host(request_host(&req).unwrap_or_default());
        if let Some(conditions) = &self.slowed {
            Network::half_latency(conditions).await;
            if let Some(flow) = &flow {
                let mut record = flow.lock();
                let names = record.rule.take().into_iter().chain([conditions.summary()]);
                record.rule = Some(names.collect::<Vec<_>>().join(", "));
            }
        }
        // A WebSocket is opened to the server here, the way other connections
        // go, a relay included; the client hears what the server answered.
        if is_websocket(&req) {
            return self.open_websocket(req, flow).await.into();
        }
        let Some(flow) = flow else {
            return self.slow_request(req).into();
        };
        let (cap, tapped, spool) = (self.log_max, Arc::clone(&flow), self.spool.clone());
        let req = req.map(move |body| TapBody::wrap(body, tapped, Side::Request, cap, spool, None));
        self.flow = Some(flow);
        self.slow_request(req).into()
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
                app: self.app_of(ctx.client_addr),
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
        let flow = self.flow.take();
        // Before a breakpoint, which makes the response again.
        if let (Some(flow), Some(upstream)) = (&flow, res.extensions().get::<Arc<Upstream>>()) {
            flow.went_over(upstream);
        }
        if let Some(flow) = &flow {
            let_its_program_be_found(flow).await;
        }
        let res = match self.pause_response.take() {
            // Events and upgrades go on for as long as they last.
            Some(held)
                if stream_cut(&res).is_none()
                    && res.status() != StatusCode::SWITCHING_PROTOCOLS =>
            {
                self.hold_response(res, held).await
            }
            _ => res,
        };
        let res = self.rewrite_response(res).await;
        let slowed = self.slowed.take();
        if let Some(conditions) = &slowed {
            Network::half_latency(conditions).await;
        }
        let res = self.answer(flow, res);
        match slowed {
            Some(conditions) => {
                let network = Arc::clone(&self.network);
                res.map(move |body| network.wrap(body, Direction::Down, conditions))
            }
            None => res,
        }
    }

    async fn handle_error(
        &mut self,
        _ctx: &HttpContext,
        err: hyper_util::client::legacy::Error,
    ) -> Response<Body> {
        let message = error_chain(&err);
        self.slowed = None;
        if let Some(flow) = self.flow.take() {
            flow.fail(message.clone());
            let mut source: Option<&(dyn std::error::Error + 'static)> = Some(&err);
            while let Some(error) = source {
                if let Some(failure) = error.downcast_ref::<TlsFailure>() {
                    flow.refused_by(failure);
                    break;
                }
                source = error.source();
            }
        }
        Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(Body::from(format!("tracewing: {message}\n")))
            .expect("valid response")
    }
}

impl FlowHandler {
    /// Opens the WebSocket `req` asks for to its server, and answers the client
    /// with what the server answered: an upgrade, with the subprotocol the
    /// server took, or its refusal. Then carries messages both ways, each
    /// written down on its way.
    async fn open_websocket(&self, req: Request<Body>, flow: Option<Arc<Flow>>) -> Response<Body> {
        let (mut parts, body) = req.into_parts();
        // The request's side ends here: an upgrade has no body.
        if let Some(flow) = &flow {
            let tapped = TapBody::wrap(
                body,
                Arc::clone(flow),
                Side::Request,
                self.log_max,
                self.spool.clone(),
                None,
            );
            let _ = http_body_util::BodyExt::collect(tapped).await;
        }
        let Some(upgrading) = parts.extensions.remove::<hyper::upgrade::OnUpgrade>() else {
            return self.websocket_failed(flow, "this connection cannot be upgraded".to_owned());
        };
        let Some(key) = parts.headers.get(header::SEC_WEBSOCKET_KEY).cloned() else {
            return self.websocket_failed(flow, "the upgrade has no Sec-WebSocket-Key".to_owned());
        };
        let secure = parts.uri.scheme_str() == Some("https");
        let mut to_ws = parts.uri.clone().into_parts();
        to_ws.scheme = Some(if secure { "wss" } else { "ws" }.parse().expect("a scheme"));
        let Ok(ws_uri) = http::Uri::from_parts(to_ws) else {
            return self
                .websocket_failed(flow, format!("cannot open a WebSocket to {}", parts.uri));
        };
        // As the client asked, but for compression, which is not passed on:
        // what is written down is the messages as they are.
        let mut ask = http::Request::builder().method(Method::GET).uri(ws_uri);
        for (name, value) in &parts.headers {
            if name.as_str() != "sec-websocket-extensions" {
                ask = ask.header(name, value);
            }
        }
        let Ok(ask) = ask.body(()) else {
            return self.websocket_failed(flow, "the upgrade cannot be passed on".to_owned());
        };
        let stream = match self.connector.connect_http1(parts.uri.clone()).await {
            Ok(stream) => stream,
            Err(error) => return self.websocket_unreached(flow, &error),
        };
        if let Some(flow) = &flow {
            flow.went_over(stream.upstream());
        }
        let (server, answer) = match tokio_tungstenite::client_async_with_config(
            ask,
            TokioIo::new(stream),
            None,
        )
        .await
        {
            Ok(both) => both,
            // Refused: the client is told as the server told it.
            Err(tungstenite::Error::Http(answer)) => {
                if let Some(conditions) = &self.slowed {
                    Network::half_latency(conditions).await;
                }
                let (parts, body) = answer.into_parts();
                let res = Response::from_parts(parts, Body::from(body.unwrap_or_default()));
                let flow = flow;
                return match flow {
                    Some(flow) => {
                        flow.on_response(&res);
                        res
                    }
                    None => res,
                };
            }
            Err(e) => return self.websocket_failed(flow, format!("WebSocket: {e}")),
        };
        let mut res = Response::builder()
            .status(StatusCode::SWITCHING_PROTOCOLS)
            .header(header::UPGRADE, "websocket")
            .header(header::CONNECTION, "Upgrade")
            .header(
                header::SEC_WEBSOCKET_ACCEPT,
                tungstenite::handshake::derive_accept_key(key.as_bytes()),
            );
        for (name, value) in answer.headers() {
            let skip = matches!(
                name.as_str(),
                "upgrade" | "connection" | "sec-websocket-accept" | "sec-websocket-extensions"
            );
            if !skip {
                res = res.header(name, value);
            }
        }
        let res = res.body(Body::empty()).expect("valid response");
        if let Some(flow) = &flow {
            flow.on_response(&res);
            flow.opened_as("websocket", Some(101));
        }
        // The answer's half of a slow round trip.
        if let Some(conditions) = &self.slowed {
            Network::half_latency(conditions).await;
        }
        let handler = self.clone();
        tokio::spawn(async move {
            let Ok(upgraded) = upgrading.await else {
                return;
            };
            let client =
                WebSocketStream::from_raw_socket(TokioIo::new(upgraded), Role::Server, None).await;
            let (client_sink, client_stream) = client.split();
            let (server_sink, server_stream) = server.split();
            tokio::spawn(
                handler
                    .clone()
                    .pump(flow.clone(), true, client_stream, server_sink),
            );
            handler.pump(flow, false, server_stream, client_sink).await;
        });
        res
    }

    /// The server could not be reached: said in the flow, and to the client.
    fn websocket_unreached(&self, flow: Option<Arc<Flow>>, error: &BoxError) -> Response<Body> {
        if let Some(flow) = &flow {
            let mut source: Option<&(dyn StdError + 'static)> = Some(error.as_ref());
            while let Some(cause) = source {
                if let Some(failure) = cause.downcast_ref::<TlsFailure>() {
                    flow.refused_by(failure);
                    break;
                }
                source = cause.source();
            }
        }
        self.websocket_failed(flow, error_chain(error.as_ref()))
    }

    fn websocket_failed(&self, flow: Option<Arc<Flow>>, message: String) -> Response<Body> {
        if let Some(flow) = flow {
            flow.fail(message.clone());
        }
        Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(Body::from(format!("tracewing: {message}\n")))
            .expect("valid response")
    }

    /// Carries one way's messages, each written down on its way. Either side
    /// ending ends it; the row is written when the other lets go too. On a
    /// slow network each message is let go when the link would have it
    /// through, half a round trip after: a queue between reading and sending,
    /// so that the latency delays messages without slowing how many go. On
    /// any other, a message goes on at once, as it came.
    async fn pump(
        self,
        flow: Option<Arc<Flow>>,
        from_client: bool,
        mut stream: impl Stream<Item = Result<WsMessage, tungstenite::Error>> + Unpin + Send + 'static,
        sink: impl Sink<WsMessage, Error = tungstenite::Error> + Unpin + Send + 'static,
    ) {
        let recorded = flow.as_ref().and_then(|flow| flow.stream());
        // A socket's calls are in its messages, so its row cannot be named
        // when it opens. Only the first few are looked at: one that speaks
        // JSON-RPC says so straight away, and one that never will should not
        // pay for the looking all its life.
        let mut look = flow.is_some().then_some(0usize);
        let direction = if from_client {
            Direction::Up
        } else {
            Direction::Down
        };
        let mut out = match self.slowed.clone() {
            Some(conditions) => Out::slow(sink, conditions),
            None => Out::Now(Box::pin(sink)),
        };
        while let Some(message) = stream.next().await {
            let message = match message {
                Ok(message) => message,
                Err(e) => {
                    tracing::debug!("WebSocket message: {e}");
                    out.send(&self.network, direction, WsMessage::Close(None))
                        .await;
                    break;
                }
            };
            if let Some(recorded) = &recorded {
                note_message(recorded, from_client, &message);
            }
            if let (Some(seen), Some(flow), Some(recorded)) = (look.as_mut(), &flow, &recorded) {
                *seen += 1;
                let found = name_the_call(flow, recorded, from_client, &message);
                if found || *seen >= RPC_LOOK_AT {
                    look = None;
                }
            }
            if !out.send(&self.network, direction, message).await {
                break;
            }
        }
        out.end().await;
    }
}

type WsSink = Pin<Box<dyn Sink<WsMessage, Error = tungstenite::Error> + Send>>;

/// Where one way's messages go: on at once, or through a slow link.
enum Out {
    Now(WsSink),
    Slow {
        conditions: Conditions,
        queue: tokio::sync::mpsc::Sender<(tokio::time::Instant, WsMessage)>,
        sending: tokio::task::JoinHandle<()>,
    },
}

impl Out {
    fn slow(
        sink: impl Sink<WsMessage, Error = tungstenite::Error> + Unpin + Send + 'static,
        conditions: Conditions,
    ) -> Self {
        let (queue, mut queued) =
            tokio::sync::mpsc::channel::<(tokio::time::Instant, WsMessage)>(1024);
        let sending = tokio::spawn(async move {
            let mut sink = sink;
            while let Some((at, message)) = queued.recv().await {
                if at > tokio::time::Instant::now() {
                    tokio::time::sleep_until(at).await;
                }
                match sink.send(message).await {
                    Err(tungstenite::Error::ConnectionClosed) | Ok(()) => {}
                    Err(e) => tracing::debug!("WebSocket send: {e}"),
                }
            }
            let _ = sink.close().await;
        });
        Self::Slow {
            conditions,
            queue,
            sending,
        }
    }

    /// Sends `message` on, or queues it for when it is due. False if the
    /// other side is gone.
    async fn send(&mut self, network: &Network, direction: Direction, message: WsMessage) -> bool {
        match self {
            Self::Now(sink) => {
                match sink.send(message).await {
                    Err(tungstenite::Error::ConnectionClosed) | Ok(()) => {}
                    Err(e) => tracing::debug!("WebSocket send: {e}"),
                }
                true
            }
            Self::Slow {
                conditions, queue, ..
            } => {
                let through = network.send(direction, message.len(), conditions);
                let half = std::time::Duration::from_millis(u64::from(conditions.latency_ms) / 2);
                let at = tokio::time::Instant::from_std(through) + half;
                queue.send((at, message)).await.is_ok()
            }
        }
    }

    /// What is queued still goes, then the end.
    async fn end(self) {
        match self {
            Self::Now(mut sink) => {
                let _ = sink.close().await;
            }
            Self::Slow { queue, sending, .. } => {
                drop(queue);
                let _ = sending.await;
            }
        }
    }
}

/// What the row of a socket says it is, from one of its messages: the method
/// a call from the client names, or the code a reply from the server failed
/// with. Writes the row again the once it learns either, and says whether it
/// did, so that the looking stops.
fn name_the_call(
    flow: &Flow,
    stream: &StreamHandle,
    from_client: bool,
    message: &WsMessage,
) -> bool {
    let WsMessage::Text(text) = message else {
        return false;
    };
    let mut record = flow.lock();
    let found = match from_client {
        true if record.rpc.is_none() => {
            record.rpc = jsonrpc::method_in(text.as_bytes());
            record.rpc.is_some()
        }
        // A call that failed answers over the socket like any other message,
        // so without this a socket of failures looks like one of successes.
        false if record.rpc_error.is_none() => {
            record.rpc_error = jsonrpc::faults_in(text.as_bytes())
                .first()
                .map(|fault| fault.code);
            record.rpc_error.is_some()
        }
        _ => return true,
    };
    if found {
        let said = record.clone();
        drop(record);
        stream.update(said);
    }
    found
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

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

/// What is said to the next hop alone, which a proxy must not pass on
/// (RFC 9110, 7.6.1), either way: `Connection` and the headers it names, and
/// the `Proxy-` and `Keep-Alive` ones. `Proxy-Authorization` would hand the
/// server the password for the proxy, and a server's `Proxy-Authenticate`
/// would have the client think the proxy asks for one. `TE` goes on only as
/// `trailers`, which gRPC needs. An upgrade keeps `Connection` and `Upgrade`.
fn drop_hop_by_hop(headers: &mut http::HeaderMap, upgrade: bool) {
    let named: Vec<HeaderName> = headers
        .get_all(header::CONNECTION)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .filter_map(|name| HeaderName::from_bytes(name.trim().as_bytes()).ok())
        .collect();
    for name in named {
        if !(upgrade && name == header::UPGRADE) {
            headers.remove(name);
        }
    }
    if !upgrade {
        headers.remove(header::CONNECTION);
        headers.remove(header::UPGRADE);
    }
    for name in [
        HeaderName::from_static("proxy-connection"),
        header::PROXY_AUTHORIZATION,
        header::PROXY_AUTHENTICATE,
        HeaderName::from_static("keep-alive"),
    ] {
        headers.remove(name);
    }
    let trailers = headers.get_all(header::TE).iter().any(|v| {
        v.to_str().is_ok_and(|v| {
            v.split(',')
                .any(|t| t.trim().eq_ignore_ascii_case("trailers"))
        })
    });
    headers.remove(header::TE);
    if trailers {
        headers.insert(header::TE, HeaderValue::from_static("trailers"));
    }
}

fn is_websocket<T>(req: &Request<T>) -> bool {
    req.headers()
        .get(header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.eq_ignore_ascii_case("websocket"))
}

/// How long a response goes on before it is listed while it lasts.
const LISTED_AFTER: std::time::Duration = std::time::Duration::from_secs(2);

/// Whether a response's body may take a while: none says how long it is, or
/// it says it is long. One that says it is short is not watched.
fn may_take_a_while<T>(res: &Response<T>) -> bool {
    if matches!(
        res.status(),
        StatusCode::SWITCHING_PROTOCOLS | StatusCode::NO_CONTENT | StatusCode::NOT_MODIFIED
    ) {
        return false;
    }
    res.headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .is_none_or(|len| len > 256 * 1024)
}

/// How a stream's body is cut into messages as it comes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cut {
    /// Server-sent events, which end in a blank line.
    Events,
    /// A JSON value a line: NDJSON, JSON Lines, JSON text sequences.
    Lines,
}

impl Cut {
    /// The flow's kind, for its row.
    fn kind(self) -> &'static str {
        match self {
            Self::Events => "sse",
            Self::Lines => "ndjson",
        }
    }

    /// Each message's kind.
    fn message(self) -> &'static str {
        match self {
            Self::Events => "event",
            Self::Lines => "line",
        }
    }
}

/// Whether a response is a stream that goes on for as long as it lasts, and
/// how its body is cut, by its content type.
fn stream_cut<T>(res: &Response<T>) -> Option<Cut> {
    let content_type = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())?
        .to_ascii_lowercase();
    let essence = content_type.split(';').next().unwrap_or_default().trim();
    match essence {
        "text/event-stream" => Some(Cut::Events),
        "application/x-ndjson"
        | "application/ndjson"
        | "application/jsonl"
        | "application/x-jsonl"
        | "application/jsonlines"
        | "application/x-jsonlines"
        | "application/json-seq"
        | "application/stream+json"
        | "application/x-json-stream" => Some(Cut::Lines),
        _ => None,
    }
}

/// The most a packed stream is unpacked to, to cut into messages: a few KB
/// packed can come to many GB. The body is kept as it came, whatever this is.
const STREAM_UNPACK_MAX: u64 = 256 << 20;

/// The most of one message waited for before it is written as it stands.
const EVENT_MAX: usize = 1024 * 1024;

/// Cuts a stream's messages apart as they pass: frames need not fall on their
/// ends.
#[derive(Debug)]
struct Splitter {
    cut: Cut,
    buffer: Vec<u8>,
}

impl Splitter {
    fn new(cut: Cut) -> Self {
        Self {
            cut,
            buffer: Vec::new(),
        }
    }

    fn feed(&mut self, data: &[u8], mut message: impl FnMut(&[u8])) {
        self.buffer.extend_from_slice(data);
        let mut from = 0;
        while let Some((end, next)) = self.end(&self.buffer[from..]) {
            self.send(&self.buffer[from..from + end], &mut message);
            from += next;
        }
        self.buffer.drain(..from);
        if self.buffer.len() > EVENT_MAX {
            self.send(&self.buffer, &mut message);
            self.buffer.clear();
        }
    }

    fn finish(&mut self, mut message: impl FnMut(&[u8])) {
        if !self.buffer.is_empty() {
            self.send(&self.buffer, &mut message);
            self.buffer.clear();
        }
    }

    fn end(&self, data: &[u8]) -> Option<(usize, usize)> {
        match self.cut {
            Cut::Events => event_end(data),
            Cut::Lines => data.iter().position(|b| *b == b'\n').map(|at| (at, at + 1)),
        }
    }

    /// A line without its `\r` or its JSON text sequence's record separator,
    /// and none at all if nothing is left.
    fn send(&self, data: &[u8], message: &mut impl FnMut(&[u8])) {
        let data = match self.cut {
            Cut::Events => data,
            Cut::Lines => {
                let data = data.strip_suffix(b"\r").unwrap_or(data);
                let data = data.strip_prefix(b"\x1e").unwrap_or(data);
                if data.trim_ascii().is_empty() {
                    return;
                }
                data
            }
        };
        message(data);
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

/// The request's URL whole, with the host from `Host` where the request line
/// has none.
fn full_url<T>(req: &Request<T>) -> String {
    let uri = req.uri();
    if uri.authority().is_some() {
        return uri.to_string();
    }
    let scheme = uri.scheme_str().unwrap_or("http");
    let path = uri.path_and_query().map_or("/", |p| p.as_str());
    format!("{scheme}://{}{path}", request_host(req).unwrap_or_default())
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

/// gRPC's status: in the trailers, or in the headers of an answer with no
/// body, or in gRPC-Web's last frame.
fn grpc_status(record: &FlowRecord, body: &BodyRecord) -> Option<u16> {
    let in_lines = |lines: &str| {
        lines
            .lines()
            .filter_map(|line| line.split_once(": "))
            .find(|(name, _)| name.eq_ignore_ascii_case("grpc-status"))
            .and_then(|(_, value)| value.trim().parse().ok())
    };
    if let Some(status) = in_lines(&record.resp_trailers).or_else(|| in_lines(&record.resp_headers))
    {
        return Some(status);
    }
    let plain = record
        .resp_encoding
        .as_deref()
        .is_none_or(|e| e.eq_ignore_ascii_case("identity"));
    if !plain || body.truncated {
        return None;
    }
    let text = match binjson::format_of(record.content_type.as_deref())? {
        binjson::Format::GrpcWeb => false,
        binjson::Format::GrpcWebText => true,
        _ => return None,
    };
    protobuf::grpc_web_status(&body.captured, text)
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
    /// The response's body has ended, so its row is written whole as it goes.
    ended: AtomicBool,
}

impl Flow {
    fn start(recorder: Recorder, client: SocketAddr, req: &Request<Body>) -> Self {
        let uri = req.uri();
        let is_websocket = req
            .headers()
            .get(header::UPGRADE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.eq_ignore_ascii_case("websocket"));

        let mut record = FlowRecord {
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
        if req.method() == http::Method::GET {
            record.graphql = graphql::operation("GET", &record.path, None, b"");
        }

        Self {
            recorder,
            started: Instant::now(),
            record: Mutex::new(record),
            ended: AtomicBool::new(false),
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

    /// What the server showed when TLS with it failed, for the inspector.
    fn refused_by(&self, failure: &TlsFailure) {
        let mut record = self.lock();
        record.conn_id = Some(failure.id);
        record.connection = Some(failure.describe());
    }

    fn went_over(&self, upstream: &Upstream) {
        let mut record = self.lock();
        record.conn_id = Some(upstream.id);
        // Made for this flow, or for others at the same time: it waited for it.
        if upstream.ready_us > record.started_us {
            record.blocked_us = Some((upstream.started_us - record.started_us).max(0));
            record.dns_us = upstream.dns_us;
            record.connect_us = Some(upstream.connect_us);
            record.tls_us = upstream.tls_us;
        }
        record.connection = upstream.tell_once();
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

    /// Writes the row now if the response has not ended, so that a slow one
    /// is listed while it goes. Its kind stays what it was.
    fn open_if_going(&self) {
        if self.ended.load(Ordering::Relaxed) || lock(&self.stream).is_some() {
            return;
        }
        let kind = self.lock().kind;
        self.opened_as(kind, None);
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
    /// A stream's messages, to cut apart as they pass, unpacked first if it
    /// came packed.
    events: Option<(StreamHandle, Splitter, Option<Unpacker>)>,
    /// Trailers sent after the body, as `name: value` lines.
    trailers: String,
}

impl TapBody {
    fn wrap(
        inner: Body,
        flow: Arc<Flow>,
        side: Side,
        cap: usize,
        spool: Spool,
        events: Option<(StreamHandle, Cut, Encoding)>,
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
            events: events.map(|(stream, cut, encoding)| {
                (
                    stream,
                    Splitter::new(cut),
                    Unpacker::new(encoding, STREAM_UNPACK_MAX),
                )
            }),
            trailers: String::new(),
        }))
    }

    fn observe(&mut self, data: &Bytes) {
        self.body.bytes += data.len() as u64;
        if let Some((stream, splitter, unpacker)) = &mut self.events {
            let kind = splitter.cut.message();
            match unpacker {
                Some(unpacker) => {
                    let plain = unpacker.feed(data);
                    splitter.feed(&plain, |message| stream.message(false, kind, message));
                }
                None => splitter.feed(data, |message| stream.message(false, kind, message)),
            }
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
        if let Some((stream, splitter, _)) = &mut self.events {
            let kind = splitter.cut.message();
            splitter.finish(|message| stream.message(false, kind, message));
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
            Side::Request => {
                let plain = record
                    .req_encoding
                    .as_deref()
                    .is_none_or(|e| e.eq_ignore_ascii_case("identity"));
                if record.graphql.is_none() && plain && !body.truncated {
                    record.graphql = graphql::operation(
                        &record.method,
                        &record.path,
                        record.req_content_type.as_deref(),
                        &body.captured,
                    );
                }
                if plain && !body.truncated {
                    record.rpc = jsonrpc::method(
                        &record.method,
                        record.req_content_type.as_deref(),
                        &body.captured,
                    );
                }
                record.request = body;
            }
            Side::Response => {
                record.resp_trailers = std::mem::take(&mut self.trailers);
                record.grpc_status = grpc_status(&record, &body);
                // JSON-RPC answers 200 and puts a failure in the body. Only
                // the reply to a call the request named is looked at, so that
                // other JSON is not parsed a second time.
                let plain = record
                    .resp_encoding
                    .as_deref()
                    .is_none_or(|e| e.eq_ignore_ascii_case("identity"));
                if record.rpc.is_some() && plain && !body.truncated {
                    record.rpc_error =
                        jsonrpc::faults(record.content_type.as_deref(), &body.captured)
                            .first()
                            .map(|fault| fault.code);
                }
                record.response = body;
                record.duration_us = Some(elapsed);
                self.flow.ended.store(true, Ordering::Relaxed);
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
                } else if let Some(trailers) = frame.trailers_ref() {
                    this.trailers.push_str(&header_lines(trailers));
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

/// How much more of a request's body is read once the server has stopped
/// reading it, and for how long, so that the flow has what the client sent.
const READ_ON_MAX: u64 = 16 << 20;
const READ_ON_WAIT: std::time::Duration = std::time::Duration::from_secs(10);

impl Drop for TapBody {
    fn drop(&mut self) {
        if self.done {
            return;
        }
        // hyper does not poll a body it already knows is empty, so an
        // end-of-stream body that is dropped still counts as complete.
        let complete = http_body::Body::is_end_stream(&self.inner);
        // A server that answers before reading a request's body, as one
        // refusing an upload may, has the rest let go unsent. The client
        // may be sending it still: it is read on, a while, so that the flow
        // has what was sent.
        if !complete
            && matches!(self.side, Side::Request)
            && let Ok(runtime) = tokio::runtime::Handle::try_current()
        {
            let rest = Self {
                inner: std::mem::replace(&mut self.inner, Body::empty()),
                flow: Arc::clone(&self.flow),
                side: Side::Request,
                cap: self.cap,
                body: std::mem::take(&mut self.body),
                done: false,
                spool: self.spool.clone(),
                file: self.file.take(),
                events: self.events.take(),
                trailers: std::mem::take(&mut self.trailers),
            };
            self.done = true;
            runtime.spawn(read_on(rest));
            return;
        }
        self.finish(complete, None);
    }
}

/// Reads what is left of a request's body that no server will read, written
/// down as it comes, up to [`READ_ON_MAX`] and [`READ_ON_WAIT`]. Cut short,
/// or by the client going, it is not complete; that is no error of the flow.
async fn read_on(mut rest: TapBody) {
    use http_body_util::BodyExt as _;
    let reading = async {
        let mut read = 0u64;
        loop {
            match rest.inner.frame().await {
                Some(Ok(frame)) => {
                    if let Some(data) = frame.data_ref() {
                        read += data.len() as u64;
                        rest.observe(data);
                        if read > READ_ON_MAX {
                            return false;
                        }
                    } else if let Some(trailers) = frame.trailers_ref() {
                        rest.trailers.push_str(&header_lines(trailers));
                    }
                }
                Some(Err(_)) => return false,
                None => return true,
            }
        }
    };
    let complete = tokio::time::timeout(READ_ON_WAIT, reading)
        .await
        .unwrap_or(false);
    rest.finish(complete, None);
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

    fn header_map(pairs: &[(&str, &str)]) -> http::HeaderMap {
        let mut map = http::HeaderMap::new();
        for (name, value) in pairs {
            map.append(
                HeaderName::from_bytes(name.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }
        map
    }

    fn names(map: &http::HeaderMap) -> Vec<String> {
        let mut names: Vec<String> = map
            .iter()
            .map(|(n, v)| format!("{n}: {}", v.to_str().unwrap()))
            .collect();
        names.sort();
        names
    }

    #[test]
    fn what_is_for_the_proxy_alone_goes_no_further() {
        let mut map = header_map(&[
            ("connection", "keep-alive, X-Hop"),
            ("x-hop", "1"),
            ("proxy-connection", "Keep-Alive"),
            ("proxy-authorization", "Basic dXNlcjpwYXNz"),
            ("proxy-authenticate", "Basic realm=\"x\""),
            ("keep-alive", "timeout=5"),
            ("te", "gzip, trailers"),
            ("upgrade", "h2c"),
            ("x-end", "1"),
        ]);
        drop_hop_by_hop(&mut map, false);
        assert_eq!(names(&map), ["te: trailers", "x-end: 1"]);

        let mut map = header_map(&[("te", "gzip")]);
        drop_hop_by_hop(&mut map, false);
        assert!(map.is_empty());

        // A WebSocket's upgrade keeps what makes it one.
        let mut map = header_map(&[
            ("connection", "Upgrade"),
            ("upgrade", "websocket"),
            ("proxy-connection", "Keep-Alive"),
            ("sec-websocket-key", "abc"),
        ]);
        drop_hop_by_hop(&mut map, true);
        assert_eq!(
            names(&map),
            [
                "connection: Upgrade",
                "sec-websocket-key: abc",
                "upgrade: websocket"
            ]
        );
    }

    #[test]
    fn the_check_keeps_each_browsers_latest_visit() {
        let check = Check::default();
        let visit = |agent: &str, secure| CheckVisit {
            user_agent: agent.into(),
            brands: String::new(),
            secure,
        };
        check.saw(visit("Firefox", false));
        check.saw(visit("Chrome", true));
        check.saw(visit("Firefox", true));
        assert_eq!(
            check.visits(),
            [visit("Chrome", true), visit("Firefox", true)]
        );
        assert_eq!(check.last(), Some(visit("Firefox", true)));
        for n in 0..VISITS_KEPT {
            check.saw(visit(&n.to_string(), true));
        }
        assert_eq!(check.visits().len(), VISITS_KEPT);
        assert_eq!(check.visits()[0].user_agent, "0");
    }

    #[test]
    fn a_device_opening_the_proxy_is_told_how_to_set_it_up() {
        let page = check_page(Some("192.168.1.5:9090"), false);
        assert!(
            page.contains("server <b>192.168.1.5</b>, port <b>9090</b>"),
            "{page}"
        );
        assert!(page.contains(r#"href="/ca.crt""#));
        assert!(page.contains("Certificate Trust"));
        let odd = check_page(Some("<script>:1"), false);
        assert!(!odd.contains("<script>"), "{odd}");
        assert!(check_page(None, true).contains("trusts tracewing"));
        assert!(check_page(None, false).contains("over <a href=\"https://tracewing.test/\">HTTPS"));
    }

    #[test]
    fn offers_of_http3_are_taken_out() {
        assert_eq!(
            without_http3(&[r#"h3=":443"; ma=86400, h2="alt.example:443"; ma=60"#]).as_deref(),
            Some(r#"h2="alt.example:443"; ma=60"#)
        );
        // The drafts' and Google's own, and over two headers.
        assert_eq!(
            without_http3(&[r#"h3-29=":443""#, r#"quic=":443"; v="46,43""#]).as_deref(),
            Some("clear")
        );
        // A comma in quotes is not between two.
        assert_eq!(
            without_http3(&[r#"h2="a,b:443", H3=":443""#]).as_deref(),
            Some(r#"h2="a,b:443""#)
        );
        assert_eq!(without_http3(&[r#"h2=":8443""#]), None);
        assert_eq!(without_http3(&["clear"]), None);
    }

    #[test]
    fn a_response_with_no_offers_is_left_alone() {
        let mut headers = http::HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain"));
        hide_http3_offers(&mut headers);
        assert_eq!(headers.len(), 1);
        headers.append(header::ALT_SVC, HeaderValue::from_static(r#"h3=":443""#));
        headers.append(header::ALT_SVC, HeaderValue::from_static(r#"h2=":443""#));
        hide_http3_offers(&mut headers);
        let values: Vec<_> = headers.get_all(header::ALT_SVC).iter().collect();
        assert_eq!(values, [r#"h2=":443""#]);
    }

    fn events_of(frames: &[&[u8]]) -> Vec<Vec<u8>> {
        let mut splitter = Splitter::new(Cut::Events);
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

    fn lines_of(frames: &[&[u8]]) -> Vec<String> {
        let mut splitter = Splitter::new(Cut::Lines);
        let mut out = Vec::new();
        for frame in frames {
            splitter.feed(frame, |line| {
                out.push(String::from_utf8_lossy(line).into_owned())
            });
        }
        splitter.finish(|line| out.push(String::from_utf8_lossy(line).into_owned()));
        out
    }

    #[test]
    fn json_lines_are_cut_wherever_frames_fall() {
        let want = [r#"{"n":0}"#, r#"{"n":1}"#];
        assert_eq!(lines_of(&[b"{\"n\":0}\n{\"n\":1}\n"]), want);
        assert_eq!(
            lines_of(&[b"{\"n\":", b"0}\n{\"n", b"\":1}"]),
            want,
            "the last needs no break"
        );
        assert_eq!(
            lines_of(&[b"{\"n\":0}\r\n\r\n\n{\"n\":1}\r\n"]),
            want,
            "\\r and blank lines"
        );
        assert_eq!(
            lines_of(&[b"\x1e{\"n\":0}\n\x1e{\"n\":1}\n"]),
            want,
            "a JSON text sequence's separators"
        );
    }

    #[test]
    fn a_stream_is_told_by_its_content_type() {
        let with = |content_type: &str| {
            let res = Response::builder()
                .header(header::CONTENT_TYPE, content_type)
                .body(())
                .unwrap();
            stream_cut(&res)
        };
        assert_eq!(with("text/event-stream; charset=utf-8"), Some(Cut::Events));
        assert_eq!(with("application/x-ndjson"), Some(Cut::Lines));
        assert_eq!(with("Application/JSONL"), Some(Cut::Lines));
        assert_eq!(with("application/json"), None);
    }

    #[test]
    fn what_never_ended_still_counts() {
        assert_eq!(events_of(&[b"data: cut off"]), [b"data: cut off".to_vec()]);
        assert!(events_of(&[]).is_empty());
    }

    #[test]
    fn an_event_with_no_end_does_not_grow_forever() {
        let mut splitter = Splitter::new(Cut::Events);
        let mut out = 0;
        splitter.feed(&vec![b'x'; EVENT_MAX + 1], |event| out += event.len());
        assert_eq!(out, EVENT_MAX + 1);
        assert!(splitter.buffer.is_empty());
    }
}
