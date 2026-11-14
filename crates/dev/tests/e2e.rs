//! End to end: client → proxy (MITM) → HTTPS origin, all in process. The
//! client trusts only the proxy's CA, so a successful response means the
//! proxy terminated TLS itself.

use rusqlite::Connection;
use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};
use tokio_rustls::rustls::pki_types::{CertificateDer, pem::PemObject};
use tracewing_dev::benchkit;
use tracewing_proxy::{
    ProxyOptions, ProxySummary,
    capture::{CHECK_HOST, Check},
    lan::{self, OtherDevices},
    pause::{Paused, Pauses, Stage, Verdict},
    rules::HostRules,
    run_proxy,
    socks::Socks,
    source::{Flow, Given, Lan, Target},
    throttle::{Conditions, Network},
    traffic_rules::{Action, AskWith, TrafficRule, TrafficRules},
};
use tracewing_store::{
    search,
    session::{self, BodyReader, stored_body},
    settings::{Live, StorageSettings},
};

struct Harness {
    ws_port: u16,
    /// The playground, plain and over TLS, its authority trusted.
    playground_port: u16,
    playground_tls_port: u16,
    recording: Arc<AtomicBool>,
    rules: Arc<RwLock<HostRules>>,
    traffic_rules: Arc<RwLock<TrafficRules>>,
    check: Arc<Check>,
    pauses: Arc<Pauses>,
    network: Arc<Network>,
    other_devices: Arc<OtherDevices>,
    /// Where a connection taken on its way is put in, as a source that took
    /// it from the OS would.
    taken: tokio::sync::mpsc::Sender<Flow>,
    /// tracewing's SOCKS5 port, for a client that was told to use it.
    socks_port: u16,
    live: Arc<Live>,
    relay: Arc<tracewing_proxy::relay::Relay>,
    tls_keys: Arc<tracewing_proxy::key_log::TlsKeys>,
    sockets: Arc<tracewing_proxy::sockets::Sockets>,
    client_certs: Arc<tracewing_proxy::client_certs::ClientCerts>,
    scripts: Arc<tracewing_proxy::script::Scripts>,
    dir: PathBuf,
    proxy_url: String,
    origin: String,
    stop: oneshot::Sender<()>,
    proxy: JoinHandle<anyhow::Result<ProxySummary>>,
}

impl Harness {
    async fn start(name: &str) -> Self {
        Self::start_with(
            name,
            StorageSettings {
                log_max_kib: 1,
                ..StorageSettings::default()
            },
        )
        .await
    }

    async fn start_with(name: &str, mut settings: StorageSettings) -> Self {
        // What is under test is the writer, not how much room the machine has.
        if settings.disk_reserve_mib == StorageSettings::default().disk_reserve_mib {
            settings.disk_reserve_mib = 1;
        }
        let _ = tracewing_proxy::crypto_provider().install_default();
        let dir = std::env::temp_dir().join(format!("tracewing-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let origin_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin_port = origin_listener.local_addr().unwrap().port();
        let origin_ca = dir.join("origin-ca.pem");
        let ca_out = origin_ca.clone();
        tokio::spawn(async move { benchkit::serve_origin(origin_listener, &ca_out).await });
        while !origin_ca.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let ws_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let ws_port = ws_listener.local_addr().unwrap().port();
        let ws_ca = dir.join("ws-ca.pem");
        let ca_out = ws_ca.clone();
        tokio::spawn(async move { benchkit::serve_ws_origin(ws_listener, &ca_out).await });
        while !ws_ca.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let plain = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let playground_port = plain.local_addr().unwrap().port();
        tokio::spawn(tracewing_dev::playground::serve(plain));
        let state = dir.join("playground");
        let playground_ca = tracewing_dev::playground::authority(&state).unwrap();
        let secure = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let playground_tls_port = secure.local_addr().unwrap().port();
        tokio::spawn(async move { tracewing_dev::playground::serve_tls(secure, &state).await });

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_url = format!("http://{}", listener.local_addr().unwrap());
        let (stop, stopped) = oneshot::channel::<()>();
        let recording = Arc::new(AtomicBool::new(true));
        let rules = Arc::new(RwLock::new(HostRules {
            no_decrypt: Vec::new(),
        }));
        let traffic_rules = Arc::new(RwLock::new(TrafficRules::default()));
        let check = Arc::new(Check::default());
        let pauses = Arc::new(Pauses::default());
        let network = Arc::new(Network::default());
        let other_devices = Arc::new(OtherDevices::new(false));
        let live = Arc::new(Live::default());
        let relay = Arc::new(tracewing_proxy::relay::Relay::default());
        let tls_keys = Arc::new(tracewing_proxy::key_log::TlsKeys::default());
        let sockets = Arc::new(tracewing_proxy::sockets::Sockets::default());
        let client_certs = Arc::new(tracewing_proxy::client_certs::ClientCerts::default());
        let scripts = Arc::new(tracewing_proxy::script::Scripts::default());
        let (given, taken) = Given::channel();
        let socks = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let socks_port = socks.local_addr().unwrap().port();
        let proxy = tokio::spawn(run_proxy(
            ProxyOptions {
                listener: Some(listener),
                std_listener: None,
                data_dir: dir.join("ca"),
                key_store: Default::default(),
                session: dir.join("session"),
                settings,
                extra_roots: vec![origin_ca, ws_ca, playground_ca],
                queue: 10_000,
                stats_every: None,
                shared: Default::default(),
                recording: Arc::clone(&recording),
                rules: Arc::clone(&rules),
                traffic_rules: Arc::clone(&traffic_rules),
                network: Arc::clone(&network),
                check: Arc::clone(&check),
                leaf_spki: Default::default(),
                pauses: Arc::clone(&pauses),
                leave_index: false,
                find_apps: true,
                stop_limit: Duration::from_secs(1),
                live: Arc::clone(&live),
                sources: vec![
                    Box::new(Lan::new(Arc::clone(&other_devices))),
                    Box::new(given),
                    Box::new(Socks::new(socks)),
                ],
                relay: Arc::clone(&relay),
                tls_keys: Arc::clone(&tls_keys),
                sockets: Arc::clone(&sockets),
                client_certs: Arc::clone(&client_certs),
                scripts: Arc::clone(&scripts),
            },
            async {
                let _ = stopped.await;
            },
        ));
        while !dir.join("ca/ca.pem").exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        Self {
            ws_port,
            playground_port,
            playground_tls_port,
            recording,
            rules,
            traffic_rules,
            check,
            pauses,
            network,
            other_devices,
            taken,
            socks_port,
            live,
            dir,
            proxy_url,
            origin: format!("https://localhost:{origin_port}"),
            relay,
            tls_keys,
            sockets,
            client_certs,
            scripts,
            stop,
            proxy,
        }
    }

    fn client(&self, http1_only: bool) -> reqwest::Client {
        benchkit::client_through_proxy(&self.proxy_url, &self.dir.join("ca/ca.pem"), http1_only)
            .unwrap()
    }

    /// Waits until `flows` flows are written, then stops the proxy.
    ///
    /// A flow is submitted when the proxy drops its last body, which can be
    /// just after the client has read the response, so poll for it.
    async fn finish(self, flows: i64) -> (ProxySummary, Connection, PathBuf) {
        let db = self.dir.join("session/flows.db");
        while !db.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let conn = Connection::open(db).unwrap();
        // Longer than a request's body that no server reads is read on for
        // (10 s), as its flow is written once that ends.
        let deadline = Instant::now() + WAIT_FOR;
        loop {
            let count: i64 = conn
                .query_row("SELECT count(*) FROM flows", [], |r| r.get(0))
                .unwrap();
            if count >= flows || Instant::now() > deadline {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let _ = self.stop.send(());
        let summary = self.proxy.await.unwrap().unwrap();
        (summary, conn, self.dir)
    }
}

/// How long a poll waits for something the proxy does behind the request:
/// a row written, a body indexed, a socket closed.
///
/// Generous on purpose. These tests contend with each other for the machine
/// — sixty-odd of them, each with its own multi-threaded runtime — and a
/// margin that is comfortable on an idle machine is a coin toss under that
/// load. Nothing waits this long while it is working: every poll leaves as
/// soon as what it waits for arrives, so the cost of the margin is paid only
/// by a test that was going to fail anyway.
const WAIT_FOR: Duration = Duration::from_secs(60);

/// client_version, path, status, resp_bytes, resp_truncated
type FlowRow = (String, String, Option<i64>, i64, i64);

fn rows(conn: &Connection, sql: &str) -> Vec<FlowRow> {
    let mut stmt = conn.prepare(sql).unwrap();
    stmt.query_map([], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
    })
    .unwrap()
    .map(Result::unwrap)
    .collect()
}

const FLOW_COLUMNS: &str =
    "SELECT client_version, path, status, resp_bytes, resp_truncated FROM flows";

/// Windows will not delete a database that is still open.
fn clean_up(conn: Connection, dir: PathBuf) {
    drop(conn);
    std::fs::remove_dir_all(dir).unwrap();
}

/// What a viewer gets for the request of a flow.
fn request_of(conn: &Connection, dir: &std::path::Path, id: i64) -> Option<Vec<u8>> {
    let body = stored_body(conn, id, "req").unwrap();
    BodyReader::new(&dir.join("session")).unpacked(&body, usize::MAX)
}

/// The id of the one flow that fits `condition`. Flows sent one after another
/// are not always written in that order: a flow is written when the proxy lets
/// go of its last body, which on a slow machine can be after the next one's.
fn id_of(conn: &Connection, condition: &str) -> i64 {
    conn.query_row(
        &format!("SELECT id FROM flows WHERE {condition}"),
        [],
        |r| r.get(0),
    )
    .unwrap_or_else(|e| panic!("no one flow where {condition}: {e}"))
}

/// What a viewer gets for the response of a flow.
fn response_of(conn: &Connection, dir: &std::path::Path, id: i64) -> Option<Vec<u8>> {
    let body = stored_body(conn, id, "resp").unwrap();
    BodyReader::new(&dir.join("session")).unpacked(&body, usize::MAX)
}

#[tokio::test(flavor = "multi_thread")]
async fn intercepts_http1_and_http2() {
    let h = Harness::start("versions").await;

    for http1_only in [true, false] {
        let res = h
            .client(http1_only)
            .get(format!("{}/json?size=300", h.origin))
            .send()
            .await
            .unwrap();
        let expected = if http1_only {
            reqwest::Version::HTTP_11
        } else {
            reqwest::Version::HTTP_2
        };
        assert_eq!(res.version(), expected);
        assert_eq!(res.status(), 200);
        assert_eq!(res.bytes().await.unwrap().len(), 300);
    }

    let (summary, conn, dir) = h.finish(2).await;
    assert_eq!(summary.dropped, 0);
    let flows = rows(&conn, &format!("{FLOW_COLUMNS} ORDER BY id"));
    assert_eq!(flows.len(), 2, "{flows:?}");
    let mut versions: Vec<&str> = flows.iter().map(|f| f.0.as_str()).collect();
    versions.sort_unstable();
    assert_eq!(versions, ["HTTP/1.1", "HTTP/2.0"]);
    for (version, path, status, resp_bytes, truncated) in &flows {
        assert_eq!(path, "/json?size=300");
        assert_eq!(*status, Some(200));
        assert_eq!(*resp_bytes, 300);
        let id = id_of(&conn, &format!("client_version = '{version}'"));
        let body = response_of(&conn, &dir, id).unwrap();
        assert_eq!(body.len(), 300);
        assert!(body.starts_with(br#"{"data":"#));
        assert_eq!(*truncated, 0);
    }
    clean_up(conn, dir);
}

/// The TLS secrets of both connections a request goes over, from the app and
/// to the server, are written while it is switched on, and only then.
#[tokio::test(flavor = "multi_thread")]
async fn tls_keys_are_written_for_wireshark_while_on() {
    let h = Harness::start("tls-keys").await;
    let file = h.dir.join("keys/tls.log");
    let get = |n: u32| {
        let client = h.client(true);
        let url = format!("{}/json?size={n}", h.origin);
        async move {
            let res = client.get(url).send().await.unwrap();
            assert_eq!(res.status(), 200);
            res.bytes().await.unwrap();
        }
    };

    assert!(!file.exists(), "nothing while off");
    // On before the first request: after it, the connection to the server
    // is kept and used again, with no new secrets.
    h.tls_keys.start(&file).unwrap();
    get(20).await;
    let text = std::fs::read_to_string(&file).unwrap();
    // A line a secret: label, the client's random and the secret, in hex.
    let mut randoms = std::collections::BTreeSet::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split(' ').collect();
        assert_eq!(parts.len(), 3, "{line}");
        assert!(
            parts[1..]
                .iter()
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_hexdigit())),
            "{line}"
        );
        assert_eq!(parts[1].len(), 64, "a client random is 32 bytes: {line}");
        randoms.insert(parts[1].to_owned());
    }
    assert!(
        text.contains("CLIENT_TRAFFIC_SECRET_0 ") && text.contains("SERVER_TRAFFIC_SECRET_0 "),
        "{text}"
    );
    assert_eq!(
        randoms.len(),
        2,
        "the app's connection and the server's: {text}"
    );

    h.tls_keys.stop();
    get(30).await;
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        text,
        "nothing after"
    );

    let (_, conn, dir) = h.finish(2).await;
    clean_up(conn, dir);
}

/// A server that asks the client for a certificate gets the one set for its
/// host, and none before it is set.
#[tokio::test(flavor = "multi_thread")]
async fn a_server_that_asks_for_a_client_certificate_gets_it() {
    use tracewing_proxy::client_certs::{ClientCert, ClientCertSettings};
    let h = Harness::start("mtls").await;
    let state = h.dir.join("playground");
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let serving = state.clone();
    tokio::spawn(async move { tracewing_dev::playground::serve_mtls(listener, &serving).await });
    let get = || {
        let client = h.client(true);
        async move {
            client
                .get(format!("https://localhost:{port}/json"))
                .send()
                .await
                .unwrap()
        }
    };

    // None set: the server will not have the connection.
    let refused = get().await;
    assert_eq!(refused.status(), 502);

    let (cert, key) = tracewing_dev::playground::client_certificate(&state).unwrap();
    h.client_certs.set(&ClientCertSettings {
        certs: vec![ClientCert {
            host: "localhost".into(),
            cert,
            key: Some(key),
        }],
    });
    let res = get().await;
    assert_eq!(res.status(), 200);
    let shown: usize = res.headers()["x-client-certificate"]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!(shown > 0, "the server saw the certificate");
    assert!(res.text().await.unwrap().starts_with('{'), "the JSON");

    let (_, conn, dir) = h.finish(2).await;
    let errors: Vec<Option<String>> = {
        let mut stmt = conn.prepare("SELECT error FROM flows ORDER BY id").unwrap();
        stmt.query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    };
    let refusal = errors[0].as_deref().expect("the refusal is in the flow");
    assert!(
        refusal.contains("asks for a client certificate"),
        "and says what to do: {refusal}"
    );
    assert_eq!(errors[1], None);
    clean_up(conn, dir);
}

/// The test sends its requests itself, so the program behind them is this one.
#[tokio::test(flavor = "multi_thread")]
async fn each_flow_knows_the_program_that_sent_it() {
    let h = Harness::start("apps").await;
    // The clients live on, their connections open, as an app's do: a socket
    // closed before it is looked for has no owner left to find, which a busy
    // machine running the whole suite showed.
    let clients = [h.client(true), h.client(false)];
    for client in &clients {
        let res = client
            .get(format!("{}/json?size=10", h.origin))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 200);
        res.bytes().await.unwrap();
    }
    h.rules.write().unwrap().leave_alone("localhost");
    let origin_ca = h.dir.join("origin-ca.pem");
    let trusts_origin = benchkit::client_through_proxy(&h.proxy_url, &origin_ca, false).unwrap();
    let res = trusts_origin
        .get(format!("{}/json?size=10", h.origin))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let (_, conn, dir) = h.finish(3).await;
    drop((clients, trusts_origin));
    type AppRow = (String, Option<String>, Option<i64>, Option<String>);
    let apps: Vec<AppRow> = conn
        .prepare("SELECT kind, app, app_pid, app_path FROM flows ORDER BY id")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(apps.len(), 3, "{apps:?}");
    let exe = std::env::current_exe().unwrap();
    let name = exe.file_stem().unwrap().to_str().unwrap();
    for (kind, app, pid, path) in &apps {
        assert_eq!(*pid, Some(i64::from(std::process::id())), "{kind}");
        assert_eq!(app.as_deref(), Some(name), "{kind}");
        assert!(path.is_some(), "{kind}");
    }
    assert!(apps.iter().any(|(kind, ..)| kind == "tunnel"));
    clean_up(conn, dir);
}

