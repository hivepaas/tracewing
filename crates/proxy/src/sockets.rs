//! The WebSockets open now, by the row each has, so that the window can send
//! a message of its own into one: to the server, as if the app had sent it, or
//! to the app, as if the server had. It is written down as the others are.

use hudsucker::tokio_tungstenite::tungstenite::Message as WsMessage;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
use tokio::sync::mpsc;
use tracewing_store::store::StreamHandle;

/// How many sent messages may wait for their turn on one side.
const WAITING_MAX: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toward {
    Server,
    App,
}

/// What the window sends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Said {
    Text(String),
    Binary(Vec<u8>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotSent {
    /// It has closed, or it was never one tracewing carries.
    NotOpen,
    /// Too many are still waiting to go.
    Busy,
}

struct Open {
    id: u64,
    stream: StreamHandle,
    to_server: mpsc::Sender<WsMessage>,
    to_app: mpsc::Sender<WsMessage>,
}

#[derive(Default)]
pub struct Sockets {
    open: Mutex<Vec<Open>>,
    next: AtomicU64,
}

impl std::fmt::Debug for Sockets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sockets")
            .field("open", &self.lock().len())
            .finish()
    }
}

/// A socket's place among those open, for as long as it is kept: the last
/// one dropped takes it off.
pub(crate) struct Registered {
    sockets: Arc<Sockets>,
    id: u64,
}

impl Drop for Registered {
    fn drop(&mut self) {
        self.sockets.lock().retain(|open| open.id != self.id);
    }
}

/// What the window sends one way, for that way's pump to carry.
pub(crate) type Inbox = mpsc::Receiver<WsMessage>;

impl Sockets {
    /// Puts a socket among those open. The inboxes are for the pump toward
    /// the server and the one toward the app.
    pub(crate) fn open(self: &Arc<Self>, stream: StreamHandle) -> (Arc<Registered>, Inbox, Inbox) {
        let (to_server, server_inbox) = mpsc::channel(WAITING_MAX);
        let (to_app, app_inbox) = mpsc::channel(WAITING_MAX);
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        self.lock().push(Open {
            id,
            stream,
            to_server,
            to_app,
        });
        let registered = Arc::new(Registered {
            sockets: Arc::clone(self),
            id,
        });
        (registered, server_inbox, app_inbox)
    }

    /// Whether the socket of that row is open, to send into.
    pub fn is_open(&self, row: i64) -> bool {
        self.lock()
            .iter()
            .any(|open| open.stream.row() == Some(row))
    }

    /// Sends a message into the socket of that row.
    pub fn send(&self, row: i64, toward: Toward, said: Said) -> Result<(), NotSent> {
        let open = self.lock();
        let Some(open) = open.iter().find(|open| open.stream.row() == Some(row)) else {
            return Err(NotSent::NotOpen);
        };
        let message = match said {
            Said::Text(text) => WsMessage::text(text),
            Said::Binary(data) => WsMessage::binary(data),
        };
        let sender = match toward {
            Toward::Server => &open.to_server,
            Toward::App => &open.to_app,
        };
        sender.try_send(message).map_err(|e| match e {
            mpsc::error::TrySendError::Full(_) => NotSent::Busy,
            mpsc::error::TrySendError::Closed(_) => NotSent::NotOpen,
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<Open>> {
        self.open.lock().unwrap_or_else(|e| e.into_inner())
    }
}
