//! A slow network, on purpose: to see an app on a train, before its users do.
//!
//! - Latency: half before the request goes on, half before the response comes
//!   back, so each request costs about one round trip more.
//! - Bandwidth: one link each way, shared by every flow it applies to, as a
//!   real one is. Bodies go through it a slice at a time, each slice waiting
//!   for the link to be free.
//! - Loss: a proxy sees TCP, which does not lose data but waits to send it
//!   again. So a slice that is "lost" waits for a retransmission instead.
//!
//! Hosts left alone are not decrypted and go through untouched by any of it.

use crate::rules::{pattern_fits, plain_host};
use anyhow::{Context as _, Result, bail};
use bytes::Bytes;
use http_body::{Frame, SizeHint};
use http_body_util::combinators::BoxBody;
use hudsucker::Body;
use serde::{Deserialize, Serialize};
use std::{
    future::Future,
    path::Path,
    pin::Pin,
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicU64, Ordering},
    },
    task::{Context, Poll},
    time::{Duration, Instant},
};

/// What the network is like.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Conditions {
    pub on: bool,
    /// What to call them, as a preset's name or `Custom`.
    pub name: String,
    /// Added to each round trip.
    pub latency_ms: u32,
    /// Kilobits a second to the client and from it; 0 for no limit.
    pub down_kbps: u32,
    pub up_kbps: u32,
    /// Of every slice of a body, how many in a hundred wait to be sent again.
    pub loss_percent: u32,
    /// Hosts it applies to, as the hosts left alone are written; none for all.
    pub hosts: Vec<String>,
}

/// Networks as people meet them. Close to what browsers' tools offer.
pub const PRESETS: [(&str, u32, u32, u32, u32); 4] = [
    // name, latency, down, up, loss
    ("Slow 3G", 400, 400, 400, 0),
    ("Fast 3G", 150, 1_600, 750, 0),
    ("4G", 50, 9_000, 9_000, 0),
    ("Poor Wi-Fi", 80, 2_000, 1_000, 3),
];

/// How much of a body goes through the link at a time.
const SLICE: usize = 16 * 1024;
/// The least a retransmission waits, as TCP's own minimum.
const RETRANSMIT_MIN: Duration = Duration::from_millis(200);

impl Conditions {
    pub fn preset(name: &str) -> Option<Self> {
        PRESETS.iter().find(|(n, ..)| *n == name).map(
            |&(name, latency_ms, down_kbps, up_kbps, loss_percent)| Self {
                on: true,
                name: name.to_owned(),
                latency_ms,
                down_kbps,
                up_kbps,
                loss_percent,
                hosts: Vec::new(),
            },
        )
    }

    /// The file's conditions; no file means a network as it is.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text =
            std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("read {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        std::fs::write(path, toml::to_string(self)?)
            .with_context(|| format!("write {}", path.display()))
    }

    /// Refuses what makes no sense, and says why.
    pub fn check(&self) -> Result<()> {
        if self.loss_percent > 100 {
            bail!("{}% is more than all of it", self.loss_percent);
        }
        if self.latency_ms > 60_000 {
            bail!("a latency of more than a minute stops everything");
        }
        Ok(())
    }

    /// Whether they slow anything down at all.
    pub fn slows(&self) -> bool {
        self.on && self.slows_when_on()
    }

    /// Whether they would, turned on.
    pub fn slows_when_on(&self) -> bool {
        self.latency_ms > 0 || self.down_kbps > 0 || self.up_kbps > 0 || self.loss_percent > 0
    }

    pub fn applies_to(&self, host: &str) -> bool {
        if !self.slows() {
            return false;
        }
        let host = plain_host(host);
        self.hosts.is_empty()
            || self
                .hosts
                .iter()
                .any(|p| pattern_fits(&plain_host(p), &host))
    }

    /// In a few words, for the toolbar and a flow's rules.
    pub fn summary(&self) -> String {
        let name = if self.name.is_empty() {
            "Slow network"
        } else {
            &self.name
        };
        format!("{name} network")
    }
}

