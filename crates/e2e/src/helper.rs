//! tracewing holding the system proxy or the block on UDP 443 in a process of
//! its own, as the window would, for a test to let go of cleanly or to kill
//! outright: `tracewing e2e-hold` (crates/app/src/e2e_hold.rs).
//!
//! It is the real program rather than the test's own, because what stands
//! behind what it holds — the watchdog, the scheduled task or timer, the
//! elevated helper of the Windows block — is started as whichever program
//! took it.

use crate::{program, tracewing_at};
use std::{
    ffi::OsStr,
    io::{BufRead, BufReader},
    path::Path,
    process::{Child, ChildStdin, ExitStatus, Stdio},
    sync::mpsc,
    time::Duration,
};

/// What `e2e-hold` says once it holds what it was asked to hold.
pub const HELD: &str = "tracewing-e2e: held";

/// `tracewing e2e-hold` running.
pub struct Helper {
    child: Child,
    input: Option<ChildStdin>,
}

impl Helper {
    /// Runs `tracewing e2e-hold` with `args` and waits until it holds.
    pub fn start<S: AsRef<OsStr>>(args: &[S]) -> Self {
        Self::start_from(&program(), args)
    }

    /// The same, from `program`.
    pub fn start_from<S: AsRef<OsStr>>(program: &Path, args: &[S]) -> Self {
        let mut child = tracewing_at(program)
            .arg("e2e-hold")
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("start tracewing e2e-hold");
        let input = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let held = BufReader::new(stdout)
                .lines()
                .map_while(Result::ok)
                .any(|line| line.ends_with(HELD));
            let _ = tx.send(held);
        });
        match rx.recv_timeout(Duration::from_secs(60)) {
            Ok(true) => Helper { child, input },
            other => {
                let _ = child.kill();
                let ended = child.wait();
                panic!("tracewing e2e-hold never held: {other:?}, {ended:?}");
            }
        }
    }

    /// The system proxy, for 127.0.0.1:`port`, with the watchdog the window
    /// starts or without one.
    pub fn system_proxy(data_dir: &Path, port: u16, watchdog: bool) -> Self {
        Self::system_proxy_from(&program(), data_dir, port, watchdog)
    }

    /// The same, from `program`.
    pub fn system_proxy_from(program: &Path, data_dir: &Path, port: u16, watchdog: bool) -> Self {
        let port = port.to_string();
        let mut args = vec![
            OsStr::new("system-proxy"),
            OsStr::new("--data-dir"),
            data_dir.as_os_str(),
            OsStr::new("--port"),
            OsStr::new(&port),
        ];
        if !watchdog {
            args.push(OsStr::new("--no-watchdog"));
        }
        Self::start_from(program, &args)
    }

    /// The block on UDP 443.
    pub fn quic_block(data_dir: &Path) -> Self {
        Self::start(&[
            OsStr::new("quic-block"),
            OsStr::new("--data-dir"),
            data_dir.as_os_str(),
        ])
    }

    /// Lets go of what it holds, as the window does when it closes.
    pub fn let_go(mut self) -> ExitStatus {
        drop(self.input.take());
        self.child.wait().unwrap()
    }

    /// Kills it outright, as a crash or `kill -9` would: nothing of it runs
    /// after.
    pub fn kill(mut self) {
        self.child.kill().unwrap();
        self.child.wait().unwrap();
    }
}

impl Drop for Helper {
    fn drop(&mut self) {
        drop(self.input.take());
        let _ = self.child.wait();
    }
}
