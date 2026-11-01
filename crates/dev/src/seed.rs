//! Synthetic sessions for UI benchmarks. Rows vary the way real traffic does
//! (hosts, paths with ids and query strings, statuses, sizes), because a list
//! of identical rows would hit the text layout caches and flatter the UI.

use anyhow::Result;
use std::{path::Path, sync::Arc};
use tracewing_store::{
    session::{self, Shared},
    settings::StorageSettings,
    store::{BodyRecord, FlowRecord, Recorder},
};

const PREFIXES: [&str; 10] = [
    "api",
    "cdn",
    "auth",
    "static",
    "events",
    "graphql",
    "img",
    "ws",
    "push",
    "telemetry",
];
const DOMAINS: [&str; 30] = [
    "shopee.vn",
    "tiki.vn",
    "github.com",
    "googleapis.com",
    "sentry.io",
    "stripe.com",
    "firebaseio.com",
    "amplitude.com",
    "segment.io",
    "cloudflare.com",
    "example.com",
    "myapp.dev",
    "zalo.me",
    "momo.vn",
    "vnexpress.net",
    "facebook.com",
    "apple.com",
    "microsoft.com",
    "openai.com",
    "anthropic.com",
    "hivepaas.com",
    "vercel.app",
    "netlify.app",
    "s3.amazonaws.com",
    "fastly.net",
    "akamaihd.net",
    "doubleclick.net",
    "mixpanel.com",
    "intercom.io",
    "algolia.net",
];
const WORDS: [&str; 12] = [
    "áo khoác",
    "laptop",
    "rust",
    "proxy",
    "giày",
    "phở",
    "iphone",
    "keyboard",
    "sách",
    "camera",
    "tai nghe",
    "đồng hồ",
];

pub struct Rng(pub u64);

impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len() as u64) as usize]
    }

    /// Picks by weight from `(value, weight)` pairs.
    fn weighted<T: Copy>(&mut self, items: &[(T, u64)]) -> T {
        let total: u64 = items.iter().map(|(_, w)| w).sum();
        let mut roll = self.below(total);
        for (value, weight) in items {
            if roll < *weight {
                return *value;
            }
            roll -= weight;
        }
        items[0].0
    }
}

const PHRASES: [&str; 16] = [
    "đặt hàng",
    "thanh toán",
    "người dùng",
    "giao hàng",
    "sản phẩm",
    "khuyến mãi",
    "tài khoản",
    "Hà Nội",
    "Thành phố Hồ Chí Minh",
    "shipping_address",
    "created_at",
    "description",
    "application",
    "the quick brown fox",
    "internal server",
    "timeout exceeded",
];

pub fn rich_body(rng: &mut Rng, id: u64, n: u64) -> String {
    let target = 500 + rng.below(3000) as usize;
    rich_json(rng, id, n, target)
}

/// JSON of about `target` bytes.
pub fn rich_text(rng: &mut Rng, target: usize) -> String {
    rich_json(rng, 1, 1, target)
}

fn rich_json(rng: &mut Rng, id: u64, n: u64, target: usize) -> String {
    let mut body = format!(
        r#"{{"id":{id},"n":{n},"trace_id":"{:016x}{:016x}","items":["#,
        rng.next_u64(),
        rng.next_u64()
    );
    while body.len() < target {
        body.push('"');
        // Most words are common and many are rare, as in real text.
        if rng.below(4) == 0 {
            body.push_str(PHRASES[rng.below(PHRASES.len() as u64) as usize]);
        } else {
            let spread = 1u64 << (4 + rng.below(13));
            body.push_str(&format!("w{}", rng.below(spread)));
        }
        body.push_str("\",");
    }
    // One body in a thousand holds this, to look for something rare.
    if rng.below(1000) == 0 {
        body.push_str("\"mã giảm giá\",");
    }
    body.push_str("0]}");
    body
}

/// Writes `rows` synthetic flows to a new session through the normal writer.
///
/// With `rich`, bodies are JSON of about 2 KB in words of many kinds with a
/// unique trace id, for measuring search: a short, repetitive body would make
/// any index look small and fast.
pub fn seed(dir: &Path, rows: u64, rich: bool) -> Result<u64> {
    session::create(dir)?;
    let settings = StorageSettings {
        session_max_mib: u64::MAX / (1 << 20),
        ..StorageSettings::default()
    };
    let (recorder, writer) =
        Recorder::spawn(dir, &settings, Arc::new(Shared::default()), None, 10_000)?;
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let started = 1_793_000_000_000_000_i64;

    for i in 0..rows {
        let host = format!("{}.{}", rng.pick(&PREFIXES), rng.pick(&DOMAINS));
        let id = rng.below(9_000_000) + 1000;
        let (path, content_type) = match rng.below(10) {
            0 => (format!("/v1/users/{id}"), "application/json"),
            1 => (
                format!("/v2/orders/{id}/items?page={}", rng.below(40)),
                "application/json",
            ),
            2 => ("/graphql".to_owned(), "application/json"),
            3 => (
                format!("/api/feed?cursor={:016x}&limit=50", rng.next_u64()),
                "application/json",
            ),
            4 => (
                format!("/static/js/app.{:08x}.js", rng.next_u64() as u32),
                "application/javascript",
            ),
            5 => (format!("/images/{id}.webp"), "image/webp"),
            6 => ("/auth/token".to_owned(), "application/json"),
            7 => ("/events/batch".to_owned(), "text/plain"),
            8 => (
                format!("/search?q={}&limit=20", rng.pick(&WORDS)),
                "application/json; charset=utf-8",
            ),
            _ => (format!("/assets/fonts/{id}.woff2"), "font/woff2"),
        };
        let method = rng.weighted(&[
            ("GET", 70),
            ("POST", 20),
            ("PUT", 4),
            ("DELETE", 3),
            ("PATCH", 2),
            ("OPTIONS", 1),
        ]);
        let status = rng.weighted(&[
            (Some(200), 800),
            (Some(201), 30),
            (Some(204), 30),
            (Some(304), 50),
            (Some(400), 20),
            (Some(401), 20),
            (Some(404), 30),
            (Some(500), 10),
            (Some(502), 5),
            (None, 5),
        ]);
        let resp_bytes = (1u64 << rng.below(18)) + rng.below(1024);
        let duration_us = ((1u64 << rng.below(20)) + rng.below(5000)) as i64;
        let body = if rich {
            rich_body(&mut rng, id, i)
        } else {
            format!(r#"{{"id":{id},"ok":true,"n":{i}}}"#)
        }
        .into_bytes();

        recorder.submit_wait(FlowRecord {
            started_us: started + i as i64 * 3_000,
            client: "127.0.0.1:52344".to_owned(),
            kind: "http",
            method: method.to_owned(),
            scheme: "https".to_owned(),
            host,
            path,
            client_version: "HTTP/2.0".to_owned(),
            server_version: status.map(|_| "HTTP/2.0".to_owned()),
            status,
            content_type: status.map(|_| content_type.to_owned()),
            ttfb_us: Some(duration_us / 2),
            duration_us: Some(duration_us),
            request: BodyRecord {
                complete: true,
                ..BodyRecord::default()
            },
            response: BodyRecord {
                bytes: resp_bytes,
                captured: body,
                truncated: false,
                complete: status.is_some(),
                ..BodyRecord::default()
            },
            error: status
                .is_none()
                .then(|| "connection reset by peer".to_owned()),
            ..FlowRecord::default()
        });
    }

    let stats = recorder.stats();
    drop(recorder);
    writer.close()?;
    Ok(stats.written.load(std::sync::atomic::Ordering::Relaxed))
}
