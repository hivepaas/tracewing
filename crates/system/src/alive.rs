//! A lock that says whether the app which turned the system proxy on is still
//! running.
//!
//! The recovery timer cannot ask the app: it is a separate program, started a
//! minute at a time, and it must never put the old settings back under a
//! healthy app. So the app holds a lock on a file for as long as the system
//! proxy is its own, and the operating system lets that lock go however the
//! app ends — a quit, a crash, `kill -9`, the machine losing power. A lock
//! nobody holds is the one honest sign that the app has gone. A process id is
//! not: the number comes round again, and the next holder of it is somebody
//! else's program.

use anyhow::{Context, Result};
use std::{fs, io, path::Path};

pub(crate) const ALIVE_FILE: &str = "sysproxy.alive";

/// The lock, held for as long as the system proxy is the app's. It goes when
/// this is dropped, and when the process ends in any way at all.
pub struct Alive(
    /// Holding the file open is what holds the lock.
    #[allow(dead_code)]
    fs::File,
);

/// Takes the lock for this process. An error means somebody else holds it,
/// which is another tracewing with the same data folder.
pub fn hold_alive(data_dir: &Path) -> Result<Alive> {
    let path = data_dir.join(ALIVE_FILE);
    let file = open(&path, true).with_context(|| format!("open {}", path.display()))?;
    lock(&file).with_context(|| format!("take the lock on {}", path.display()))?;
    Ok(Alive(file))
}

/// Whether the app that turned the system proxy on has gone.
///
/// Only a lock this can take says so. Everything else is answered with
/// `false`, which leaves the settings as they are: a lock somebody holds, a
/// file that cannot be read. The file never being there at all is the one
/// other `true`, for a folder written by a version that kept no lock.
///
/// A `false` is worth nothing a minute later. Any program on the machine, in
/// the instant between forking and starting the new program, holds a copy of
/// what its parent had open, and that can be this lock. So this is asked
/// again and again rather than once, and the worst a wrong `false` costs is
/// another minute of waiting.
pub fn app_is_gone(data_dir: &Path) -> bool {
    let path = data_dir.join(ALIVE_FILE);
    match open(&path, false) {
        Ok(file) => lock(&file).is_ok(),
        Err(e) if e.kind() == io::ErrorKind::NotFound => true,
        Err(_) => false,
    }
}

