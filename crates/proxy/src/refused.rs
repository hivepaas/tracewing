//! A client that turned tracewing's certificate down, kept as a flow.
//!
//! When a program does not trust tracewing's authority, or pins a
//! certificate of its own, it ends the connection before it asks for
//! anything, and there is no request to record: without this, the program
//! fails and the list shows nothing at all, which looks like the program
//! never came. A row that says
//! the certificate was turned down, by whom and for which host, is what tells
//! somebody to install the authority where that program looks, or to stop
//! decrypting the host.
//!
//! Programs do it in three ways, and all three are seen:
//!
//! - **An alert about the certificate during the handshake** (`unknown_ca`,
//!   `bad_certificate`), as rustls and Go do.
//! - **The same alert just after it**: a client that checks the certificate
//!   once the handshake is done sends it then.
//! - **Nothing at all**: the handshake finishes and the client goes, with a
//!   reset or a close, without a request. curl on OpenSSL does this, and so
//!   does OkHttp when a pin does not match. A connection that started TLS
//!   and was gone within [`QUICK`] without asking anything is kept as one
//!   that hung up; a browser's preconnection, opened and left unused, stays
//!   open far longer than that.
//!
//! The handshake is hudsucker's, and so is what it does when one fails: it
//! logs an error with `tracing` and drops the connection, and tells the
//! handler nothing. So the error is taken from there. Each `CONNECT` runs in a
//! `process_connect` span, which is the span current when the handler is
//! asked whether to decrypt it ([`expect`]) and when it is shown the hello
//! ([`hello`]); the handler leaves what it wants done under that span's id,
//! and [`Watch`] does it when a failed handshake is logged in that span, or
//! when the span closes with no request asked ([`served`]) soon after the
//! hello. The first request takes the entry away, and so does the span's
//! close, whatever the connection did.
//!
//! [`layer`] enables that one span and hudsucker's errors and nothing else,
//! so every other span stays as cheap as with no subscriber at all: the
//! path that carries a million flows makes no span per request.
//!
//! If hudsucker changes these names, the test in `crates/dev/tests/e2e.rs`
//! that a refused certificate is kept fails.

