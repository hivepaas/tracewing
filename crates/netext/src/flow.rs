//! What the extension decides about a flow, apart from macOS's objects: whether
//! to take it, and what it says to tracewing's listener when it does. Plain
//! values in and out, so that it is tested on every system.

/// The bundle id of the extension, which is also its signing id.
pub const EXTENSION_ID: &str = "com.hivepaas.tracewing.proxy";

/// The keys of the configuration tracewing saves with the proxy
/// (`NETunnelProviderProtocol.providerConfiguration`), each a string.
pub const PORT_KEY: &str = "port";
/// The signing ids of the apps whose flows to take, one per line.
pub const APPS_KEY: &str = "apps";

/// What tracewing's listener is told of the app behind a `CONNECT`.
pub const APP_HEADER: &str = "X-Tracewing-App";

/// Programs whose flows are never taken, whatever the configuration says:
/// tracewing's own connections out, which would come back to it forever.
const NEVER: &[&str] = &["com.hivepaas.tracewing", EXTENSION_ID];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    /// Where tracewing listens, on 127.0.0.1.
    pub port: u16,
    pub apps: Vec<String>,
}

impl Config {
    /// The configuration from its two strings; none when the port is not one.
    pub fn parse(port: &str, apps: &str) -> Option<Config> {
        let port = port.trim().parse().ok().filter(|&p| p != 0)?;
        let apps = apps
            .lines()
            .map(str::trim)
            .filter(|a| !a.is_empty() && !NEVER.contains(a))
            .map(str::to_owned)
            .collect();
        Some(Config { port, apps })
    }

    /// The two strings of this configuration, as tracewing saves them.
    pub fn to_strings(&self) -> (String, String) {
        (self.port.to_string(), self.apps.join("\n"))
    }

    /// Whether a flow from `app` to `ip`, port `port`, goes to tracewing.
    pub fn takes(&self, app: &str, ip: &str, port: u16) -> bool {
        !NEVER.contains(&app)
            && self.apps.iter().any(|a| a == app)
            // What already goes to tracewing goes on as it is.
            && !(is_loopback(ip) && port == self.port)
    }
}

fn is_loopback(ip: &str) -> bool {
    ip.parse::<std::net::IpAddr>()
        .is_ok_and(|ip| ip.is_loopback())
}

/// The host to name in the `CONNECT`: the name the app asked for where it gave
/// one and it is a plain host name, else the address it connects to.
pub fn host<'a>(name: Option<&'a str>, ip: &'a str) -> &'a str {
    match name {
        Some(name) if is_host_name(name) => name,
        _ => ip,
    }
}

/// A DNS name, or an address: nothing that could end the request line early.
fn is_host_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 253
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._:%".contains(&b))
}

/// `host:port`, an IPv6 address in brackets.
pub fn authority(host: &str, port: u16) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

/// The request that opens the tunnel through tracewing for one flow.
pub fn connect_request(host: &str, port: u16, app: &str, pid: Option<u32>) -> String {
    let authority = authority(host, port);
    // A signing id is a reverse-DNS name; anything else is left out rather
    // than let into a header.
    let app: String = app
        .chars()
        .filter(|c| c.is_ascii_graphic() && *c != ';')
        .collect();
    let pid = pid.map(|p| format!("; pid={p}")).unwrap_or_default();
    format!("CONNECT {authority} HTTP/1.1\r\nHost: {authority}\r\n{APP_HEADER}: {app}{pid}\r\n\r\n")
}

