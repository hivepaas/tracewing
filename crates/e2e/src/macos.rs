//! What a test on a Mac needs that macOS would otherwise ask a person for,
//! done with `sudo` instead: a runner gives it without asking, and on a Mac
//! somebody runs the suite at, `sudo` asks once in the terminal. Each puts
//! back what it changed when it is dropped, and takes out only what it put
//! there: a Mac may be somebody's own.

use std::{
    net::IpAddr,
    path::{Path, PathBuf},
    process::Command,
};

/// The name a site of the tests' is asked for by, where a client needs a
/// name rather than an address.
pub const SITE_NAME: &str = "e2e-site.test";

/// `name` for `ip` in /etc/hosts, until dropped.
pub struct Hosts(String);

impl Hosts {
    pub fn add(ip: IpAddr, name: &str) -> Self {
        let line = format!("{ip} {name}");
        let status = Command::new("sudo")
            .args(["sh", "-c", &format!("echo '{line}' >> /etc/hosts")])
            .status()
            .unwrap();
        assert!(status.success(), "add {name} to /etc/hosts");
        flush();
        Hosts(name.to_owned())
    }
}

impl Drop for Hosts {
    fn drop(&mut self) {
        let _ = Command::new("sudo")
            .args(["sed", "-i", "", &format!("/ {}$/d", self.0), "/etc/hosts"])
            .status();
        flush();
    }
}

/// What macOS has cached of names, forgotten, so /etc/hosts is read again.
fn flush() {
    let _ = Command::new("sudo")
        .args(["dscacheutil", "-flushcache"])
        .status();
    let _ = Command::new("sudo")
        .args(["killall", "-HUP", "mDNSResponder"])
        .status();
}

/// A certificate trusted as a root by this Mac, for every user, until
/// dropped: what an administrator's "Always Trust" does, by `security` with
/// sudo. macOS still asks a person before it changes, even under sudo, and
/// nothing lets a runner skip that: so only a test somebody answers for
/// (`only_where_someone_answers!`, with --attended) may use it.
/// Taken out by its own SHA-1, never by name, so another CA of tracewing's
/// stays.
pub struct Trusted {
    pem: PathBuf,
    sha1: String,
}

impl Trusted {
    pub fn root(pem: &Path) -> Self {
        let out = Command::new("/usr/bin/openssl")
            .args(["x509", "-noout", "-fingerprint", "-sha1", "-in"])
            .arg(pem)
            .output()
            .unwrap();
        assert!(out.status.success(), "fingerprint {}", pem.display());
        let sha1 = String::from_utf8_lossy(&out.stdout)
            .split('=')
            .nth(1)
            .unwrap_or_default()
            .trim()
            .replace(':', "");
        let status = Command::new("sudo")
            .args([
                "/usr/bin/security",
                "add-trusted-cert",
                "-d",
                "-r",
                "trustRoot",
                "-k",
                "/Library/Keychains/System.keychain",
            ])
            .arg(pem)
            .status()
            .unwrap();
        assert!(status.success(), "trust {}", pem.display());
        Trusted {
            pem: pem.to_owned(),
            sha1,
        }
    }
}

impl Drop for Trusted {
    fn drop(&mut self) {
        let _ = Command::new("sudo")
            .args(["/usr/bin/security", "remove-trusted-cert", "-d"])
            .arg(&self.pem)
            .status();
        let _ = Command::new("sudo")
            .args([
                "/usr/bin/security",
                "delete-certificate",
                "-Z",
                &self.sha1,
                "/Library/Keychains/System.keychain",
            ])
            .status();
    }
}
