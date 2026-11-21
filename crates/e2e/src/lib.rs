//! Tests that change the computer they run on: its system proxy, its trust
//! stores, its keychain, its firewall. They are what the rest of tracewing's
//! tests stand in for with fakes, and the reason those fakes exist is that
//! nobody wants their own machine's proxy or trust store touched by `cargo
//! test`.
//!
//! So nothing here runs by accident. A test does its work only when both
//! [`ENV`] is `1` and the [`MARKER`] file exists, which only a container, a
//! VM or a CI runner made for the purpose is given (`scripts/e2e_linux.sh`,
//! `scripts/e2e_windows.ps1`). Anywhere else every test returns at once, so
//! `cargo test --workspace` stays harmless. docs/design/e2e.md says what each
//! scenario covers and why.

#[cfg(target_os = "linux")]
pub mod desktop;
#[cfg(windows)]
#[path = "desktop_windows.rs"]
pub mod desktop;
#[cfg(target_os = "macos")]
#[path = "desktop_macos.rs"]
pub mod desktop;
pub mod helper;
pub mod site;

use tracewing_proxy::ca_key::KeyStore;

use std::{
    net::{SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    time::{Duration, Instant},
};

/// What says the tests may run.
pub const ENV: &str = "TRACEWING_E2E";

/// What says this machine may be changed. Not in a user's home, where a
/// test could make one by mistake: making it needs an administrator.
pub const MARKER: &str = if cfg!(windows) {
    r"C:\tracewing-e2e-throwaway"
} else if cfg!(target_os = "macos") {
    "/Library/Application Support/tracewing-e2e-throwaway"
} else {
    "/etc/tracewing-e2e-throwaway"
};

/// Whether this machine is one these tests may change.
pub fn allowed() -> bool {
    std::env::var(ENV).is_ok_and(|v| v == "1") && Path::new(MARKER).exists()
}

/// Returns from the test, saying so, unless this machine may be changed.
#[macro_export]
macro_rules! only_where_allowed {
    () => {
        if !$crate::allowed() {
            eprintln!(
                "skipped: changes this computer; runs only with {}=1 and {} present",
                $crate::ENV,
                $crate::MARKER
            );
            return;
        }
    };
}

/// What says somebody is at this Mac to answer its dialogs: an
/// administrator's password for the machine's trust store and the firewall,
/// a user's for their own trust settings. `scripts/e2e_macos.sh --attended`
/// sets it; a CI runner has nobody to answer, so the tests that need it say
/// so and end, unless they run as root, which macOS does not ask.
pub const ATTENDED: &str = "TRACEWING_E2E_ATTENDED";

/// Whether a test that makes macOS ask may run here: anywhere but a Mac, and
/// on a Mac with somebody to answer or as root.
pub fn may_ask() -> bool {
    if !cfg!(target_os = "macos") {
        return true;
    }
    std::env::var(ATTENDED).is_ok_and(|v| v == "1")
        || Command::new("id")
            .arg("-u")
            .output()
            .is_ok_and(|out| String::from_utf8_lossy(&out.stdout).trim() == "0")
}

/// Returns from the test, saying so, where macOS would ask and nobody is
/// there to answer.
#[macro_export]
macro_rules! only_where_someone_answers {
    () => {
        if !$crate::may_ask() {
            eprintln!(
                "skipped: macOS asks for a password here; runs with {}=1 or as root",
                $crate::ATTENDED
            );
            return;
        }
    };
}

/// The `tracewing` program under test: `TRACEWING_E2E_BIN`, or the one on
/// the PATH.
pub fn program() -> PathBuf {
    std::env::var_os("TRACEWING_E2E_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(if cfg!(windows) {
                "tracewing.exe"
            } else {
                "tracewing"
            })
        })
}

/// The program under test, [`program`].
pub fn tracewing() -> Command {
    tracewing_at(&program())
}

/// `program`, as a test runs it: an installed package's, say.
pub fn tracewing_at(program: &Path) -> Command {
    let mut command = Command::new(program);
    // Whatever the machine says about proxies is not this test's.
    for name in [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
    ] {
        command.env_remove(name);
    }
    command
}