/// Whether the head of tracewing's answer opens the tunnel: a 2xx status.
pub fn tunnel_opened(head: &[u8]) -> bool {
    let line = head.split(|&b| b == b'\n').next().unwrap_or_default();
    let mut parts = line.split(|&b| b == b' ');
    matches!(parts.next(), Some(b"HTTP/1.1" | b"HTTP/1.0"))
        && parts
            .next()
            .is_some_and(|code| code.len() == 3 && code[0] == b'2')
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

    fn config() -> Config {
        Config::parse("20399", "com.apple.curl\n org.python.python \n\n").unwrap()
    }

    #[test]
    fn the_configuration_is_read_from_its_strings() {
        let c = config();
        assert_eq!(c.port, 20399);
        assert_eq!(c.apps, ["com.apple.curl", "org.python.python"]);
        assert_eq!(Config::parse(&c.to_strings().0, &c.to_strings().1), Some(c));
        assert_eq!(Config::parse("0", "com.apple.curl"), None);
        assert_eq!(Config::parse("nine", "com.apple.curl"), None);
        // tracewing itself is never named, whatever was saved.
        assert!(
            Config::parse("1", "com.hivepaas.tracewing")
                .unwrap()
                .apps
                .is_empty()
        );
    }

    #[test]
    fn only_the_apps_named_are_taken() {
        let c = config();
        assert!(c.takes("com.apple.curl", "93.184.215.14", 443));
        assert!(c.takes("org.python.python", "2606:4700::6810:85e5", 80));
        assert!(!c.takes("com.apple.Safari", "93.184.215.14", 443));
        assert!(!c.takes("", "93.184.215.14", 443));
        // A server on this computer is taken; tracewing's own port is not.
        assert!(c.takes("com.apple.curl", "127.0.0.1", 8080));
        assert!(!c.takes("com.apple.curl", "127.0.0.1", 20399));
        assert!(!c.takes("com.apple.curl", "::1", 20399));
    }

    #[test]
    fn tracewing_is_never_taken() {
        let c = Config {
            port: 9090,
            apps: vec!["com.hivepaas.tracewing".into(), EXTENSION_ID.into()],
        };
        assert!(!c.takes("com.hivepaas.tracewing", "93.184.215.14", 443));
        assert!(!c.takes(EXTENSION_ID, "93.184.215.14", 443));
    }

    #[test]
    fn the_name_asked_for_is_named_when_it_is_one() {
        assert_eq!(host(Some("example.com"), "93.184.215.14"), "example.com");
        assert_eq!(host(None, "93.184.215.14"), "93.184.215.14");
        assert_eq!(host(Some(""), "93.184.215.14"), "93.184.215.14");
        assert_eq!(
            host(Some("evil.com\r\nX: 1"), "93.184.215.14"),
            "93.184.215.14"
        );
        assert_eq!(host(Some("a b"), "::1"), "::1");
    }

    #[test]
    fn the_request_names_the_host_and_the_app() {
        assert_eq!(
            connect_request("example.com", 443, "com.apple.curl", Some(4242)),
            "CONNECT example.com:443 HTTP/1.1\r\nHost: example.com:443\r\n\
             X-Tracewing-App: com.apple.curl; pid=4242\r\n\r\n"
        );
        assert_eq!(
            connect_request("2606:4700::1", 80, "a\r\nb; c", None),
            "CONNECT [2606:4700::1]:80 HTTP/1.1\r\nHost: [2606:4700::1]:80\r\n\
             X-Tracewing-App: abc\r\n\r\n"
        );
    }

    #[test]
    fn a_2xx_answer_opens_the_tunnel() {
        assert!(tunnel_opened(b"HTTP/1.1 200 OK\r\n\r\n"));
        assert!(tunnel_opened(
            b"HTTP/1.0 200 Connection established\r\n\r\n"
        ));
        assert!(!tunnel_opened(b"HTTP/1.1 502 Bad Gateway\r\n\r\n"));
        assert!(!tunnel_opened(b"HTTP/1.1 2000\r\n\r\n"));
        assert!(!tunnel_opened(b"SSH-2.0-OpenSSH\r\n"));
        assert!(!tunnel_opened(b""));
    }

    #[test]
    fn the_pid_is_the_sixth_word_of_the_audit_token() {
        let mut token = [0u8; 32];
        token[20..24].copy_from_slice(&4242u32.to_ne_bytes());
        assert_eq!(pid_of(&token), Some(4242));
        assert_eq!(pid_of(&token[..23]), None);
    }
}