use hudsucker::rustls::{self, AlertDescription};
use std::{
    collections::HashMap,
    error::Error as StdError,
    fmt, io,
    net::SocketAddr,
    sync::{
        LazyLock, Mutex, Once,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use tracing::{
    Event, Level, Metadata, Subscriber,
    field::{Field, Visit},
    span,
};
use tracing_subscriber::{
    filter,
    layer::{Context, Layer},
    prelude::*,
    registry::LookupSpan,
};

/// Where hudsucker's connections are handled, and the names it uses there.
const HUDSUCKER: &str = "hudsucker::proxy::internal";
const CONNECT_SPAN: &str = "process_connect";
const HANDSHAKE_FAILED: &str = "Failed to establish TLS connection";
/// What it logs when the connection fails after the handshake, which is
/// where a client that checks the certificate only once the handshake is
/// done (curl on OpenSSL) sends its alert.
const SERVING_FAILED: &str = "HTTPS connect error";

/// What a flow's error starts with when the client turned the certificate down.
pub const TURNED_DOWN: &str = "The client turned down tracewing's certificate";
/// When it hung up during the handshake, or just after it without asking
/// for anything, which is how some programs turn a certificate down.
pub const HUNG_UP: &str =
    "The client hung up after tracewing's certificate, before asking for anything";
/// Any other handshake with the client that failed.
pub const FAILED: &str = "TLS with the client failed";

/// Why the handshake with a client failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// It sent an alert about the certificate, named as TLS names it.
    TurnedDown(String),
    /// It closed the connection during the handshake, or just after it
    /// without asking for anything.
    HungUp,
    /// Anything else, as the error said.
    Other(String),
}

impl Failure {
    fn of(err: &(dyn StdError + 'static)) -> Self {
        let mut at = Some(err);
        while let Some(err) = at {
            if let Some(err) = err.downcast_ref::<rustls::Error>() {
                return match err {
                    rustls::Error::AlertReceived(alert) if about_the_certificate(*alert) => {
                        Failure::TurnedDown(format!("{alert:?}"))
                    }
                    other => Failure::Other(other.to_string()),
                };
            }
            if let Some(io) = err.downcast_ref::<io::Error>() {
                if matches!(
                    io.kind(),
                    io::ErrorKind::UnexpectedEof
                        | io::ErrorKind::ConnectionReset
                        | io::ErrorKind::ConnectionAborted
                        | io::ErrorKind::BrokenPipe
                ) {
                    return Failure::HungUp;
                }
                if let Some(inner) = io.get_ref() {
                    at = Some(inner as &(dyn StdError + 'static));
                    continue;
                }
            }
            at = err.source();
        }
        Failure::Other(err.to_string())
    }
}

/// The alerts a client sends when it will not accept the certificate.
fn about_the_certificate(alert: AlertDescription) -> bool {
    matches!(
        alert,
        AlertDescription::BadCertificate
            | AlertDescription::UnsupportedCertificate
            | AlertDescription::CertificateRevoked
            | AlertDescription::CertificateExpired
            | AlertDescription::CertificateUnknown
            | AlertDescription::UnknownCA
    )
}

impl fmt::Display for Failure {
    /// The error a flow is kept with. The window reads it back with [`said`].
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::TurnedDown(alert) => write!(f, "{TURNED_DOWN} ({alert})"),
            Failure::HungUp => f.write_str(HUNG_UP),
            Failure::Other(error) => write!(f, "{FAILED}: {error}"),
        }
    }
}

/// What a flow's error says about the handshake with its client, if that is
/// what it is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Said {
    TurnedDown,
    HungUp,
    Failed,
}

pub fn said(error: &str) -> Option<Said> {
    if error.starts_with(TURNED_DOWN) {
        Some(Said::TurnedDown)
    } else if error.starts_with(HUNG_UP) {
        Some(Said::HungUp)
    } else if error.starts_with(FAILED) {
        Some(Said::Failed)
    } else {
        None
    }
}

/// How soon a client that started TLS has to be gone, without a request,
/// to be kept as one that hung up.
pub const QUICK: Duration = Duration::from_secs(3);

type Tell = Box<dyn FnOnce(Failure, Duration) + Send>;

/// A connection that has asked for nothing yet.
struct Waiting {
    since: Instant,
    client: SocketAddr,
    /// Its client said hello in TLS.
    hello: bool,
    tell: Tell,
}

/// The connections waited on, by their span, and their spans by the address
/// their client came from, which is what a request knows of its connection.
#[derive(Default)]
struct Waits {
    by_span: HashMap<span::Id, Waiting>,
    by_client: HashMap<SocketAddr, span::Id>,
}

static WAITING: LazyLock<Mutex<Waits>> = LazyLock::new(Mutex::default);
/// How many there are, so that a request on a busy proxy with none waiting
/// costs one load and no lock.
static HOW_MANY: AtomicUsize = AtomicUsize::new(0);

fn waits() -> std::sync::MutexGuard<'static, Waits> {
    WAITING.lock().unwrap_or_else(|e| e.into_inner())
}

impl Waits {
    fn take(&mut self, id: &span::Id) -> Option<Waiting> {
        let waiting = self.by_span.remove(id)?;
        if self.by_client.get(&waiting.client) == Some(id) {
            self.by_client.remove(&waiting.client);
        }
        HOW_MANY.store(self.by_span.len(), Ordering::Relaxed);
        Some(waiting)
    }
}

fn take(id: &span::Id) -> Option<Waiting> {
    waits().take(id)
}

/// Has `tell` called if the connection being handled now, from `client`,
/// turns the certificate down, with why and how long it went on. Called
/// from the handler while hudsucker asks it about a `CONNECT`. Nothing
/// happens if [`layer`] is not what `tracing` reports to.
pub fn expect(client: SocketAddr, tell: impl FnOnce(Failure, Duration) + Send + 'static) {
    let Some(id) = tracing::Span::current().id() else {
        return;
    };
    let mut waits = waits();
    // An address used again is a connection after the last one closed.
    if let Some(old) = waits.by_client.insert(client, id.clone()) {
        waits.by_span.remove(&old);
    }
    waits.by_span.insert(
        id,
        Waiting {
            since: Instant::now(),
            client,
            hello: false,
            tell: Box::new(tell),
        },
    );
    HOW_MANY.store(waits.by_span.len(), Ordering::Relaxed);
}

/// The connection being handled now started TLS. Called from the handler
/// when hudsucker shows it the hello.
pub fn hello() {
    if HOW_MANY.load(Ordering::Relaxed) == 0 {
        return;
    }
    let Some(id) = tracing::Span::current().id() else {
        return;
    };
    if let Some(waiting) = waits().by_span.get_mut(&id) {
        waiting.hello = true;
    }
}

