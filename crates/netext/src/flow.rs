//! What the extension decides about a flow, apart from macOS's objects: which
//! flows to take, and what to say of one. Plain values in and out, so that it
//! is tested on every system. How it is said is
//! [`tracewing_handoff`]'s.

use tracewing_handoff::{Hello, Key};

/// The bundle id of the extension, which is also its signing id.
pub const EXTENSION_ID: &str = "com.hivepaas.tracewing.proxy";

/// The keys of the configuration tracewing saves with the proxy
/// (`NETunnelProviderProtocol.providerConfiguration`), each a string.
pub mod keys {
    /// The port, on 127.0.0.1, that tracewing takes the extension's flows on.
    pub const PORT: &str = "port";
    /// The key both ends prove they hold, as 64 hex digits.
    pub const KEY: &str = "key";
    /// The signing ids of the apps whose flows to take, one per line.
    pub const APPS: &str = "apps";
    /// tracewing's other ports on this computer, its proxy port among them,
    /// separated by commas: a flow going to one of them goes on as it is.
    pub const OWN: &str = "own";
}

/// Programs whose flows are never taken, whatever the configuration says:
/// tracewing's own connections out, which would come back to it forever.
const NEVER: &[&str] = &["com.hivepaas.tracewing", EXTENSION_ID];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    /// Where tracewing takes the extension's flows, on 127.0.0.1.
    pub port: u16,
    pub key: Key,
    pub apps: Vec<String>,
    /// tracewing's other ports.
    pub own: Vec<u16>,
}

impl Config {
    /// The configuration from what was saved, `saved` giving each key's
    /// string; none without a port and a key.
    pub fn read(saved: impl Fn(&str) -> Option<String>) -> Option<Config> {
        let port = saved(keys::PORT)?.trim().parse().ok().filter(|&p| p != 0)?;
        let key = Key::from_hex(&saved(keys::KEY)?)?;
        let apps = saved(keys::APPS)
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .filter(|a| !a.is_empty() && !NEVER.contains(a))
            .map(str::to_owned)
            .collect();
        let own = saved(keys::OWN)
            .unwrap_or_default()
            .split(',')
            .filter_map(|p| p.trim().parse().ok())
            .collect();
        Some(Config {
            port,
            key,
            apps,
            own,
        })
    }