/// What a server says to the next hop alone does not reach the client
/// either; the flow still shows what the server sent.
#[tokio::test(flavor = "multi_thread")]
async fn what_the_server_says_to_the_proxy_alone_goes_no_further() {
    let h = Harness::start("hop-back").await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(tracewing_dev::playground::serve(listener));
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::http(&h.proxy_url).unwrap())
        .build()
        .unwrap();
    let res = client
        .get(format!("http://localhost:{port}/hop"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let headers = res.headers().clone();
    res.bytes().await.unwrap();
    for gone in ["connection", "x-hop", "keep-alive", "proxy-authenticate"] {
        assert!(
            !headers.contains_key(gone),
            "{gone} reached the client: {headers:?}"
        );
    }
    assert_eq!(headers["x-end"], "1");

    let (_, conn, dir) = h.finish(1).await;
    let mut reader = BodyReader::new(&dir.join("session"));
    let (_, response) = session::stored_headers(&conn, &mut reader, 1)
        .unwrap()
        .expect("kept");
    drop(reader);
    assert!(
        response.iter().any(|(n, _)| n == "proxy-authenticate"),
        "the flow shows what the server sent: {response:?}"
    );
    clean_up(conn, dir);
}

/// A program that asks once and is gone as soon as it is answered, as curl
/// is, is found all the same, though the server answers at once.
#[tokio::test(flavor = "multi_thread")]
async fn a_program_gone_once_answered_is_found() {
    let h = Harness::start("quick-apps").await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(tracewing_dev::playground::serve(listener));
    let out = h.dir.join("curl-out");
    const CALLS: usize = 5;
    for _ in 0..CALLS {
        let ran = tokio::process::Command::new("curl")
            .args(["-s", "--noproxy", "", "-x", &h.proxy_url, "-o"])
            .arg(&out)
            .arg(format!("http://127.0.0.1:{port}/json"))
            .status()
            .await;
        match ran {
            Ok(status) => assert!(status.success(), "curl: {status}"),
            Err(e) => {
                eprintln!("no curl here ({e}): skipped");
                return;
            }
        }
    }
    let (_, conn, dir) = h.finish(CALLS as i64).await;
    let apps: Vec<Option<String>> = conn
        .prepare("SELECT app FROM flows ORDER BY id")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(apps, vec![Some("curl".to_owned()); CALLS]);
    clean_up(conn, dir);
}

/// Server-sent events must pass through as they are produced, not after the
/// whole response has been buffered.
#[tokio::test(flavor = "multi_thread")]
async fn streams_without_buffering() {
    let h = Harness::start("stream").await;
    // Connections and certificates are not what is being timed.
    let client = h.client(false);
    client
        .get(format!("{}/", h.origin))
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let started = Instant::now();
    let mut res = client
        .get(format!("{}/stream?events=4&gap_ms=300", h.origin))
        .send()
        .await
        .unwrap();

    let first = res.chunk().await.unwrap().unwrap();
    let first_at = started.elapsed();
    assert!(first.starts_with(b"data: event 0"));
    while res.chunk().await.unwrap().is_some() {}
    let total = started.elapsed();

    assert!(
        first_at < Duration::from_millis(300),
        "first event after {first_at:?}"
    );
    assert!(total >= Duration::from_millis(900), "total {total:?}");

    // The row is written as the stream opens, and its body when it ends, which
    // can be a moment after the client has read the last of it.
    let open = Connection::open(h.dir.join("session/flows.db")).unwrap();
    wait_for_row(&open, "sse", |row| !row.0).await;
    drop(open);
    let (_, conn, dir) = h.finish(2).await;
    let stream = id_of(&conn, "kind = 'sse'");
    let body = String::from_utf8(response_of(&conn, &dir, stream).unwrap()).unwrap();
    assert_eq!(body.matches("data: event").count(), 4);
    clean_up(conn, dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn reports_upstream_errors() {
    let h = Harness::start("error").await;
    // Nothing listens on this port.
    let port = {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let res = h
        .client(false)
        .get(format!("https://localhost:{port}/"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 502);

    let (_, conn, dir) = h.finish(1).await;
    let (status, error): (Option<i64>, Option<String>) = conn
        .query_row("SELECT status, error FROM flows", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!(status, None);
    assert!(error.is_some_and(|e| e.contains("onnect")));
    clean_up(conn, dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_refused_certificate_is_kept_with_the_error() {
    let h = Harness::start("refused").await;
    // An origin with a CA of its own, which the proxy does not trust.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let other_ca = h.dir.join("other-ca.pem");
    let ca_out = other_ca.clone();
    tokio::spawn(async move { benchkit::serve_origin(listener, &ca_out).await });
    while !other_ca.exists() {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let res = h
        .client(false)
        .get(format!("https://localhost:{port}/json"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 502);

    let (_, conn, dir) = h.finish(1).await;
    let (error, conn_id, segment, offset, len): (String, Option<i64>, i64, i64, i64) = conn
        .query_row(
            "SELECT error, conn_id, conn_segment, conn_offset, conn_len FROM flows",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .unwrap();
    assert!(error.contains("TLS with localhost"), "{error}");
    assert!(conn_id.is_some());
    let facts = BodyReader::new(&dir.join("session"))
        .read_place(session::LogPlace {
            segment: segment as u32,
            offset: offset as u64,
            len: len as u32,
        })
        .unwrap();
    let facts: serde_json::Value = serde_json::from_slice(&facts).unwrap();
    let tls = &facts["tls"];
    assert!(
        tls["error"].as_str().unwrap().contains("certificate"),
        "{tls}"
    );
    assert!(!tls["certificates"].as_array().unwrap().is_empty());
    assert!(
        facts["remote"]
            .as_str()
            .unwrap()
            .ends_with(&port.to_string())
    );
    clean_up(conn, dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn spools_whole_bodies_past_the_cap() {
    let h = Harness::start("spool").await;
    const SIZE: usize = 5 * 1024 * 1024;

    let client = h.client(false);
    let small = client
        .get(format!("{}/json?size=300", h.origin))
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    assert_eq!(small.len(), 300);
    let sent = client
        .get(format!("{}/json?size={SIZE}", h.origin))
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    assert_eq!(sent.len(), SIZE);

    let (summary, conn, dir) = h.finish(2).await;
    assert_eq!(summary.spool_lost, 0);
    assert_eq!(summary.spool_errors, 0);
    assert_eq!(summary.spooled_files, 1, "the small body goes in the log");

    let (name, lost): (String, i64) = conn
        .query_row(
            "SELECT resp_spool, resp_spool_lost FROM flows WHERE resp_bytes = ?1",
            [SIZE as i64],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(lost, 0);
    let kept = std::fs::read(dir.join("session/files").join(name)).unwrap();
    assert!(kept == sent, "the file holds exactly what the client got");
    let (large_id, small_id) = (
        id_of(&conn, &format!("resp_bytes = {SIZE}")),
        id_of(&conn, "resp_bytes = 300"),
    );
    assert!(
        response_of(&conn, &dir, large_id).unwrap() == sent,
        "and a viewer finds it"
    );
    assert_eq!(response_of(&conn, &dir, small_id).unwrap(), small);

    let none: Option<String> = conn
        .query_row(
            "SELECT resp_spool FROM flows WHERE resp_bytes = 300",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(none, None);
    clean_up(conn, dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_full_spool_never_touches_the_traffic() {
    // No room to speak of, so nothing of the body gets past what is in memory.
    let h = Harness::start_with(
        "spool-full",
        StorageSettings {
            log_max_kib: 1,
            spool_budget_mib: 0,
            ..StorageSettings::default()
        },
    )
    .await;
    const SIZE: usize = 2 * 1024 * 1024;
    let sent = h
        .client(false)
        .get(format!("{}/json?size={SIZE}", h.origin))
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    assert_eq!(
        sent.len(),
        SIZE,
        "the client gets the whole body regardless"
    );

    let (summary, conn, dir) = h.finish(1).await;
    assert_eq!(summary.spool_lost, 1);
    let lost: i64 = conn
        .query_row("SELECT resp_spool_lost FROM flows", [], |r| r.get(0))
        .unwrap();
    assert_eq!(lost, 1, "the flow says its body is not whole");
    let kept = response_of(&conn, &dir, 1).unwrap();
    assert_eq!(kept, sent[..1024], "and what memory held is still there");
    clean_up(conn, dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_index_follows_the_flows() {
    let h = Harness::start("search").await;
    let client = h.client(false);
    for i in 0..3 {
        client
            .get(format!("{}/json?size=300&mark=đặt-hàng-{i}", h.origin))
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
    }
    let (summary, _, dir) = h.finish(3).await;
    assert_eq!(summary.indexed, 3);
    let found = search::find(&dir.join("session"), "urls", "json?size=300", 1).unwrap();
    assert_eq!(found, [3, 2, 1]);
    std::fs::remove_dir_all(dir).unwrap();
}

/// A body sent packed is kept packed and found by its words.
#[tokio::test(flavor = "multi_thread")]
async fn packed_bodies_are_kept_as_sent_and_searched_as_text() {
    let h = Harness::start_with("packed", StorageSettings::default()).await;
    let client = h.client(false);
    let mut sent = Vec::new();
    for enc in ["gzip", "br", "zstd", "identity"] {
        let body = client
            .get(format!("{}/json?size=3000&enc={enc}", h.origin))
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        sent.push((enc, body));
    }
    let (summary, conn, dir) = h.finish(4).await;
    assert_eq!(summary.indexed, 4);
    let session_dir = dir.join("session");

    for (id, (enc, wire)) in (1i64..).zip(&sent) {
        let (len, ours, encoding): (i64, bool, Option<String>) = conn
            .query_row(
                "SELECT resp_len, resp_zstd, resp_encoding FROM flows WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        if *enc == "identity" {
            assert_eq!(encoding, None);
            assert!(ours, "plain text gets compressed");
            assert!((len as usize) < wire.len());
        } else {
            assert_eq!(encoding.as_deref(), Some(*enc));
            assert!(!ours, "packed already: kept as it came");
            assert_eq!(len as usize, wire.len());
        }
        let text = response_of(&conn, &dir, id).unwrap();
        assert!(text.len() >= 3000, "{enc}: {} bytes", text.len());
        assert!(text.starts_with(br#"{"id":1,"#), "{enc}");
    }
    let found = search::find(&session_dir, "bodies", "trace_id", 1).unwrap();
    assert_eq!(found, [4, 3, 2, 1]);
    clean_up(conn, dir);
}

/// The session stays inside its limit while traffic goes on, by losing the oldest
/// flows and giving their disk back.
#[tokio::test(flavor = "multi_thread")]
async fn a_full_session_drops_its_oldest_flows() {
    let h = Harness::start_with(
        "trim",
        StorageSettings {
            session_max_mib: 64,
            compress_stored: false,
            ..StorageSettings::default()
        },
    )
    .await;
    let client = h.client(false);
    // 240 MB in all, some in the log (under `log_max`, 64 KiB) and most in
    // files of their own.
    //
    // Few and large rather than many and small. What this test is for is a
    // session past its size dropping its oldest flows, not round trips, and
    // round trips are what cost: each one pays scheduling latency while the
    // rest of the suite is on the machine, and 1200 of them took 46 s
    // alongside the others against 2.6 s alone — the whole suite's wall
    // time, since every other test here is done inside five seconds.
    const FLOWS: usize = 120;
    for i in 0..FLOWS {
        let size = if i % 2 == 0 { 32_000 } else { 4_000_000 };
        let body = client
            .get(format!("{}/json?size={size}", h.origin))
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(body.len(), size, "the client gets it whole regardless");
    }
    // Waiting for the last flow is enough: rows go as segments do.
    let (summary, conn, dir) = h.finish(0).await;
    assert_eq!(summary.rows_lost, 0);
    assert!(summary.trimmed > 0, "{summary:?}");
    let session_dir = dir.join("session");
    let on_disk = dir_bytes(&session_dir);
    assert!(
        on_disk < 96 << 20,
        "the limit is 64 MiB and a little, found {on_disk}: {summary:?}"
    );
    let (first, last, count): (i64, i64, i64) = conn
        .query_row("SELECT min(id), max(id), count(*) FROM flows", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .unwrap();
    assert!(first > 1, "the oldest went");
    assert_eq!(count, FLOWS as i64 - summary.trimmed as i64);
    assert!(
        response_of(&conn, &dir, last).is_some(),
        "the newest is there"
    );
    let files = std::fs::read_dir(session::files_dir(&session_dir))
        .unwrap()
        .count();
    // Sixty bodies went to files of their own, and 64 MiB holds about
    // sixteen of them: the rest must have gone with their flows.
    assert!(
        files < 20,
        "files of dropped flows are gone too, {files} left"
    );
    clean_up(conn, dir);
}

fn dir_bytes(dir: &std::path::Path) -> u64 {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            let meta = entry.metadata().unwrap();
            if meta.is_dir() {
                dir_bytes(&entry.path())
            } else {
                meta.len()
            }
        })
        .sum()
}

/// One setting of each kind actually changes what is stored.
#[tokio::test(flavor = "multi_thread")]
async fn the_settings_are_obeyed() {
    let h = Harness::start_with(
        "settings",
        StorageSettings {
            log_max_kib: 1,
            body_max_mib: 1,
            compress_stored: false,
            index_bodies: false,
            ..StorageSettings::default()
        },
    )
    .await;
    let client = h.client(false);
    for size in [500, 3 << 20] {
        let body = client
            .get(format!("{}/json?size={size}", h.origin))
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(body.len(), size);
    }
    let (_, conn, dir) = h.finish(2).await;
    let (len, ours): (i64, bool) = conn
        .query_row(
            "SELECT resp_len, resp_zstd FROM flows WHERE resp_bytes = 500",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!((len, ours), (500, false), "compress_stored");

    let (lost, truncated): (bool, bool) = conn
        .query_row(
            "SELECT resp_spool_lost, resp_truncated FROM flows WHERE resp_bytes > 500",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert!(lost && truncated, "body_max_mib");
    let large = id_of(&conn, "resp_bytes > 500");
    let kept = response_of(&conn, &dir, large).unwrap().len();
    assert!(kept <= 1 << 20 && kept > 0, "body_max_mib, kept {kept}");

    let session_dir = dir.join("session");
    assert_eq!(
        search::find(&session_dir, "urls", "size=500", 1).unwrap(),
        [id_of(&conn, "resp_bytes = 500")]
    );
    assert!(
        search::find(&session_dir, "bodies", "data", 1).is_err(),
        "index_bodies: there is no such table"
    );
    clean_up(conn, dir);
}

/// Short of disk, flows are still listed and say their bodies were not kept.
#[tokio::test(flavor = "multi_thread")]
async fn a_low_disk_keeps_the_rows_and_not_the_bodies() {
    let h = Harness::start_with(
        "low-disk",
        StorageSettings {
            log_max_kib: 1,
            // More than any disk has.
            disk_reserve_mib: 1 << 40,
            ..StorageSettings::default()
        },
    )
    .await;
    let client = h.client(false);
    for size in [500, 5000] {
        let body = client
            .get(format!("{}/json?size={size}", h.origin))
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(body.len(), size, "the client gets it whole regardless");
    }
    let (summary, conn, dir) = h.finish(2).await;
    assert_eq!(summary.written, 2);
    assert_eq!(summary.rows_lost, 0);
    assert!(summary.shed_low_disk >= 1, "{summary:?}");
    assert_eq!(summary.spool_lost, 1, "{summary:?}");
    let (shed, lost): (i64, i64) = conn
        .query_row(
            "SELECT sum(resp_shed), sum(resp_spool_lost) FROM flows",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!((shed, lost), (2, 1), "both say so");
    assert_eq!(response_of(&conn, &dir, 1), None);
    clean_up(conn, dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_full_queue_sheds_bodies_and_keeps_the_rows() {
    let h = Harness::start_with(
        "shed",
        StorageSettings {
            queue_budget_mib: 0,
            ..StorageSettings::default()
        },
    )
    .await;
    let sent = h
        .client(false)
        .get(format!("{}/json?size=500", h.origin))
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    assert_eq!(sent.len(), 500, "the client gets the whole body regardless");

    let (summary, conn, dir) = h.finish(1).await;
    assert_eq!(summary.shed, 1);
    assert_eq!(summary.dropped, 0);
    let (bytes, shed): (i64, i64) = conn
        .query_row("SELECT resp_bytes, resp_shed FROM flows", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!(bytes, 500, "the row still knows the size");
    assert_eq!(response_of(&conn, &dir, 1), None);
    assert_eq!(shed, 1, "and says the body was let go");
    clean_up(conn, dir);
}

/// gRPC's trailers come after the body over HTTP/2: they are kept with the
/// headers, and the status they hold goes in the row.
#[tokio::test(flavor = "multi_thread")]
async fn grpc_trailers_and_status_are_kept() {
    let h = Harness::start("grpc").await;
    let res = h
        .client(false)
        .post(format!("{}/shop.Orders/Get", h.origin))
        .header("content-type", "application/grpc")
        .header("te", "trailers")
        .body(vec![0u8, 0, 0, 0, 0])
        .send()
        .await
        .unwrap();
    assert_eq!(res.version(), reqwest::Version::HTTP_2);
    res.bytes().await.unwrap();
    let (_, conn, dir) = h.finish(1).await;
    let status: Option<u16> = conn
        .query_row("SELECT grpc_status FROM flows WHERE id = 1", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(status, Some(5));
    let mut reader = BodyReader::new(&dir.join("session"));
    let head = session::stored_head(&conn, &mut reader, 1)
        .unwrap()
        .expect("kept");
    drop(reader);
    assert!(
        head.trailers
            .iter()
            .any(|(n, v)| n == "grpc-message" && v == "order%2042%20is%20gone"),
        "{head:?}"
    );
    assert!(
        head.response.iter().all(|(n, _)| n != "grpc-status"),
        "trailers are not headers: {head:?}"
    );
    clean_up(conn, dir);
}

/// A JSON-RPC call through the proxy: the method named in the row, the other
/// calls of a batch counted, and a reverted call marked although it answered
/// 200. The bodies are real traffic: see `crates/store/testdata/jsonrpc`.
#[tokio::test(flavor = "multi_thread")]
async fn json_rpc_calls_are_named_and_their_failures_marked() {
    let h = Harness::start("jsonrpc").await;
    let call = concat!(
        r#"{"jsonrpc":"2.0","id":5,"method":"eth_call","params":"#,
        r#"[{"to":"0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48","#,
        r#""data":"0x70a08231000000000000000000000000d8da6bf26964af9d7eed9e03e53415d37aa96045"},"#,
        r#""latest"]}"#
    );
    let batch = concat!(
        r#"[{"jsonrpc":"2.0","id":1,"method":"eth_chainId"},"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"eth_blockNumber"},"#,
        r#"{"jsonrpc":"2.0","id":3,"method":"net_version"}]"#
    );
    for (path, body) in [("/rpc", call), ("/rpc?fail=1", call), ("/rpc", batch)] {
        h.client(false)
            .post(format!("{}{path}", h.origin))
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
    }
    let (_, conn, dir) = h.finish(3).await;
    let about = |id: i64| -> (Option<String>, Option<i64>, Option<u16>) {
        conn.query_row(
            "SELECT rpc, rpc_error, status FROM flows WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap()
    };
    let plain = about(id_of(&conn, "rpc = 'eth_call' AND rpc_error IS NULL"));
    assert_eq!(plain.0.as_deref(), Some("eth_call"));
    assert_eq!(plain.2, Some(200));
    let reverted = about(id_of(&conn, "rpc_error IS NOT NULL"));
    assert_eq!(reverted.0.as_deref(), Some("eth_call"));
    assert_eq!(
        (reverted.1, reverted.2),
        (Some(3), Some(200)),
        "a reverted call answers 200 and says so in the body"
    );
    let batched = about(id_of(&conn, "rpc LIKE '%+%'"));
    assert_eq!(
        batched.0.as_deref(),
        Some("eth_chainId +2"),
        "the first call, and how many followed it"
    );
    assert_eq!(batched.1, None, "every call of it went through");
    clean_up(conn, dir);
}

/// Headers come back in the inspector, from the log rather than the row.
#[tokio::test(flavor = "multi_thread")]
async fn headers_are_kept() {
    let h = Harness::start("headers").await;
    h.client(false)
        .post(format!("{}/json?size=30", h.origin))
        .header("x-trace", "đặt-hàng".as_bytes())
        .body("{}")
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let (_, conn, dir) = h.finish(1).await;
    let mut reader = BodyReader::new(&dir.join("session"));
    let (request, response) = session::stored_headers(&conn, &mut reader, 1)
        .unwrap()
        .expect("kept");
    drop(reader);
    assert!(
        request
            .iter()
            .any(|(n, v)| n == "x-trace" && v == "đặt-hàng"),
        "{request:?}"
    );
    assert!(
        response
            .iter()
            .any(|(n, v)| n == "content-type" && v == "application/json"),
        "{response:?}"
    );
    clean_up(conn, dir);
}

/// Apps are not told of HTTP/3, which would take them around the proxy; the
/// flow still shows what the server offered.
#[tokio::test(flavor = "multi_thread")]
async fn offers_of_http3_do_not_reach_the_client() {
    let h = Harness::start("http3").await;
    let client = h.client(false);
    let mut seen = Vec::new();
    for path in ["/alt-svc", "/alt-svc?only=h3"] {
        let res = client
            .get(format!("{}{path}", h.origin))
            .send()
            .await
            .unwrap();
        let values: Vec<String> = res
            .headers()
            .get_all("alt-svc")
            .iter()
            .map(|v| v.to_str().unwrap().to_owned())
            .collect();
        seen.push(values);
        res.bytes().await.unwrap();
    }
    assert_eq!(
        seen,
        [
            vec![r#"h2="alt.example:443"; ma=60"#.to_owned()],
            vec!["clear".to_owned()],
        ]
    );
    let (_, conn, dir) = h.finish(2).await;
    let mut reader = BodyReader::new(&dir.join("session"));
    let offered = id_of(&conn, "path = '/alt-svc'");
    let (_, response) = session::stored_headers(&conn, &mut reader, offered)
        .unwrap()
        .expect("kept");
    drop(reader);
    assert!(
        response
            .iter()
            .any(|(n, v)| n == "alt-svc" && v.starts_with(r#"h3=":443""#)),
        "{response:?}"
    );
    clean_up(conn, dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn offers_of_http3_can_be_let_through() {
    let h = Harness::start_with(
        "http3-kept",
        StorageSettings {
            hide_http3: false,
            ..StorageSettings::default()
        },
    )
    .await;
    let res = h
        .client(false)
        .get(format!("{}/alt-svc?only=h3", h.origin))
        .send()
        .await
        .unwrap();
    assert_eq!(
        res.headers()["alt-svc"],
        r#"h3=":443"; ma=86400, h3-29=":443""#
    );
    res.bytes().await.unwrap();
    let (_, conn, dir) = h.finish(1).await;
    clean_up(conn, dir);
}

/// Some settings apply to the next request, with no restart: whether offers
/// of HTTP/3 are hidden, and how much of a body is kept.
#[tokio::test(flavor = "multi_thread")]
async fn some_settings_apply_while_it_runs() {
    let h = Harness::start("live").await;
    const SIZE: usize = 2 * 1024 * 1024;
    let client = h.client(false);
    let offer = || async {
        let res = client
            .get(format!("{}/alt-svc?only=h3", h.origin))
            .send()
            .await
            .unwrap();
        let offer = res.headers()["alt-svc"].to_str().unwrap().to_owned();
        res.bytes().await.unwrap();
        offer
    };
    let large = || async {
        let res = client
            .get(format!("{}/json?size={SIZE}", h.origin))
            .send()
            .await
            .unwrap();
        assert_eq!(
            res.bytes().await.unwrap().len(),
            SIZE,
            "the client gets it all"
        );
    };
    assert_eq!(offer().await, "clear");
    large().await;

    h.live.apply(&StorageSettings {
        hide_http3: false,
        body_max_mib: 1,
        ..StorageSettings::default()
    });
    assert!(offer().await.starts_with(r#"h3=":443""#));
    large().await;

    let (_, conn, dir) = h.finish(4).await;
    let lost: Vec<i64> = conn
        .prepare(&format!(
            "SELECT resp_spool_lost FROM flows WHERE resp_bytes = {SIZE} ORDER BY id"
        ))
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(lost, [0, 1], "kept whole, then past the new limit");
    clean_up(conn, dir);
}

/// Paused, traffic goes through and nothing is written down.
#[tokio::test(flavor = "multi_thread")]
async fn a_pause_records_nothing_and_lets_traffic_through() {
    let h = Harness::start("pause").await;
    let client = h.client(false);
    let get = |size: u32| {
        let (client, origin) = (client.clone(), h.origin.clone());
        async move {
            client
                .get(format!("{origin}/json?size={size}"))
                .send()
                .await
                .unwrap()
                .bytes()
                .await
                .unwrap()
                .len()
        }
    };
    assert_eq!(get(100).await, 100);
    h.recording.store(false, Ordering::Relaxed);
    assert_eq!(get(200).await, 200);
    h.recording.store(true, Ordering::Relaxed);
    assert_eq!(get(300).await, 300);
    let (summary, conn, dir) = h.finish(2).await;
    assert_eq!(summary.written, 2);
    let sizes: Vec<i64> = conn
        .prepare("SELECT resp_bytes FROM flows ORDER BY resp_bytes")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(sizes, [100, 300]);
    clean_up(conn, dir);
}

/// A stream that goes on must not keep the app from closing.
#[tokio::test(flavor = "multi_thread")]
async fn an_open_stream_does_not_hold_up_a_stop() {
    let h = Harness::start("open-stream").await;
    let mut res = h
        .client(false)
        .get(format!("{}/stream?events=100000&gap_ms=100", h.origin))
        .send()
        .await
        .unwrap();
    assert!(res.chunk().await.unwrap().is_some(), "it has started");
    let started = Instant::now();
    let (_, conn, dir) = h.finish(0).await;
    assert!(
        // It must not hang on the open stream; a hang is far past this,
        // and the margin is for a machine the rest of the suite is on.
        started.elapsed() < Duration::from_secs(10),
        "took {:?}",
        started.elapsed()
    );
    drop(res);
    clean_up(conn, dir);
}

/// A host left alone reaches the client with its own certificate, so a client that
/// trusts only the origin's CA gets through; decrypted, it would not.
#[tokio::test(flavor = "multi_thread")]
async fn a_host_left_alone_is_not_decrypted() {
    let h = Harness::start("left-alone").await;
    let origin_ca = h.dir.join("origin-ca.pem");
    let trusts_origin = benchkit::client_through_proxy(&h.proxy_url, &origin_ca, false).unwrap();

    // Decrypted first: the proxy's certificate is not one this client trusts.
    let refused = trusts_origin
        .get(format!("{}/json?size=30", h.origin))
        .send()
        .await;
    assert!(refused.is_err(), "the proxy decrypted it");

    h.rules.write().unwrap().leave_alone("localhost");
    let trusts_origin = benchkit::client_through_proxy(&h.proxy_url, &origin_ca, false).unwrap();
    let body = trusts_origin
        .get(format!("{}/json?size=30", h.origin))
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    assert_eq!(body.len(), 30, "it reached the client as it was");

    // And back: the next connection is decrypted again.
    h.rules.write().unwrap().decrypt("localhost");
    let body = h
        .client(false)
        .get(format!("{}/json?size=20", h.origin))
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    assert_eq!(body.len(), 20);

    let (_, conn, dir) = h.finish(2).await;
    let tunnels: Vec<(String, String, String)> = conn
        .prepare("SELECT method, host, path FROM flows WHERE kind = 'tunnel'")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(tunnels.len(), 1, "{tunnels:?}");
    assert_eq!(tunnels[0].0, "CONNECT");
    assert!(tunnels[0].1.starts_with("localhost"), "{tunnels:?}");
    assert_eq!(tunnels[0].2, "", "where it went is all that is known");
    let decrypted: i64 = conn
        .query_row("SELECT count(*) FROM flows WHERE kind = 'http'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(decrypted, 1, "only the last request was read");
    clean_up(conn, dir);
}

/// The test page has no server: the proxy answers, and says who came.
#[tokio::test(flavor = "multi_thread")]
async fn the_test_page_says_who_reached_it() {
    let h = Harness::start("check").await;
    assert_eq!(h.check.last(), None);
    let page = h
        .client(false)
        .get(format!("https://{CHECK_HOST}/"))
        .header("user-agent", "Firefox/144.0")
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("It works"), "{page}");
    let visit = h.check.last().expect("seen");
    assert_eq!(visit.user_agent, "Firefox/144.0");
    assert!(visit.secure);

    let plain = h
        .client(true)
        .get(format!("http://{CHECK_HOST}/"))
        .send()
        .await
        .unwrap();
    assert_eq!(plain.status(), 200);
    assert!(!h.check.last().expect("seen").secure);

    // A phone gets the certificate there, before it trusts it: over plain HTTP.
    let ca = h
        .client(true)
        .get(format!("http://{CHECK_HOST}/ca.crt"))
        .send()
        .await
        .unwrap();
    assert_eq!(ca.status(), 200);
    assert_eq!(
        ca.headers()["content-type"],
        "application/x-x509-ca-cert",
        "phones install it by its type"
    );
    let der = ca.bytes().await.unwrap();
    let pem = std::fs::read(h.dir.join("ca/ca.pem")).unwrap();
    let expected = CertificateDer::from_pem_slice(&pem).unwrap();
    assert_eq!(&der[..], expected.as_ref());

    let (summary, conn, dir) = h.finish(0).await;
    assert_eq!(summary.written, 0, "the test is not traffic");
    clean_up(conn, dir);
}

/// A phone that scanned the Set up page's code opens the proxy's own address,
/// before its proxy is set: it is told how, and gets the certificate there.
#[tokio::test(flavor = "multi_thread")]
async fn the_proxy_itself_tells_a_device_how_to_set_up() {
    let h = Harness::start("direct").await;
    let direct = reqwest::Client::builder().no_proxy().build().unwrap();
    let addr = h.proxy_url.trim_start_matches("http://").to_owned();
    let (host, port) = addr.rsplit_once(':').unwrap();
    let page = direct
        .get(format!("{}/", h.proxy_url))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("Set up this device"), "{page}");
    assert!(
        page.contains(&format!("server <b>{host}</b>, port <b>{port}</b>")),
        "{page}"
    );
    let ca = direct
        .get(format!("{}/ca.crt", h.proxy_url))
        .send()
        .await
        .unwrap();
    assert_eq!(ca.headers()["content-type"], "application/x-x509-ca-cert");
    let der = ca.bytes().await.unwrap();
    let pem = std::fs::read(h.dir.join("ca/ca.pem")).unwrap();
    assert_eq!(
        &der[..],
        CertificateDer::from_pem_slice(&pem).unwrap().as_ref()
    );
    // Not through the proxy: the check is still to come.
    assert_eq!(h.check.last(), None);

    let (summary, conn, dir) = h.finish(0).await;
    assert_eq!(summary.written, 0);
    clean_up(conn, dir);
}

/// Waits for the proxy to listen for other devices where it is asked to, or to
/// stop.
async fn wait_for_listening(devices: &OtherDevices, there: SocketAddr, listening: bool) {
    let deadline = Instant::now() + WAIT_FOR;
    while devices.listening().contains(&there) != listening {
        assert!(
            Instant::now() < deadline,
            "listening at {:?}, problem {:?}",
            devices.listening(),
            devices.problem()
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// Other devices are let in, and out again, while the proxy runs: a phone
/// in the middle of something keeps its connection, and a new one is turned
/// away.
#[tokio::test(flavor = "multi_thread")]
async fn other_devices_come_and_go_without_a_restart() {
    let h = Harness::start("lan").await;
    let Some(ip) = lan::addresses().first().copied() else {
        eprintln!("this machine is on no network: nothing to test");
        let (_, conn, dir) = h.finish(0).await;
        clean_up(conn, dir);
        return;
    };
    let port: u16 = h.proxy_url.rsplit_once(':').unwrap().1.parse().unwrap();
    let there = SocketAddr::from((ip, port));
    let reached = || async move {
        tokio::time::timeout(
            Duration::from_secs(3),
            tokio::net::TcpStream::connect(there),
        )
        .await
        .is_ok_and(|connected| connected.is_ok())
    };
    assert!(!reached().await, "off, nothing listens at {there}");

    h.other_devices.set(true);
    wait_for_listening(&h.other_devices, there, true).await;
    let phone =
        benchkit::client_through_proxy(&format!("http://{there}"), &h.dir.join("ca/ca.pem"), true)
            .unwrap();
    let res = phone
        .get(format!("{}/json?size=10", h.origin))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    res.bytes().await.unwrap();

    h.other_devices.set(false);
    wait_for_listening(&h.other_devices, there, false).await;
    assert!(
        !reached().await,
        "off again, {there} turns new devices away"
    );
    // Its tunnel is still open, and the pool sends the next request down it.
    let res = phone
        .get(format!("{}/json?size=20", h.origin))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    res.bytes().await.unwrap();

    let (summary, conn, dir) = h.finish(2).await;
    assert_eq!(summary.written, 2);
    let from: Vec<String> = conn
        .prepare("SELECT client FROM flows ORDER BY id")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(from.len(), 2, "{from:?}");
    // From this machine, though not always from the address it went to: the
    // system picks the one it sends from.
    let ours = lan::addresses();
    for addr in &from {
        assert!(
            ours.iter().any(|ip| addr.starts_with(&format!("{ip}:"))),
            "{addr} is not one of {ours:?}"
        );
    }
    clean_up(conn, dir);
}

/// A session cut off by a crash can still be read, and its index finished.
#[test]
fn a_session_survives_being_killed() {
    let dir = std::env::temp_dir().join(format!("tracewing-killed-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let session_dir = dir.join("session");
    let mut seeding = std::process::Command::new(env!("CARGO_BIN_EXE_tracewing-dev"))
        .args(["seed", "--rich", "--rows", "100000000", "--session"])
        .arg(&session_dir)
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let body_written = session_dir.join("bodies/00000001.log");
    let deadline = Instant::now() + WAIT_FOR;
    while std::fs::metadata(&body_written).map_or(0, |m| m.len()) < 20 << 20 {
        assert!(Instant::now() < deadline, "nothing written");
        std::thread::sleep(Duration::from_millis(20));
    }
    seeding.kill().unwrap();
    seeding.wait().unwrap();

    let about = session::info(&session_dir);
    assert!(about.readable);
    assert!(about.flows > 0, "{about:?}");
    let settings = StorageSettings::default();
    let indexer = search::Indexer::catch_up(&session_dir, &settings).unwrap();
    let stats = indexer.stats();
    indexer.finish().unwrap();
    assert_eq!(stats.indexed.load(Ordering::Relaxed), about.flows);
    assert_eq!(stats.errors.load(Ordering::Relaxed), 0);

    let conn = Connection::open(session_dir.join("flows.db")).unwrap();
    let mut reader = BodyReader::new(&session_dir);
    let last = about.flows as i64;
    let body = stored_body(&conn, last, "resp").unwrap();
    let text = reader
        .unpacked(&body, usize::MAX)
        .expect("the last row's body was flushed before it");
    assert!(text.starts_with(b"{\"id\":"), "{:?}", &text[..20]);
    drop(conn);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The stream's row, once the writer has got there: open, status, messages so far.
fn stream_row(conn: &Connection, kind: &str) -> Option<(bool, Option<i64>, i64)> {
    conn.query_row(
        "SELECT open, status, (SELECT count(*) FROM messages WHERE flow_id = flows.id)
         FROM flows WHERE kind = ?1",
        [kind],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
    .ok()
}

async fn wait_for_row(
    conn: &Connection,
    kind: &str,
    done: impl Fn(&(bool, Option<i64>, i64)) -> bool,
) -> (bool, Option<i64>, i64) {
    let deadline = Instant::now() + WAIT_FOR;
    loop {
        let row = stream_row(conn, kind);
        if let Some(row) = row.filter(&done) {
            return row;
        }
        assert!(
            Instant::now() < deadline,
            "{kind}: {:?}",
            stream_row(conn, kind)
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// A stream opened by a request with a body, as a streamed call to an LLM is,
/// leaves the bodies of the flows after it alone.
#[tokio::test(flavor = "multi_thread")]
async fn a_posted_stream_keeps_the_bodies_after_it() {
    let h = Harness::start("posted-stream").await;
    let client = h.client(false);
    let mut stream = client
        .post(format!("{}/stream?events=2&gap_ms=10", h.origin))
        .header("content-type", "application/json")
        .body(r#"{"model":"m","stream":true,"messages":[{"role":"user","content":"hi"}]}"#)
        .send()
        .await
        .unwrap();
    while stream.chunk().await.unwrap().is_some() {}
    client
        .get(format!("{}/json?size=40", h.origin))
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let (summary, conn, dir) = h.finish(2).await;
    let after = response_of(&conn, &dir, id_of(&conn, "path = '/json?size=40'"))
        .expect("the body after the stream was kept");
    assert_eq!(after.len(), 40);
    let shed: i64 = conn
        .query_row(
            "SELECT count(*) FROM flows WHERE req_shed OR resp_shed",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(shed, 0, "{summary:?}");
    clean_up(conn, dir);
}

/// A slow network slows what it applies to, both ways, and the flows it slowed
/// say so.
#[tokio::test(flavor = "multi_thread")]
async fn a_slow_network_slows_what_it_applies_to() {
    let h = Harness::start("slow").await;
    let client = h.client(false);
    // The connection and its TLS are made first, and not timed.
    client
        .get(format!("{}/", h.origin))
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    h.network.set(Conditions {
        on: true,
        name: "Test".to_owned(),
        latency_ms: 200,
        down_kbps: 800,
        up_kbps: 400,
        ..Conditions::default()
    });

    // 100 KB down at 100 KB a second, and a round trip of 200 ms more.
    let started = Instant::now();
    let body = client
        .get(format!("{}/json?size=100000", h.origin))
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let down = started.elapsed();
    assert_eq!(body.len(), 100_000);
    // That it was slowed is the lower bound's to say. The upper one only
    // says it was not slowed absurdly, so it can afford the margin that a
    // loaded machine needs.
    assert!(down >= Duration::from_millis(1150), "down in {down:?}");
    assert!(down < Duration::from_secs(12), "down in {down:?}");

    // 25 KB up at 50 KB a second, to a server that reads it all first.
    let started = Instant::now();
    let got = client
        .post(format!("{}/upload", h.origin))
        .body(vec![b'x'; 25_000])
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let up = started.elapsed();
    assert_eq!(got, "25000");
    assert!(up >= Duration::from_millis(650), "up in {up:?}");

    // Every slice lost: each waits to be sent again, with no other limit.
    h.network.set(Conditions {
        on: true,
        name: "Test".to_owned(),
        loss_percent: 100,
        ..Conditions::default()
    });
    let started = Instant::now();
    client
        .get(format!("{}/json?size=50000", h.origin))
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let lossy = started.elapsed();
    assert!(
        lossy >= Duration::from_millis(600),
        "four slices, in {lossy:?}"
    );

    // Only for another host: this one is fast again.
    h.network.set(Conditions {
        hosts: vec!["elsewhere.test".to_owned()],
        ..h.network.conditions()
    });
    let started = Instant::now();
    client
        .get(format!("{}/json?size=100000", h.origin))
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let fast = started.elapsed();
    assert!(fast < Duration::from_millis(600), "not slowed, in {fast:?}");

    let (_, conn, dir) = h.finish(5).await;
    let slowed: i64 = conn
        .query_row(
            "SELECT count(*) FROM flows WHERE rule = 'Test network'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(slowed, 3);
    clean_up(conn, dir);
}

/// Server-sent events show as they come, not when the stream ends.
#[tokio::test(flavor = "multi_thread")]
async fn events_show_while_the_stream_lasts() {
    const EVENTS: i64 = 20;
    let h = Harness::start("events").await;
    let session_dir = h.dir.join("session");
    let mut res = h
        .client(false)
        .get(format!("{}/stream?events={EVENTS}&gap_ms=100", h.origin))
        .send()
        .await
        .unwrap();
    assert!(res.chunk().await.unwrap().is_some());
    let conn = Connection::open(session_dir.join("flows.db")).unwrap();
    // Long enough to be seen open on a slow runner: 4 events in 0.6 s once
    // ended before Windows CI looked.
    let row = wait_for_row(&conn, "sse", |row| row.0 && row.2 >= 1).await;
    assert!(
        row.2 < EVENTS,
        "it is listed while it is still going: {row:?}"
    );
    assert_eq!(row.1, Some(200));
    while res.chunk().await.unwrap().is_some() {}
    drop(conn);

    let (_, conn, dir) = h.finish(1).await;
    let row = wait_for_row(&conn, "sse", |row| !row.0).await;
    assert_eq!(row.2, EVENTS, "{row:?}");
    let (id, bytes): (i64, i64) = conn
        .query_row(
            "SELECT id, resp_bytes FROM flows WHERE kind = 'sse'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(id, 1, "written again in the same place");
    assert!(bytes > 0, "the whole body is known at the end");
    assert_eq!(
        response_of(&conn, &dir, 1).unwrap().len() as i64,
        bytes,
        "and kept as well"
    );
    let messages = session::stored_messages(&conn, 1, 0, 100).unwrap();
    assert_eq!(messages.len() as i64, EVENTS);
    let mut reader = BodyReader::new(&dir.join("session"));
    for (i, message) in messages.iter().enumerate() {
        assert_eq!(message.kind, "event");
        assert!(!message.from_client);
        let text = reader.read_place(message.place.unwrap()).unwrap();
        assert_eq!(text, format!("data: event {i}").as_bytes());
    }
    drop(reader);
    clean_up(conn, dir);
}

/// JSON a line, as Ollama and many APIs stream it: listed while it lasts, a
/// message a line however the frames cut them, and searched once over.
#[tokio::test(flavor = "multi_thread")]
async fn json_lines_show_while_the_stream_lasts() {
    const LINES: i64 = 20;
    let h = Harness::start_with("ndjson", StorageSettings::default()).await;
    let session_dir = h.dir.join("session");
    let mut res = h
        .client(false)
        .get(format!(
            "{}/stream?format=ndjson&events={LINES}&gap_ms=100",
            h.origin
        ))
        .send()
        .await
        .unwrap();
    assert!(res.chunk().await.unwrap().is_some());
    let conn = Connection::open(session_dir.join("flows.db")).unwrap();
    let row = wait_for_row(&conn, "ndjson", |row| row.0 && row.2 >= 1).await;
    assert!(
        row.2 < LINES,
        "it is listed while it is still going: {row:?}"
    );
    while res.chunk().await.unwrap().is_some() {}
    drop(conn);

    let (_, conn, dir) = h.finish(1).await;
    let row = wait_for_row(&conn, "ndjson", |row| !row.0).await;
    assert_eq!(row.2, LINES, "{row:?}");
    let messages = session::stored_messages(&conn, 1, 0, 100).unwrap();
    let mut reader = BodyReader::new(&dir.join("session"));
    for (i, message) in messages.iter().enumerate() {
        assert_eq!(message.kind, "line");
        let text = reader.read_place(message.place.unwrap()).unwrap();
        assert_eq!(text, format!(r#"{{"n":{i}}}"#).as_bytes());
    }
    drop(reader);
    let found = search::find(&dir.join("session"), "bodies", "n", 1).unwrap();
    assert_eq!(found, [1], "its body is searched");
    clean_up(conn, dir);
}

/// A stream that comes packed is unpacked as it comes, and cut as one that
/// did not: its events show as they arrive, its body is kept as it came.
#[tokio::test(flavor = "multi_thread")]
async fn packed_streams_show_as_they_come() {
    const EVENTS: i64 = 12;
    let h = Harness::start("packed-streams").await;
    let session_dir = h.dir.join("session");
    let conn = loop {
        if let Ok(conn) = Connection::open(session_dir.join("flows.db")) {
            break conn;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    };
    // A client that does not unpack, so that what it gets is what was sent.
    let client = h.client(false);
    for (at, enc) in ["gzip", "br", "zstd"].into_iter().enumerate() {
        let mut res = client
            .get(format!(
                "{}/stream?events={EVENTS}&gap_ms=100&enc={enc}",
                h.origin
            ))
            .header("accept-encoding", enc)
            .send()
            .await
            .unwrap();
        assert_eq!(res.headers()["content-encoding"], enc, "left packed");
        let id = at as i64 + 1;
        let open = |conn: &Connection| -> Option<(bool, i64)> {
            conn.query_row(
                "SELECT open, (SELECT count(*) FROM messages WHERE flow_id = ?1) FROM flows WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok()
        };
        let deadline = Instant::now() + WAIT_FOR;
        loop {
            if open(&conn).is_some_and(|(open, messages)| open && messages >= 1) {
                break;
            }
            assert!(Instant::now() < deadline, "{enc}: {:?}", open(&conn));
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        while res.chunk().await.unwrap().is_some() {}
    }
    drop(conn);

    let (_, conn, dir) = h.finish(3).await;
    for id in 1..=3i64 {
        let messages = session::stored_messages(&conn, id, 0, 100).unwrap();
        assert_eq!(messages.len() as i64, EVENTS, "flow {id}");
        let mut reader = BodyReader::new(&dir.join("session"));
        let last = reader
            .read_place(messages.last().unwrap().place.unwrap())
            .unwrap();
        assert_eq!(last, format!("data: event {}", EVENTS - 1).as_bytes());
    }
    clean_up(conn, dir);
}

/// Any other response still going after a while is listed then, and written
/// whole again when it ends; one that ends soon is written once.
#[tokio::test(flavor = "multi_thread")]
async fn a_slow_response_is_listed_while_it_lasts() {
    let h = Harness::start("slow-response").await;
    let session_dir = h.dir.join("session");
    let client = h.client(false);
    let quick = client
        .get(format!(
            "{}/stream?format=text&events=3&gap_ms=10",
            h.origin
        ))
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    assert_eq!(quick.as_ref(), b"line 0\nline 1\nline 2\n");
    let mut res = client
        .get(format!(
            "{}/stream?format=text&events=20&gap_ms=200",
            h.origin
        ))
        .send()
        .await
        .unwrap();
    let conn = Connection::open(session_dir.join("flows.db")).unwrap();
    let started = Instant::now();
    let deadline = started + WAIT_FOR;
    let listed = loop {
        let row: Option<(bool, String)> = conn
            .query_row("SELECT open, kind FROM flows WHERE id = 2", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .ok();
        if let Some(row) = row {
            break row;
        }
        assert!(Instant::now() < deadline, "the slow one was never listed");
        tokio::time::sleep(Duration::from_millis(20)).await;
    };
    assert!(listed.0, "listed while it lasts");
    assert_eq!(listed.1, "http", "its kind as it was");
    assert!(
        started.elapsed() < Duration::from_secs(4),
        "{:?}",
        started.elapsed()
    );
    while res.chunk().await.unwrap().is_some() {}
    drop(conn);

    let (_, conn, dir) = h.finish(2).await;
    let (open, bytes): (bool, i64) = conn
        .query_row("SELECT open, resp_bytes FROM flows WHERE id = 2", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert!(!open);
    assert_eq!(
        bytes as usize,
        (0..20).map(|i| format!("line {i}\n").len()).sum::<usize>()
    );
    assert_eq!(
        response_of(&conn, &dir, 2).unwrap().len() as i64,
        bytes,
        "kept whole"
    );
    clean_up(conn, dir);
}

/// A stream's body is found by its words once it ends, though its row was
/// indexed while it was still going, with no body yet.
#[tokio::test(flavor = "multi_thread")]
async fn a_stream_is_searched_by_what_it_said_once_over() {
    let h = Harness::start_with("stream-search", StorageSettings::default()).await;
    let session_dir = h.dir.join("session");
    let mut res = h
        .client(false)
        .get(format!("{}/stream?events=6&gap_ms=200", h.origin))
        .send()
        .await
        .unwrap();
    // The index has seen the open row: its URL is found.
    let deadline = Instant::now() + WAIT_FOR;
    while search::find(&session_dir, "urls", "stream", 1).unwrap_or_default() != [1] {
        assert!(Instant::now() < deadline, "the open row was never indexed");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    while res.chunk().await.unwrap().is_some() {}

    let (_, conn, dir) = h.finish(1).await;
    let row = wait_for_row(&conn, "sse", |row| !row.0).await;
    assert_eq!(row.2, 6, "{row:?}");
    let found = search::find(&dir.join("session"), "bodies", "event 5", 1).unwrap();
    assert_eq!(found, [1], "what the stream said is found");
    clean_up(conn, dir);
}

/// A WebSocket has a row from its upgrade, and each message on it, both ways.
/// A socket carries its calls in its messages, so nothing could name its row
/// when it opened. It is named while it is still open, which is when anyone
/// looking for it needs it — a dapp's socket stays open for the session.
#[tokio::test(flavor = "multi_thread")]
async fn a_sockets_row_is_named_while_it_is_still_open() {
    use hudsucker::{
        futures::{SinkExt, StreamExt},
        tokio_tungstenite::tungstenite::Message,
    };
    let h = Harness::start("ws-rpc").await;
    let proxy: std::net::SocketAddr = h.proxy_url.trim_start_matches("http://").parse().unwrap();
    let open = async |path: &str| {
        benchkit::websocket_through_proxy(
            proxy,
            &h.dir.join("ca/ca.pem"),
            "localhost",
            h.ws_port,
            path,
        )
        .await
        .unwrap()
    };
    let subscribe = r#"{"jsonrpc":"2.0","id":1,"method":"eth_subscribe","params":["newHeads"]}"#;

    let mut ws = open("/rpc").await;
    ws.send(Message::text(subscribe)).await.unwrap();
    // The answer, then what the subscription sends of its own.
    ws.next().await.unwrap().unwrap();
    ws.next().await.unwrap().unwrap();

    let conn = Connection::open(h.dir.join("session/flows.db")).unwrap();
    type SocketRow = (Option<String>, Option<i64>, bool);
    let named = async |conn: &Connection, done: fn(&SocketRow) -> bool| -> SocketRow {
        let deadline = Instant::now() + WAIT_FOR;
        loop {
            let row: SocketRow = conn
                .query_row(
                    "SELECT rpc, rpc_error, open FROM flows WHERE kind = 'websocket' ORDER BY id DESC LIMIT 1",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .ok()
                .unwrap_or((None, None, false));
            if done(&row) || Instant::now() > deadline {
                return row;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    };
    let (rpc, error, still_open) = named(&conn, |row| row.0.is_some()).await;
    assert_eq!(rpc.as_deref(), Some("eth_subscribe"));
    assert_eq!(error, None, "nothing failed");
    assert!(still_open, "named while it is going, not at its end");
    drop(conn);
    ws.close(None).await.unwrap();
    while ws.next().await.is_some() {}

    // A socket whose calls fail: the reply says so, and the row says it too,
    // where its 101 says nothing is wrong.
    let mut failing = open("/rpc?fail=1").await;
    failing.send(Message::text(subscribe)).await.unwrap();
    failing.next().await.unwrap().unwrap();
    let conn = Connection::open(h.dir.join("session/flows.db")).unwrap();
    // Both halves, each learnt by the side of the socket that saw it.
    let (rpc, error, _) = named(&conn, |row| row.0.is_some() && row.1.is_some()).await;
    assert_eq!(rpc.as_deref(), Some("eth_subscribe"));
    assert_eq!(error, Some(3), "the call reverted, over a socket");
    drop(conn);
    failing.close(None).await.unwrap();
    while failing.next().await.is_some() {}

    // One that says something else is left alone.
    let mut chat = open("/chat").await;
    chat.send(Message::text("xin chào")).await.unwrap();
    chat.next().await.unwrap().unwrap();
    chat.close(None).await.unwrap();
    while chat.next().await.is_some() {}

    let (_, conn, dir) = h.finish(3).await;
    let rows: Vec<(Option<String>, Option<i64>)> = {
        let mut stmt = conn
            .prepare("SELECT rpc, rpc_error FROM flows WHERE kind = 'websocket' ORDER BY id")
            .unwrap();
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    };
    assert_eq!(
        rows,
        [
            (Some("eth_subscribe".to_owned()), None),
            (Some("eth_subscribe".to_owned()), Some(3)),
            (None, None),
        ],
        "the chat socket says nothing, and is not made to"
    );
    clean_up(conn, dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn websocket_messages_are_recorded_both_ways() {
    use hudsucker::{
        futures::{SinkExt, StreamExt},
        tokio_tungstenite::tungstenite::Message,
    };
    let h = Harness::start("websocket").await;
    let proxy: std::net::SocketAddr = h.proxy_url.trim_start_matches("http://").parse().unwrap();
    let mut ws = benchkit::websocket_through_proxy(
        proxy,
        &h.dir.join("ca/ca.pem"),
        "localhost",
        h.ws_port,
        "/chat?room=1",
    )
    .await
    .unwrap();

    ws.send(Message::text("xin chào")).await.unwrap();
    assert_eq!(
        ws.next().await.unwrap().unwrap(),
        Message::text("xin chào"),
        "it reached the server and back"
    );
    ws.send(Message::binary(vec![1, 2, 3])).await.unwrap();
    assert!(ws.next().await.unwrap().unwrap().is_binary());

    let conn = Connection::open(h.dir.join("session/flows.db")).unwrap();
    let row = wait_for_row(&conn, "websocket", |row| row.2 >= 4).await;
    assert!(row.0, "still open: {row:?}");
    assert_eq!(row.1, Some(101), "not shown as an error");
    drop(conn);

    ws.close(None).await.unwrap();
    while ws.next().await.is_some() {}
    let (_, conn, dir) = h.finish(1).await;
    let row = wait_for_row(&conn, "websocket", |row| !row.0).await;
    assert_eq!(row.1, Some(101));
    let path: String = conn
        .query_row("SELECT path FROM flows WHERE kind = 'websocket'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(path, "/chat?room=1");

    let messages = session::stored_messages(&conn, 1, 0, 100).unwrap();
    let mut reader = BodyReader::new(&dir.join("session"));
    let seen: Vec<(bool, &str, Vec<u8>)> = messages
        .iter()
        .take(4)
        .map(|m| {
            (
                m.from_client,
                m.kind.as_str(),
                reader.read_place(m.place.unwrap()).unwrap(),
            )
        })
        .collect();
    let hello = "xin chào".as_bytes().to_vec();
    assert_eq!(
        seen,
        [
            (true, "text", hello.clone()),
            (false, "text", hello),
            (true, "binary", vec![1, 2, 3]),
            (false, "binary", vec![1, 2, 3]),
        ]
    );
    assert!(
        messages.iter().any(|m| m.kind == "close" && m.from_client),
        "{messages:?}"
    );
    drop(reader);
    clean_up(conn, dir);
}

/// A WebSocket with no TLS asked for by its whole URL, `ws://`, as curl asks a
/// proxy: it reaches the server without TLS, and is kept as any other.
#[tokio::test(flavor = "multi_thread")]
async fn a_plain_websocket_asked_for_by_its_url_goes_through() {
    use hudsucker::{
        futures::{SinkExt, StreamExt},
        tokio_tungstenite::{
            WebSocketStream,
            tungstenite::{Message, protocol::Role},
        },
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let h = Harness::start("ws-url").await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(tracewing_dev::playground::serve(listener));
    let proxy = h.proxy_url.trim_start_matches("http://");
    let mut tcp = tokio::net::TcpStream::connect(proxy).await.unwrap();
    let ask = format!(
        "GET ws://localhost:{port}/ws?ticks=0 HTTP/1.1\r\nHost: localhost:{port}\r\n\
         Upgrade: websocket\r\nConnection: Upgrade\r\n\
         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"
    );
    tcp.write_all(ask.as_bytes()).await.unwrap();
    let mut reply = Vec::new();
    while !reply.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        tcp.read_exact(&mut byte).await.unwrap();
        reply.push(byte[0]);
    }
    let reply = String::from_utf8_lossy(&reply);
    assert!(reply.starts_with("HTTP/1.1 101"), "{reply}");
    let mut ws = WebSocketStream::from_raw_socket(tcp, Role::Client, None).await;
    let hello = ws.next().await.unwrap().unwrap();
    assert!(hello.to_text().unwrap().contains("hello"), "{hello:?}");
    ws.send(Message::text("xin chào")).await.unwrap();
    assert_eq!(ws.next().await.unwrap().unwrap(), Message::text("xin chào"));
    ws.close(None).await.unwrap();
    while ws.next().await.is_some() {}

    let (_, conn, dir) = h.finish(1).await;
    let (scheme, host, path, status): (String, String, String, Option<u16>) = conn
        .query_row(
            "SELECT scheme, host, path, status FROM flows WHERE kind = 'websocket'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(
        (scheme.as_str(), host, path.as_str(), status),
        (
            "http",
            format!("localhost:{port}"),
            "/ws?ticks=0",
            Some(101)
        )
    );
    clean_up(conn, dir);
}

/// What a client says to the proxy alone goes no further: the server never
/// sees the password for the proxy. The flow still shows what was sent.
#[tokio::test(flavor = "multi_thread")]
async fn what_is_for_the_proxy_alone_goes_no_further() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let h = Harness::start("hop-by-hop").await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(tracewing_dev::playground::serve(listener));
    let proxy = h.proxy_url.trim_start_matches("http://");
    let mut tcp = tokio::net::TcpStream::connect(proxy).await.unwrap();
    let ask = format!(
        "GET http://localhost:{port}/echo HTTP/1.1\r\nHost: localhost:{port}\r\n\
         Proxy-Connection: Keep-Alive\r\nProxy-Authorization: Basic dXNlcjpwYXNz\r\n\
         Connection: close, X-Hop\r\nX-Hop: 1\r\nKeep-Alive: timeout=5\r\n\
         TE: trailers\r\nX-End: 1\r\n\r\n"
    );
    tcp.write_all(ask.as_bytes()).await.unwrap();
    let mut reply = Vec::new();
    tcp.read_to_end(&mut reply).await.unwrap();
    let reply = String::from_utf8_lossy(&reply);
    let body = reply.split_once("\r\n\r\n").expect("an answer").1;
    let echo: serde_json::Value = serde_json::from_str(body).unwrap();
    let seen: Vec<&str> = echo["headers"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    for gone in [
        "proxy-connection",
        "proxy-authorization",
        "connection",
        "x-hop",
        "keep-alive",
    ] {
        assert!(!seen.contains(&gone), "{gone} reached the server: {seen:?}");
    }
    assert_eq!(echo["headers"]["x-end"], "1");
    assert_eq!(echo["headers"]["te"], "trailers");

    let (_, conn, dir) = h.finish(1).await;
    let mut reader = BodyReader::new(&dir.join("session"));
    let (request, _) = session::stored_headers(&conn, &mut reader, 1)
        .unwrap()
        .expect("kept");
    drop(reader);
    assert!(
        request.iter().any(|(n, _)| n == "proxy-connection"),
        "the flow shows what the client sent: {request:?}"
    );
    clean_up(conn, dir);
}

/// A company's proxy, with a password: HTTPS through a tunnel it makes, and
/// plain HTTP passed to it whole, the password with both; neither name looked
/// up here. A wrong password is said in the flow.
#[tokio::test(flavor = "multi_thread")]
async fn servers_are_reached_through_a_company_proxy() {
    use tracewing_dev::fake_relay;
    use tracewing_proxy::relay::RelaySettings;
    let h = Harness::start("relay-http").await;
    let plain = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let plain_port = plain.local_addr().unwrap().port();
    tokio::spawn(tracewing_dev::playground::serve(plain));
    let corp = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let corp_port = corp.local_addr().unwrap().port();
    let seen = fake_relay::Seen::default();
    tokio::spawn(fake_relay::serve_http(
        corp,
        Some(("an".to_owned(), "p@ss".to_owned())),
        Arc::clone(&seen),
    ));
    let through = |password: &str| RelaySettings {
        proxy: format!("http://an:{password}@127.0.0.1:{corp_port}"),
        bypass: String::new(),
    };
    h.relay.set(&through("p%40ss")).unwrap();

    let client = h.client(false);
    let secure = client
        .get(format!("{}/json?size=10", h.origin))
        .send()
        .await
        .unwrap();
    assert_eq!(secure.status(), 200);
    secure.bytes().await.unwrap();
    let plain = client
        .get(format!("http://localhost:{plain_port}/json"))
        .send()
        .await
        .unwrap();
    assert_eq!(plain.status(), 200);
    plain.bytes().await.unwrap();
    let origin_port = h.origin.rsplit(':').next().unwrap().to_owned();
    assert_eq!(
        *seen.lock().unwrap(),
        [
            format!("CONNECT localhost:{origin_port} auth=yes"),
            format!("GET http://localhost:{plain_port}/json auth=yes"),
        ]
    );

    // A wrong password: the relay says no, and so does the flow. A change
    // applies to connections made after it, so this one is to another name.
    h.relay.set(&through("wrong")).unwrap();
    let refused = client
        .get(format!("https://127.0.0.1:{origin_port}/json?size=1"))
        .send()
        .await
        .unwrap();
    assert_eq!(refused.status(), 502);
    refused.bytes().await.unwrap();

    let (_, conn, dir) = h.finish(3).await;
    let error: String = conn
        .query_row("SELECT error FROM flows WHERE error IS NOT NULL", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert!(
        error.contains("refused the user name and password"),
        "{error}"
    );
    clean_up(conn, dir);
}

/// A company's proxy that takes NTLM alone: tracewing signs in with NTLM's
/// three messages on one connection, never sending the password, and from
/// then on asks it with NTLM straight away, plain HTTP through a tunnel too.
#[tokio::test(flavor = "multi_thread")]
async fn servers_are_reached_through_a_relay_that_wants_ntlm() {
    use tracewing_dev::fake_relay;
    use tracewing_proxy::relay::RelaySettings;
    let h = Harness::start("relay-ntlm").await;
    let plain = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let plain_port = plain.local_addr().unwrap().port();
    tokio::spawn(tracewing_dev::playground::serve(plain));
    let corp = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let corp_port = corp.local_addr().unwrap().port();
    let seen = fake_relay::Seen::default();
    tokio::spawn(fake_relay::serve_http_ntlm(
        corp,
        ("CORP\\ann".to_owned(), "s3cret".to_owned()),
        Arc::clone(&seen),
    ));
    h.relay
        .set(&RelaySettings {
            proxy: format!("http://CORP%5Cann:s3cret@127.0.0.1:{corp_port}"),
            bypass: String::new(),
        })
        .unwrap();
    let client = h.client(false);
    let origin_port = h.origin.rsplit(':').next().unwrap().to_owned();

    // Plain HTTP first, passed on whole: the relay asks for NTLM, which the
    // app sees once.
    let first = client
        .get(format!("http://localhost:{plain_port}/json"))
        .send()
        .await
        .unwrap();
    assert_eq!(first.status(), 407);
    first.bytes().await.unwrap();
    // HTTPS: Basic is not offered, so NTLM, on one connection.
    let secure = client
        .get(format!("{}/json?size=10", h.origin))
        .send()
        .await
        .unwrap();
    assert_eq!(secure.status(), 200);
    secure.bytes().await.unwrap();
    // Plain HTTP again: through a tunnel now, signed in.
    let again = client
        .get(format!("http://localhost:{plain_port}/json"))
        .send()
        .await
        .unwrap();
    assert_eq!(again.status(), 200);
    again.bytes().await.unwrap();

    let seen = seen.lock().unwrap().clone();
    assert_eq!(
        seen,
        [
            format!("GET http://localhost:{plain_port}/json auth=no"),
            format!("CONNECT localhost:{origin_port} auth=ntlm"),
            format!("CONNECT localhost:{plain_port} auth=ntlm"),
        ],
        "NTLM straight away once asked; no password passed on"
    );
    let (_, conn, dir) = h.finish(3).await;
    clean_up(conn, dir);
}

/// A SOCKS5 proxy, with a password, and hosts passed by.
#[tokio::test(flavor = "multi_thread")]
async fn servers_are_reached_through_socks5_or_passed_by() {
    use tracewing_dev::fake_relay;
    use tracewing_proxy::relay::RelaySettings;
    let h = Harness::start("relay-socks").await;
    let corp = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let corp_port = corp.local_addr().unwrap().port();
    let seen = fake_relay::Seen::default();
    tokio::spawn(fake_relay::serve_socks5(
        corp,
        Some(("an".to_owned(), "pw".to_owned())),
        Arc::clone(&seen),
    ));
    h.relay
        .set(&RelaySettings {
            proxy: format!("socks5://an:pw@127.0.0.1:{corp_port}"),
            bypass: String::new(),
        })
        .unwrap();
    let res = h
        .client(false)
        .get(format!("{}/json?size=10", h.origin))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    res.bytes().await.unwrap();
    let origin_port = h.origin.rsplit(':').next().unwrap().to_owned();
    assert_eq!(
        *seen.lock().unwrap(),
        [format!("SOCKS5 localhost:{origin_port} auth=yes")]
    );

    // localhost passed by: the next connection goes straight.
    h.relay
        .set(&RelaySettings {
            proxy: format!("socks5://an:pw@127.0.0.1:{corp_port}"),
            bypass: "localhost".to_owned(),
        })
        .unwrap();
    let res = h
        .client(true)
        .get(format!("{}/json?size=10&n=2", h.origin))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    res.bytes().await.unwrap();
    assert_eq!(seen.lock().unwrap().len(), 1, "not through the relay");
    let (_, conn, dir) = h.finish(2).await;
    clean_up(conn, dir);
}

/// The playground's `/ws` through the proxy, over TLS or not.
async fn playground_ws(
    h: &Harness,
    secure: bool,
    path: &str,
    headers: &[(&str, &str)],
) -> Result<
    (benchkit::AnyWebSocket, http::Response<Option<Vec<u8>>>),
    hudsucker::tokio_tungstenite::tungstenite::Error,
> {
    let proxy: std::net::SocketAddr = h.proxy_url.trim_start_matches("http://").parse().unwrap();
    let port = if secure {
        h.playground_tls_port
    } else {
        h.playground_port
    };
    benchkit::websocket_ask(
        proxy,
        &h.dir.join("ca/ca.pem"),
        &benchkit::WsAsk {
            secure,
            host: "localhost",
            port,
            path,
            headers,
        },
    )
    .await
}

fn json_of(message: &hudsucker::tokio_tungstenite::tungstenite::Message) -> serde_json::Value {
    serde_json::from_str(message.to_text().unwrap()).unwrap()
}

/// What the client sends with its upgrade reaches the server, over TLS and
/// not: its origin, cookies and own headers.
#[tokio::test(flavor = "multi_thread")]
async fn a_websockets_headers_reach_the_server() {
    use hudsucker::futures::StreamExt;
    let h = Harness::start("ws-headers").await;
    for secure in [true, false] {
        let (mut ws, _) = playground_ws(
            &h,
            secure,
            "/ws?ticks=0",
            &[
                ("origin", "https://app.example"),
                ("cookie", "session=abc"),
                ("x-test", "order-42"),
            ],
        )
        .await
        .unwrap();
        let hello = json_of(&ws.next().await.unwrap().unwrap());
        assert_eq!(hello["origin"], "https://app.example", "secure: {secure}");
        assert_eq!(hello["cookie"], "session=abc");
        assert_eq!(hello["x_test"], "order-42");
        ws.close(None).await.unwrap();
        while ws.next().await.is_some() {}
    }
    let (_, conn, dir) = h.finish(2).await;
    clean_up(conn, dir);
}

/// The subprotocol the server takes is the one the client is told of.
#[tokio::test(flavor = "multi_thread")]
async fn a_websockets_subprotocol_is_agreed_end_to_end() {
    use hudsucker::futures::StreamExt;
    let h = Harness::start("ws-protocol").await;
    let (mut ws, answer) = playground_ws(
        &h,
        true,
        "/ws?ticks=0",
        &[("sec-websocket-protocol", "chat.v2, chat.v1")],
    )
    .await
    .unwrap();
    let hello = json_of(&ws.next().await.unwrap().unwrap());
    assert_eq!(hello["protocol"], "chat.v2", "the server took the first");
    assert_eq!(
        answer
            .headers()
            .get("sec-websocket-protocol")
            .map(|v| v.to_str().unwrap()),
        Some("chat.v2"),
        "and the client is told"
    );
    ws.close(None).await.unwrap();
    while ws.next().await.is_some() {}
    let (_, conn, dir) = h.finish(1).await;
    clean_up(conn, dir);
}

/// Large binary messages both ways, and a ping answered.
#[tokio::test(flavor = "multi_thread")]
async fn a_websocket_carries_large_messages_and_pings() {
    use hudsucker::{
        futures::{SinkExt, StreamExt},
        tokio_tungstenite::tungstenite::Message,
    };
    const MIB: usize = 1 << 20;
    let h = Harness::start("ws-large").await;
    let (mut ws, _) = playground_ws(&h, true, &format!("/ws?ticks=0&big={MIB}"), &[])
        .await
        .unwrap();
    assert!(ws.next().await.unwrap().unwrap().is_text(), "the hello");
    let big = ws.next().await.unwrap().unwrap();
    let expected: Vec<u8> = (0..MIB).map(|i| (i % 251) as u8).collect();
    assert_eq!(big.into_data().to_vec(), expected, "from the server");
    let sent: Vec<u8> = (0..MIB).map(|i| (i % 13) as u8).collect();
    ws.send(Message::binary(sent.clone())).await.unwrap();
    assert_eq!(
        ws.next().await.unwrap().unwrap().into_data().to_vec(),
        sent,
        "to it and back"
    );
    ws.send(Message::Ping(b"are you there".to_vec().into()))
        .await
        .unwrap();
    let pong = ws.next().await.unwrap().unwrap();
    assert_eq!(pong, Message::Pong(b"are you there".to_vec().into()));
    ws.close(None).await.unwrap();
    while ws.next().await.is_some() {}
    let (_, conn, dir) = h.finish(1).await;
    clean_up(conn, dir);
}

/// A message the window sends into an open socket reaches the side it is
/// sent toward, and is written down with the others, that way.
#[tokio::test(flavor = "multi_thread")]
async fn a_message_sent_from_the_window_goes_into_the_socket() {
    use hudsucker::{futures::StreamExt, tokio_tungstenite::tungstenite::Message};
    use tracewing_proxy::sockets::{NotSent, Said, Toward};
    let h = Harness::start("ws-send").await;
    let (mut ws, _) = playground_ws(&h, false, "/ws?ticks=0", &[]).await.unwrap();
    assert!(ws.next().await.unwrap().unwrap().is_text(), "the hello");

    // Its row, once written, is the one to send into.
    let conn = Connection::open(h.dir.join("session/flows.db")).unwrap();
    let deadline = Instant::now() + WAIT_FOR;
    let row = loop {
        let row: Option<i64> = conn
            .query_row(
                "SELECT id FROM flows WHERE kind = 'websocket' AND open = 1",
                [],
                |r| r.get(0),
            )
            .ok();
        if let Some(row) = row.filter(|row| h.sockets.is_open(*row)) {
            break row;
        }
        assert!(Instant::now() < deadline, "the socket's row");
        tokio::time::sleep(Duration::from_millis(10)).await;
    };
    assert_eq!(
        h.sockets
            .send(row + 1000, Toward::App, Said::Text("nowhere".into())),
        Err(NotSent::NotOpen)
    );

    // To the app, as if from the server.
    h.sockets
        .send(row, Toward::App, Said::Text("from tracewing".into()))
        .unwrap();
    assert_eq!(
        ws.next().await.unwrap().unwrap(),
        Message::text("from tracewing")
    );
    // To the server, as if from the app: it echoes it back.
    h.sockets
        .send(row, Toward::Server, Said::Binary(vec![1, 2, 3]))
        .unwrap();
    assert_eq!(
        ws.next().await.unwrap().unwrap(),
        Message::binary(vec![1, 2, 3])
    );

    ws.close(None).await.unwrap();
    while ws.next().await.is_some() {}
    let deadline = Instant::now() + WAIT_FOR;
    while h.sockets.is_open(row) {
        assert!(Instant::now() < deadline, "closed, it is no longer open");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(
        h.sockets
            .send(row, Toward::Server, Said::Text("late".into())),
        Err(NotSent::NotOpen)
    );
    drop(conn);

    let (_, conn, dir) = h.finish(1).await;
    let messages: Vec<(bool, String, i64)> = {
        let mut stmt = conn
            .prepare("SELECT from_client, kind, bytes FROM messages WHERE flow_id = ?1 AND kind != 'close' ORDER BY id")
            .unwrap();
        stmt.query_map([row], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .map(Result::unwrap)
            .skip(1)
            .collect()
    };
    assert_eq!(
        messages,
        [
            (false, "text".to_owned(), 14),
            (true, "binary".to_owned(), 3),
            (false, "binary".to_owned(), 3),
        ],
        "after the hello: sent to the app, sent to the server, its echo"
    );
    clean_up(conn, dir);
}

/// A close's code and reason go through, whichever side closes first, and
/// the socket's row says it ended.
#[tokio::test(flavor = "multi_thread")]
async fn a_websockets_close_and_its_code_go_through() {
    use hudsucker::{
        futures::{SinkExt, StreamExt},
        tokio_tungstenite::tungstenite::{
            Message,
            protocol::{CloseFrame, frame::coding::CloseCode},
        },
    };
    let h = Harness::start("ws-close").await;
    // The server closes.
    let (mut ws, _) = playground_ws(&h, true, "/ws?ticks=0&close=4000", &[])
        .await
        .unwrap();
    assert!(ws.next().await.unwrap().unwrap().is_text());
    let closed = ws.next().await.unwrap().unwrap();
    match closed {
        Message::Close(Some(frame)) => {
            assert_eq!(u16::from(frame.code), 4000);
            assert_eq!(frame.reason.as_str(), "bye");
        }
        other => panic!("not a close: {other:?}"),
    }
    while ws.next().await.is_some() {}
    // The client closes, and the server's echo of it comes back.
    let (mut ws, _) = playground_ws(&h, false, "/ws?ticks=0", &[]).await.unwrap();
    assert!(ws.next().await.unwrap().unwrap().is_text());
    ws.send(Message::Close(Some(CloseFrame {
        code: CloseCode::from(4001),
        reason: "done".into(),
    })))
    .await
    .unwrap();
    let mut echoed = None;
    while let Some(Ok(message)) = ws.next().await {
        if let Message::Close(frame) = message {
            echoed = frame;
        }
    }
    assert_eq!(echoed.map(|f| u16::from(f.code)), Some(4001));
    let (_, conn, dir) = h.finish(2).await;
    let open: i64 = conn
        .query_row(
            "SELECT count(*) FROM flows WHERE kind = 'websocket' AND open",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(open, 0, "both rows say they ended");
    clean_up(conn, dir);
}

/// Many sockets at once, each its own messages, and each recorded.
#[tokio::test(flavor = "multi_thread")]
async fn many_websockets_at_once_keep_to_themselves() {
    use hudsucker::{
        futures::{SinkExt, StreamExt},
        tokio_tungstenite::tungstenite::Message,
    };
    const SOCKETS: usize = 20;
    let h = Harness::start("ws-many").await;
    let mut tasks = Vec::new();
    for n in 0..SOCKETS {
        let (mut ws, _) = playground_ws(&h, n % 2 == 0, "/ws?ticks=0", &[])
            .await
            .unwrap();
        tasks.push(tokio::spawn(async move {
            assert!(ws.next().await.unwrap().unwrap().is_text());
            for i in 0..10 {
                let text = format!("socket {n} message {i}");
                ws.send(Message::text(text.clone())).await.unwrap();
                assert_eq!(ws.next().await.unwrap().unwrap(), Message::text(text));
            }
            ws.close(None).await.unwrap();
            while ws.next().await.is_some() {}
        }));
    }
    for task in tasks {
        task.await.unwrap();
    }
    let (_, conn, dir) = h.finish(SOCKETS as i64).await;
    let messages: i64 = conn
        .query_row(
            "SELECT count(*) FROM messages WHERE kind = 'text'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        messages,
        (SOCKETS * (1 + 10 * 2)) as i64,
        "hello, then each way"
    );
    clean_up(conn, dir);
}

/// Not recording, a socket works all the same.
#[tokio::test(flavor = "multi_thread")]
async fn a_websocket_works_while_nothing_is_recorded() {
    use hudsucker::{
        futures::{SinkExt, StreamExt},
        tokio_tungstenite::tungstenite::Message,
    };
    let h = Harness::start("ws-paused").await;
    h.recording
        .store(false, std::sync::atomic::Ordering::Relaxed);
    let (mut ws, _) = playground_ws(&h, true, "/ws?ticks=0", &[]).await.unwrap();
    assert!(ws.next().await.unwrap().unwrap().is_text());
    ws.send(Message::text("still here")).await.unwrap();
    assert_eq!(
        ws.next().await.unwrap().unwrap(),
        Message::text("still here")
    );
    ws.close(None).await.unwrap();
    while ws.next().await.is_some() {}
    let (_, conn, dir) = h.finish(0).await;
    clean_up(conn, dir);
}

/// A server that will not upgrade: the client is told so, with its status.
#[tokio::test(flavor = "multi_thread")]
async fn a_websocket_the_server_refuses_is_refused() {
    use hudsucker::tokio_tungstenite::tungstenite::Error;
    let h = Harness::start("ws-refused").await;
    for secure in [true, false] {
        match playground_ws(&h, secure, "/ws?refuse=403", &[]).await {
            Err(Error::Http(answer)) => assert_eq!(answer.status(), 403, "secure: {secure}"),
            Err(other) => panic!("secure: {secure}: {other}"),
            Ok(_) => panic!("secure: {secure}: an upgrade the server refused"),
        }
    }
    let (_, conn, dir) = h.finish(2).await;
    let statuses: Vec<Option<u16>> = conn
        .prepare("SELECT status FROM flows ORDER BY id")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(statuses, [Some(403), Some(403)]);
    clean_up(conn, dir);
}

/// WebSockets through a company's proxy, as other connections go: over TLS
/// and not, through CONNECT with its password, and through SOCKS5; a host
/// passed by goes straight; a refusal is said to the client and in the flow.
#[tokio::test(flavor = "multi_thread")]
async fn websockets_go_through_a_company_proxy() {
    use hudsucker::{
        futures::{SinkExt, StreamExt},
        tokio_tungstenite::tungstenite::{Error, Message},
    };
    use tracewing_dev::fake_relay;
    use tracewing_proxy::relay::RelaySettings;
    let h = Harness::start("ws-relay").await;
    let bind = || async { TcpListener::bind("127.0.0.1:0").await.unwrap() };
    let http_relay = bind().await;
    let http_port = http_relay.local_addr().unwrap().port();
    let socks_relay = bind().await;
    let socks_port = socks_relay.local_addr().unwrap().port();
    let seen = fake_relay::Seen::default();
    let user = || Some(("an".to_owned(), "pw".to_owned()));
    tokio::spawn(fake_relay::serve_http(
        http_relay,
        user(),
        Arc::clone(&seen),
    ));
    tokio::spawn(fake_relay::serve_socks5(
        socks_relay,
        user(),
        Arc::clone(&seen),
    ));
    let through = |proxy: String, bypass: &str| RelaySettings {
        proxy,
        bypass: bypass.to_owned(),
    };
    let echo = |mut ws: benchkit::AnyWebSocket, text: &'static str| async move {
        assert!(ws.next().await.unwrap().unwrap().is_text(), "the hello");
        ws.send(Message::text(text)).await.unwrap();
        assert_eq!(ws.next().await.unwrap().unwrap(), Message::text(text));
        ws.close(None).await.unwrap();
        while ws.next().await.is_some() {}
    };
    let (tls, plain) = (h.playground_tls_port, h.playground_port);

    h.relay
        .set(&through(format!("http://an:pw@127.0.0.1:{http_port}"), ""))
        .unwrap();
    for secure in [true, false] {
        let (ws, _) = playground_ws(&h, secure, "/ws?ticks=0", &[]).await.unwrap();
        echo(ws, "through CONNECT").await;
    }
    h.relay
        .set(&through(
            format!("socks5://an:pw@127.0.0.1:{socks_port}"),
            "",
        ))
        .unwrap();
    let (ws, _) = playground_ws(&h, true, "/ws?ticks=0", &[]).await.unwrap();
    echo(ws, "through SOCKS5").await;
    // Plain WebSockets go through a tunnel too, as browsers have them.
    assert_eq!(
        *seen.lock().unwrap(),
        [
            format!("CONNECT localhost:{tls} auth=yes"),
            format!("CONNECT localhost:{plain} auth=yes"),
            format!("SOCKS5 localhost:{tls} auth=yes"),
        ]
    );

    // Passed by: straight.
    h.relay
        .set(&through(
            format!("socks5://an:pw@127.0.0.1:{socks_port}"),
            "localhost",
        ))
        .unwrap();
    let (ws, _) = playground_ws(&h, true, "/ws?ticks=0", &[]).await.unwrap();
    echo(ws, "straight").await;
    assert_eq!(seen.lock().unwrap().len(), 3, "not through the relay");

    // A wrong password: the client is told, and so is the flow.
    h.relay
        .set(&through(
            format!("http://an:wrong@127.0.0.1:{http_port}"),
            "",
        ))
        .unwrap();
    match playground_ws(&h, true, "/ws?ticks=0", &[]).await {
        Err(Error::Http(answer)) => assert_eq!(answer.status(), 502),
        Err(other) => panic!("{other}"),
        Ok(_) => panic!("a socket through a relay that refused"),
    }

    let (_, conn, dir) = h.finish(5).await;
    let error: String = conn
        .query_row("SELECT error FROM flows WHERE error IS NOT NULL", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert!(
        error.contains("refused the user name and password"),
        "{error}"
    );
    clean_up(conn, dir);
}

/// A WebSocket's row knows its connection now, as other flows' do: when it
/// was made and what TLS the server spoke. And a header that is not ASCII
/// reaches the server as it was sent.
#[tokio::test(flavor = "multi_thread")]
async fn a_websocket_knows_its_connection() {
    use hudsucker::futures::StreamExt;
    let h = Harness::start("ws-connection").await;
    let (mut ws, _) = playground_ws(&h, true, "/ws?ticks=0", &[("x-test", "đặt-hàng")])
        .await
        .unwrap();
    let hello = json_of(&ws.next().await.unwrap().unwrap());
    assert_eq!(hello["x_test"], "đặt-hàng");
    ws.close(None).await.unwrap();
    while ws.next().await.is_some() {}
    let (_, conn, dir) = h.finish(1).await;
    let (conn_id, connect_us, tls_us, kind): (Option<i64>, Option<i64>, Option<i64>, String) = conn
        .query_row(
            "SELECT conn_id, connect_us, tls_us, kind FROM flows",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(kind, "websocket");
    assert!(conn_id.is_some(), "its connection");
    assert!(connect_us.is_some() && tls_us.is_some(), "and its timing");
    let facts: Option<i64> = conn
        .query_row("SELECT conn_len FROM flows", [], |r| r.get(0))
        .unwrap();
    assert!(
        facts.is_some_and(|len| len > 0),
        "what the server showed is kept"
    );
    clean_up(conn, dir);
}

/// A rule sends a WebSocket elsewhere, as it does any request: the socket
/// opens to the other server, and its row says which rule did it.
#[tokio::test(flavor = "multi_thread")]
async fn a_websocket_is_sent_elsewhere_by_a_rule() {
    use hudsucker::futures::StreamExt;
    let h = Harness::start("ws-remote").await;
    h.traffic_rules.write().unwrap().rules.push(rule(
        &format!("localhost:{}/ws*", h.ws_port),
        Action::MapRemote {
            to: format!("ws://localhost:{}", h.playground_port),
        },
    ));
    let proxy: std::net::SocketAddr = h.proxy_url.trim_start_matches("http://").parse().unwrap();
    // To the echo server, which says nothing first; the playground says hello.
    let (mut ws, _) = benchkit::websocket_ask(
        proxy,
        &h.dir.join("ca/ca.pem"),
        &benchkit::WsAsk {
            secure: true,
            host: "localhost",
            port: h.ws_port,
            path: "/ws?ticks=0",
            headers: &[],
        },
    )
    .await
    .unwrap();
    let hello = json_of(&ws.next().await.unwrap().unwrap());
    assert_eq!(hello["hello"], "tracewing playground");
    ws.close(None).await.unwrap();
    while ws.next().await.is_some() {}
    let ws_port = h.ws_port;
    let (_, conn, dir) = h.finish(1).await;
    let (kind, rule_name): (String, Option<String>) = conn
        .query_row("SELECT kind, rule FROM flows", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!(kind, "websocket");
    assert_eq!(rule_name, Some(format!("rule for localhost:{ws_port}/ws*")));
    clean_up(conn, dir);
}

/// A slow network slows a WebSocket as it does other flows: its upgrade a
/// round trip, each message half of one each way, and its bytes by the
/// link. Messages sent together arrive together, a round trip later, not one
/// after another.
#[tokio::test(flavor = "multi_thread")]
async fn a_slow_network_slows_a_websocket() {
    use hudsucker::{
        futures::{SinkExt, StreamExt},
        tokio_tungstenite::tungstenite::Message,
    };
    use std::time::Instant;
    const LATENCY: Duration = Duration::from_millis(600);
    let h = Harness::start("ws-slow").await;
    h.network.set(Conditions {
        on: true,
        name: "Test".to_owned(),
        latency_ms: LATENCY.as_millis() as u32,
        ..Conditions::default()
    });
    let opening = Instant::now();
    let (mut ws, _) = playground_ws(&h, false, "/ws?ticks=0", &[]).await.unwrap();
    assert!(
        opening.elapsed() >= LATENCY,
        "the upgrade: {:?}",
        opening.elapsed()
    );
    assert!(ws.next().await.unwrap().unwrap().is_text(), "the hello");

    let one = Instant::now();
    ws.send(Message::text("one")).await.unwrap();
    assert_eq!(ws.next().await.unwrap().unwrap(), Message::text("one"));
    assert!(
        one.elapsed() >= LATENCY,
        "a message there and back: {:?}",
        one.elapsed()
    );

    let five = Instant::now();
    for n in 0..5 {
        ws.send(Message::text(format!("m{n}"))).await.unwrap();
    }
    for n in 0..5 {
        assert_eq!(
            ws.next().await.unwrap().unwrap(),
            Message::text(format!("m{n}"))
        );
    }
    let took = five.elapsed();
    assert!(
        took >= LATENCY && took < LATENCY * 2,
        "five together: {took:?}"
    );
    ws.close(None).await.unwrap();
    while ws.next().await.is_some() {}

    // The link's bandwidth: 50 kB at 400 kbit/s is a second.
    h.network.set(Conditions {
        on: true,
        name: "Test".to_owned(),
        down_kbps: 400,
        ..Conditions::default()
    });
    let (mut ws, _) = playground_ws(&h, false, "/ws?ticks=0&big=50000", &[])
        .await
        .unwrap();
    assert!(ws.next().await.unwrap().unwrap().is_text());
    let big = Instant::now();
    assert_eq!(ws.next().await.unwrap().unwrap().len(), 50_000);
    assert!(
        big.elapsed() >= Duration::from_millis(800),
        "50 kB: {:?}",
        big.elapsed()
    );
    ws.close(None).await.unwrap();
    while ws.next().await.is_some() {}

    let (_, conn, dir) = h.finish(2).await;
    let rule: Option<String> = conn
        .query_row("SELECT rule FROM flows ORDER BY id LIMIT 1", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert!(
        rule.is_some_and(|r| r.contains("Test")),
        "the row says it was slowed"
    );
    clean_up(conn, dir);
}

/// Dropping the oldest flows takes their messages along.
#[tokio::test(flavor = "multi_thread")]
async fn a_full_session_drops_messages_with_their_flows() {
    use hudsucker::{futures::SinkExt, tokio_tungstenite::tungstenite::Message};
    let h = Harness::start_with(
        "trim-messages",
        StorageSettings {
            session_max_mib: 64,
            compress_stored: false,
            index_bodies: false,
            ..StorageSettings::default()
        },
    )
    .await;
    let proxy: std::net::SocketAddr = h.proxy_url.trim_start_matches("http://").parse().unwrap();
    let mut ws = benchkit::websocket_through_proxy(
        proxy,
        &h.dir.join("ca/ca.pem"),
        "localhost",
        h.ws_port,
        "/",
    )
    .await
    .unwrap();
    ws.send(Message::text("early")).await.unwrap();
    ws.close(None).await.unwrap();
    let client = h.client(false);
    for _ in 0..1200 {
        client
            .get(format!("{}/json?size=60000", h.origin))
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
    }
    let (summary, conn, dir) = h.finish(0).await;
    assert!(summary.trimmed > 0, "{summary:?}");
    let first: i64 = conn
        .query_row("SELECT min(id) FROM flows", [], |r| r.get(0))
        .unwrap();
    let orphans: i64 = conn
        .query_row(
            "SELECT count(*) FROM messages WHERE flow_id < ?1",
            [first],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(orphans, 0);
    clean_up(conn, dir);
}

/// What writing messages down costs a busy WebSocket: the same round trips with
/// recording on and off. Prints, and checks only that nothing was lost.
///
/// `WS_BENCH=200000 cargo test --release -p tracewing-dev --test e2e websocket_bench -- --nocapture`
#[tokio::test(flavor = "multi_thread")]
async fn websocket_bench() {
    use hudsucker::{
        futures::{SinkExt, StreamExt},
        tokio_tungstenite::tungstenite::Message,
    };
    let Some(count) = std::env::var("WS_BENCH").ok() else {
        return;
    };
    let count: u64 = count.parse().unwrap();
    let h = Harness::start_with(
        "ws-bench",
        StorageSettings {
            disk_reserve_mib: 1,
            ..StorageSettings::default()
        },
    )
    .await;
    let proxy: std::net::SocketAddr = h.proxy_url.trim_start_matches("http://").parse().unwrap();
    let body = "x".repeat(500);
    for recording in [false, true, false, true] {
        h.recording.store(recording, Ordering::Relaxed);
        let mut ws = benchkit::websocket_through_proxy(
            proxy,
            &h.dir.join("ca/ca.pem"),
            "localhost",
            h.ws_port,
            "/",
        )
        .await
        .unwrap();
        let started = Instant::now();
        for _ in 0..count {
            ws.send(Message::text(body.clone())).await.unwrap();
            ws.next().await.unwrap().unwrap();
        }
        let took = started.elapsed().as_secs_f64();
        println!(
            "recording {recording}: {:.0} round trips/s, {:.1} µs each",
            count as f64 / took,
            took * 1e6 / count as f64
        );
        ws.close(None).await.unwrap();
        while ws.next().await.is_some() {}
    }
    let (summary, conn, dir) = h.finish(0).await;
    let kept: i64 = conn
        .query_row("SELECT count(*) FROM messages", [], |r| r.get(0))
        .unwrap();
    println!("{kept} messages kept, {summary:?}");
    assert!(kept as u64 >= count * 4, "both ways, twice");
    clean_up(conn, dir);
}

fn rule(url: &str, action: Action) -> TrafficRule {
    TrafficRule {
        name: format!("rule for {url}"),
        enabled: true,
        url: url.to_owned(),
        method: String::new(),
        action,
    }
}

/// Which version the server is asked with, which ALPN alone settles: the
/// rule pins it, and the origin says which it saw.
#[tokio::test(flavor = "multi_thread")]
async fn a_rule_settles_the_version_a_server_is_asked_with() {
    for (version, expected) in [
        (AskWith::Http1, "version=HTTP/1.1\n"),
        (AskWith::Http2, "version=HTTP/2.0\n"),
    ] {
        let h = Harness::start("ask-server-with").await;
        let port = h.origin.rsplit(':').next().unwrap().to_owned();
        // Set before the first request: the version is agreed as the
        // connection is made, and a connection already made is kept to be
        // used again.
        *h.traffic_rules.write().unwrap() = TrafficRules {
            rules: vec![rule(
                &format!("localhost:{port}"),
                Action::AskServerWith { version },
            )],
        };
        let echoed = h
            .client(false)
            .get(format!("{}/echo", h.origin))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert!(
            echoed.starts_with(expected),
            "asked with {version:?}: {echoed}"
        );

        let (_, conn, dir) = h.finish(1).await;
        let (rule_name, server): (Option<String>, Option<String>) = conn
            .query_row("SELECT rule, server_version FROM flows", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(
            rule_name.as_deref(),
            Some(format!("rule for localhost:{port}").as_str()),
            "the flow says which rule touched it"
        );
        assert_eq!(
            server.as_deref(),
            Some(expected.trim_end().trim_start_matches("version=")),
            "and the version the server answered with"
        );
        clean_up(conn, dir);
    }
}

/// Every action, through the real proxy, and each flow saying which rule touched it.
#[tokio::test(flavor = "multi_thread")]
async fn rules_change_traffic_and_say_so() {
    let h = Harness::start("traffic-rules").await;
    let port = h.origin.rsplit(':').next().unwrap().to_owned();
    let folder = h.dir.join("mock");
    std::fs::create_dir_all(folder.join("v1")).unwrap();
    std::fs::write(folder.join("v1/me.json"), r#"{"name":"mocked"}"#).unwrap();
    std::fs::write(h.dir.join("secret"), "not this").unwrap();
    *h.traffic_rules.write().unwrap() = TrafficRules {
        rules: vec![
            rule("blocked.test", Action::Block { status: 451 }),
            rule(
                "mock.test/*",
                Action::MapLocal {
                    path: folder.clone(),
                },
            ),
            rule(
                "elsewhere.test",
                Action::MapRemote {
                    to: h.origin.clone(),
                },
            ),
            rule(
                &format!("localhost:{port}/json?size=33*"),
                Action::SetResponseHeader {
                    header: "x-from-rule".to_owned(),
                    value: "yes".to_owned(),
                },
            ),
        ],
    };
    let client = h.client(false);

    let res = client.get("https://blocked.test/").send().await.unwrap();
    assert_eq!(res.status(), 451, "with no server anywhere");

    let res = client
        .get("https://mock.test/v1/me.json?x=1")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.headers()["content-type"], "application/json");
    assert_eq!(res.text().await.unwrap(), r#"{"name":"mocked"}"#);

    let res = client
        .get("https://mock.test/../secret")
        .send()
        .await
        .unwrap();
    assert_ne!(
        res.text().await.unwrap(),
        "not this",
        "no way out of the folder"
    );
    let res = client
        .get("https://mock.test/v1/%2e%2e/%2e%2e/secret")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 404);

    let res = client
        .get("https://elsewhere.test/json?size=40")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.bytes().await.unwrap().len(), 40, "the path went along");

    let res = client
        .get(format!("{}/json?size=33", h.origin))
        .send()
        .await
        .unwrap();
    assert_eq!(res.headers()["x-from-rule"], "yes");
    res.bytes().await.unwrap();

    // Off applies at once, and the plain request is not touched.
    h.traffic_rules.write().unwrap().rules.clear();
    let res = client
        .get(format!("{}/json?size=34", h.origin))
        .send()
        .await
        .unwrap();
    assert!(res.headers().get("x-from-rule").is_none());
    res.bytes().await.unwrap();

    let (_, conn, dir) = h.finish(7).await;
    // A flow is written as it ends, so ids go by when each ended. The requests
    // were sent one after another: order them by when they started.
    let (ids, rows): (Vec<_>, Vec<_>) = conn
        .prepare("SELECT id, host, status, rule FROM flows ORDER BY started_us, id")
        .unwrap()
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                (
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<i64>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ),
            ))
        })
        .unwrap()
        .map(Result::unwrap)
        .unzip();
    assert_eq!(rows.len(), 7, "{rows:?}");
    assert_eq!(
        rows[0],
        (
            "blocked.test".to_owned(),
            Some(451),
            Some("rule for blocked.test".to_owned())
        )
    );
    assert_eq!(rows[1].0, "mock.test", "the URL as the client asked");
    let server: Option<String> = conn
        .query_row(
            "SELECT server_version FROM flows WHERE id = ?1",
            [ids[1]],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(server, None, "no server was asked");
    assert_eq!(rows[4].0, "elsewhere.test", "the URL as the client asked");
    assert_eq!(rows[4].2.as_deref(), Some("rule for elsewhere.test"));
    assert!(rows[5].2.is_some(), "{rows:?}");
    assert_eq!(rows[6].2, None, "{rows:?}");
    assert_eq!(
        response_of(&conn, &dir, ids[1]).unwrap(),
        br#"{"name":"mocked"}"#,
        "what the rule answered is recorded too"
    );
    clean_up(conn, dir);
}

/// Rules that change what goes to the server and what comes back: a header
/// taken out, the query, both bodies, and the status.
#[tokio::test(flavor = "multi_thread")]
async fn rules_rewrite_both_ways() {
    let h = Harness::start("rewrite").await;
    h.traffic_rules.write().unwrap().rules.extend([
        rule(
            "*/echo*",
            Action::RemoveRequestHeader {
                header: "x-secret".to_owned(),
            },
        ),
        rule(
            "*/echo*",
            Action::SetQuery {
                name: "env".to_owned(),
                value: "staging two".to_owned(),
            },
        ),
        rule(
            "*/echo*",
            Action::RemoveQuery {
                name: "debug".to_owned(),
            },
        ),
        rule(
            "*/echo*",
            Action::ReplaceRequestBody {
                find: "alice".to_owned(),
                replace: "bob".to_owned(),
            },
        ),
        rule(
            "*/echo*",
            Action::ReplaceResponseBody {
                find: "body=".to_owned(),
                replace: "sent=".to_owned(),
            },
        ),
        rule("*/echo*", Action::SetStatus { status: 418 }),
        rule(
            "*/json*",
            Action::ReplaceResponseBody {
                find: String::new(),
                replace: "{\"whole\":true}".to_owned(),
            },
        ),
    ]);
    let client = h.client(false);

    let res = client
        .post(format!("{}/echo?debug=1&env=prod&page=2", h.origin))
        .header("x-secret", "token")
        .header("x-kept", "yes")
        .body("hello alice, alice")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 418, "the server's status, changed");
    let text = res.text().await.unwrap();
    assert!(text.contains("query=env=staging%20two&page=2\n"), "{text}");
    assert!(!text.contains("x-secret"), "{text}");
    assert!(text.contains("x-kept: yes"), "{text}");
    assert!(
        text.ends_with("sent=hello bob, bob"),
        "both bodies changed: {text}"
    );

    // Packed by the server: unpacked to be changed, and sent on unpacked.
    let res = client
        .get(format!("{}/json?size=500&enc=gzip", h.origin))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert!(res.headers().get("content-encoding").is_none());
    assert_eq!(res.text().await.unwrap(), r#"{"whole":true}"#);

    let (_, conn, dir) = h.finish(2).await;
    let (path, status): (String, i64) = conn
        .query_row(
            "SELECT path, status FROM flows WHERE path LIKE '/echo%'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        path, "/echo?env=staging%20two&page=2",
        "what was sent is on record"
    );
    assert_eq!(status, 418, "and what the client got");
    clean_up(conn, dir);
}

/// The first message held, once there is one. `client` is the request that
/// should be held: if it ends first, it went through or failed, which is a
/// bug, not a slow machine. The first request through a new proxy can take
/// seconds on a busy CI runner (its leaf certificate is made then), so the
/// wait is long, but it ends as soon as there is something.
async fn first_held<T>(pauses: &Pauses, client: &tokio::task::JoinHandle<T>) -> Paused {
    let deadline = Instant::now() + WAIT_FOR;
    while Instant::now() < deadline {
        if let Some(held) = pauses.held().into_iter().next() {
            return held;
        }
        assert!(
            !client.is_finished(),
            "the request ended without being held"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("nothing was held in 30 s");
}

/// A rule's script changes a request and its response, answers one by itself,
/// and, broken, leaves traffic as it was.
#[tokio::test(flavor = "multi_thread")]
async fn scripts_change_traffic_and_a_broken_one_leaves_it_alone() {
    let h = Harness::start("scripts").await;
    let script = h.dir.join("change.js");
    std::fs::write(
        &script,
        r#"
        function onRequest(request) {
          if (request.url.includes("/mock")) {
            return { response: { status: 404, headers: { "content-type": "text/plain" },
                                 body: "mocked by " + request.method } };
          }
          request.headers["x-script"] = "yes";
          request.body = request.body.toUpperCase();
          console.log("sent", request.method, request.url);
        }
        function onResponse(request, response) {
          const echoed = JSON.parse(response.body);
          response.body = JSON.stringify({ server_saw: echoed.headers["x-script"],
                                           body: echoed.body });
          response.headers["x-changed"] = "1";
        }
        "#,
    )
    .unwrap();
    let broken = h.dir.join("broken.js");
    std::fs::write(&broken, "function onRequest(r) { r.nope.nope = 1; }").unwrap();
    h.traffic_rules.write().unwrap().rules.extend([
        rule(
            "*/echo*",
            Action::Script {
                path: script.clone(),
            },
        ),
        rule(
            "*/mock*",
            Action::Script {
                path: script.clone(),
            },
        ),
        rule(
            "*/text*",
            Action::Script {
                path: broken.clone(),
            },
        ),
    ]);
    let client = h.client(true);
    let base = format!("http://localhost:{}", h.playground_port);

    let res = client
        .post(format!("{base}/echo"))
        .body("hello")
        .send()
        .await
        .unwrap();
    assert_eq!(res.headers()["x-changed"], "1");
    let got: serde_json::Value = serde_json::from_str(&res.text().await.unwrap()).unwrap();
    assert_eq!(
        got["server_saw"], "yes",
        "the server had the script's header"
    );
    assert_eq!(got["body"], "HELLO", "and its body");

    let res = client.get(format!("{base}/mock")).send().await.unwrap();
    assert_eq!(res.status(), 404);
    assert_eq!(res.text().await.unwrap(), "mocked by GET");

    let res = client.get(format!("{base}/text")).send().await.unwrap();
    assert_eq!(res.status(), 200, "a broken script changes nothing");
    assert!(!res.text().await.unwrap().is_empty());

    let log = |path: &std::path::Path| {
        // Its scripts are the harness's own, kept by the proxy.
        h.scripts.log(path)
    };
    assert!(
        log(&script)
            .lines
            .iter()
            .any(|l| l.starts_with("sent POST"))
    );
    assert_eq!(log(&script).error, None);
    assert!(log(&broken).error.unwrap().starts_with("TypeError"));

    let (_, conn, dir) = h.finish(3).await;
    let bodies: Vec<(String, i64)> = {
        let mut stmt = conn
            .prepare("SELECT path, req_bytes FROM flows ORDER BY id")
            .unwrap();
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    };
    assert_eq!(bodies[0], ("/echo".to_owned(), 5), "the flow has what went");
    clean_up(conn, dir);
}

/// The next thing held, whatever is waiting for it.
async fn next_held(pauses: &Pauses) -> Paused {
    let deadline = Instant::now() + WAIT_FOR;
    while Instant::now() < deadline {
        if let Some(held) = pauses.held().into_iter().next() {
            return held;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("nothing was held in 30 s");
}

/// A socket's messages held each way: let go as they came, changed, or
/// dropped; what is written down is what went on.
#[tokio::test(flavor = "multi_thread")]
async fn a_sockets_messages_hold_until_let_go() {
    use hudsucker::{
        futures::{SinkExt, StreamExt},
        tokio_tungstenite::tungstenite::Message,
    };
    let h = Harness::start("ws-breakpoints").await;
    h.traffic_rules.write().unwrap().rules.extend([
        rule("*/ws*", Action::PauseMessagesToServer),
        rule("*/ws*", Action::PauseMessagesToApp),
    ]);
    let (mut ws, _) = playground_ws(&h, false, "/ws?ticks=0", &[]).await.unwrap();

    // The server's hello, held before the app has it.
    let hello = next_held(&h.pauses).await;
    assert_eq!(hello.stage, Stage::ToApp);
    assert_eq!(hello.rule, "rule for */ws*");
    assert_eq!(hello.message.method, "text");
    assert!(
        hello.message.url.ends_with("/ws?ticks=0"),
        "{}",
        hello.message.url
    );
    assert!(hello.message.headers.is_empty());
    h.pauses
        .decide(hello.id, Verdict::Send(hello.message.clone()));
    assert!(ws.next().await.unwrap().unwrap().is_text());

    // Changed on its way to the server, and its echo again on its way back.
    ws.send(Message::text("edit me")).await.unwrap();
    let up = next_held(&h.pauses).await;
    assert_eq!(up.stage, Stage::ToServer);
    assert_eq!(up.message.body, b"edit me");
    let mut edited = up.message.clone();
    edited.body = b"edited".to_vec();
    h.pauses.decide(up.id, Verdict::Send(edited));
    let echo = next_held(&h.pauses).await;
    assert_eq!(echo.stage, Stage::ToApp);
    assert_eq!(echo.message.body, b"edited", "the server had the edit");
    let mut twice = echo.message.clone();
    twice.body = b"twice".to_vec();
    h.pauses.decide(echo.id, Verdict::Send(twice));
    assert_eq!(ws.next().await.unwrap().unwrap(), Message::text("twice"));

    // Dropped: the server never has it, so nothing comes back for it.
    ws.send(Message::text("drop me")).await.unwrap();
    let dropped = next_held(&h.pauses).await;
    assert_eq!(dropped.message.body, b"drop me");
    h.pauses.decide(dropped.id, Verdict::Drop);
    // Binary stays binary, though it is not text.
    ws.send(Message::binary(vec![0, 159, 146, 150]))
        .await
        .unwrap();
    let up = next_held(&h.pauses).await;
    assert_eq!(up.message.method, "binary");
    h.pauses.decide(up.id, Verdict::Send(up.message.clone()));
    let echo = next_held(&h.pauses).await;
    h.pauses.release_all();
    assert_eq!(echo.message.body, [0, 159, 146, 150], "not \"drop me\"");
    assert_eq!(
        ws.next().await.unwrap().unwrap(),
        Message::binary(vec![0, 159, 146, 150])
    );

    // The rules gone, the next messages go straight through.
    h.traffic_rules.write().unwrap().rules.clear();
    ws.close(None).await.unwrap();
    let deadline = Instant::now() + WAIT_FOR;
    loop {
        h.pauses.release_all();
        match tokio::time::timeout(Duration::from_millis(50), ws.next()).await {
            Ok(None) => break,
            _ => assert!(Instant::now() < deadline, "the close"),
        }
    }

    let (_, conn, dir) = h.finish(1).await;
    let written: Vec<(bool, i64)> = {
        let mut stmt = conn
            .prepare("SELECT from_client, bytes FROM messages WHERE kind IN ('text', 'binary') ORDER BY id")
            .unwrap();
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .skip(1)
            .collect()
    };
    assert_eq!(
        written,
        [(true, 6), (false, 5), (true, 4), (false, 4)],
        "after the hello: edited, twice, then binary both ways; nothing dropped"
    );
    clean_up(conn, dir);
}

/// Breakpoints through the real proxy: a request changed on its way, a response
/// changed on its way back, and one dropped.
#[tokio::test(flavor = "multi_thread")]
async fn breakpoints_hold_until_let_go() {
    let h = Harness::start("breakpoints").await;
    h.traffic_rules.write().unwrap().rules.extend([
        rule("*/echo?size=5*", Action::PauseRequest),
        rule("*/json?size=6*", Action::PauseResponse),
        rule("*/json?size=7*", Action::PauseRequest),
    ]);
    let client = h.client(false);

    // The request: sent to another size, with another body. To /echo, which
    // reads the body before it answers: a body a server does not read can be
    // let go before it is sent, and then no one has it.
    let sent = tokio::spawn({
        let client = client.clone();
        let url = format!("{}/echo?size=50", h.origin);
        async move {
            client
                .post(url)
                .body("as sent")
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        }
    });
    let held = first_held(&h.pauses, &sent).await;
    assert_eq!(held.stage, Stage::Request);
    assert_eq!(held.rule, "rule for */echo?size=5*");
    assert_eq!(held.message.method, "POST");
    assert_eq!(held.message.body, b"as sent");
    let mut edited = held.message.clone();
    edited.url = edited.url.replace("size=50", "size=90");
    edited.body = b"as edited".to_vec();
    assert!(h.pauses.decide(held.id, Verdict::Send(edited)));
    let echoed = sent.await.unwrap();
    assert!(
        echoed.contains("\nquery=size=90\n"),
        "the edited URL was asked for: {echoed}"
    );
    assert!(
        echoed.ends_with("body=as edited"),
        "with the edited body: {echoed}"
    );

    // The response: another status and body.
    let got = tokio::spawn({
        let client = client.clone();
        let url = format!("{}/json?size=60", h.origin);
        async move {
            let res = client.get(url).send().await.unwrap();
            (res.status().as_u16(), res.text().await.unwrap())
        }
    });
    let held = first_held(&h.pauses, &got).await;
    assert_eq!(held.stage, Stage::Response);
    assert_eq!(held.message.status, 200);
    assert_eq!(held.message.body.len(), 60);
    let mut edited = held.message.clone();
    edited.status = 201;
    edited.body = b"changed".to_vec();
    assert!(h.pauses.decide(held.id, Verdict::Send(edited)));
    assert_eq!(got.await.unwrap(), (201, "changed".to_owned()));

    // Dropped: the server never hears of it.
    let dropped = tokio::spawn({
        let client = client.clone();
        let url = format!("{}/json?size=70", h.origin);
        async move {
            let res = client.get(url).send().await.unwrap();
            (res.status().as_u16(), res.text().await.unwrap())
        }
    });
    let held = first_held(&h.pauses, &dropped).await;
    assert!(h.pauses.decide(held.id, Verdict::Drop));
    let (status, text) = dropped.await.unwrap();
    assert_eq!(status, 502);
    assert!(text.contains("dropped at a breakpoint"), "{text}");

    let (_, conn, dir) = h.finish(3).await;
    let rows: Vec<(String, Option<i64>, Option<String>)> = conn
        .prepare("SELECT path, status, server_version FROM flows ORDER BY started_us, id")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(rows[0].0, "/echo?size=90", "what was sent is on record");
    assert_eq!(rows[1].1, Some(201), "what the client got is on record");
    assert_eq!(
        (rows[2].1, rows[2].2.as_deref()),
        (Some(502), None),
        "no server was asked"
    );
    let edited: i64 = conn
        .query_row(
            "SELECT id FROM flows WHERE path = '/echo?size=90'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        request_of(&conn, &dir, edited).unwrap(),
        b"as edited",
        "the edited body is on record"
    );
    clean_up(conn, dir);
}

/// A server that answers before reading a request's body: the client's
/// body is read on and kept all the same, as it was sent.
#[tokio::test(flavor = "multi_thread")]
async fn a_body_the_server_does_not_read_is_kept() {
    let h = Harness::start("unread-body").await;
    let client = h.client(false);
    let late = hudsucker::futures::stream::once(async {
        tokio::time::sleep(Duration::from_millis(200)).await;
        Ok::<_, std::io::Error>(hyper::body::Frame::data(bytes::Bytes::from_static(
            b"sent late",
        )))
    });
    let res = client
        .post(format!("{}/json?size=10", h.origin))
        .body(reqwest::Body::wrap(http_body_util::StreamBody::new(late)))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200, "answered before the body came");
    res.bytes().await.unwrap();
    let (_, conn, dir) = h.finish(1).await;
    let (bytes, complete, error): (i64, bool, Option<String>) = conn
        .query_row(
            "SELECT req_bytes, req_complete, error FROM flows",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!((bytes, complete, error), (9, true, None));
    assert_eq!(request_of(&conn, &dir, 1).unwrap(), b"sent late");
    clean_up(conn, dir);
}

/// A large upload a server will not read is read on only so far, and the
/// client still has its answer.
#[tokio::test(flavor = "multi_thread")]
async fn an_unread_upload_is_read_on_only_so_far() {
    const MIB: usize = 1 << 20;
    let h = Harness::start("unread-upload").await;
    let client = h.client(false);
    let chunks = hudsucker::futures::stream::iter((0..32).map(|_| {
        Ok::<_, std::io::Error>(hyper::body::Frame::data(bytes::Bytes::from(vec![
            b'u';
            MIB
        ])))
    }));
    let started = std::time::Instant::now();
    let res = client
        .post(format!("{}/json?size=10", h.origin))
        .body(reqwest::Body::wrap(http_body_util::StreamBody::new(chunks)))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    res.bytes().await.unwrap();
    let (_, conn, dir) = h.finish(1).await;
    assert!(started.elapsed() < Duration::from_secs(10), "not held up");
    let (bytes, complete, error): (i64, bool, Option<String>) = conn
        .query_row(
            "SELECT req_bytes, req_complete, error FROM flows",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert!(bytes as usize <= 18 * MIB, "read on so far: {bytes}");
    assert!(!complete, "and said to be cut");
    assert_eq!(error, None, "which is no error of the flow");
    clean_up(conn, dir);
}

/// Paused, rules still apply: they are about the app under test, not the recording.
#[tokio::test(flavor = "multi_thread")]
async fn rules_apply_while_paused() {
    let h = Harness::start("rules-paused").await;
    h.traffic_rules
        .write()
        .unwrap()
        .rules
        .push(rule("blocked.test", Action::Block { status: 451 }));
    h.recording.store(false, Ordering::Relaxed);
    let res = h
        .client(false)
        .get("https://blocked.test/")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 451);
    let (summary, conn, dir) = h.finish(0).await;
    assert_eq!(summary.written, 0);
    clean_up(conn, dir);
}

/// A connection taken on its way — what a local-mode source will hand over,
/// from an app that was never told a proxy exists — is recorded like any
/// other, as the program the source named and from the address it really came
/// from rather than the loopback it reaches the proxy over.
#[tokio::test(flavor = "multi_thread")]
async fn a_taken_connection_is_recorded_as_the_app_that_made_it() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let h = Harness::start("taken").await;
    let port = h.playground_port;
    let (mut client, its_end) = tokio::io::duplex(64 * 1024);
    h.taken
        .send(Flow {
            stream: Box::new(its_end),
            client: "192.168.1.13:51234".parse().unwrap(),
            to: Target::Named {
                host: "localhost".to_owned(),
                port,
            },
            who: Some(tracewing_store::app::App {
                pid: 4123,
                name: "node".to_owned(),
                path: Some("/usr/bin/node".to_owned()),
            }),
        })
        .await
        .unwrap();
    // It asks for no proxy and says no CONNECT: it just starts talking.
    client
        .write_all(b"GET /json HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let mut said = Vec::new();
    client.read_to_end(&mut said).await.unwrap();
    let said = String::from_utf8_lossy(&said);
    assert!(said.starts_with("HTTP/1.1 200"), "{said}");

    let (summary, conn, dir) = h.finish(1).await;
    assert_eq!(summary.written, 1);
    let (client_addr, app, pid, path, host): (String, Option<String>, Option<i64>, String, String) =
        conn.query_row(
            "SELECT client, app, app_pid, path, host FROM flows",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .unwrap();
    assert_eq!(
        client_addr, "192.168.1.13:51234",
        "the client the source saw, not the loopback the handover came over"
    );
    assert_eq!(app.as_deref(), Some("node"), "named by the source");
    assert_eq!(pid, Some(4123), "and not looked for by address");
    assert_eq!(path, "/json");
    assert_eq!(host, format!("localhost:{port}"));
    clean_up(conn, dir);
}

/// Why a taken connection is not wrapped in `CONNECT`: inside a tunnel the
/// proxy reads TLS and a plain `GET`, and tunnels the rest blind, so this
/// `POST` would have gone through unseen. It is recorded with its body, and
/// the playground's echo proves the server was asked properly too.
///
/// It hands over a bare address rather than a name, as a source that deals in
/// packets would, so the flow is named by that address.
#[tokio::test(flavor = "multi_thread")]
async fn a_taken_plain_post_is_recorded_body_and_all() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let h = Harness::start("taken-post").await;
    let port = h.playground_port;
    let (mut client, its_end) = tokio::io::duplex(64 * 1024);
    h.taken
        .send(Flow {
            stream: Box::new(its_end),
            client: "10.0.0.7:4000".parse().unwrap(),
            to: Target::Addr(format!("127.0.0.1:{port}").parse().unwrap()),
            who: None,
        })
        .await
        .unwrap();
    let body = r#"{"order":"X-9901"}"#;
    client
        .write_all(
            format!(
                "POST /echo HTTP/1.1\r\nHost: api.test\r\nContent-Type: application/json\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .as_bytes(),
        )
        .await
        .unwrap();
    let mut said = Vec::new();
    client.read_to_end(&mut said).await.unwrap();
    let said = String::from_utf8_lossy(&said);
    assert!(said.starts_with("HTTP/1.1 200"), "{said}");
    assert!(
        said.contains(r#"\"order\":\"X-9901\""#),
        "the server was asked with the body, and echoed it: {said}"
    );

    let (summary, conn, dir) = h.finish(1).await;
    assert_eq!(summary.written, 1, "the POST was recorded, not tunnelled");
    let (method, host, path, client_addr, req_bytes): (String, String, String, String, i64) = conn
        .query_row(
            "SELECT method, host, path, client, req_bytes FROM flows",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .unwrap();
    assert_eq!(method, "POST");
    assert_eq!(path, "/echo");
    assert_eq!(client_addr, "10.0.0.7:4000");
    assert_eq!(
        host,
        format!("127.0.0.1:{port}"),
        "named by the address the source handed over, not by the Host header"
    );
    assert_eq!(
        req_bytes,
        body.len() as i64,
        "the body was read, not missed"
    );
    let id: i64 = conn
        .query_row("SELECT id FROM flows", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        request_of(&conn, &dir, id).as_deref(),
        Some(body.as_bytes()),
        "and kept"
    );
    clean_up(conn, dir);
}

/// The other way in. A taken connection that starts a TLS handshake is given
/// a tunnel on the client's behalf, because that is the only way the proxy
/// will terminate TLS. The client here trusts nothing but the proxy's own
/// authority, so a `200` means the proxy decrypted it rather than passing it
/// along.
#[tokio::test(flavor = "multi_thread")]
async fn a_taken_tls_connection_is_decrypted() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio_rustls::rustls::{ClientConfig, RootCertStore, pki_types::ServerName};
    let h = Harness::start("taken-tls").await;
    let port = h.playground_tls_port;
    let (client, its_end) = tokio::io::duplex(64 * 1024);
    h.taken
        .send(Flow {
            stream: Box::new(its_end),
            client: "10.0.0.7:4001".parse().unwrap(),
            to: Target::Named {
                host: "localhost".to_owned(),
                port,
            },
            who: None,
        })
        .await
        .unwrap();

    let mut roots = RootCertStore::empty();
    for cert in CertificateDer::pem_file_iter(h.dir.join("ca/ca.pem")).unwrap() {
        roots.add(cert.unwrap()).unwrap();
    }
    let config = ClientConfig::builder_with_provider(Arc::new(tracewing_proxy::crypto_provider()))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let mut tls = tokio_rustls::TlsConnector::from(Arc::new(config))
        .connect(ServerName::try_from("localhost").unwrap(), client)
        .await
        .expect("the proxy's certificate, for a connection it was handed");
    tls.write_all(b"GET /json HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let mut said = Vec::new();
    tls.read_to_end(&mut said).await.unwrap();
    let said = String::from_utf8_lossy(&said);
    assert!(said.starts_with("HTTP/1.1 200"), "{said}");

    let (summary, conn, dir) = h.finish(1).await;
    assert_eq!(summary.written, 1);
    let (scheme, host, path, client_addr): (String, String, String, String) = conn
        .query_row("SELECT scheme, host, path, client FROM flows", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })
        .unwrap();
    assert_eq!(scheme, "https", "decrypted, not tunnelled");
    assert_eq!(host, format!("localhost:{port}"));
    assert_eq!(path, "/json");
    assert_eq!(client_addr, "10.0.0.7:4001");
    clean_up(conn, dir);
}

/// A client told to use tracewing's SOCKS5 port, speaking RFC 1928 by hand so
/// that the test is a second implementation of the handshake rather than the
/// same one twice. It asks for a name, and the flow is recorded under that
/// name: a SOCKS5 client says where it is going, so nothing has to be read
/// out of a ClientHello to find out.
#[tokio::test(flavor = "multi_thread")]
async fn a_socks5_client_is_recorded_by_the_name_it_asked_for() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let h = Harness::start("socks").await;
    let port = h.playground_port;
    let mut client = socks5_to(h.socks_port, "localhost", port).await;

    // And from here it simply talks, as it would to the server itself.
    client
        .write_all(b"GET /json HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let mut said = Vec::new();
    client.read_to_end(&mut said).await.unwrap();
    let said = String::from_utf8_lossy(&said);
    assert!(said.starts_with("HTTP/1.1 200"), "{said}");

    let (summary, conn, dir) = h.finish(1).await;
    assert_eq!(summary.written, 1);
    let (host, path, method): (String, String, String) = conn
        .query_row("SELECT host, path, method FROM flows", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .unwrap();
    assert_eq!(
        host,
        format!("localhost:{port}"),
        "the name the client asked for, not the address behind it"
    );
    assert_eq!(path, "/json");
    assert_eq!(method, "GET");
    clean_up(conn, dir);
}

/// A SOCKS5 client's handshake, written by hand: the greeting, then `CONNECT`
/// to a name. What comes back is the socket, with the conversation left to the
/// caller, which is exactly what a SOCKS5 client has at this point.
async fn socks5_to(socks_port: u16, host: &str, port: u16) -> tokio::net::TcpStream {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut client = tokio::net::TcpStream::connect(("127.0.0.1", socks_port))
        .await
        .unwrap();

    // Version 5, one way in offered, and that way is no authentication.
    client.write_all(&[5, 1, 0]).await.unwrap();
    let mut agreed = [0u8; 2];
    client.read_exact(&mut agreed).await.unwrap();
    assert_eq!(agreed, [5, 0], "version 5, no authentication");

    // CONNECT, to the name and port asked for.
    let mut request = vec![5, 1, 0, 3, host.len() as u8];
    request.extend_from_slice(host.as_bytes());
    request.extend_from_slice(&port.to_be_bytes());
    client.write_all(&request).await.unwrap();
    let mut answer = [0u8; 10];
    client.read_exact(&mut answer).await.unwrap();
    assert_eq!(answer[..2], [5, 0], "it may go ahead: {answer:02x?}");
    client
}

/// The program behind a SOCKS5 connection is found, which it only is because
/// `apps` is told that the SOCKS5 port is one of tracewing's own ways in: the
/// client's socket goes to that port, not to the proxy's.
///
/// The server is asked to take its time, because the program is looked for
/// while the request is on its way and the flow keeps only what was found by
/// the time the answer comes back.
#[tokio::test]
async fn a_socks5_flow_says_which_program_sent_it() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let h = Harness::start("socks-app").await;
    let mut client = socks5_to(h.socks_port, "localhost", h.playground_port).await;
    client
        .write_all(b"GET /slow?ms=300 HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let mut said = Vec::new();
    client.read_to_end(&mut said).await.unwrap();
    let said = String::from_utf8_lossy(&said);
    assert!(said.starts_with("HTTP/1.1 200"), "{said}");

    let (_, conn, dir) = h.finish(1).await;
    let (app, pid, path): (Option<String>, Option<i64>, Option<String>) = conn
        .query_row("SELECT app, app_pid, app_path FROM flows", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .unwrap();
    // This test is the program that sent it.
    assert_eq!(pid, Some(i64::from(std::process::id())));
    assert!(
        app.is_some_and(|app| !app.is_empty()),
        "a name for it: {path:?}"
    );
    clean_up(conn, dir);
}