/// Waits for the app that turned the system proxy on to go, however long that
/// takes, and returns the moment it has.
///
/// The same lock as `app_is_gone`, asked to wait rather than to answer. A
/// program forked in the meantime with a copy of the lock only makes this wait
/// for it too, which is the right answer: the lock comes free when the last
/// holder of it does. The lock is let go again on the way out, so that a
/// tracewing started a moment later can take it.
#[cfg(unix)]
pub fn wait_until_gone(data_dir: &Path) -> io::Result<()> {
    match open(&data_dir.join(ALIVE_FILE), false) {
        Ok(file) => wait_for_lock(&file),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Windows: the app's handle shares the file with nobody, so the open itself
/// is what waits, a second at a time, and the open that succeeds is the lock.
#[cfg(windows)]
pub fn wait_until_gone(data_dir: &Path) -> io::Result<()> {
    const SHARING_VIOLATION: i32 = 32;
    let path = data_dir.join(ALIVE_FILE);
    loop {
        match open(&path, false) {
            Ok(_) => return Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) if e.raw_os_error() == Some(SHARING_VIOLATION) => {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
            Err(e) => return Err(e),
        }
    }
}

/// The file itself is never emptied or removed: the lock is all it is for, and
/// a path that stays put is a path two processes can race for safely.
fn open(path: &Path, create: bool) -> io::Result<fs::File> {
    let mut options = fs::OpenOptions::new();
    options
        .read(true)
        .write(true)
        .create(create)
        .truncate(false);
    #[cfg(windows)]
    {
        // Windows has no advisory lock of its own kind: the holder asks to
        // share the file with nobody, and every other open of it fails for as
        // long as the handle lives. The open is the lock.
        std::os::windows::fs::OpenOptionsExt::share_mode(&mut options, 0);
    }
    options.open(path)
}

#[cfg(unix)]
fn lock(file: &fs::File) -> io::Result<()> {
    use std::os::unix::io::AsRawFd;
    // The lock belongs to this open file, and Rust opens files so that a
    // child process does not inherit them: the watchdog tracewing starts does
    // not hold the app's lock, and does not have to be told not to.
    // SAFETY: a descriptor borrowed from a file that is open.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(windows)]
fn lock(_file: &fs::File) -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn wait_for_lock(file: &fs::File) -> io::Result<()> {
    use std::os::unix::io::AsRawFd;
    loop {
        // SAFETY: a descriptor borrowed from a file that is open.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } == 0 {
            return Ok(());
        }
        let e = io::Error::last_os_error();
        if e.kind() != io::ErrorKind::Interrupted {
            return Err(e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn folder(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("alive-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Whether the lock comes free within `limit`.
    ///
    /// A process forked anywhere in this program holds a copy of every open
    /// descriptor, the lock among them, for the moment between the fork and
    /// the exec that closes it again — and the tests beside this one start
    /// programs of their own. So a single look can say "still running" for
    /// somebody else's `sh -c`. The timer meets the same moment and answers
    /// it the same way, by looking again later; it has a minute, and this has
    /// a few seconds.
    fn gone_within(dir: &Path, limit: Duration) -> bool {
        let until = Instant::now() + limit;
        loop {
            if app_is_gone(dir) {
                return true;
            }
            if Instant::now() >= until {
                return false;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn the_lock_says_who_is_running() {
        let dir = folder("held");
        // A folder nothing has ever locked: whatever wrote a journal there is
        // not running now.
        assert!(app_is_gone(&dir));

        let held = hold_alive(&dir).unwrap();
        assert!(!app_is_gone(&dir), "the lock is held");
        // A second app on the same folder is told so rather than let through.
        assert!(hold_alive(&dir).is_err());

        drop(held);
        assert!(
            gone_within(&dir, Duration::from_secs(5)),
            "dropped with the app's last breath"
        );
        fs::remove_dir_all(&dir).ok();
    }

    /// The agent on macOS waits rather than asks: it has to wake when the app
    /// goes, and not before.
    #[test]
    fn a_wait_ends_when_the_app_does() {
        let dir = folder("wait");
        // A folder nothing has ever locked: no wait at all.
        wait_until_gone(&dir).unwrap();

        let held = hold_alive(&dir).unwrap();
        let (done, ended) = std::sync::mpsc::channel();
        let waiting = dir.clone();
        std::thread::spawn(move || done.send(wait_until_gone(&waiting).is_ok()));
        assert!(
            ended.recv_timeout(Duration::from_millis(300)).is_err(),
            "it waits for as long as the app holds the lock"
        );
        drop(held);
        assert_eq!(
            ended.recv_timeout(Duration::from_secs(10)),
            Ok(true),
            "and ends when the app lets it go"
        );
        // It let the lock go again on its way out, for the next tracewing.
        assert!(gone_within(&dir, Duration::from_secs(5)));
        fs::remove_dir_all(&dir).ok();
    }

    /// The watchdog outlives the app on purpose. If it inherited the lock, the
    /// app would look alive for as long as the watchdog ran, and the timer
    /// would never put the settings back.
    #[cfg(unix)]
    #[test]
    fn a_child_process_does_not_inherit_the_lock() {
        let dir = folder("child");
        let held = hold_alive(&dir).unwrap();
        let mut child = std::process::Command::new("sleep")
            .arg("30")
            .spawn()
            .unwrap();
        drop(held);
        // A child that held the lock would hold it for the whole half-minute,
        // so waiting a few seconds tells the two apart.
        let gone = gone_within(&dir, Duration::from_secs(5));
        child.kill().ok();
        child.wait().ok();
        assert!(gone, "the child holds nothing");
        fs::remove_dir_all(&dir).ok();
    }
}