    /// What tracewing saves, key and string.
    pub fn to_saved(&self) -> Vec<(&'static str, String)> {
        vec![
            (keys::PORT, self.port.to_string()),
            (keys::KEY, self.key.to_hex()),
            (keys::APPS, self.apps.join("\n")),
            (
                keys::OWN,
                self.own
                    .iter()
                    .map(u16::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
            ),
        ]
    }

    /// Whether a flow from `app` to `ip`, port `port`, goes to tracewing.
    pub fn takes(&self, app: &str, ip: &str, port: u16) -> bool {
        !NEVER.contains(&app)
            && self.apps.iter().any(|a| a == app)
            // What already goes to tracewing goes on as it is.
            && !(is_loopback(ip) && (port == self.port || self.own.contains(&port)))
    }
}

fn is_loopback(ip: &str) -> bool {
    ip.parse::<std::net::IpAddr>()
        .is_ok_and(|ip| ip.is_loopback())
}

/// What the extension says of a flow: the name the app asked for where it
/// gave one that is a plain host name, else the address it connects to.
pub fn hello(name: Option<&str>, ip: &str, port: u16, app: &str, pid: Option<u32>) -> Hello {
    let host = match name {
        Some(name) if tracewing_handoff::is_host(name) => name,
        _ => ip,
    };
    Hello {
        host: host.to_owned(),
        port,
        app: app.to_owned(),
        pid,
    }
}

/// The process id in an audit token (`audit_token_t`, eight 32-bit words: the
/// sixth is the pid).
pub fn pid_of(audit_token: &[u8]) -> Option<u32> {
    let word = audit_token.get(20..24)?;
    Some(u32::from_ne_bytes(word.try_into().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn key() -> Key {
        Key::from_bytes([3; 32])
    }

    fn saved(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |k| map.get(k).cloned()
    }

    fn config() -> Config {
        Config::read(saved(&[
            ("port", "20399"),
            ("key", &key().to_hex()),
            ("apps", "com.apple.curl\n org.python.python \n\n"),
            ("own", "9090, 1080"),
        ]))
        .unwrap()
    }

    #[test]
    fn the_configuration_is_read_as_it_was_saved() {
        let c = config();
        assert_eq!(c.port, 20399);
        assert_eq!(c.key, key());
        assert_eq!(c.apps, ["com.apple.curl", "org.python.python"]);
        assert_eq!(c.own, [9090, 1080]);
        let again: HashMap<&str, String> = c.to_saved().into_iter().collect();
        assert_eq!(Config::read(|k| again.get(k).cloned()), Some(c));
    }

    #[test]
    fn no_port_or_no_key_is_no_configuration() {
        let hex = key().to_hex();
        assert_eq!(Config::read(saved(&[("key", &hex)])), None);
        assert_eq!(Config::read(saved(&[("port", "0"), ("key", &hex)])), None);
        assert_eq!(Config::read(saved(&[("port", "9")])), None);
        assert_eq!(
            Config::read(saved(&[("port", "9"), ("key", "short")])),
            None
        );
        let bare = Config::read(saved(&[("port", "9"), ("key", &hex)])).unwrap();
        assert!(bare.apps.is_empty() && bare.own.is_empty());
        // tracewing itself is never named, whatever was saved.
        let named = Config::read(saved(&[
            ("port", "9"),
            ("key", &hex),
            ("apps", "com.hivepaas.tracewing"),
        ]))
        .unwrap();
        assert!(named.apps.is_empty());
    }

    #[test]
    fn only_the_apps_named_are_taken() {
        let c = config();
        assert!(c.takes("com.apple.curl", "93.184.215.14", 443));
        assert!(c.takes("org.python.python", "2606:4700::6810:85e5", 80));
        assert!(!c.takes("com.apple.Safari", "93.184.215.14", 443));
        assert!(!c.takes("", "93.184.215.14", 443));
        // A server on this computer is taken; tracewing's own ports are not.
        assert!(c.takes("com.apple.curl", "127.0.0.1", 8080));
        assert!(!c.takes("com.apple.curl", "127.0.0.1", 20399));
        assert!(!c.takes("com.apple.curl", "::1", 20399));
        assert!(
            !c.takes("com.apple.curl", "127.0.0.1", 9090),
            "curl --proxy"
        );
        assert!(
            c.takes("com.apple.curl", "93.184.215.14", 9090),
            "elsewhere"
        );
    }

    #[test]
    fn tracewing_is_never_taken() {
        let c = Config {
            apps: vec!["com.hivepaas.tracewing".into(), EXTENSION_ID.into()],
            ..config()
        };
        assert!(!c.takes("com.hivepaas.tracewing", "93.184.215.14", 443));
        assert!(!c.takes(EXTENSION_ID, "93.184.215.14", 443));
    }

    #[test]
    fn the_name_asked_for_is_said_when_it_is_one() {
        let said = |name| hello(name, "93.184.215.14", 443, "com.apple.curl", Some(1)).host;
        assert_eq!(said(Some("example.com")), "example.com");
        assert_eq!(said(None), "93.184.215.14");
        assert_eq!(said(Some("")), "93.184.215.14");
        assert_eq!(said(Some("evil.com\r\nX: 1")), "93.184.215.14");
    }

    #[test]
    fn the_pid_is_the_sixth_word_of_the_audit_token() {
        let mut token = [0u8; 32];
        token[20..24].copy_from_slice(&4242u32.to_ne_bytes());
        assert_eq!(pid_of(&token), Some(4242));
        assert_eq!(pid_of(&token[..23]), None);
    }
}
