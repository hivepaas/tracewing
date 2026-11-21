//! A program's HTTP/3, captured through a **real adapter**, on this machine.
//!
//! Everything about this path is tested in pieces: the catcher's table and
//! its three answers for UDP 443, the door, the way out over QUIC, and the
//! packets carried between a client and the door in memory. What no piece
//! reaches is the join -- a real QUIC client whose datagrams leave by a
//! route into a real TUN device, a real stack turning them into a flow, the
//! real sieve asking the operating system which program sent them, and the
//! real way out asking a server. That join is what `docs/design/http3.md`
//! called unverified, and this is what verifies it.
//!
//! **In a network namespace of its own, which is what makes it harmless.**
//! The nine lines of [`adapter::script`] need an administrator on a real
//! machine, and while the device holds the default route this machine's
//! traffic goes into it. Run under `unshare --user --net --map-root-user`
//! both of those stop being true: the kernel grants the capabilities inside
//! a user namespace of ours, so no password is asked, and the device, the
//! addresses and the routes exist only in a network namespace that goes when
//! the process does. Nothing of the real machine is touched, and the test
//! re-executes itself there rather than trusting the caller to -- run
//! directly, it would make an adapter and a default route on somebody's own
//! machine.
//!
//! **What is real and what is not.** Real: the adapter and its script, the
//! stack, the sieve and `Apps`, the catcher, the door, the recording core,
//! the way out over QUIC, and both ends' certificates from tracewing's own
//! CA. Not real: the server is on the loopback rather than across a network,
//! and the client's packets are addressed to `203.0.113.9:443`, which is
//! where the default route sends them -- so what the client sends to and
//! what the proxy reaches are two different places, which is exactly the
//! shape of a captured flow. Not real either: **the way out is not pinned**,
//! because there is nothing in a namespace to pin it to. On a real machine
//! `Pinned` keeps tracewing's own sockets off its own adapter, and a
//! namespace has one interface -- ours -- so `roads::out_by` answers
//! nowhere and the way out cannot be made at all. The first run of this
//! test proved that the hard way: every request came back 502 from the way
//! out, having gone through the adapter, the catcher, the door and the core
//! to get there. So the way in is assembled here without `Pinned`.
//!
//! **And the pinning is not covered by `local_live_linux` either**, which
//! this file said until TraceW 2 corrected it: that test builds the source
//! and asserts on the routing table, then lets go, without ever calling
//! `start` -- so `Pinned::start` and the watch behind it are reached by
//! neither of us. What covers which part of the pinning, and what is left,
//! is written down on `local_source::Pinned`.
//!
//! `#[ignore]`d because it wants `unshare` and a few seconds -- and
//! because **it is refused wherever AppArmor confines the caller**, which
//! is not what this file said first. On Ubuntu 24.04 and newer,
//! `kernel.apparmor_restrict_unprivileged_userns` is 1 and
//! `/etc/apparmor.d/unprivileged_userns` -- the profile an unconfined
//! process transitions to when it makes a user namespace -- carries
//! `allow userns` together with `audit deny capability`. So the namespace
//! is made and the capabilities inside it are not, which is why
//! `unshare --user` alone succeeds and `--map-root-user` does not:
//!
//! ```text
//! $ aa-exec -p unconfined -- unshare --user --map-root-user true
//! unshare: write failed /proc/self/uid_map: Operation not permitted    exit 1
//! $ unshare --user --map-root-user true                                exit 0
//! ```
//!
//! Both of those are this machine, minutes apart. The second works because
//! the session that wrote this runs under Ubuntu's `code` profile, which
//! grants `userns` because Electron needs it. So **"it works on a
//! developer's machine and the runner refuses it" was wrong**, and this
//! file said it: TraceW 2 measured the cause on the runner by flipping the
//! sysctl both ways and back (#372), and the same refusal is one terminal
//! away here. `cat /proc/self/attr/current` says which profile you are in.
//!
//! **Where the restriction exists at all is narrower than it sounds**, and
//! worth knowing before reading the paragraph above as "this test does not
//! run on Linux". It is an Ubuntu-originated control, on by default since
//! 23.10; Debian's apparmor packaging sets the same sysctl; a kernel
//! without that patch has no such file at all, so a Fedora or Arch machine
//! runs this test from an ordinary terminal. One file is the whole
//! question:
//!
//! ```text
//! $ cat /proc/sys/kernel/apparmor_restrict_unprivileged_userns
//! 1          # the caller's profile decides -- this machine
//! 0          # off, nothing mediates it
//! (missing)  # the kernel has no such restriction
//! ```
//!
//! Where it is `1`, which profile you are in decides, and the profiles that
//! grant `userns` are the reason this is easy to misread as working: 93 of
//! the 390 in `/etc/apparmor.d` here carry a `userns` rule -- every
//! browser, every Electron app, `podman`, `runc`, `steam` -- because
//! sandboxing needs one. A `cargo test` from a terminal is none of them.
//!
//! To run it from a terminal under that restriction, be in a profile that
//! grants `userns` or
//! `sudo sysctl -w kernel.apparmor_restrict_unprivileged_userns=0` first.
//! Which is also why un-ignoring it would have been worse than the red CI
//! run showed: it would have broken `cargo test` for anybody in a terminal,
//! not only in CI.
//!
//! ```sh
//! cargo test -p tracewing --release --test local_live_http3 -- --ignored --nocapture
//! ```

