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
//! to get there. So the way in is assembled here without `Pinned`, and the
//! pinning is what [`local_live_linux`](local_live_linux) tests instead.
//!
//! `#[ignore]`d because it wants `unshare` and a few seconds:
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
    let Some(()) = ours_to_run("a_chosen_program_s_http3_goes_through_tracewing_on_a_real_adapter")
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
    let Some(()) = ours_to_run("a_program_nobody_chose_gets_nowhere_over_quic") else {
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
}

/// Whether this process is the one that does the work.
///
/// `None` means it was the outer one: it has just run the inner one inside a
/// namespace and asserted that it passed, and the caller should stop.
fn ours_to_run(test: &str) -> Option<()> {
    if std::env::var_os(IN_OURS).is_some() {
        return Some(());
    }
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
        .env(IN_OURS, "1")
        .status()
        .expect("`unshare` is in util-linux and should be here");
    assert!(
        status.success(),
        "the run inside the namespace failed; its output is above"
    );
    None
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
