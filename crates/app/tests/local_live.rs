//! The local way in, on a real Windows machine: assembled, and then carrying
//! traffic.
//!
//! Everything else about this way in is tested where it can be: the sieve's
//! rules, the switch's states, the tab's pictures. What no Linux test reaches
//! is the assembly itself on Windows -- `Apps::spawn`, the adapter coming up
//! with its address and its two `/1` halves, the sieve and the bind and the
//! echo going into one `Tun`, and the whole of it coming down again -- and
//! then what it is for (docs/design/e2e.md, S8):
//!
//! - a chosen program's HTTPS goes into the adapter, is kept by the sieve,
//!   and is recorded decrypted, while the same request from a program nobody
//!   chose is not;
//! - what nobody chose goes on as it did before the mode was on: HTTPS with
//!   the server's own certificate, DNS over UDP, `ping`;
//! - a chosen program's HTTP/3 is caught and recorded, from this test's own
//!   QUIC client and from Chrome's.
//!
//! The mode is turned on as the window turns it on: an [`Engine`], then
//! [`local_source::start`], then the source handed to the engine's
//! `serving`. The way out is pinned to the machine's own interface, as it is
//! for anybody (unlike `local_live_http3` on Linux, whose namespace has no
//! interface to pin to).
//!
//! The clients send to `203.0.113.9`, a documentation address the adapter's
//! `/1` routes take, under a name whose server is on this machine's loopback:
//! what a client sends to and what the proxy reaches are two places, which is
//! the shape of a captured flow.
//!
//! `#[ignore]`d because it needs an administrator and `wintun.dll`, and
//! because it puts an adapter on the machine for as long as it runs. One at a
//! time, as they share one adapter by a fixed GUID:
//!
//! ```powershell
//! $env:WINTUN_DLL = "$HOME\wintun-dist\wintun\bin\amd64\wintun.dll"
//! cargo test -p tracewing --release --test local_live -- --ignored --nocapture --test-threads=1
//! ```
//!
//! The Chrome test also adds `h3.test` to this machine's hosts file for as
//! long as it runs, which `scripts/e2e_windows.ps1 -Wintun` allows on a
//! machine marked for these tests.

#![cfg(windows)]

use std::{
    net::{SocketAddr, UdpSocket},
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::{Duration, Instant},
};
use tracewing::{
    engine::{Engine, EngineOptions},
    local_apps::{Local, Picked},
    local_source::{self, Http3, Stopper},
};
use tracewing_dev::over_http3::{self, SAID};
use tracewing_proxy::{ca::LocalCa, ca_key::KeyStore, crypto_provider, roads, source::By};
use tracewing_store::settings::StorageSettings;

/// Where the clients send: a documentation address, which the adapter's
/// routes take and nothing answers. Only tracewing can make it reach a
/// server.
const AS_IF_THE_SERVER: &str = "203.0.113.9";

/// The curl every Windows since 1803 has, which is the program chosen.
const CURL: &str = r"C:\Windows\System32\curl.exe";

/// How many ways out of ours the table holds at `bits`, in either family.
fn ours_at(luid_rows: &[roads::Road], bits: u8) -> usize {
    luid_rows.iter().filter(|road| road.bits == bits).count()
}

/// Rows belonging to an interface whose LUID is not any of `theirs`, which is
/// how the adapter's own are found without being told its LUID.
fn new_rows(before: &[roads::Road], now: &[roads::Road]) -> Vec<roads::Road> {
    let was: Vec<u64> = before.iter().map(|road| road.luid).collect();
    now.iter()
        .filter(|road| !was.contains(&road.luid))
        .copied()
        .collect()
}