/// A folder of the test's own, empty.
pub fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tracewing-e2e-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A port nothing listens on, for a moment.
pub fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// Waits for `done`, or fails the test saying what it waited for.
pub fn wait_for(what: &str, limit: Duration, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + limit;
    while !done() {
        assert!(Instant::now() < deadline, "waited {limit:?} for {what}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// An HTTPS server of the test's own, `localhost` and `127.0.0.1`, under an
/// authority of its own that only tracewing is told to trust.
pub struct Origin {
    pub port: u16,
    /// That authority's certificate.
    pub ca: PathBuf,
    _runtime: tokio::runtime::Runtime,
}

impl Origin {
    pub fn start(dir: &Path) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let listener = runtime
            .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        let ca = dir.join("origin-ca.pem");
        let out = ca.clone();
        runtime.spawn(async move { tracewing_dev::benchkit::serve_origin(listener, &out).await });
        wait_for("the origin's authority", Duration::from_secs(10), || {
            ca.exists()
        });
        Origin {
            port,
            ca,
            _runtime: runtime,
        }
    }
}

/// `tracewing capture` running, until dropped.
pub struct Capture {
    child: Child,
    pub port: u16,
    pub data_dir: PathBuf,
}

impl Capture {
    /// Starts it in `data_dir`, its CA's key in a file there, trusting
    /// `origin`'s authority as a user does: in the `trusted-servers` folder.
    pub fn start(data_dir: &Path, origin: &Origin) -> Self {
        Self::start_keeping_the_key_in(data_dir, origin, KeyStore::File)
    }

    /// The same, its CA's key where `store` says: `System` is the keychain,
    /// as the window keeps it.
    pub fn start_keeping_the_key_in(data_dir: &Path, origin: &Origin, store: KeyStore) -> Self {
        Self::start_from(&program(), data_dir, origin, store)
    }

    /// The same, from `program`.
    pub fn start_from(program: &Path, data_dir: &Path, origin: &Origin, store: KeyStore) -> Self {
        Self::start_trusting(program, data_dir, &origin.ca, store)
    }

    /// The same, trusting the servers whose authority is `server_ca`.
    pub fn start_trusting(
        program: &Path,
        data_dir: &Path,
        server_ca: &Path,
        store: KeyStore,
    ) -> Self {
        let trusted = data_dir.join("trusted-servers");
        std::fs::create_dir_all(&trusted).unwrap();
        std::fs::copy(server_ca, trusted.join("origin.pem")).unwrap();
        let port = free_port();
        let mut command = tracewing_at(program);
        command.arg("capture");
        if store == KeyStore::File {
            command.arg("--ca-key-in-file");
        }
        let child = command
            .args(["--port", &port.to_string(), "--data-dir"])
            .arg(data_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap_or_else(|e| {
                panic!(
                    "start {} capture: set TRACEWING_E2E_BIN or put tracewing on the PATH: {e}",
                    program.display()
                )
            });
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        wait_for("tracewing to listen", Duration::from_secs(30), || {
            TcpStream::connect(addr).is_ok()
        });
        wait_for("its CA", Duration::from_secs(10), || {
            data_dir.join("ca.pem").exists()
        });
        Capture {
            child,
            port,
            data_dir: data_dir.to_owned(),
        }
    }

    pub fn ca_pem(&self) -> PathBuf {
        self.data_dir.join("ca.pem")
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// `curl` through the proxy on `port`, trusting what the system trusts and
/// nothing else.
pub fn curl_through(port: u16, url: &str) -> Output {
    curl_through_with(port, &[url])
}

/// The same, with more of curl's arguments: the URL is among them.
pub fn curl_through_with(port: u16, args: &[&str]) -> Output {
    let mut curl = Command::new("curl");
    // Windows' own curl asks Schannel, which by default refuses a certificate
    // whose revocation it cannot check, and a CA made on this computer
    // publishes no list of revoked ones. Browsers do not fail on that.
    if cfg!(windows) {
        curl.arg("--ssl-no-revoke");
    }
    curl.args(["--silent", "--show-error", "--max-time", "10", "--proxy"])
        .arg(format!("http://127.0.0.1:{port}"))
        .args(args)
        .env_remove("CURL_CA_BUNDLE")
        .env_remove("SSL_CERT_FILE")
        .env_remove("SSL_CERT_DIR")
        .output()
        .expect("run curl")
}

/// What `program` has recorded in `data_dir` so far, as a HAR: written with
/// `tracewing export`, as a user writes one.
pub fn har_of(program: &Path, data_dir: &Path) -> String {
    let file = data_dir.with_extension("har");
    let out = tracewing_at(program)
        .arg("export")
        .arg("--har")
        .arg(&file)
        .arg("--data-dir")
        .arg(data_dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "export a HAR: {}", said(&out));
    std::fs::read_to_string(&file).unwrap()
}

/// What a command said, for a failed assertion.
pub fn said(out: &Output) -> String {
    format!(
        "exit {:?}\n{}{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}
