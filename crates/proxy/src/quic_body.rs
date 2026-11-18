//! Bodies between `h3`'s streams and the body the core takes, each way.
//!
//! Both halves of HTTP/3 need this. [`crate::quic_server`] reads a client's
//! body and writes a server's answer; [`crate::quic_upstream`] writes a
//! request's body and reads what the server says. The two directions are the
//! same work, and `h3` gives each of them its own type -- a client's stream
//! and a server's are not one type even where they do the same thing -- so
//! what they have in common is named here, [`Arrives`] and [`Takes`], and the
//! pumps over it are written once rather than four times.
//!
//! The futures are spelled out with `+ Send` rather than written as `async
//! fn`, because [`carried`] hands its half to a task of its own and a task's
//! future has to be `Send`. An `async fn` in a trait says nothing about that.

use bytes::{Buf, Bytes};
use http::HeaderMap;
use http_body::Frame;
use http_body_util::{BodyExt as _, combinators::BoxBody};
use hudsucker::Body;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::sync::mpsc;

/// How many frames wait here while the other end reads them. Small on
/// purpose: a body is pumped as it arrives, and anything larger is the
/// spool's business rather than this channel's.
const WAITING: usize = 8;

/// A stream a body arrives on: the receiving half of either of `h3`'s.
pub(crate) trait Arrives: Send + 'static {
    fn data(&mut self) -> impl Future<Output = Streamed<Option<Bytes>>> + Send;
    fn trailers(&mut self) -> impl Future<Output = Streamed<Option<HeaderMap>>> + Send;
}

/// A stream a body goes out on: the sending half of either of `h3`'s.
pub(crate) trait Takes: Send + 'static {
    fn data(&mut self, data: Bytes) -> impl Future<Output = Streamed<()>> + Send;
    fn trailers(&mut self, trailers: HeaderMap) -> impl Future<Output = Streamed<()>> + Send;
    /// Says there is no more, which is what ends the request or the answer.
    fn done(&mut self) -> impl Future<Output = Streamed<()>> + Send;
}

type Streamed<T> = Result<T, h3::error::StreamError>;

impl Arrives for h3::server::RequestStream<h3_quinn::RecvStream, Bytes> {
    async fn data(&mut self) -> Streamed<Option<Bytes>> {
        Ok(whole(self.recv_data().await?))
    }

    async fn trailers(&mut self) -> Streamed<Option<HeaderMap>> {
        self.recv_trailers().await
    }
}

impl Arrives for h3::client::RequestStream<h3_quinn::RecvStream, Bytes> {
    async fn data(&mut self) -> Streamed<Option<Bytes>> {
        Ok(whole(self.recv_data().await?))
    }

    async fn trailers(&mut self) -> Streamed<Option<HeaderMap>> {
        self.recv_trailers().await
    }
}

impl Takes for h3::server::RequestStream<h3_quinn::SendStream<Bytes>, Bytes> {
    async fn data(&mut self, data: Bytes) -> Streamed<()> {
        self.send_data(data).await
    }

    async fn trailers(&mut self, trailers: HeaderMap) -> Streamed<()> {
        self.send_trailers(trailers).await
    }

    async fn done(&mut self) -> Streamed<()> {
        self.finish().await
    }
}

impl Takes for h3::client::RequestStream<h3_quinn::SendStream<Bytes>, Bytes> {
    async fn data(&mut self, data: Bytes) -> Streamed<()> {
        self.send_data(data).await
    }

    async fn trailers(&mut self, trailers: HeaderMap) -> Streamed<()> {
        self.send_trailers(trailers).await
    }

    async fn done(&mut self) -> Streamed<()> {
        self.finish().await
    }
}

/// What `h3` hands over of a chunk, as one piece. `h3` gives a `Buf` that may
/// be in several, and the core's bodies carry `Bytes`.
fn whole(chunk: Option<impl Buf>) -> Option<Bytes> {
    chunk.map(|mut chunk| chunk.copy_to_bytes(chunk.remaining()))
}

/// The body on `from`, as the core takes bodies: read off the stream by a
/// task of its own and handed over a frame at a time.
pub(crate) fn carried(mut from: impl Arrives) -> Body {
    let (tx, rx) = mpsc::channel(WAITING);
    tokio::spawn(async move {
        loop {
            match from.data().await {
                Ok(Some(data)) => {
                    // Whoever was reading let go of the body: nothing more
                    // is wanted, so nothing more is read.
                    if tx.send(Ok(Frame::data(data))).await.is_err() {
                        return;
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    let _ = tx.send(Err(as_error(&e))).await;
                    return;
                }
            }
        }
        // Trailers come after the body, and gRPC's status is in them, so they
        // are carried rather than dropped.
        if let Ok(Some(trailers)) = from.trailers().await {
            let _ = tx.send(Ok(Frame::trailers(trailers))).await;
        }
    });
    Body::from(BoxBody::new(Fed(rx)))
}

/// `body` out over `into`, to its end, which `into` is then told of.
pub(crate) async fn poured(into: &mut impl Takes, mut body: Body) -> anyhow::Result<()> {
    while let Some(frame) = body.frame().await {
        match frame?.into_data() {
            Ok(data) => into.data(data).await?,
            // Anything that is not data is trailers, which end the body.
            Err(frame) => {
                if let Ok(trailers) = frame.into_trailers() {
                    into.trailers(trailers).await?;
                }
                break;
            }
        }
    }
    into.done().await?;
    Ok(())
}

/// What the core's bodies carry as a failure. An `h3` error is none of
/// [`hudsucker::Error`]'s variants, so it goes in as the io error it reads as.
pub(crate) fn as_error(e: &h3::error::StreamError) -> hudsucker::Error {
    hudsucker::Error::Io(std::io::Error::other(e.to_string()))
}

/// The receiving end of a body being pumped: see [`carried`].
struct Fed(mpsc::Receiver<Result<Frame<Bytes>, hudsucker::Error>>);

impl http_body::Body for Fed {
    type Data = Bytes;
    type Error = hudsucker::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Self::Error>>> {
        self.0.poll_recv(cx)
    }
}
