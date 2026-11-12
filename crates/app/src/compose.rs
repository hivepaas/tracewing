//! Sending a request from the window: one composed there, or a flow sent again.
//! It goes through tracewing's own proxy, so it is recorded like any other, and
//! rules and breakpoints apply to it too.

use anyhow::{Context as _, Result, bail};
use std::{
    net::SocketAddr,
    path::Path,
    time::{Duration, Instant},
};
use tracewing_i18n::tr;
use tracewing_proxy::pause::{HOLD_MAX, Message};
use tracewing_store::decode::{self, Encoding};

/// Long enough for a breakpoint on the way, which holds it too.
const WAIT_MAX: Duration = Duration::from_secs(10 * 60);

/// Headers the client works out for itself.
const LEFT_OUT: [&str; 5] = [
    "content-length",
    "host",
    "connection",
    "proxy-connection",
    "transfer-encoding",
];

/// What came back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer {
    pub status: u16,
    pub version: String,
    pub headers: Vec<(String, String)>,
    /// Unpacked, and at most `HOLD_MAX` of it.
    pub body: Vec<u8>,
    /// The whole body as it came, packed or not.
    pub bytes: u64,
    pub took: Duration,
}

/// Asked to stop a send that is on its way. Dropping it, or sending on it,
/// stops the one `send` it was made for.
pub type Stop = tokio::sync::oneshot::Sender<()>;

/// The other end, which `send` waits on.
pub type Stopped = tokio::sync::oneshot::Receiver<()>;

/// A pair of them, for a send about to start.
pub fn stopper() -> (Stop, Stopped) {
    tokio::sync::oneshot::channel()
}

/// Sends `message` through the proxy at `proxy`, trusting its CA as well as
/// the system's roots (hosts tracewing does not decrypt show their own).
/// Redirects are not followed: each step is worth seeing.
///
/// `None` when `stop` asked it to stop first: the request is dropped where it
/// stands, which closes the connection, rather than held to `WAIT_MAX`.
pub fn send(
    proxy: SocketAddr,
    ca_pem: &Path,
    message: &Message,
    stop: Stopped,
) -> Result<Option<Answer>> {
    let ca = std::fs::read(ca_pem).with_context(|| format!("read {}", ca_pem.display()))?;
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(format!("http://{proxy}"))?)
        .tls_certs_merge([reqwest::Certificate::from_pem(&ca)?])
        .redirect(reqwest::redirect::Policy::none())
        .timeout(WAIT_MAX)
        .build()?;
    let method =
        reqwest::Method::from_bytes(message.method.trim().as_bytes()).with_context(|| {
            tr!(
                "\"{method}\" is not a method",
                method = message.method.as_str()
            )
        })?;
    let mut request = client.request(method, message.url.trim());
    for (name, value) in &message.headers {
        if !LEFT_OUT.contains(&name.to_ascii_lowercase().as_str()) {
            request = request.header(name.trim(), value.trim());
        }
    }
    if !message.body.is_empty() {
        request = request.body(message.body.clone());
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async move {
        // Waiting on `stop` alongside the request: when it wins, the request
        // future is dropped here and its connection goes with it.
        tokio::select! {
            biased;
            _ = stop => Ok(None),
            answer = asked(request) => answer.map(Some),
        }
    })
}

/// The request itself, from sending it to the whole answer in hand.
async fn asked(request: reqwest::RequestBuilder) -> Result<Answer> {
    let started = Instant::now();
    let mut response = request.send().await?;
    let status = response.status().as_u16();
    let version = format!("{:?}", response.version());
    let headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_owned(),
                String::from_utf8_lossy(value.as_bytes()).into_owned(),
            )
        })
        .collect();
    let mut raw = Vec::new();
    let mut bytes = 0u64;
    while let Some(chunk) = response.chunk().await? {
        bytes += chunk.len() as u64;
        let room = HOLD_MAX.saturating_sub(raw.len());
        raw.extend_from_slice(&chunk[..chunk.len().min(room)]);
    }
    let took = started.elapsed();
    let encoding = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-encoding"))
        .map(|(_, value)| value.as_str());
    let body = match Encoding::from_header(encoding) {
        Encoding::Identity | Encoding::Unknown => raw,
        packed => decode::unpack(&raw, packed, HOLD_MAX).unwrap_or(raw),
    };
    Ok(Answer {
        status,
        version,
        headers,
        body,
        bytes,
        took,
    })
}