/// When a link that sends `kbps` is free again, having sent `bytes` from
/// `start` or from when it was last free, whichever is later.
fn free_after(free_at: Instant, now: Instant, bytes: usize, kbps: u32) -> Instant {
    let from = free_at.max(now);
    if kbps == 0 {
        return from;
    }
    let micros = bytes as u64 * 8 * 1000 / u64::from(kbps);
    from + Duration::from_micros(micros)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// To the client.
    Down,
    /// From it.
    Up,
}

impl Default for Network {
    fn default() -> Self {
        Self::new(Conditions::default())
    }
}

/// The network the proxy's traffic goes through, as it is now.
#[derive(Debug)]
pub struct Network {
    conditions: RwLock<Conditions>,
    down: Mutex<Instant>,
    up: Mutex<Instant>,
    /// For loss, which needs no more than a coin.
    seed: AtomicU64,
}

impl Network {
    pub fn new(conditions: Conditions) -> Self {
        let now = Instant::now();
        Self {
            conditions: RwLock::new(conditions),
            down: Mutex::new(now),
            up: Mutex::new(now),
            seed: AtomicU64::new(0x9E37_79B9_7F4A_7C15),
        }
    }

    pub fn conditions(&self) -> Conditions {
        self.conditions
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// From the next request on.
    pub fn set(&self, conditions: Conditions) {
        *self.conditions.write().unwrap_or_else(|e| e.into_inner()) = conditions;
    }

    /// The conditions a request to `host` meets, if it is slowed at all.
    pub fn for_host(&self, host: &str) -> Option<Conditions> {
        let conditions = self.conditions.read().unwrap_or_else(|e| e.into_inner());
        conditions.applies_to(host).then(|| conditions.clone())
    }

    /// When a slice of `bytes` going `direction` is through.
    fn send(&self, direction: Direction, bytes: usize, conditions: &Conditions) -> Instant {
        let (link, kbps) = match direction {
            Direction::Down => (&self.down, conditions.down_kbps),
            Direction::Up => (&self.up, conditions.up_kbps),
        };
        let mut free_at = link.lock().unwrap_or_else(|e| e.into_inner());
        *free_at = free_after(*free_at, Instant::now(), bytes, kbps);
        let mut done = *free_at;
        if conditions.loss_percent > 0 && self.coin() < conditions.loss_percent {
            let round_trip = Duration::from_millis(u64::from(conditions.latency_ms));
            done += (round_trip * 2).max(RETRANSMIT_MIN);
        }
        done
    }

    /// 0 to 99, well enough spread for a coin.
    fn coin(&self) -> u32 {
        let mut x = self
            .seed
            .fetch_add(0x9E37_79B9_7F4A_7C15, Ordering::Relaxed);
        x ^= x >> 30;
        x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x ^= x >> 27;
        x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^= x >> 31;
        (x % 100) as u32
    }

    /// Half of a round trip's latency, which a request and its response each
    /// wait.
    pub async fn half_latency(conditions: &Conditions) {
        let half = Duration::from_millis(u64::from(conditions.latency_ms) / 2);
        if !half.is_zero() {
            tokio::time::sleep(half).await;
        }
    }

    /// `body`, through the link going `direction`.
    pub fn wrap(
        self: &Arc<Self>,
        body: Body,
        direction: Direction,
        conditions: Conditions,
    ) -> Body {
        let limited = match direction {
            Direction::Down => conditions.down_kbps > 0,
            Direction::Up => conditions.up_kbps > 0,
        };
        if !limited && conditions.loss_percent == 0 {
            return body;
        }
        Body::from(BoxBody::new(SlowBody {
            inner: body,
            network: Arc::clone(self),
            direction,
            conditions,
            waiting: None,
            rest: Bytes::new(),
        }))
    }
}

struct SlowBody {
    inner: Body,
    network: Arc<Network>,
    direction: Direction,
    conditions: Conditions,
    /// A slice on its way, and when it is through.
    waiting: Option<(Bytes, Pin<Box<tokio::time::Sleep>>)>,
    /// What is left of the last frame, to go a slice at a time.
    rest: Bytes,
}

impl SlowBody {
    /// The next slice of what is left, on its way.
    fn send_next(&mut self) {
        let size = self.rest.len().min(SLICE);
        let slice = self.rest.split_to(size);
        let done = self
            .network
            .send(self.direction, slice.len(), &self.conditions);
        let sleep = tokio::time::sleep_until(done.into());
        self.waiting = Some((slice, Box::pin(sleep)));
    }
}

impl http_body::Body for SlowBody {
    type Data = Bytes;
    type Error = hudsucker::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Self::Error>>> {
        let this = self.get_mut();
        loop {
            if let Some((_, sleep)) = &mut this.waiting {
                if sleep.as_mut().poll(cx).is_pending() {
                    return Poll::Pending;
                }
                let (slice, _) = this.waiting.take().expect("just looked");
                return Poll::Ready(Some(Ok(Frame::data(slice))));
            }
            if !this.rest.is_empty() {
                this.send_next();
                continue;
            }
            match Pin::new(&mut this.inner).poll_frame(cx) {
                Poll::Ready(Some(Ok(frame))) => match frame.into_data() {
                    Ok(data) if data.is_empty() => continue,
                    Ok(data) => {
                        this.rest = data;
                        this.send_next();
                    }
                    // Trailers go as they come.
                    Err(frame) => return Poll::Ready(Some(Ok(frame))),
                },
                other => return other,
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        self.waiting.is_none() && self.rest.is_empty() && self.inner.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        let pending = self.rest.len() + self.waiting.as_ref().map_or(0, |(s, _)| s.len());
        let inner = self.inner.size_hint();
        let mut hint = SizeHint::new();
        hint.set_lower(inner.lower() + pending as u64);
        if let Some(upper) = inner.upper() {
            hint.set_upper(upper + pending as u64);
        }
        hint
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_are_whole_conditions() {
        let slow = Conditions::preset("Slow 3G").unwrap();
        assert!(slow.on && slow.slows());
        assert_eq!((slow.latency_ms, slow.down_kbps), (400, 400));
        assert_eq!(slow.summary(), "Slow 3G network");
        assert!(Conditions::preset("Dial-up").is_none());
        for (name, ..) in PRESETS {
            Conditions::preset(name).unwrap().check().unwrap();
        }
    }

    #[test]
    fn they_apply_to_the_hosts_named_or_to_all() {
        let mut conditions = Conditions::preset("4G").unwrap();
        assert!(conditions.applies_to("anything.example:443"));
        conditions.hosts = vec!["*.example.com".into(), "api.test".into()];
        assert!(conditions.applies_to("example.com"));
        assert!(conditions.applies_to("cdn.example.com:443"));
        assert!(conditions.applies_to("API.test"));
        assert!(!conditions.applies_to("other.test"));
        conditions.on = false;
        assert!(!conditions.applies_to("api.test"));
        assert!(
            !Conditions {
                on: true,
                ..Conditions::default()
            }
            .slows(),
            "nothing to slow"
        );
    }

    #[test]
    fn a_link_sends_one_slice_after_another() {
        let now = Instant::now();
        // 1000 bytes at 80 kbit/s: 100 ms.
        let first = free_after(now, now, 1000, 80);
        assert_eq!(first - now, Duration::from_millis(100));
        // Sent while the link is busy, it goes after.
        let second = free_after(first, now, 1000, 80);
        assert_eq!(second - now, Duration::from_millis(200));
        // An idle link starts now, not when it was last free.
        let later = now + Duration::from_secs(5);
        assert_eq!(
            free_after(first, later, 1000, 80) - later,
            Duration::from_millis(100)
        );
        assert_eq!(free_after(first, later, 1000, 0), later, "no limit");
    }

    #[test]
    fn nonsense_is_refused() {
        let too_lossy = Conditions {
            loss_percent: 101,
            ..Conditions::default()
        };
        assert!(too_lossy.check().is_err());
    }

    #[test]
    fn a_coin_is_fair_enough() {
        let network = Network::new(Conditions::default());
        let heads = (0..10_000).filter(|_| network.coin() < 10).count();
        assert!((800..1200).contains(&heads), "{heads} of 10000 under 10");
    }

    #[test]
    fn they_are_kept_as_written() {
        let conditions = Conditions {
            hosts: vec!["*.example.com".into()],
            ..Conditions::preset("Poor Wi-Fi").unwrap()
        };
        let text = toml::to_string(&conditions).unwrap();
        assert_eq!(toml::from_str::<Conditions>(&text).unwrap(), conditions);
        assert_eq!(
            toml::from_str::<Conditions>("").unwrap(),
            Conditions::default()
        );
    }
}
