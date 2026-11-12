//! The TLS secrets of each connection, written as browsers write them for
//! `SSLKEYLOGFILE`, so that Wireshark can read a capture of tracewing's own
//! traffic: from apps to it, and from it to servers. Off unless switched on,
//! and off again at the next start, unless `SSLKEYLOGFILE` says where to write.

use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

/// The variable browsers and curl read for the same file.
pub const ENV: &str = "SSLKEYLOGFILE";

#[derive(Debug, Default)]
pub struct TlsKeys {
    on: AtomicBool,
    file: Mutex<Option<(PathBuf, File)>>,
    written: AtomicU64,
    /// Why the last write failed, if it did.
    problem: Mutex<Option<String>>,
}

impl TlsKeys {
    /// Off, unless `SSLKEYLOGFILE` names a file it can be written to.
    pub fn from_env() -> Self {
        let keys = Self::default();
        if let Some(path) = std::env::var_os(ENV).filter(|p| !p.is_empty())
            && let Err(e) = keys.start(Path::new(&path))
        {
            eprintln!(
                "tracewing: cannot write TLS keys to {}: {e}",
                path.to_string_lossy()
            );
        }
        keys
    }

    /// Writes to the end of `path` from the next connection on. Only its
    /// owner may read a file it makes.
    pub fn start(&self, path: &Path) -> std::io::Result<()> {
        let opened = self.open(path);
        if let Err(e) = &opened {
            *self.problem.lock().unwrap_or_else(|e| e.into_inner()) = Some(e.to_string());
        }
        opened
    }

    fn open(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir)?;
        }
        let mut options = OpenOptions::new();
        options.create(true).append(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let file = options.open(path)?;
        *self.file.lock().unwrap_or_else(|e| e.into_inner()) = Some((path.to_owned(), file));
        *self.problem.lock().unwrap_or_else(|e| e.into_inner()) = None;
        self.on.store(true, Ordering::Relaxed);
        Ok(())
    }

    pub fn stop(&self) {
        self.on.store(false, Ordering::Relaxed);
        *self.file.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// Where it writes, while it does.
    pub fn path(&self) -> Option<PathBuf> {
        self.file
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|(path, _)| path.clone())
    }

    /// Lines written since it was made.
    pub fn written(&self) -> u64 {
        self.written.load(Ordering::Relaxed)
    }

    pub fn problem(&self) -> Option<String> {
        self.problem
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

impl hudsucker::rustls::KeyLog for TlsKeys {
    fn log(&self, label: &str, client_random: &[u8], secret: &[u8]) {
        let mut line =
            String::with_capacity(label.len() + 2 * (client_random.len() + secret.len()) + 3);
        line.push_str(label);
        line.push(' ');
        hex_into(&mut line, client_random);
        line.push(' ');
        hex_into(&mut line, secret);
        line.push('\n');
        let mut file = self.file.lock().unwrap_or_else(|e| e.into_inner());
        let Some((_, file)) = file.as_mut() else {
            return;
        };
        // One write a line: Wireshark reads it as it grows.
        match file.write_all(line.as_bytes()) {
            Ok(()) => {
                self.written.fetch_add(1, Ordering::Relaxed);
            }
            Err(e) => {
                *self.problem.lock().unwrap_or_else(|e| e.into_inner()) = Some(e.to_string());
            }
        }
    }

    fn will_log(&self, _label: &str) -> bool {
        self.on.load(Ordering::Relaxed)
    }
}

fn hex_into(out: &mut String, bytes: &[u8]) {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    for b in bytes {
        out.push(DIGITS[(b >> 4) as usize] as char);
        out.push(DIGITS[(b & 15) as usize] as char);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hudsucker::rustls::KeyLog;

    #[test]
    fn it_writes_lines_as_browsers_do_while_on() {
        let dir = std::env::temp_dir().join(format!("key-log-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("keys").join("tls.log");
        let keys = TlsKeys::default();
        assert!(!keys.will_log("CLIENT_RANDOM"));
        keys.log("CLIENT_RANDOM", &[1, 2], &[0xab]);

        keys.start(&path).unwrap();
        assert!(keys.will_log("CLIENT_TRAFFIC_SECRET_0"));
        keys.log("CLIENT_TRAFFIC_SECRET_0", &[0, 0xff], &[0x10, 0x2a]);
        keys.stop();
        keys.log("SERVER_TRAFFIC_SECRET_0", &[1], &[2]);
        assert!(!keys.will_log("SERVER_TRAFFIC_SECRET_0"));

        // Again: it goes on at the end.
        keys.start(&path).unwrap();
        keys.log("EXPORTER_SECRET", &[3], &[4]);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "CLIENT_TRAFFIC_SECRET_0 00ff 102a\nEXPORTER_SECRET 03 04\n"
        );
        assert_eq!(keys.written(), 2);
        assert_eq!(keys.path().as_deref(), Some(path.as_path()));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600, "only its owner reads it");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