#![cfg(target_os = "linux")]

use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use tracewing::local_source::{self, Http3};
use tracewing_dev::over_http3::{self, SAID};
use tracewing_proxy::{
    ProxyOptions,
    apps::Apps,
    ca::LocalCa,
    ca_key::KeyStore,
    crypto_provider, run_proxy,
    sieve::{self, Chosen},
    source::By,
    tun::Tun,
    tun_linux::TunDevice,
    upstream,
};
use tracewing_store::settings::{Live, StorageSettings};

/// Set on the run inside the namespace, so that the outer one knows to make
/// the namespace and the inner one knows it is in it.
const IN_OURS: &str = "TRACEWING_LIVE_HTTP3_NS";

/// Where the client's packets are addressed. Nothing of ours listens there:
/// it is a documentation address that the default route sends into the
/// adapter, which is the whole point -- the client is talking to a server
/// and tracewing is what answers.
const AS_IF_THE_SERVER: &str = "203.0.113.9:443";

/// A chosen program's HTTP/3 reaches the server and comes back, by way of
/// the adapter.
#[test]
#[ignore = "wants `unshare`, and makes an adapter in a namespace of its own"]
fn a_chosen_program_s_http3_goes_through_tracewing_on_a_real_adapter() {
    let Some(witness) =
        ours_to_run("a_chosen_program_s_http3_goes_through_tracewing_on_a_real_adapter")
    else {
        return;
    };
    let dir = made();
    the_adapter();
    // The program the sieve is to keep is this test binary, which is also
    // the program that will send the QUIC: the sieve and the catcher both
    // ask the operating system who holds the socket, and this is the answer
    // they have to come back with.
    let kept = std::env::current_exe().expect("this test binary");
    let (answer, arrivals, saw) = runtime().block_on(asked(&dir, kept));

    let (status, body) = answer.expect("the answer comes back through the adapter");
    assert_eq!(status, http::StatusCode::OK);
    assert_eq!(body, SAID, "the server's own body, not the proxy's");
    assert_eq!(
        saw.first().map(|arrived| arrived.path.clone()),
        Some("/orders".to_owned()),
        "and the real server is where it arrived"
    );
    assert_eq!(
        arrivals.of(By::Tun).count,
        1,
        "by the way in that caught it, which only the catcher's own entry can say"
    );
    let _ = std::fs::remove_dir_all(&dir);
    reached(&witness);
}

