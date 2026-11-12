//! Breakpoints: a request or a response that a rule says to stop is held here,
//! whole, until the window says what to send on: the same, something changed,
//! or nothing at all. If nobody ever says, as when the window goes, it goes on
//! as it was.
//!
//! A body is held in memory to be edited, so only up to `HOLD_MAX`: a bigger
//! one goes on without stopping. One that came packed is unpacked to be read,
//! and goes on unpacked.

use bytes::{Bytes, BytesMut};
use http::{
    HeaderMap, HeaderName, HeaderValue, Method, Request, Response, StatusCode, Uri, header,
};
use http_body::{Body as HttpBody, Frame, SizeHint};
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use hudsucker::Body;
use std::{
    pin::Pin,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    task::{Context, Poll},
    time::SystemTime,
};
use tokio::sync::oneshot;
use tracewing_store::decode::{self, Encoding};

/// The largest body held to be edited.
pub const HOLD_MAX: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Request,
    Response,
    /// A WebSocket's message from the app, before the server gets it.
    ToServer,
    /// One from the server, before the app gets it.
    ToApp,
}

impl Stage {
    /// A WebSocket's message, not a request or a response.
    pub fn is_message(self) -> bool {
        matches!(self, Self::ToServer | Self::ToApp)
    }
}

/// What is held, and what goes on in its place. A WebSocket's message has
/// its kind, `text` or `binary`, for a method, the socket's URL, and no
/// status or headers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    /// The request's, also for a response, where it cannot be changed.
    pub method: String,
    pub url: String,
    /// A response's.
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct Paused {
    pub id: u64,
    pub stage: Stage,
    /// The rule that stopped it.
    pub rule: String,
    pub since: SystemTime,
    pub message: Message,
}

#[derive(Debug)]
pub enum Verdict {
    Send(Message),
    /// The client gets a 502 that says so, and the server never hears of it.
    Drop,
}

/// Everything held, shared by the proxy and the window.
#[derive(Debug, Default)]
pub struct Pauses {
    held: Mutex<Vec<(Paused, oneshot::Sender<Verdict>)>>,
    next: AtomicU64,
    /// Goes up at each change, for whoever shows them to tell.
    version: AtomicU64,
}