/// A request came from `client`: whatever its connection does now, it did
/// not turn the certificate down.
pub fn served(client: SocketAddr) {
    if HOW_MANY.load(Ordering::Relaxed) == 0 {
        return;
    }
    let mut waits = waits();
    if let Some(id) = waits.by_client.get(&client).cloned() {
        waits.take(&id);
    }
}

/// Makes [`layer`] what `tracing` reports to, once per process. A program
/// that sets up `tracing` itself adds [`layer`] to its own subscriber
/// instead, and this then does nothing.
pub fn watch() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ =
            tracing::subscriber::set_global_default(tracing_subscriber::registry().with(layer()));
    });
}

/// The `tracing` layer that sees a client's handshake fail. Added through
/// [`layer`], which filters what reaches it.
pub struct Watch;

fn watched(metadata: &Metadata<'_>) -> bool {
    metadata.target() == HUDSUCKER
        && if metadata.is_span() {
            metadata.name() == CONNECT_SPAN
        } else {
            *metadata.level() == Level::ERROR
        }
}

/// [`Watch`], with a filter of its own: it sees only what it watches, and
/// leaves what a log beside it shows to that log's filter.
pub fn layer<S: Subscriber + for<'a> LookupSpan<'a>>() -> impl Layer<S> {
    Watch.with_filter(filter::filter_fn(watched))
}

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for Watch {
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let mut seen = Seen::default();
        event.record(&mut seen);
        let failure = seen.error.unwrap_or(Failure::Other(String::new()));
        match seen.message.as_deref() {
            Some(HANDSHAKE_FAILED) => {}
            // After the handshake only an alert about the certificate is
            // this module's business: anything else is a connection that
            // went wrong while it was being served.
            Some(SERVING_FAILED) if matches!(failure, Failure::TurnedDown(_)) => {}
            _ => return,
        }
        let Some(span) = ctx.event_span(event) else {
            return;
        };
        let Some(waiting) = take(&span.id()) else {
            return;
        };
        (waiting.tell)(failure, waiting.since.elapsed());
    }

    fn on_close(&self, id: span::Id, _: Context<'_, S>) {
        let Some(waiting) = take(&id) else {
            return;
        };
        let took = waiting.since.elapsed();
        if waiting.hello && took < QUICK {
            (waiting.tell)(Failure::HungUp, took);
        }
    }
}

/// An event's message, and its error, as hudsucker logs them.
#[derive(Default)]
struct Seen {
    message: Option<String>,
    error: Option<Failure>,
}

impl Visit for Seen {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{value:?}"));
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_owned());
        }
    }

    fn record_error(&mut self, field: &Field, value: &(dyn StdError + 'static)) {
        if field.name() == "error" {
            self.error = Some(Failure::of(value));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn io_of(err: rustls::Error) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, err)
    }

    #[test]
    fn an_alert_about_the_certificate_is_a_refusal() {
        let err = io_of(rustls::Error::AlertReceived(AlertDescription::UnknownCA));
        assert_eq!(
            Failure::of(&err),
            Failure::TurnedDown("UnknownCA".to_owned())
        );
        let err = io_of(rustls::Error::AlertReceived(
            AlertDescription::CertificateUnknown,
        ));
        assert!(matches!(Failure::of(&err), Failure::TurnedDown(_)));
    }

    #[test]
    fn another_alert_is_not() {
        let err = io_of(rustls::Error::AlertReceived(
            AlertDescription::ProtocolVersion,
        ));
        assert!(matches!(Failure::of(&err), Failure::Other(_)));
    }

    #[test]
    fn a_connection_closed_early_is_a_hang_up() {
        let err = io::Error::new(io::ErrorKind::UnexpectedEof, "tls handshake eof");
        assert_eq!(Failure::of(&err), Failure::HungUp);
    }

    #[test]
    fn what_is_kept_is_read_back() {
        for (failure, it) in [
            (
                Failure::TurnedDown("UnknownCA".to_owned()),
                Said::TurnedDown,
            ),
            (Failure::HungUp, Said::HungUp),
            (Failure::Other("no shared cipher".to_owned()), Said::Failed),
        ] {
            assert_eq!(said(&failure.to_string()), Some(it), "{failure}");
        }
        assert_eq!(said("connection refused"), None);
    }
}
