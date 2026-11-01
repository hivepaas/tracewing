//! End to end: client → proxy (MITM) → HTTPS origin, all in process. The
//! client trusts only the proxy's CA, so a successful response means the
//! proxy terminated TLS itself.

use rusqlite::Connection;
use std::{
    path::PathBuf,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};
use tracewing_dev::benchkit;
use tracewing_proxy::{
    ProxyOptions, ProxySummary,
    capture::{CHECK_HOST, Check},
    rules::HostRules,
    run_proxy,
};
use tracewing_store::{
    search,
    session::{self, BodyReader, stored_body},
    settings::StorageSettings,
};

struct Harness {
    recording: Arc<AtomicBool>,
    rules: Arc<RwLock<HostRules>>,
    check: Arc<Check>,
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

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_url = format!("http://{}", listener.local_addr().unwrap());
        let (stop, stopped) = oneshot::channel::<()>();
        let recording = Arc::new(AtomicBool::new(true));
        let rules = Arc::new(RwLock::new(HostRules {
            no_decrypt: Vec::new(),
        }));
        let check = Arc::new(Check::default());
        let proxy = tokio::spawn(run_proxy(
            ProxyOptions {
                listener: Some(listener),
                std_listener: None,
                data_dir: dir.join("ca"),
                session: dir.join("session"),
                settings,
                extra_roots: vec![origin_ca],
                queue: 10_000,
                stats_every: None,
                shared: Default::default(),
                recording: Arc::clone(&recording),
                rules: Arc::clone(&rules),
                check: Arc::clone(&check),
                leave_index: false,
                stop_limit: Duration::from_secs(1),
            },
            async {
                let _ = stopped.await;
            },
        ));
        while !dir.join("ca/ca.pem").exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        Self {
            recording,
            rules,
            check,
            dir,
            proxy_url,
            origin: format!("https://localhost:{origin_port}"),
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
        let deadline = Instant::now() + Duration::from_secs(5);
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
    assert_eq!(flows[0].0, "HTTP/1.1");
    assert_eq!(flows[1].0, "HTTP/2.0");
    for (i, (_, path, status, resp_bytes, truncated)) in flows.iter().enumerate() {
        assert_eq!(path, "/json?size=300");
        assert_eq!(*status, Some(200));
        assert_eq!(*resp_bytes, 300);
        let body = response_of(&conn, &dir, i as i64 + 1).unwrap();
        assert_eq!(body.len(), 300);
        assert!(body.starts_with(br#"{"data":"#));
        assert_eq!(*truncated, 0);
    }
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

    let (_, conn, dir) = h.finish(2).await;
    let body = String::from_utf8(response_of(&conn, &dir, 2).unwrap()).unwrap();
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
    assert!(
        response_of(&conn, &dir, 2).unwrap() == sent,
        "and a viewer finds it"
    );
    assert_eq!(response_of(&conn, &dir, 1).unwrap(), small);

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
    // 240 MB in all, some in the log and most in files of their own.
    for i in 0..1200 {
        let size = if i % 2 == 0 { 32_000 } else { 370_000 };
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
    assert_eq!(count, 1200 - summary.trimmed as i64);
    assert!(
        response_of(&conn, &dir, last).is_some(),
        "the newest is there"
    );
    let files = std::fs::read_dir(session::files_dir(&session_dir))
        .unwrap()
        .count();
    assert!(
        files < 200,
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
            "SELECT resp_len, resp_zstd FROM flows WHERE id = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!((len, ours), (500, false), "compress_stored");

    let (lost, truncated): (bool, bool) = conn
        .query_row(
            "SELECT resp_spool_lost, resp_truncated FROM flows WHERE id = 2",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert!(lost && truncated, "body_max_mib");
    let kept = response_of(&conn, &dir, 2).unwrap().len();
    assert!(kept <= 1 << 20 && kept > 0, "body_max_mib, kept {kept}");

    let session_dir = dir.join("session");
    assert_eq!(
        search::find(&session_dir, "urls", "size=500", 1).unwrap(),
        [1]
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
        .prepare("SELECT resp_bytes FROM flows ORDER BY id")
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
        started.elapsed() < Duration::from_secs(4),
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

    let (summary, conn, dir) = h.finish(0).await;
    assert_eq!(summary.written, 0, "the test is not traffic");
    clean_up(conn, dir);
}