/// The whole assembly: it comes up, it is on the machine, and it goes away.
#[test]
#[ignore = "needs Administrator and wintun.dll, and puts an adapter on this machine"]
fn the_local_way_in_comes_up_and_goes_away_again() {
    let dir = made("comes-up");
    let before = roads::roads_now().expect("this machine's routes");

    let local = Local {
        on: true,
        // An empty list keeps nothing, which would test less.
        programs: vec![Picked::at(CURL.into())],
    };
    let started = local_source::start(&local, vec![9090], &dir, &local_source::Http3::off())
        .expect("the way in assembles")
        .expect("the switch is on, so there is a source");

    let now = roads::roads_now().expect("this machine's routes");
    let ours = new_rows(&before, &now);
    println!(
        "the adapter brought {} row(s), {} of them /1",
        ours.len(),
        ours_at(&ours, 1)
    );
    for road in &ours {
        println!(
            "  if {} bits {} metric {} v6 {} connected {}",
            road.interface, road.bits, road.metric, road.v6, road.connected
        );
    }
    assert_eq!(
        ours_at(&ours, 1),
        4,
        "both halves of each address space: that is what makes the machine route into it"
    );
    // And it is never the way out, which is the loop this mode has to avoid.
    let luid = ours.first().expect("rows of our own").luid;
    for v6 in [false, true] {
        let out = roads::out_by(&now, luid, v6);
        println!("way out for {}: {out:?}", if v6 { "IPv6" } else { "IPv4" });
        if let Some(out) = out {
            assert!(
                !ours.iter().any(|road| road.interface == out),
                "the way out is our own adapter, interface {out}"
            );
        }
    }

    // Ending it the way the window does.
    started.stopper.stop();
    drop(started);
    let after = roads::roads_now().expect("this machine's routes");
    assert!(
        new_rows(&before, &after).is_empty(),
        "the adapter's rows are still in the table after it was stopped and dropped"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A chosen program's HTTPS is recorded through the adapter, decrypted, and
/// the same request from a program nobody chose is not; and what nobody chose
/// keeps working as it did before the mode was on.
#[test]
#[ignore = "needs Administrator and wintun.dll, and puts an adapter on this machine"]
fn a_chosen_program_is_recorded_and_the_rest_go_on_as_before() {
    let dir = made("chosen");
    // The same curl, somewhere else: to the sieve, another program.
    let other = dir.join("elsewhere").join("curl.exe");
    std::fs::create_dir_all(other.parent().unwrap()).unwrap();
    std::fs::copy(CURL, &other).expect("a copy of curl");

    // What this machine reaches with the mode off. What it reaches then it
    // must still reach with the mode on; what it does not (a runner that
    // refuses ICMP, a machine with no internet) is said and not asked.
    let reached_before = Reached::now(&other);
    println!("before the mode is on: {reached_before:?}");

    let runtime = runtime();
    let origin = runtime.block_on(Origin::start(&dir));
    let roads_before = roads::roads_now().expect("this machine's routes");
    let on = On::start(
        &dir,
        vec![PathBuf::from(CURL)],
        std::slice::from_ref(&origin.ca),
    );

    // The chosen curl, sent into the adapter under the origin's name.
    let marker = format!("through-the-adapter-{}", std::process::id());
    let url = format!("https://localhost:{}/echo", origin.port);
    let out = asked_with(CURL, &on, origin.port, &marker, &url);
    assert!(
        out.status.success() && String::from_utf8_lossy(&out.stdout).contains(&marker),
        "the chosen curl's request through the adapter: {}",
        said(&out)
    );
    let har = on.har_with(&marker, Duration::from_secs(15));
    assert!(
        har.matches(marker.as_str()).count() >= 2,
        "the HAR has not got both what was sent and what came back: {har}"
    );

    // The other curl, the same request: passed straight on to the address it
    // asked for, where nothing answers, and nothing of it recorded.
    let unchosen = format!("nobody-chose-this-{}", std::process::id());
    let out = asked_with(other.to_str().unwrap(), &on, origin.port, &unchosen, &url);
    assert!(
        !out.status.success(),
        "a program nobody chose reached the origin, so it was kept: {}",
        said(&out)
    );
    // The same wait as for the chosen one, which was enough for it.
    std::thread::sleep(Duration::from_secs(3));
    let har = on.har();
    assert!(
        !har.contains(&unchosen),
        "a program nobody chose was recorded: {har}"
    );

    // And everything else as before.
    let reached_now = Reached::now(&other);
    println!("with the mode on: {reached_now:?}");
    reached_now.at_least(&reached_before);
    let har = on.har();
    assert!(
        !har.contains("example.com"),
        "the other curl's HTTPS was recorded: {har}"
    );
    assert!(on.engine.arrivals.of(By::Tun).count >= 1, "by the way in");

    on.stop();
    let after = roads::roads_now().expect("this machine's routes");
    assert!(
        new_rows(&roads_before, &after).is_empty(),
        "the adapter's rows are still in the table after the mode went off"
    );
    drop(origin);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A chosen program's HTTP/3, caught at the adapter and read: this test's
/// own QUIC client, chosen, reaches an HTTP/3 server through tracewing, and
/// the request is recorded.
#[test]
#[ignore = "needs Administrator and wintun.dll, and puts an adapter on this machine"]
fn a_chosen_program_s_http3_is_recorded_through_the_adapter() {
    let dir = made("http3");
    let runtime = runtime();
    // The program that sends the QUIC is this test binary.
    let on = On::start(
        &dir,
        vec![std::env::current_exe().expect("this test binary")],
        &[],
    );
    let ca = Arc::clone(&on.ca);
    let (answer, saw) = runtime.block_on(async {
        let server = over_http3::Server::start(&ca, "localhost").expect("an HTTP/3 server");
        on.door().await;
        let answer = tokio::time::timeout(
            Duration::from_secs(10),
            over_http3::ask(
                format!("{AS_IF_THE_SERVER}:443").parse().unwrap(),
                "localhost",
                &format!("https://localhost:{}/orders", server.port),
                over_http3::trusting(&ca).expect("the CA's own certificate"),
            ),
        )
        .await
        .unwrap_or_else(|_| Err(anyhow::anyhow!("nothing came back in time")));
        (answer, server.saw())
    });
    let caught = on.engine.arrivals.of(By::Tun).count;
    let (status, body) = answer.unwrap_or_else(|why| {
        panic!(
            "no answer through the adapter: {why:#}; caught by the way in: {caught}; reached the server: {:?}",
            saw.iter().map(|a| &a.path).collect::<Vec<_>>()
        )
    });
    assert_eq!(status, http::StatusCode::OK);
    assert_eq!(body, SAID, "the server's own body, not the proxy's");
    assert_eq!(
        saw.first().map(|arrived| arrived.path.as_str()),
        Some("/orders"),
        "the real server is where it arrived"
    );
    assert!(caught >= 1, "by the way in that caught it");
    let har = on.har_with("/orders", Duration::from_secs(15));
    assert!(over_http3_in(&har), "recorded as HTTP/3: {har}");
    on.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

/// Chrome's own HTTP/3 through the adapter: a real browser, told to speak
/// QUIC to a name that resolves where the adapter's routes go, reaches the
/// server through tracewing, as `local_live_http3` checks on Linux.
///
/// Chrome trusts tracewing's leaves by their key
/// (`--ignore-certificate-errors-spki-list`), as Set up starts it. The proxy
/// finds `h3.test` on the loopback by a line of this machine's hosts file,
/// there for as long as the test runs.
#[test]
#[ignore = "needs Administrator, wintun.dll and Chrome, and puts an adapter on this machine"]
fn chromes_http3_goes_through_the_adapter() {
    let Some(chrome) = [
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    ]
    .into_iter()
    .map(PathBuf::from)
    .find(|path| path.exists()) else {
        eprintln!("skipped: no Chrome here");
        return;
    };
    // The hosts file is this machine's: changed only where it is marked as
    // one the tests may change (crates/e2e).
    if !Path::new(r"C:\tracewing-e2e-throwaway").exists() {
        eprintln!("skipped: not a machine marked for tests that change it");
        return;
    }
    let dir = made("chrome-http3");
    let _hosts = Hosts::add("h3.test");
    let runtime = runtime();
    let on = On::start(&dir, vec![chrome.clone()], &[]);
    let ca = Arc::clone(&on.ca);
    let server = runtime
        .block_on(async { over_http3::Server::start_at(&ca, "h3.test", 443) })
        .expect("an HTTP/3 server on 443");
    runtime.block_on(on.door());

    let profile = dir.join("profile");
    let mut browser = Command::new(&chrome)
        .args([
            "--headless=new",
            "--disable-gpu",
            "--no-first-run",
            "--no-default-browser-check",
            "--enable-quic",
            "--enable-logging=stderr",
            "--vmodule=*quic*=1",
            "--origin-to-force-quic-on=h3.test:443",
        ])
        .arg(format!(
            "--host-resolver-rules=MAP h3.test {AS_IF_THE_SERVER}"
        ))
        .arg(format!(
            "--ignore-certificate-errors-spki-list={}",
            on.leaf_spki()
        ))
        .arg(format!("--user-data-dir={}", profile.display()))
        // What Chrome's network stack did, kept with the rest when it fails.
        .arg(format!(
            "--log-net-log={}",
            dir.join("netlog.json").display()
        ))
        .args(["--dump-dom", "https://h3.test/orders"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("start Chrome");
    let deadline = Instant::now() + Duration::from_secs(60);
    while browser.try_wait().expect("Chrome").is_none() {
        if Instant::now() > deadline {
            let _ = browser.kill();
            panic!("Chrome did not finish in 60 s");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let mut page = String::new();
    std::io::Read::read_to_string(&mut browser.stdout.take().unwrap(), &mut page).unwrap();
    let mut said = String::new();
    std::io::Read::read_to_string(&mut browser.stderr.take().unwrap(), &mut said).unwrap();
    // All of it, kept beside the test's other files, which stay when it
    // fails: the lines below are only what a failure says first.
    let kept = dir.join("chrome-stderr.log");
    std::fs::write(&kept, &said).unwrap();
    let why: Vec<&str> = said
        .lines()
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            line.contains("Page load failed")
                || line.contains("net::ERR_")
                || (lower.contains("quic")
                    && ["error", "close", "fail"].iter().any(|w| lower.contains(w)))
        })
        .collect();
    // Where the chain broke, before whether it did: nothing caught is the
    // routes or the sieve, caught but unseen by the server is the door or
    // its way out, seen with an empty page is the way back.
    let caught = on.engine.arrivals.of(By::Tun).count;
    let seen: Vec<String> = server.saw().iter().map(|a| a.path.clone()).collect();
    assert!(
        page.contains(std::str::from_utf8(SAID).unwrap()),
        "Chrome did not get the server's own body.\n  its page: {page:?}\n  its reasons: {why:?}\n  all it said: {}\n  caught by the way in: {caught}\n  reached the real server: {seen:?}",
        kept.display()
    );
    assert_eq!(
        seen.first().map(String::as_str),
        Some("/orders"),
        "the real server is where it arrived"
    );
    let har = on.har_with("h3.test", Duration::from_secs(15));
    assert!(over_http3_in(&har), "recorded as HTTP/3: {har}");
    drop(server);
    on.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

/// The mode on, as the window turns it on, keeping `kept`.
struct On {
    engine: Engine,
    stopper: Stopper,
    ca: Arc<LocalCa>,
}

impl On {
    /// `trusted` are the roots of the servers the way out is to reach,
    /// besides tracewing's own (which the HTTP/3 servers' leaves are from).
    fn start(dir: &Path, kept: Vec<PathBuf>, trusted: &[PathBuf]) -> Self {
        let _ = crypto_provider().install_default();
        // What the proxy says, among it why a way in did not start: it is
        // said in the log and nowhere else.
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .with_writer(std::io::stderr)
            .try_init();
        let data_dir = dir.join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        // Made before the engine, which loads the same one: the servers'
        // leaves come from it, and the way out has to trust it as it starts.
        let ca = Arc::new(
            LocalCa::load_or_create(&data_dir, crypto_provider(), KeyStore::File)
                .expect("an authority of our own"),
        );
        let mut extra_roots = vec![data_dir.join("ca.pem")];
        extra_roots.extend_from_slice(trusted);
        let engine = Engine::start(EngineOptions {
            data_dir: data_dir.clone(),
            settings: StorageSettings {
                disk_reserve_mib: 1,
                capture_http3: true,
                ..StorageSettings::default()
            },
            rules: Default::default(),
            traffic_rules: Default::default(),
            ports: 20_700..20_710,
            extra_roots,
            key_store: KeyStore::File,
            // The whole question is which program sent what.
            find_apps: true,
        })
        .expect("the engine");
        let local = Local {
            on: true,
            programs: kept.into_iter().map(Picked::at).collect(),
        };
        let http3 = Http3 {
            live: Arc::clone(&engine.live),
            door: Arc::clone(&engine.http3),
            handed: Arc::clone(&engine.handed),
        };
        let started = local_source::start(&local, vec![engine.addr.port()], &data_dir, &http3)
            .expect("the way in assembles")
            .expect("the switch is on, so there is a source");
        engine
            .serving
            .try_send(started.source)
            .unwrap_or_else(|_| panic!("the engine did not take the source"));
        // The proxy starts the source on its own runtime: the pinning first,
        // then the pump. A moment for both, and anything it said.
        std::thread::sleep(Duration::from_secs(2));
        assert!(engine.failure().is_none(), "{:?}", engine.failure());
        Self {
            engine,
            stopper: started.stopper,
            ca,
        }
    }

    /// The key of the leaves the proxy shows, which Set up hands Chrome.
    /// The proxy's own: every start makes a new one, so the authority this
    /// test made first has a different one, and Chrome refused the door's
    /// certificate ("46: certificate unknown") while it was given that.
    fn leaf_spki(&self) -> String {
        self.engine
            .leaf_spki
            .get()
            .expect("the proxy has loaded its authority")
            .clone()
    }

    /// Where the HTTP/3 door listens, once it does.
    async fn door(&self) -> SocketAddr {
        for _ in 0..600 {
            if let Some(at) = self.engine.http3.at() {
                return at;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("the door never said where it listens");
    }

    fn har(&self) -> String {
        let mut out = Vec::new();
        tracewing_store::har::export(&self.engine.session, None, Default::default(), &mut out)
            .expect("the session as HAR");
        String::from_utf8(out).unwrap()
    }

    /// The HAR, once it holds `what`.
    fn har_with(&self, what: &str, within: Duration) -> String {
        let deadline = Instant::now() + within;
        loop {
            let har = self.har();
            if har.contains(what) {
                return har;
            }
            assert!(
                Instant::now() < deadline,
                "{what} never reached the HAR: {har}"
            );
            std::thread::sleep(Duration::from_millis(200));
        }
    }

    /// Off, as the window turns it off, and the engine stopped.
    fn stop(mut self) {
        self.stopper.stop();
        self.engine.stop();
        drop(self.engine);
        // The adapter goes with its last handle, which the pump lets go of
        // as it ends.
        std::thread::sleep(Duration::from_secs(2));
    }
}

/// curl, `program`, asking `url` with its connection sent to the documentation
/// address instead: into the adapter, under the origin's name.
fn asked_with(program: &str, on: &On, port: u16, body: &str, url: &str) -> std::process::Output {
    Command::new(program)
        .args([
            "--silent",
            "--show-error",
            "--max-time",
            "10",
            "--ssl-no-revoke",
        ])
        .arg("--connect-to")
        .arg(format!("localhost:{port}:{AS_IF_THE_SERVER}:{port}"))
        .arg("--cacert")
        .arg(on.ca.cert_path())
        .args(["--data", body, url])
        .output()
        .expect("run curl")
}

/// An HTTPS origin on this machine's loopback, in both families, with a
/// certificate for `localhost` from an authority of its own.
struct Origin {
    port: u16,
    ca: PathBuf,
}

impl Origin {
    async fn start(dir: &Path) -> Self {
        let at = dir.join("origin");
        let ca = tracewing_dev::playground::authority(&at).expect("the origin's authority");
        let v4 = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = v4.local_addr().unwrap().port();
        let serving = at.clone();
        tokio::spawn(async move { tracewing_dev::playground::serve_tls(v4, &serving).await });
        // `localhost` may be asked for at `::1` first.
        if let Ok(v6) = tokio::net::TcpListener::bind(("::1", port)).await {
            let serving = at.clone();
            tokio::spawn(async move { tracewing_dev::playground::serve_tls(v6, &serving).await });
        }
        Self { port, ca }
    }
}

/// What this machine reaches, by programs nobody chose.
#[derive(Debug, Default)]
struct Reached {
    /// HTTPS to a public site, with the site's own certificate.
    https: bool,
    /// A DNS question over UDP to a public resolver.
    dns: bool,
    /// `ping` to the same resolver.
    ping: bool,
}

impl Reached {
    fn now(curl: &Path) -> Self {
        Self {
            https: Command::new(curl)
                .args([
                    "--silent",
                    "--max-time",
                    "15",
                    "--ssl-no-revoke",
                    "-o",
                    "NUL",
                    "https://example.com/",
                ])
                .status()
                .is_ok_and(|s| s.success()),
            dns: dns_answers("1.1.1.1:53"),
            ping: Command::new("ping")
                .args(["-n", "2", "-w", "3000", "1.1.1.1"])
                .output()
                .is_ok_and(|out| {
                    out.status.success() && String::from_utf8_lossy(&out.stdout).contains("TTL=")
                }),
        }
    }

    /// Everything `before` reached, reached again.
    fn at_least(&self, before: &Self) {
        assert!(
            self.https || !before.https,
            "HTTPS from a program nobody chose stopped working"
        );
        assert!(
            self.dns || !before.dns,
            "DNS over UDP from a program nobody chose stopped working"
        );
        assert!(self.ping || !before.ping, "ping stopped being answered");
    }
}

/// Whether `server` answers a DNS question for `example.com`.
fn dns_answers(server: &str) -> bool {
    let Ok(socket) = UdpSocket::bind("0.0.0.0:0") else {
        return false;
    };
    let _ = socket.set_read_timeout(Some(Duration::from_secs(3)));
    let mut question = vec![0x7e, 0x57, 0x01, 0x00, 0, 1, 0, 0, 0, 0, 0, 0];
    for label in ["example", "com"] {
        question.push(label.len() as u8);
        question.extend_from_slice(label.as_bytes());
    }
    question.extend_from_slice(&[0, 0, 1, 0, 1]);
    for _ in 0..3 {
        if socket.send_to(&question, server).is_err() {
            return false;
        }
        let mut answer = [0u8; 512];
        if let Ok((n, _)) = socket.recv_from(&mut answer)
            && n > 12
            && answer[..2] == question[..2]
        {
            return true;
        }
    }
    false
}

/// A line of this machine's hosts file naming the loopback, taken out
/// however the test ends.
struct Hosts(String);

const HOSTS: &str = r"C:\Windows\System32\drivers\etc\hosts";
const OURS: &str = "# tracewing-e2e";

impl Hosts {
    fn add(name: &str) -> Self {
        let mut text = std::fs::read_to_string(HOSTS).unwrap_or_default();
        if !text.is_empty() && !text.ends_with('\n') {
            text.push_str("\r\n");
        }
        text.push_str(&format!("127.0.0.1 {name} {OURS}\r\n"));
        std::fs::write(HOSTS, text).expect("the hosts file, as an administrator");
        let _ = Command::new("ipconfig").arg("/flushdns").output();
        Self(name.to_owned())
    }
}

impl Drop for Hosts {
    fn drop(&mut self) {
        if let Ok(text) = std::fs::read_to_string(HOSTS) {
            let kept: String = text
                .split_inclusive('\n')
                .filter(|line| !(line.contains(OURS) && line.contains(&self.0)))
                .collect();
            let _ = std::fs::write(HOSTS, kept);
        }
        let _ = Command::new("ipconfig").arg("/flushdns").output();
    }
}

/// Whether a HAR says HTTP/3 anywhere, in either of the ways a version is
/// written.
fn over_http3_in(har: &str) -> bool {
    har.contains("HTTP/3") || har.contains(r#""httpVersion":"h3""#)
}

fn said(out: &std::process::Output) -> String {
    format!(
        "{}\n{}{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn made(what: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tracewing-local-live-{what}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("a runtime")
}
