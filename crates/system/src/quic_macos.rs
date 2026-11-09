//! macOS: a rule of `pf`, the packet filter, in an anchor of its own under
//! `com.apple/`, which the system's `pf.conf` already reads. A shell runs as
//! root through the password dialog, puts the rule in, and stays until this
//! process says stop or is gone, then takes it out. pf forgets every rule at a
//! restart, so a power cut leaves nothing either.

use anyhow::{Context, Result, bail};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

const ANCHOR: &str = "com.apple/tracewing";
/// How long the password dialog may stay open.
const ASK_LIMIT: Duration = Duration::from_secs(10 * 60);
/// How long a stop may take: the shell looks once a second.
const STOP_LIMIT: Duration = Duration::from_secs(5);

pub struct QuicBlock {
    shell: Child,
    stop: PathBuf,
}

fn quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', r"'\''"))
}

/// The root shell, on one line. It says it is ready by making `ready`, and
/// ends when `stop` appears or `pid` is gone. `return` answers a client at once,
/// so it falls back to TCP without waiting.
fn script(pid: u32, ready: &Path, stop: &Path) -> String {
    let (ready, stop) = (quote(ready), quote(stop));
    [
        format!("a={ANCHOR}"),
        "echo 'block return out quick proto udp from any to any port 443' \
         | /sbin/pfctl -q -a \"$a\" -f - || exit 3"
            .to_owned(),
        "t=$(/sbin/pfctl -E 2>&1 | /usr/bin/sed -n 's/^Token : //p')".to_owned(),
        format!(": > {ready}"),
        format!("while /bin/kill -0 {pid} 2>/dev/null && [ ! -e {stop} ]; do /bin/sleep 1; done"),
        "/sbin/pfctl -q -a \"$a\" -F rules".to_owned(),
        "if [ -n \"$t\" ]; then /sbin/pfctl -q -X \"$t\"; fi".to_owned(),
        format!("/bin/rm -f {ready} {stop}"),
    ]
    .join("; ")
}

fn as_admin(script: &str) -> String {
    let escaped = script.replace('\\', r"\\").replace('"', "\\\"");
    format!("do shell script \"{escaped}\" with administrator privileges")
}

fn is_root() -> bool {
    // SAFETY: no arguments, and it cannot fail.
    unsafe { libc::geteuid() == 0 }
}

pub fn block_quic(data_dir: &Path) -> Result<QuicBlock> {
    let dir = data_dir.join("quic");
    fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let (ready, stop) = (dir.join("ready"), dir.join("stop"));
    // Left by a shell that a restart cut short.
    let _ = fs::remove_file(&ready);
    let _ = fs::remove_file(&stop);
    let script = script(std::process::id(), &ready, &stop);
    let mut command = if is_root() {
        let mut command = Command::new("/bin/sh");
        command.arg("-c").arg(&script);
        command
    } else {
        let mut command = Command::new("/usr/bin/osascript");
        command.arg("-e").arg(as_admin(&script));
        command
    };
    let mut shell = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("start the shell that blocks QUIC")?;

    let deadline = Instant::now() + ASK_LIMIT;
    loop {
        if ready.exists() {
            return Ok(QuicBlock { shell, stop });
        }
        if let Some(status) = shell.try_wait()? {
            let mut said = String::new();
            if let Some(mut stderr) = shell.stderr.take() {
                let _ = stderr.read_to_string(&mut said);
            }
            bail!("{}", failure(status.code(), &said));
        }
        if Instant::now() > deadline {
            let _ = fs::write(&stop, b"");
            let _ = shell.kill();
            bail!("no password was given");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// What went wrong, from what `osascript` said.
fn failure(code: Option<i32>, said: &str) -> String {
    let said = said.trim();
    if said.contains("-128") || said.contains("User canceled") {
        return "administrator rights were not given".to_owned();
    }
    if said.contains("(3)") || code == Some(3) {
        return format!("pf did not take the rule: {said}");
    }
    if said.is_empty() {
        return "the shell that blocks QUIC stopped without saying why".to_owned();
    }
    said.to_owned()
}

impl QuicBlock {
    pub fn release(mut self) -> Result<()> {
        self.end();
        Ok(())
    }

    fn end(&mut self) {
        if fs::write(&self.stop, b"").is_err() {
            return;
        }
        let deadline = Instant::now() + STOP_LIMIT;
        while Instant::now() < deadline {
            if !matches!(self.shell.try_wait(), Ok(None)) {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

impl Drop for QuicBlock {
    fn drop(&mut self) {
        if !matches!(self.shell.try_wait(), Ok(Some(_))) {
            self.end();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shell_puts_the_rule_in_and_takes_it_out() {
        let script = script(
            42,
            Path::new("/Users/a b/quic/ready"),
            Path::new("/Users/it's/quic/stop"),
        );
        assert!(script.contains("pfctl -q -a \"$a\" -f -"), "{script}");
        assert!(script.contains("kill -0 42"), "{script}");
        assert!(script.contains(": > '/Users/a b/quic/ready'"), "{script}");
        assert!(
            script.contains(r"[ ! -e '/Users/it'\''s/quic/stop' ]"),
            "{script}"
        );
        assert!(script.contains("-F rules"), "{script}");
        // One line, for AppleScript.
        assert!(!script.contains('\n'));
        let admin = as_admin(&script);
        assert!(admin.contains(r#"-a \"$a\""#), "{admin}");
    }

    #[test]
    fn failures_say_why() {
        assert_eq!(
            failure(Some(1), "0:120: execution error: User canceled. (-128)"),
            "administrator rights were not given"
        );
        assert!(
            failure(Some(1), "execution error: pfctl: Syntax error (3)").starts_with("pf did not")
        );
    }
}
