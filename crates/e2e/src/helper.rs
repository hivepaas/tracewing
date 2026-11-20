//! A test that runs part of itself as a process of its own, for the test to
//! kill: the app holding the system proxy, or the block on UDP 443, when what
//! is being checked is what happens after the holder dies however it dies.
//!
//! The process is the test program itself, asked for one `#[ignore]`d test by
//! name, with what it should do in [`HELPER`]. The helper says [`READY`] on a
//! line of its own once it holds what it holds, then waits to be killed.

use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::Duration,
};

/// What the helper is asked to do, as the helper test reads it.
pub const HELPER: &str = "TRACEWING_E2E_HELPER";

/// What the helper says once it holds what it was asked to hold. It ends
/// a line rather than making one: libtest, told `--nocapture`, writes the
/// test's name and ` ... ` first, on the same line.
pub const READY: &str = "tracewing-e2e: held";

/// The helper's own side: what it was asked to do, if it was started as one.
pub fn asked() -> Option<String> {
    std::env::var(HELPER).ok()
}

/// The helper's own side: says it holds what it holds, then waits for its end.
pub fn hold_until_killed() -> ! {
    println!("{READY}");
    std::io::stdout().flush().unwrap();
    loop {
        std::thread::sleep(Duration::from_secs(3600));
    }
}

/// A helper running.
pub struct Helper(Child);

impl Helper {
    /// Runs the test named `test` of this program as a helper, asked `what`,
    /// and waits until it says it is ready.
    pub fn start(test: &str, what: &str) -> Self {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                test,
                "--nocapture",
                "--test-threads=1",
            ])
            .env(HELPER, what)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let ready = BufReader::new(stdout)
                .lines()
                .map_while(Result::ok)
                .any(|line| line.ends_with(READY));
            let _ = tx.send(ready);
        });
        match rx.recv_timeout(Duration::from_secs(30)) {
            Ok(true) => Helper(child),
            other => {
                let _ = child.kill();
                panic!("the helper {test} never said it was ready: {other:?}");
            }
        }
    }

    /// Kills it outright, as a crash or `kill -9` would: nothing of it runs
    /// after.
    pub fn kill(mut self) {
        self.0.kill().unwrap();
        self.0.wait().unwrap();
    }
}

impl Drop for Helper {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
