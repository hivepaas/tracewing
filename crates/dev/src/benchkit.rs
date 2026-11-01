//! Benchmark helpers: an HTTPS origin with its own CA, and a load generator
//! that goes through the proxy and trusts only the proxy's CA, so every
//! successful request proves the proxy intercepted it.

use anyhow::{Context, Result};
use bytes::Bytes;
use http::{Request, Response, StatusCode, header};
use http_body_util::{BodyExt, Full, StreamBody, combinators::BoxBody};
use hudsucker::{
    rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, Issuer, KeyPair, KeyUsagePurpose},
    rustls::{
        ServerConfig,
        pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
    },
};
use hyper::{body::Frame, body::Incoming, service::service_fn};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder,
};
use std::{
    collections::{BTreeMap, HashMap},
    convert::Infallible,
    io::{Read, Write},
    path::Path,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

type OriginBody = BoxBody<Bytes, Infallible>;

/// Serves the benchmark routes over TLS (HTTP/1.1 and HTTP/2) until the task
/// is dropped. Writes the origin's CA to `ca_out` so the proxy can trust it.
///
/// Routes:
/// - `/json?size=N`: a JSON body of N bytes;
/// - `/json?size=N&enc=gzip`: JSON of varied words, about N bytes, sent packed
///   (`gzip`, `br`, `zstd`, or `identity` for none). From a pool made beforehand,
///   so that what is timed is the proxy and not the origin compressing;
/// - `/stream?events=N&gap_ms=M`: N server-sent events, M ms apart;
/// - anything else: `ok`.
pub async fn serve_origin(listener: TcpListener, ca_out: &Path) -> Result<()> {
    let config = origin_tls(ca_out)?;
    let acceptor = TlsAcceptor::from(Arc::new(config));

    loop {
        let (tcp, _) = listener.accept().await?;
        tcp.set_nodelay(true)?;
        let acceptor = acceptor.clone();
        tokio::spawn(async move {
            let Ok(tls) = acceptor.accept(tcp).await else {
                return;
            };
            let _ = Builder::new(TokioExecutor::new())
                .serve_connection(TokioIo::new(tls), service_fn(origin_route))
                .await;
        });
    }
}

async fn origin_route(req: Request<Incoming>) -> Result<Response<OriginBody>, Infallible> {
    let query = parse_query(req.uri().query().unwrap_or(""));
    let number = |key: &str, default: u64| {
        query
            .get(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };

    let response = match req.uri().path() {
        "/json" => {
            let size = number("size", 1024) as usize;
            let packed = query.get("enc").map(|enc| (*enc, packed_json(enc, size)));
            let body = match &packed {
                Some((_, body)) => body.clone(),
                None if query.contains_key("rich") => rich_json(),
                None => json_body(size),
            };
            let mut response = Response::builder().header(header::CONTENT_TYPE, "application/json");
            if let Some((enc, _)) = packed.filter(|(enc, _)| *enc != "identity") {
                response = response.header(header::CONTENT_ENCODING, enc);
            }
            response.body(Full::new(body).boxed())
        }
        "/stream" => {
            let events = number("events", 5);
            let gap = Duration::from_millis(number("gap_ms", 200));
            let stream = hudsucker::futures::stream::unfold(0, move |i| async move {
                if i >= events {
                    return None;
                }
                if i > 0 {
                    tokio::time::sleep(gap).await;
                }
                let event = Bytes::from(format!("data: event {i}\n\n"));
                Some((Ok::<_, Infallible>(Frame::data(event)), i + 1))
            });
            Response::builder()
                .header(header::CONTENT_TYPE, "text/event-stream")
                .body(BodyExt::boxed(StreamBody::new(stream)))
        }
        _ => Response::builder().body(Full::new(Bytes::from_static(b"ok")).boxed()),
    };
    Ok(response.unwrap_or_else(|_| {
        let mut res = Response::new(Full::new(Bytes::new()).boxed());
        *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        res
    }))
}

/// A different body of about 2 KB each time, the kind search has to work for.
fn rich_json() -> Bytes {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let mut rng = crate::seed::Rng(n.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
    Bytes::from(crate::seed::rich_body(&mut rng, n, n))
}

const POOL: u64 = 4096;

type Pools = Mutex<HashMap<(String, usize), Arc<Vec<Bytes>>>>;

/// One of `POOL` bodies, each different, in the encoding asked for.
fn packed_json(encoding: &str, size: usize) -> Bytes {
    static POOLS: OnceLock<Pools> = OnceLock::new();
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let pool = {
        let mut pools = POOLS
            .get_or_init(Default::default)
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        Arc::clone(
            pools
                .entry((encoding.to_owned(), size))
                .or_insert_with(|| Arc::new(make_pool(encoding, size))),
        )
    };
    pool[(NEXT.fetch_add(1, Ordering::Relaxed) % POOL) as usize].clone()
}

fn make_pool(encoding: &str, size: usize) -> Vec<Bytes> {
    (0..POOL)
        .map(|n| {
            let mut rng = crate::seed::Rng((n + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
            let text = crate::seed::rich_text(&mut rng, size).into_bytes();
            Bytes::from(pack(&text, encoding))
        })
        .collect()
}

fn pack(text: &[u8], encoding: &str) -> Vec<u8> {
    match encoding {
        "gzip" => {
            let mut out = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::new(6));
            out.write_all(text).expect("a Vec takes any bytes");
            out.finish().expect("a Vec takes any bytes")
        }
        "br" => {
            let mut out = Vec::new();
            brotli::CompressorReader::new(text, 4096, 5, 22)
                .read_to_end(&mut out)
                .expect("a Vec takes any bytes");
            out
        }
        "zstd" => zstd::encode_all(text, 3).expect("a Vec takes any bytes"),
        _ => text.to_vec(),
    }
}

fn json_body(size: usize) -> Bytes {
    const HEAD: &[u8] = br#"{"data":""#;
    const TAIL: &[u8] = br#""}"#;
    let filler = size.saturating_sub(HEAD.len() + TAIL.len());
    let mut body = Vec::with_capacity(size.max(HEAD.len() + TAIL.len()));
    body.extend_from_slice(HEAD);
    body.extend(std::iter::repeat_n(b'x', filler));
    body.extend_from_slice(TAIL);
    Bytes::from(body)
}

fn parse_query(query: &str) -> BTreeMap<&str, &str> {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .collect()
}

/// A CA and a `localhost`/`127.0.0.1` leaf for the origin.
fn origin_tls(ca_out: &Path) -> Result<ServerConfig> {
    let ca_key = KeyPair::generate()?;
    let mut ca_params = CertificateParams::default();
    ca_params
        .distinguished_name
        .push(DnType::CommonName, "tracewing bench origin CA");
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
    ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    let ca_cert = ca_params.self_signed(&ca_key)?;
    std::fs::write(ca_out, ca_cert.pem()).with_context(|| format!("write {}", ca_out.display()))?;
    let issuer = Issuer::new(ca_params, ca_key);

    let leaf_key = KeyPair::generate()?;
    let leaf_params = CertificateParams::new(vec!["localhost".to_owned(), "127.0.0.1".to_owned()])?;
    let leaf = leaf_params.signed_by(&leaf_key, &issuer)?;

    let mut config =
        ServerConfig::builder_with_provider(Arc::new(tracewing_proxy::crypto_provider()))
            .with_safe_default_protocol_versions()?
            .with_no_client_auth()
            .with_single_cert(
                vec![CertificateDer::from(leaf.der().to_vec())],
                PrivateKeyDer::from(PrivatePkcs8KeyDer::from(leaf_key.serialize_der())),
            )?;
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(config)
}

pub struct LoadOptions {
    pub proxy: String,
    pub url: String,
    pub proxy_ca: std::path::PathBuf,
    pub requests: u64,
    pub concurrency: usize,
    pub http1_only: bool,
    /// Samples this process's resident memory while the load runs.
    pub watch_pid: Option<u32>,
}

#[derive(Debug)]
pub struct LoadReport {
    pub ok: u64,
    pub failed: u64,
    pub first_error: Option<String>,
    pub elapsed: Duration,
    pub latencies_us: Vec<u32>,
    pub versions: BTreeMap<String, u64>,
    pub peak_rss_mib: Option<f64>,
}

impl LoadReport {
    pub fn percentile_ms(&self, p: f64) -> f64 {
        if self.latencies_us.is_empty() {
            return 0.0;
        }
        let index = ((self.latencies_us.len() - 1) as f64 * p).round() as usize;
        self.latencies_us[index] as f64 / 1000.0
    }

    pub fn print(&self) {
        let total = self.ok + self.failed;
        println!(
            "{total} requests in {:.1}s: {:.0} req/s, {} ok, {} failed",
            self.elapsed.as_secs_f64(),
            total as f64 / self.elapsed.as_secs_f64(),
            self.ok,
            self.failed
        );
        println!(
            "latency ms: p50 {:.2}, p95 {:.2}, p99 {:.2}, max {:.2}",
            self.percentile_ms(0.50),
            self.percentile_ms(0.95),
            self.percentile_ms(0.99),
            self.percentile_ms(1.0)
        );
        println!("client versions: {:?}", self.versions);
        if let Some(peak) = self.peak_rss_mib {
            println!("proxy peak rss: {peak:.0} MiB");
        }
        if let Some(e) = &self.first_error {
            println!("first error: {e}");
        }
    }
}

pub fn client_through_proxy(
    proxy: &str,
    proxy_ca: &Path,
    http1_only: bool,
) -> Result<reqwest::Client> {
    let ca = std::fs::read(proxy_ca).with_context(|| format!("read {}", proxy_ca.display()))?;
    let mut builder = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(proxy)?)
        .tls_certs_only([reqwest::Certificate::from_pem(&ca)?]);
    if http1_only {
        builder = builder.http1_only();
    }
    Ok(builder.build()?)
}

pub async fn run_load(opts: LoadOptions) -> Result<LoadReport> {
    let client = client_through_proxy(&opts.proxy, &opts.proxy_ca, opts.http1_only)?;
    let next = Arc::new(AtomicU64::new(0));
    let running = Arc::new(AtomicBool::new(true));

    let watcher = opts.watch_pid.map(|pid| {
        let running = Arc::clone(&running);
        tokio::spawn(async move {
            let pid = pid.to_string();
            let mut peak: Option<f64> = None;
            while running.load(Ordering::Relaxed) {
                if let Some(rss) = tracewing_proxy::peak_rss_mib_of(&pid) {
                    peak = Some(peak.map_or(rss, |p| p.max(rss)));
                }
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
            peak
        })
    });

    let started = Instant::now();
    let mut workers = Vec::with_capacity(opts.concurrency);
    for _ in 0..opts.concurrency {
        let (client, next, url, total) = (
            client.clone(),
            Arc::clone(&next),
            opts.url.clone(),
            opts.requests,
        );
        workers.push(tokio::spawn(async move {
            let mut latencies = Vec::new();
            let mut versions = BTreeMap::<String, u64>::new();
            let (mut failed, mut first_error) = (0u64, None);
            while next.fetch_add(1, Ordering::Relaxed) < total {
                let sent = Instant::now();
                let result = async {
                    let res = client.get(&url).send().await?.error_for_status()?;
                    let version = format!("{:?}", res.version());
                    res.bytes().await?;
                    Ok::<_, reqwest::Error>(version)
                }
                .await;
                match result {
                    Ok(version) => {
                        latencies.push(sent.elapsed().as_micros().min(u32::MAX as u128) as u32);
                        *versions.entry(version).or_default() += 1;
                    }
                    Err(e) => {
                        failed += 1;
                        first_error
                            .get_or_insert_with(|| tracewing_proxy::capture::error_chain(&e));
                    }
                }
            }
            (latencies, versions, failed, first_error)
        }));
    }

    let mut report = LoadReport {
        ok: 0,
        failed: 0,
        first_error: None,
        elapsed: Duration::ZERO,
        latencies_us: Vec::new(),
        versions: BTreeMap::new(),
        peak_rss_mib: None,
    };
    for worker in workers {
        let (latencies, versions, failed, first_error) = worker.await?;
        report.ok += latencies.len() as u64;
        report.latencies_us.extend(latencies);
        for (version, count) in versions {
            *report.versions.entry(version).or_default() += count;
        }
        report.failed += failed;
        if report.first_error.is_none() {
            report.first_error = first_error;
        }
    }
    report.elapsed = started.elapsed();
    report.latencies_us.sort_unstable();

    running.store(false, Ordering::Relaxed);
    if let Some(watcher) = watcher {
        report.peak_rss_mib = watcher.await?;
    }
    Ok(report)
}
