//! Linux: an nftables table with the `owner` flag, which only the `nft` that
//! made it can change and which the kernel takes away as soon as that `nft`
//! exits. It runs as root, through polkit's dialog, and reads its commands from
//! this process until they stop coming: when the block is let go, or when this
//! process ends, however it ends.

use anyhow::{Context, Result, bail};
use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

/// Made, then listed: the listing says it is in place.
const RULES: &str = "\
add table inet tracewing { flags owner; }
add chain inet tracewing quic { type filter hook output priority filter; policy accept; }
add rule inet tracewing quic udp dport 443 reject
list table inet tracewing
";
/// How long polkit's dialog may stay open.
const ASK_LIMIT: Duration = Duration::from_secs(10 * 60);
/// After an error, how long to wait for the rest of what `nft` says.
const ERROR_TAIL: Duration = Duration::from_millis(300);

pub struct QuicBlock {
    nft: Child,
    commands: Option<ChildStdin>,
}

enum Said {
    InPlace,
    Error(String),
    Ended,
}

fn nft_path() -> Option<&'static str> {
    ["/usr/sbin/nft", "/sbin/nft", "/usr/bin/nft"]
        .into_iter()
        .find(|path| Path::new(path).exists())
}

fn is_root() -> bool {
    // SAFETY: no arguments, and it cannot fail.
    unsafe { libc::geteuid() == 0 }
}

pub fn block_quic(_data_dir: &Path) -> Result<QuicBlock> {
    let nft = nft_path().context("nft is not installed, and tracewing blocks QUIC with it")?;
    let mut command = if is_root() {
        Command::new(nft)
    } else {
        let mut command = Command::new("pkexec");
        command.arg(nft);
        command
    };
    let mut child = command
        .arg("--interactive")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("start nft")?;
    let mut commands = child.stdin.take().context("no pipe to nft")?;
    let stdout = child.stdout.take().context("no pipe from nft")?;
    let stderr = child.stderr.take().context("no pipe from nft")?;
    let (tx, rx) = mpsc::channel();
    let said = tx.clone();
    std::thread::spawn(move || {
        let listed = BufReader::new(stdout)
            .lines()
            .map_while(Result::ok)
            .any(|line| line.contains("udp dport 443"));
        let _ = said.send(if listed { Said::InPlace } else { Said::Ended });
    });
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if tx.send(Said::Error(line)).is_err() {
                break;
            }
        }
    });
    // Waits in the pipe while polkit asks; one that never runs is told below.
    let _ = commands.write_all(RULES.as_bytes());

    let deadline = Instant::now() + ASK_LIMIT;
    let mut errors = Vec::new();
    loop {
        let wait = if errors.is_empty() {
            deadline.saturating_duration_since(Instant::now())
        } else {
            ERROR_TAIL
        };
        match rx.recv_timeout(wait) {
            Ok(Said::InPlace) if errors.is_empty() => {
                return Ok(QuicBlock {
                    nft: child,
                    commands: Some(commands),
                });
            }
            Ok(Said::Error(line)) => errors.push(line),
            Ok(Said::InPlace | Said::Ended) | Err(_) => break,
        }
    }
    // Without its commands, nft ends, and the table with it. Before it runs,
    // pkexec is still this user's and can be stopped.
    drop(commands);
    let _ = child.kill();
    let code = child.wait().ok().and_then(|status| status.code());
    bail!("{}", failure(code, &errors))
}

/// What went wrong, from what `pkexec` and `nft` said.
fn failure(code: Option<i32>, errors: &[String]) -> String {
    let said = errors
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let first = said.first().copied().unwrap_or_default();
    if first.contains("Not authorized")
        || first.contains("dismissed")
        || (said.is_empty() && matches!(code, Some(126 | 127)))
    {
        return "administrator rights were not given".to_owned();
    }
    if first.contains("authentication agent") {
        return "no password dialog could be shown (no polkit agent is running)".to_owned();
    }
    if said.is_empty() {
        return "nft stopped without saying why".to_owned();
    }
    let text = said.join(" ");
    if text.contains("owner") || text.contains("Not supported") {
        return format!(
            "{first}. Blocking QUIC needs Linux 5.12 and nftables 0.9.9 or later, \
             which can take the block away by themselves"
        );
    }
    first.to_owned()
}

impl QuicBlock {
    pub fn release(mut self) -> Result<()> {
        self.end();
        Ok(())
    }

    fn end(&mut self) {
        if self.commands.take().is_some() {
            let _ = self.nft.wait();
        }
    }
}

impl Drop for QuicBlock {
    fn drop(&mut self) {
        self.end();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &[&str]) -> Vec<String> {
        text.iter().map(|l| l.to_string()).collect()
    }

    #[test]
    fn failures_say_why() {
        assert_eq!(
            failure(Some(126), &[]),
            "administrator rights were not given"
        );
        assert_eq!(
            failure(
                Some(127),
                &lines(&["Error executing command as another user: Not authorized"])
            ),
            "administrator rights were not given"
        );
        assert!(
            failure(
                Some(127),
                &lines(&[
                    "Error executing command as another user: No authentication agent found."
                ])
            )
            .contains("polkit")
        );
        let old = failure(
            Some(0),
            &lines(&[
                "Error: syntax error, unexpected owner",
                "add table inet tracewing { flags owner; }",
                "                                 ^^^^^",
            ]),
        );
        assert!(old.starts_with("Error: syntax error"), "{old}");
        assert!(old.contains("Linux 5.12"), "{old}");
        assert_eq!(failure(Some(1), &[]), "nft stopped without saying why");
    }

    #[test]
    fn the_rules_end_with_the_listing_that_says_they_are_in() {
        assert!(RULES.contains("flags owner"));
        assert!(RULES.trim_end().ends_with("list table inet tracewing"));
    }
}
