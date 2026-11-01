//! Which hosts are not decrypted. Their traffic goes through as it is, unread: an
//! app that pins its certificates fails if it is decrypted, and some traffic is
//! nobody's business. Patterns are a host, or `*.example.com` for it and everything
//! under it.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// What is left alone until the user says otherwise: the systems' own services,
/// which pin their certificates, and which a debugging session breaking would
/// take a while to notice.
const DEFAULT_NO_DECRYPT: [&str; 5] = [
    "*.apple.com",
    "*.icloud.com",
    "*.mzstatic.com",
    "*.windowsupdate.com",
    "*.update.microsoft.com",
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HostRules {
    pub no_decrypt: Vec<String>,
}

impl Default for HostRules {
    fn default() -> Self {
        Self {
            no_decrypt: DEFAULT_NO_DECRYPT.map(str::to_owned).to_vec(),
        }
    }
}

/// `host:443` and `HOST` are the same host.
fn plain_host(host: &str) -> String {
    let host = match host.rsplit_once(':') {
        Some((name, port)) if port.bytes().all(|b| b.is_ascii_digit()) => name,
        _ => host,
    };
    host.trim_end_matches('.').to_ascii_lowercase()
}

fn pattern_fits(pattern: &str, host: &str) -> bool {
    match pattern.strip_prefix("*.") {
        Some(domain) => {
            host == domain
                || (host.len() > domain.len()
                    && host.ends_with(domain)
                    && host.as_bytes()[host.len() - domain.len() - 1] == b'.')
        }
        None => pattern == host,
    }
}

impl HostRules {
    /// The file's rules; no file means the defaults.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text =
            std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("read {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        std::fs::write(path, toml::to_string(self)?)
            .with_context(|| format!("write {}", path.display()))
    }

    /// One pattern a line, as a person writes them.
    pub fn from_lines(text: &str) -> Self {
        Self {
            no_decrypt: text
                .lines()
                .map(|line| line.trim().to_ascii_lowercase())
                .filter(|line| !line.is_empty())
                .collect(),
        }
    }

    pub fn to_lines(&self) -> String {
        self.no_decrypt.join("\n")
    }

    /// Whether this host's traffic is left alone.
    pub fn leaves_alone(&self, host: &str) -> bool {
        let host = plain_host(host);
        self.no_decrypt
            .iter()
            .any(|pattern| pattern_fits(pattern, &host))
    }

    pub fn leave_alone(&mut self, host: &str) {
        if !self.leaves_alone(host) {
            self.no_decrypt.push(plain_host(host));
        }
    }

    /// Takes away whatever leaves this host alone. A pattern for its whole domain
    /// goes with it: the list is there to read, and nothing is hidden.
    pub fn decrypt(&mut self, host: &str) {
        let host = plain_host(host);
        self.no_decrypt
            .retain(|pattern| !pattern_fits(pattern, &host));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(patterns: &[&str]) -> HostRules {
        HostRules {
            no_decrypt: patterns.iter().map(|p| (*p).to_owned()).collect(),
        }
    }

    #[test]
    fn hosts_and_domains_fit() {
        let r = rules(&["*.bank.vn", "api.example.com"]);
        for yes in [
            "bank.vn",
            "www.bank.vn",
            "a.b.bank.vn:443",
            "WWW.BANK.VN",
            "www.bank.vn.",
            "api.example.com:8443",
        ] {
            assert!(r.leaves_alone(yes), "{yes}");
        }
        for no in [
            "notbank.vn",
            "bank.vn.evil.com",
            "example.com",
            "x.api.example.com",
        ] {
            assert!(!r.leaves_alone(no), "{no}");
        }
    }

    #[test]
    fn nothing_listed_means_everything_is_decrypted() {
        assert!(!rules(&[]).leaves_alone("anything.com"));
    }

    #[test]
    fn a_bracketed_address_keeps_its_colons() {
        let r = rules(&["[::1]"]);
        assert!(r.leaves_alone("[::1]:443"));
        assert!(r.leaves_alone("[::1]"));
    }

    #[test]
    fn lines_go_both_ways() {
        let r = HostRules::from_lines("  *.Bank.vn \n\n\tapi.example.com\n");
        assert_eq!(r, rules(&["*.bank.vn", "api.example.com"]));
        assert_eq!(HostRules::from_lines(&r.to_lines()), r);
    }

    #[test]
    fn adding_and_taking_away() {
        let mut r = rules(&["*.bank.vn"]);
        r.leave_alone("www.bank.vn:443");
        assert_eq!(r, rules(&["*.bank.vn"]), "already left alone");
        r.leave_alone("pinned.dev:443");
        assert_eq!(r, rules(&["*.bank.vn", "pinned.dev"]));
        r.decrypt("www.bank.vn");
        assert_eq!(r, rules(&["pinned.dev"]), "its pattern goes");
        assert!(!r.leaves_alone("www.bank.vn"));
    }

    #[test]
    fn no_file_means_the_defaults_and_a_broken_one_says_so() {
        let dir = std::env::temp_dir().join(format!("rules-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("decrypt.toml");
        assert_eq!(HostRules::load(&path).unwrap(), HostRules::default());
        assert!(HostRules::default().leaves_alone("gateway.icloud.com"));
        let mine = rules(&["a.dev"]);
        mine.save(&path).unwrap();
        assert_eq!(HostRules::load(&path).unwrap(), mine);
        std::fs::write(&path, "no_decrypt = 3").unwrap();
        assert!(HostRules::load(&path).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
