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

/// The `tracewing` program under test: `TRACEWING_E2E_BIN`, or the one on
/// the PATH, which is the installed package's when one is installed.
pub fn tracewing() -> Command {
    let program = std::env::var_os("TRACEWING_E2E_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(if cfg!(windows) {
                "tracewing.exe"
            } else {
                "tracewing"
            })
        });
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
        let trusted = data_dir.join("trusted-servers");
        std::fs::create_dir_all(&trusted).unwrap();
        std::fs::copy(&origin.ca, trusted.join("origin.pem")).unwrap();
        let port = free_port();
        let child = tracewing()
            .args([
                "capture",
                "--ca-key-in-file",
                "--port",
                &port.to_string(),
                "--data-dir",
            ])
            .arg(data_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("start tracewing capture: set TRACEWING_E2E_BIN or put tracewing on the PATH");
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
    Command::new("curl")
        .args(["--silent", "--show-error", "--max-time", "10", "--proxy"])
        .arg(format!("http://127.0.0.1:{port}"))
        .arg(url)
        .env_remove("CURL_CA_BUNDLE")
        .env_remove("SSL_CERT_FILE")
        .env_remove("SSL_CERT_DIR")
        .output()
        .expect("run curl")
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