/// A program nobody chose keeps what it had: its QUIC is refused and it gets
/// nowhere, although capture is on for somebody else.
///
/// The same run as above with one thing changed, which is what makes the
/// first one mean something: without this, a test that captured everything
/// would pass it just as well.
#[test]
#[ignore = "wants `unshare`, and makes an adapter in a namespace of its own"]
fn a_program_nobody_chose_gets_nowhere_over_quic() {
    let Some(witness) = ours_to_run("a_program_nobody_chose_gets_nowhere_over_quic") else {
        return;
    };
    let dir = made();
    the_adapter();
    // A program every distribution has, and not the one sending the QUIC.
    let kept = PathBuf::from("/usr/bin/true");
    let (answer, arrivals, saw) = runtime().block_on(asked(&dir, kept));

    assert!(
        answer.is_err(),
        "a program nobody chose must not get through: {answer:?}"
    );
    assert!(saw.is_empty(), "and nothing reached the server: {saw:?}");
    // **One, and it is the refused attempt rather than a flow.** An
    // `Arrival` counts a refused QUIC attempt as much as a request, on
    // purpose -- otherwise a tab would say nothing had come in while a row
    // of one was on screen -- so this says the refusal happened and was
    // said, which is what a program nobody chose is owed. It also says the
    // attempt was said *once* however many times the client tried, which is
    // `quic::Attempts`' doing.
    assert_eq!(
        arrivals.of(By::Tun).count,
        1,
        "the refused attempt, said once"
    );
    let _ = std::fs::remove_dir_all(&dir);
    reached(&witness);
}