/// A recorded request, to send again. Its body must have been kept whole.
pub fn from_flow(
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: Option<Vec<u8>>,
    body_whole: bool,
) -> Result<Message> {
    if !body_whole {
        bail!(tr!(
            "its request body was not kept whole, so it cannot be sent again"
        ));
    }
    let mut headers = headers.to_vec();
    // A body read back from the session is unpacked.
    headers.retain(|(name, _)| !name.eq_ignore_ascii_case("content-encoding"));
    Ok(Message {
        method: method.to_owned(),
        url: url.to_owned(),
        status: 0,
        headers,
        body: body.unwrap_or_default(),
    })
}

/// `201 Created · HTTP/2 · 1.2 KB · 85 ms`, for the line over an answer.
pub fn summary(answer: &Answer) -> String {
    let reason = http::StatusCode::from_u16(answer.status)
        .ok()
        .and_then(|s| s.canonical_reason())
        .map(|r| format!(" {r}"))
        .unwrap_or_default();
    let size = match answer.bytes {
        b if b < 1024 => format!("{b} B"),
        b if b < 1024 * 1024 => format!("{:.1} KB", b as f64 / 1024.0),
        b => format!("{:.1} MB", b as f64 / (1024.0 * 1024.0)),
    };
    format!(
        "{}{reason} · {} · {size} · {} ms",
        answer.status,
        answer.version,
        answer.took.as_millis()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flow_goes_again_unpacked_and_whole() {
        let headers = vec![
            ("content-type".to_owned(), "application/json".to_owned()),
            ("content-encoding".to_owned(), "gzip".to_owned()),
        ];
        let message = from_flow(
            "POST",
            "https://a.test/x",
            &headers,
            Some(b"{}".to_vec()),
            true,
        )
        .unwrap();
        assert_eq!(message.headers, [headers[0].clone()]);
        assert_eq!(message.body, b"{}");
        assert!(
            from_flow("POST", "https://a.test/x", &headers, None, false)
                .unwrap_err()
                .to_string()
                .contains("not kept whole")
        );
    }

    /// A proxy that accepts and then says nothing: without a stop, the send
    /// would sit there until `WAIT_MAX`, ten minutes away.
    #[test]
    fn a_send_stops_where_it_stands() {
        use std::net::TcpListener;

        let dir = std::env::temp_dir().join(format!("tracewing-compose-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        tracewing_proxy::ca::LocalCa::load_or_create(
            &dir,
            tracewing_proxy::crypto_provider(),
            Default::default(),
        )
        .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let proxy = listener.local_addr().unwrap();
        let held = std::thread::spawn(move || {
            let accepted = listener.accept();
            std::thread::sleep(Duration::from_secs(2));
            drop(accepted);
        });
        let message = Message {
            method: "GET".to_owned(),
            url: "http://held.test/wait".to_owned(),
            status: 0,
            headers: Vec::new(),
            body: Vec::new(),
        };

        let (stop, stopped) = stopper();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            drop(stop);
        });
        let started = Instant::now();
        let answer = send(proxy, &dir.join("ca.pem"), &message, stopped).unwrap();
        let took = started.elapsed();

        assert!(answer.is_none(), "a stopped send has no answer");
        assert!(took < Duration::from_secs(5), "it waited {took:?}");
        let _ = held.join();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_summary_says_it_all() {
        let answer = Answer {
            status: 201,
            version: "HTTP/2.0".to_owned(),
            headers: Vec::new(),
            body: Vec::new(),
            bytes: 1229,
            took: Duration::from_millis(85),
        };
        assert_eq!(summary(&answer), "201 Created · HTTP/2.0 · 1.2 KB · 85 ms");
    }
}