impl Pauses {
    pub fn held(&self) -> Vec<Paused> {
        self.lock()
            .iter()
            .map(|(paused, _)| paused.clone())
            .collect()
    }

    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Relaxed)
    }

    /// Lets one go. False if it is not held any more.
    pub fn decide(&self, id: u64, verdict: Verdict) -> bool {
        let found = {
            let mut held = self.lock();
            held.iter()
                .position(|(paused, _)| paused.id == id)
                .map(|at| held.remove(at))
        };
        let Some((_, reply)) = found else {
            return false;
        };
        self.version.fetch_add(1, Ordering::Relaxed);
        // The connection may have gone meanwhile; nothing is waiting then.
        let _ = reply.send(verdict);
        true
    }

    /// Lets every one go on as it was.
    pub fn release_all(&self) {
        let held = std::mem::take(&mut *self.lock());
        self.version.fetch_add(1, Ordering::Relaxed);
        for (paused, reply) in held {
            let _ = reply.send(Verdict::Send(paused.message));
        }
    }

    /// Holds `message` until the window decides.
    pub(crate) async fn hold(&self, stage: Stage, rule: String, message: Message) -> Verdict {
        let (reply, verdict) = oneshot::channel();
        let paused = Paused {
            id: self.next.fetch_add(1, Ordering::Relaxed) + 1,
            stage,
            rule,
            since: SystemTime::now(),
            message: message.clone(),
        };
        self.lock().push((paused, reply));
        self.version.fetch_add(1, Ordering::Relaxed);
        verdict.await.unwrap_or(Verdict::Send(message))
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<(Paused, oneshot::Sender<Verdict>)>> {
        self.held.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// What reading a message to hold came to.
pub(crate) enum Read<T, P> {
    Held(Held<P>),
    /// Its body is too big to hold, and it goes on as it is.
    Passed(T),
}

/// A message to hold, and what it was as it came, to send on byte for byte if
/// nothing is changed.
pub(crate) struct Held<P> {
    pub message: Message,
    parts: P,
    raw: Bytes,
}

impl Held<http::request::Parts> {
    pub fn unchanged(self) -> Request<Body> {
        Request::from_parts(self.parts, Body::from(Full::new(self.raw)))
    }
}

impl Held<http::response::Parts> {
    pub fn unchanged(self) -> Response<Body> {
        Response::from_parts(self.parts, Body::from(Full::new(self.raw)))
    }
}

enum Whole {
    Read(Bytes),
    TooBig(Body),
}

/// A body read whole, or, past `HOLD_MAX`, given back to be sent on as it is.
async fn read_whole(mut body: Body) -> Whole {
    let mut buf = BytesMut::new();
    while let Some(frame) = body.frame().await {
        let Ok(frame) = frame else {
            // Cut off: what came goes on, and the client sees it end early.
            return Whole::TooBig(rejoin(buf.freeze(), Body::empty()));
        };
        if let Ok(data) = frame.into_data() {
            buf.extend_from_slice(&data);
            if buf.len() > HOLD_MAX {
                return Whole::TooBig(rejoin(buf.freeze(), body));
            }
        }
    }
    Whole::Read(buf.freeze())
}

/// What was read, then the rest.
fn rejoin(read: Bytes, rest: Body) -> Body {
    Body::from(BoxBody::new(Rejoined {
        read: Some(read).filter(|r| !r.is_empty()),
        rest,
    }))
}

struct Rejoined {
    read: Option<Bytes>,
    rest: Body,
}

impl HttpBody for Rejoined {
    type Data = Bytes;
    type Error = hudsucker::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Self::Error>>> {
        if let Some(read) = self.read.take() {
            return Poll::Ready(Some(Ok(Frame::data(read))));
        }
        Pin::new(&mut self.rest).poll_frame(cx)
    }

    fn is_end_stream(&self) -> bool {
        self.read.is_none() && self.rest.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        let mut hint = self.rest.size_hint();
        let read = self.read.as_ref().map_or(0, |r| r.len() as u64);
        hint.set_lower(hint.lower() + read);
        if let Some(upper) = hint.upper() {
            hint.set_upper(upper + read);
        }
        hint
    }
}

fn headers_of(map: &HeaderMap) -> Vec<(String, String)> {
    map.iter()
        .map(|(name, value)| {
            (
                name.as_str().to_owned(),
                String::from_utf8_lossy(value.as_bytes()).into_owned(),
            )
        })
        .collect()
}

/// A packed body unpacked, and its headers made to say so. Left as it is if it
/// cannot be.
fn unpacked(headers: &mut Vec<(String, String)>, body: Bytes) -> Vec<u8> {
    let encoding = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-encoding"))
        .map(|(_, value)| value.as_str());
    let encoding = Encoding::from_header(encoding);
    if matches!(encoding, Encoding::Identity | Encoding::Unknown) {
        return body.to_vec();
    }
    match decode::unpack(&body, encoding, HOLD_MAX) {
        Some(plain) => {
            headers.retain(|(name, _)| !name.eq_ignore_ascii_case("content-encoding"));
            plain
        }
        None => body.to_vec(),
    }
}

/// The parts of a request, to hold.
pub(crate) async fn request_message(
    req: Request<Body>,
    url: String,
) -> Read<Request<Body>, http::request::Parts> {
    let (parts, body) = req.into_parts();
    let body = match read_whole(body).await {
        Whole::Read(body) => body,
        Whole::TooBig(body) => return Read::Passed(Request::from_parts(parts, body)),
    };
    let mut headers = headers_of(&parts.headers);
    let plain = unpacked(&mut headers, body.clone());
    Read::Held(Held {
        message: Message {
            method: parts.method.to_string(),
            url,
            status: 0,
            headers,
            body: plain,
        },
        parts,
        raw: body,
    })
}

pub(crate) async fn response_message(
    res: Response<Body>,
    method: &str,
    url: &str,
) -> Read<Response<Body>, http::response::Parts> {
    let (parts, body) = res.into_parts();
    let body = match read_whole(body).await {
        Whole::Read(body) => body,
        Whole::TooBig(body) => return Read::Passed(Response::from_parts(parts, body)),
    };
    let mut headers = headers_of(&parts.headers);
    let plain = unpacked(&mut headers, body.clone());
    Read::Held(Held {
        message: Message {
            method: method.to_owned(),
            url: url.to_owned(),
            status: parts.status.as_u16(),
            headers,
            body: plain,
        },
        parts,
        raw: body,
    })
}

/// Headers as edited. The body's length is ours to say, since it may have
/// changed, so whatever was said of it before goes.
fn header_map(headers: &[(String, String)], body_len: usize) -> Result<HeaderMap, String> {
    let mut map = HeaderMap::new();
    for (name, value) in headers {
        let name = HeaderName::from_bytes(name.trim().as_bytes())
            .map_err(|_| format!("\"{name}\" cannot be a header's name"))?;
        if name == header::CONTENT_LENGTH || name == header::TRANSFER_ENCODING {
            continue;
        }
        let value = HeaderValue::from_str(value.trim())
            .map_err(|_| format!("the value of {name} cannot be sent"))?;
        map.append(name, value);
    }
    map.insert(header::CONTENT_LENGTH, HeaderValue::from(body_len));
    Ok(map)
}

/// A request to send on, from what was held and edited.
pub fn request_from(message: Message) -> Result<Request<Body>, String> {
    let method = Method::from_bytes(message.method.trim().as_bytes())
        .map_err(|_| format!("\"{}\" is not a method", message.method))?;
    let uri: Uri = message
        .url
        .trim()
        .parse()
        .map_err(|_| format!("\"{}\" is not a URL", message.url))?;
    if uri.authority().is_none() {
        return Err(format!("\"{}\" needs a host", message.url));
    }
    let mut headers = header_map(&message.headers, message.body.len())?;
    // Where it goes is the URL's to say, edited or not.
    if let Some(authority) = uri.authority()
        && let Ok(host) = HeaderValue::from_str(authority.as_str())
    {
        headers.insert(header::HOST, host);
    }
    let mut req = Request::new(Body::from(Full::new(Bytes::from(message.body))));
    *req.method_mut() = method;
    *req.uri_mut() = uri;
    *req.headers_mut() = headers;
    Ok(req)
}

pub fn response_from(message: Message) -> Result<Response<Body>, String> {
    let status = StatusCode::from_u16(message.status)
        .map_err(|_| format!("{} is not a status", message.status))?;
    let headers = header_map(&message.headers, message.body.len())?;
    let mut res = Response::new(Body::from(Full::new(Bytes::from(message.body))));
    *res.status_mut() = status;
    *res.headers_mut() = headers;
    Ok(res)
}

pub(crate) fn dropped() -> Response<Body> {
    Response::builder()
        .status(StatusCode::BAD_GATEWAY)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from(
            "tracewing: dropped at a breakpoint\n".to_owned(),
        ))
        .expect("valid response")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message() -> Message {
        Message {
            method: "POST".into(),
            url: "https://api.test/v1/items?x=1".into(),
            status: 200,
            headers: vec![
                ("content-type".into(), "application/json".into()),
                ("content-length".into(), "2".into()),
                ("x-two".into(), "a".into()),
                ("x-two".into(), "b".into()),
            ],
            body: br#"{"a":1}"#.to_vec(),
        }
    }

    #[tokio::test]
    async fn an_edited_request_goes_as_edited() {
        let req = request_from(message()).unwrap();
        assert_eq!(req.method(), Method::POST);
        assert_eq!(req.uri().host(), Some("api.test"));
        assert_eq!(
            req.headers()[header::CONTENT_LENGTH],
            "7",
            "its length, not the old one"
        );
        assert_eq!(req.headers().get_all("x-two").iter().count(), 2);
        let Read::Held(back) = request_message(req, "https://api.test/v1/items?x=1".into()).await
        else {
            panic!("held")
        };
        assert_eq!(back.message.body, br#"{"a":1}"#);
    }

    #[test]
    fn what_cannot_be_sent_says_why() {
        let mut bad = message();
        bad.method = "PO ST".into();
        assert!(request_from(bad).unwrap_err().contains("method"));
        let mut bad = message();
        bad.url = "/v1/items".into();
        assert!(request_from(bad).unwrap_err().contains("host"));
        let mut bad = message();
        bad.status = 1000;
        assert!(response_from(bad).unwrap_err().contains("status"));
        let mut bad = message();
        bad.headers.push(("bad name".into(), "x".into()));
        assert!(response_from(bad).unwrap_err().contains("bad name"));
    }

    #[tokio::test]
    async fn a_packed_body_is_held_unpacked() {
        let packed = zstd::encode_all(&b"hello there"[..], 3).unwrap();
        let res = Response::builder()
            .header("content-encoding", "zstd")
            .body(Body::from(Full::new(Bytes::from(packed))))
            .unwrap();
        let Read::Held(held) = response_message(res, "GET", "https://a.test/").await else {
            panic!("held")
        };
        assert_eq!(held.message.body, b"hello there");
        assert!(
            !held
                .message
                .headers
                .iter()
                .any(|(n, _)| n == "content-encoding")
        );
        let as_it_was = held.unchanged();
        assert_eq!(
            as_it_was.headers()["content-encoding"],
            "zstd",
            "packed as it came"
        );
    }

    #[tokio::test]
    async fn a_body_too_big_to_hold_goes_on_whole() {
        let big = vec![7u8; HOLD_MAX + 10];
        let req = Request::new(Body::from(Full::new(Bytes::from(big.clone()))));
        let Read::Passed(back) = request_message(req, "https://a.test/".into()).await else {
            panic!("passed")
        };
        let body = back.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body.len(), big.len());
    }

    #[tokio::test]
    async fn held_until_decided_and_as_it_was_if_let_go() {
        let pauses = std::sync::Arc::new(Pauses::default());
        let waiting = {
            let pauses = std::sync::Arc::clone(&pauses);
            tokio::spawn(async move { pauses.hold(Stage::Request, "r".into(), message()).await })
        };
        while pauses.held().is_empty() {
            tokio::task::yield_now().await;
        }
        let id = pauses.held()[0].id;
        let mut edited = message();
        edited.body = b"changed".to_vec();
        assert!(pauses.decide(id, Verdict::Send(edited)));
        assert!(!pauses.decide(id, Verdict::Drop), "once only");
        match waiting.await.unwrap() {
            Verdict::Send(m) => assert_eq!(m.body, b"changed"),
            Verdict::Drop => panic!("not dropped"),
        }

        let waiting = {
            let pauses = std::sync::Arc::clone(&pauses);
            tokio::spawn(async move { pauses.hold(Stage::Response, "r".into(), message()).await })
        };
        while pauses.held().is_empty() {
            tokio::task::yield_now().await;
        }
        pauses.release_all();
        match waiting.await.unwrap() {
            Verdict::Send(m) => assert_eq!(m, message()),
            Verdict::Drop => panic!("not dropped"),
        }
    }
}