/// Chrome's own HTTP/3, through the adapter: a real browser, told to speak
/// QUIC to a name that resolves where the default route goes, reaches the
/// server through tracewing. docs/design/e2e.md, S8.
///
/// What is not the browser's own: the name. `h3.test` is mapped by Chrome's
/// resolver to the same documentation address as above, so its datagrams go
/// into the adapter; the proxy's way out finds it on this namespace's
/// loopback, by an `/etc/hosts` mounted over the real one in the mount
/// namespace this test already has. And Chrome trusts tracewing's CA from an
/// NSS database in a home of the test's own, as Set up's Install puts it in a
/// user's.
#[test]
#[ignore = "wants `unshare` and Chrome, and makes an adapter in a namespace of its own"]
fn chromes_http3_goes_through_tracewing_on_a_real_adapter() {
    // The program, not the script that starts it: what the sieve sees send.
    let Some(chrome) = ["/opt/google/chrome/chrome", "/usr/lib/chromium/chromium"]
        .into_iter()
        .map(PathBuf::from)
        .find(|path| path.exists())
    else {
        eprintln!("skipped: no Chrome here");
        return;
    };
    let Some(witness) = ours_to_run("chromes_http3_goes_through_tracewing_on_a_real_adapter")
    else {
        return;
    };
    let dir = made();
    the_adapter();
    let hosts = dir.join("hosts");
    std::fs::write(
        &hosts,
        "127.0.0.1 localhost h3.test\n::1 localhost h3.test\n",
    )
    .expect("a hosts file of our own");
    run("mount", &["--bind", hosts.to_str().unwrap(), "/etc/hosts"]);

    let runtime = runtime();
    let Assembled {
        stop,
        ca,
        http3,
        arrivals,
    } = runtime.block_on(assembled(&dir, chrome.clone()));
    let server = runtime
        .block_on(async { over_http3::Server::start_at(&ca, "h3.test", 443) })
        .expect("an HTTP/3 server on 443");
    let door = runtime.block_on(waited_for_the_door(&http3));
    println!("the door is at {door}, the server on 443");

    // tracewing's CA, where Chrome on Linux reads what to trust.
    let home = dir.join("home");
    let nss = home.join(".pki/nssdb");
    std::fs::create_dir_all(&nss).unwrap();
    let db = format!("sql:{}", nss.display());
    run("certutil", &["-N", "--empty-password", "-d", &db]);
    run(
        "certutil",
        &[
            "-A",
            "-d",
            &db,
            "-n",
            "tracewing",
            "-t",
            "C,,",
            "-i",
            ca.cert_path().to_str().unwrap(),
        ],
    );

    let mut browser = std::process::Command::new(&chrome)
        .args([
            "--headless=new",
            // Root in a namespace of its own, which Chrome's sandbox refuses.
            "--no-sandbox",
            "--disable-gpu",
            "--disable-dev-shm-usage",
            "--no-first-run",
            "--enable-quic",
            // What QUIC says when it gives up, on stderr with the rest.
            "--enable-logging=stderr",
            "--vmodule=*quic*=1",
            "--origin-to-force-quic-on=h3.test:443",
        ])
        .arg(format!(
            "--host-resolver-rules=MAP h3.test {}",
            AS_IF_THE_SERVER.trim_end_matches(":443")
        ))
        .arg(format!("--user-data-dir={}", dir.join("profile").display()))
        .args(["--dump-dom", "https://h3.test/orders"])
        .env("HOME", &home)
        .stdout(std::process::Stdio::piped())
        // Where Chrome says why a page did not load: `--dump-dom` writes
        // nothing and exits 0 when it does not, so stdout cannot tell a
        // refused certificate from a lost handshake.
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("start Chrome");
    let deadline = std::time::Instant::now() + Duration::from_secs(60);
    while browser.try_wait().expect("Chrome").is_none() {
        assert!(
            std::time::Instant::now() < deadline,
            "Chrome did not finish in 60 s"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    let mut page = String::new();
    std::io::Read::read_to_string(&mut browser.stdout.take().unwrap(), &mut page).unwrap();
    let mut said = String::new();
    std::io::Read::read_to_string(&mut browser.stderr.take().unwrap(), &mut said).unwrap();
    // Chrome's reason, without the lines of dbus it says on every start.
    let why: Vec<&str> = said
        .lines()
        .filter(|line| {
            line.contains("Page load failed")
                || line.contains("net::ERR_")
                || (line.to_ascii_lowercase().contains("quic")
                    && ["error", "close", "fail"]
                        .iter()
                        .any(|word| line.to_ascii_lowercase().contains(word)))
        })
        .collect();

    // Read before the assertion below, so that a failure says *where* the
    // chain broke rather than only that it did: nothing caught means the
    // datagrams never reached tracewing (routing, or the sieve), caught but
    // unseen by the server means the door or its way out, and seen by the
    // server with an empty page means the way back.
    let caught = arrivals.of(By::Tun).count;
    let seen: Vec<String> = server.saw().iter().map(|a| a.path.clone()).collect();
    assert!(
        page.contains(std::str::from_utf8(SAID).unwrap()),
        "Chrome did not get the server's own body.\n  its page: {page:?}\n           its reasons: {why:?}\n  caught by the way in: {caught}\n           reached the real server: {seen:?}"
    );
    assert_eq!(
        seen.first().map(String::as_str),
        Some("/orders"),
        "the real server is where it arrived"
    );
    assert!(caught >= 1, "by the way in that caught it");
    let _ = stop.send(());
    let _ = std::fs::remove_dir_all(&dir);
    reached(&witness);
}

/// Whether this process is the one that does the work, and where it is to
/// leave word that it finished.
///
/// `None` means it was the outer one: it has just run the inner one inside a
/// namespace, asserted that it passed **and that it ran at all**, and the
/// caller should stop.
///
/// **The exit status cannot tell "it passed" from "there was nothing to
/// do".** A libtest run that executes no test exits 0, so an inner run that
/// skipped this test -- because the filter stopped matching, or because
/// `--ignored` was dropped, or because somebody ignored it for an unrelated
/// reason -- would be reported as a pass by the outer one. TraceW 2 hit
/// exactly that on #368 while putting `#[ignore]` back, and caught it only
/// by reading the counts rather than the exit code. So the inner run leaves
/// a witness file as its last act and the outer one looks for it: a run
/// that measured nothing now says so.
fn ours_to_run(test: &str) -> Option<PathBuf> {
    if let Some(witness) = std::env::var_os(IN_OURS) {
        // Why each client's QUIC went where it did, on stderr above the
        // failure: the catcher's refusals are otherwise silent, and the one
        // flaky run Chrome had (`ERR_QUIC_PROTOCOL_ERROR`, nothing caught)
        // could say no more than that.
        let _ = tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .with_env_filter("warn,tracewing::local_source=debug")
            .try_init();
        return Some(PathBuf::from(witness));
    }
    let witness = std::env::temp_dir().join(format!(
        "tracewing-live-http3-{test}-{}.witness",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&witness);
    let exe = std::env::current_exe().expect("this test binary");
    let status = std::process::Command::new("unshare")
        // `--mount`, and the `sysfs` mounted below, are not spare belt. A
        // bare network namespace leaves `/sys` as the old one mounted it,
        // so `adapter::look` reads the real machine's interfaces and says
        // the device is missing while `ip` has just made it -- which is
        // what the first run of this test did. `/proc` needs nothing: the
        // net sysctls under `/proc/sys/net` and the tables under
        // `/proc/net` are resolved against the reading task's own network
        // namespace, and remounting it would want a PID namespace as well.
        .args(["--user", "--net", "--mount", "--map-root-user"])
        .arg(&exe)
        .args([
            test,
            "--exact",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(IN_OURS, &witness)
        .status()
        .expect("`unshare` is in util-linux and should be here");
    assert!(
        status.success(),
        "the run inside the namespace failed; its output is above"
    );
    let ran = witness.exists();
    let _ = std::fs::remove_file(&witness);
    assert!(
        ran,
        "the run inside the namespace exited {status} without reaching the end of {test}, \
         so nothing was measured"
    );
    None
}

/// Left by the inner run as its last act. See [`ours_to_run`].
fn reached(witness: &std::path::Path) {
    std::fs::write(witness, "reached the end").expect("a witness the outer run can see");
}

/// The adapter, by the real step, in this namespace.
///
/// **`make_adapter` itself, not the nine lines copied out of it.**
/// `run_as_admin` runs the script with `sh -c` when `id -u` is `0`, which
/// `--map-root-user` makes true, so the privileged step takes itself and
/// polkit is never asked -- and the look-before-asking that decides it, and
/// the uid the device is made for, are the real ones rather than this
/// test's idea of them. TraceW 2 found that while taking `local_live_linux`
/// the same way.
///
/// The loopback comes up here rather than in the step because a real
/// machine's is already up and a fresh namespace's is not -- and the server
/// this test reaches is on it.
fn the_adapter() {
    // This namespace's own view of its own interfaces: see the note in
    // `ours_to_run`.
    run("mount", &["-t", "sysfs", "sysfs", "/sys"]);
    run("ip", &["link", "set", "lo", "up"]);
    tracewing_system::adapter::make_adapter().expect("the privileged step takes itself here");
    let look = tracewing_system::adapter::look();
    assert!(look.ready(), "the adapter is missing {:?}", look.missing());
}

fn run(program: &str, args: &[&str]) {
    let status = std::process::Command::new(program)
        .args(args)
        .status()
        .unwrap_or_else(|e| panic!("{program}: {e}"));
    assert!(status.success(), "{program} {args:?} failed: {status}");
}

fn made() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tracewing-live-http3-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a directory of our own");
    dir
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("a runtime")
}

/// tracewing's local way in, assembled on the device the real script made,
/// keeping `kept`'s traffic: what both clients below go through.
struct Assembled {
    stop: tokio::sync::oneshot::Sender<()>,
    ca: Arc<LocalCa>,
    http3: Http3,
    arrivals: Arc<tracewing_proxy::source::Arrivals>,
}

async fn assembled(dir: &std::path::Path, kept: PathBuf) -> Assembled {
    let _ = crypto_provider().install_default();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a proxy port");
    let port = listener.local_addr().expect("its address").port();

    // What the proxy and the way in share: the settings that say capture is
    // on, the table the door reads each client's facts from, and where the
    // door listens once it is open.
    let settings = StorageSettings {
        capture_http3: true,
        ..Default::default()
    };
    let live = Arc::new(Live::new(&settings));
    let http3 = Http3 {
        live: Arc::clone(&live),
        door: Arc::default(),
        handed: Arc::default(),
    };
    let arrivals: Arc<tracewing_proxy::source::Arrivals> = Arc::default();

    // The way in, with the app's own sieve and the app's own catcher, on
    // the device the real script made. Everything but `Pinned`: see the top
    // of this file for why the pinning cannot be here.
    let apps = Apps::spawn(vec![port]).expect("the thread that names programs");
    let chosen = Chosen::of(vec![kept.display().to_string()]);
    let sieve = sieve::by_programs(apps.clone(), chosen.clone(), upstream::straight_to());
    let device = Arc::new(TunDevice::open(tracewing_system::adapter::DEVICE).expect("the device"));
    let source = Tun::new(Arc::clone(&device), sieve, upstream::bound_datagrams())
        .catching(local_source::catching(apps, chosen, http3.clone()));

    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let data_dir = dir.join("ca");
    std::fs::create_dir_all(&data_dir).expect("a place for the CA");
    // **Made here, before the proxy, and for two reasons.** The server below
    // needs a leaf from the same authority the door shows one from, so a
    // client trusting it trusts both ends; and the way out has to be told to
    // trust it, which `extra_roots` does and which is read as the proxy
    // starts -- so the file has to exist by then. Letting the proxy make it
    // and loading it afterwards also raced: both ends called
    // `load_or_create` at once and the way out refused the leaf with
    // `BadSignature`, which reads like a wrong certificate and was two
    // certificates.
    let ca = Arc::new(
        LocalCa::load_or_create(&data_dir, crypto_provider(), KeyStore::File)
            .expect("an authority of our own"),
    );
    tokio::spawn(run_proxy(
        ProxyOptions {
            listener: Some(listener),
            std_listener: None,
            data_dir: data_dir.clone(),
            key_store: KeyStore::File,
            session: dir.join("session"),
            settings,
            // The test's own server, trusted the way `e2e` trusts its
            // origin: a proxy does not trust its own CA upstream, and
            // should not.
            extra_roots: vec![data_dir.join("ca.pem")],
            queue: 10_000,
            stats_every: None,
            shared: Default::default(),
            recording: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            rules: Default::default(),
            traffic_rules: Default::default(),
            network: Default::default(),
            check: Default::default(),
            arrivals: Arc::clone(&arrivals),
            leaf_spki: Default::default(),
            pauses: Default::default(),
            leave_index: false,
            // The whole question this test asks is which program sent a
            // datagram, so the thread that answers it has to be running.
            find_apps: true,
            stop_limit: Duration::from_secs(1),
            live: Arc::clone(&live),
            later: None,
            sources: vec![Box::new(source)],
            relay: Default::default(),
            tls_keys: Default::default(),
            sockets: Default::default(),
            client_certs: Default::default(),
            scripts: Default::default(),
            handed: Arc::clone(&http3.handed),
            http3: Arc::clone(&http3.door),
        },
        async {
            let _ = stopped.await;
        },
    ));
    Assembled {
        stop,
        ca,
        http3,
        arrivals,
    }
}

/// The whole assembly, and one HTTP/3 question through it.
///
/// `kept` is the program the sieve and the catcher are to keep, which is the
/// one thing the two tests above differ by.
async fn asked(
    dir: &std::path::Path,
    kept: PathBuf,
) -> (
    anyhow::Result<(http::StatusCode, Vec<u8>)>,
    Arc<tracewing_proxy::source::Arrivals>,
    Vec<over_http3::Arrived>,
) {
    let Assembled {
        stop,
        ca,
        http3,
        arrivals,
    } = assembled(dir, kept).await;

    let server = over_http3::Server::start(&ca, "localhost").expect("an HTTP/3 server");
    let door = waited_for_the_door(&http3).await;
    println!("the door is at {door}, the server on port {}", server.port);

    let answer = tokio::time::timeout(
        // Long enough for an answer that takes milliseconds, and short
        // enough that the test which expects none does not sit here.
        Duration::from_secs(10),
        over_http3::ask(
            AS_IF_THE_SERVER.parse().expect("a literal"),
            "localhost",
            &format!("https://localhost:{}/orders", server.port),
            over_http3::trusting(&ca).expect("the CA's own certificate"),
        ),
    )
    .await
    .unwrap_or_else(|_| Err(anyhow::anyhow!("nothing came back in time")));

    let saw = server.saw();
    // Letting go: the proxy, and the device with it. On a real machine the
    // device stays and its routes go dead, which is what makes a hard kill
    // safe; here the whole namespace goes.
    let _ = stop.send(());
    (answer, arrivals, saw)
}

async fn waited_for_the_door(http3: &Http3) -> SocketAddr {
    for _ in 0..600 {
        if let Some(at) = http3.door.at() {
            return at;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the door never said where it listens");
}
