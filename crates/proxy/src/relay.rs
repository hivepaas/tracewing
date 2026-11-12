//! Another proxy that tracewing's own connections to servers go through, as
//! a company's network asks: an HTTP proxy, or SOCKS5. The app's traffic
//! still comes to tracewing, which reads it as ever, then goes on through
//! this one. Kept in `relay.toml`; a change applies to the next connection.
//!
//! The server's name goes to the relay as it is, for the relay to look up:
//! inside a company, its own DNS often cannot answer for the internet.

use anyhow::{Context as _, Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

pub const FILE: &str = "relay.toml";
/// The longest a relay may take to say it has made the tunnel.
const ANSWER_WAIT: Duration = Duration::from_secs(30);
/// The most of a relay's answer to CONNECT that is read.
const ANSWER_MAX: usize = 16 * 1024;

/// What the user writes: the relay's URL, empty for none, and the hosts to
/// reach straight.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RelaySettings {
    /// `http://proxy.corp:8080`, `http://user:password@proxy.corp:8080`,
    /// `socks5://proxy.corp:1080`.
    pub proxy: String,
    /// Hosts reached without it, as `NO_PROXY` has them: `example.com` is it
    /// and everything under it, `*` is every host.
    pub bypass: String,
}

impl Default for RelaySettings {
    fn default() -> Self {
        Self {
            proxy: String::new(),
            bypass: "localhost, 127.0.0.1, ::1".to_owned(),
        }
    }
}

impl RelaySettings {
    /// None, or what `relay.toml` in `data_dir` says.
    pub fn load(data_dir: &Path) -> Result<Self> {
        let path = data_dir.join(FILE);
        if !path.exists() {
            return Ok(Self::default());
        }
        let text =
            std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let settings: Self =
            toml::from_str(&text).with_context(|| format!("read {}", path.display()))?;
        Route::parse(&settings)?;
        Ok(settings)
    }

    pub fn save(&self, data_dir: &Path) -> Result<()> {
        Route::parse(self)?;
        let path = data_dir.join(FILE);
        std::fs::write(&path, toml::to_string(self)?)
            .with_context(|| format!("write {}", path.display()))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Http,
    Socks5,
}

/// A relay as it is used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    pub kind: Kind,
    pub host: String,
    pub port: u16,
    pub user: Option<(String, String)>,
    bypass: Vec<String>,
}

impl Route {
    /// `None` for no relay; an error for one that cannot be used.
    pub fn parse(settings: &RelaySettings) -> Result<Option<Self>> {
        let url = settings.proxy.trim();
        if url.is_empty() {
            return Ok(None);
        }
        let (scheme, rest) = url
            .split_once("://")
            .with_context(|| format!("\"{url}\" needs a scheme: http:// or socks5://"))?;
        let (kind, default_port) = match scheme.to_ascii_lowercase().as_str() {
            "http" => (Kind::Http, 8080),
            "socks5" | "socks5h" => (Kind::Socks5, 1080),
            "https" => bail!("a relay spoken to over TLS is not supported yet: use http://"),
            other => bail!("{other}:// is not a relay tracewing can use: http:// or socks5://"),
        };
        let rest = rest.split('/').next().unwrap_or_default();
        let (user, address) = match rest.rsplit_once('@') {
            Some((user, address)) => {
                let (name, password) = user.split_once(':').unwrap_or((user, ""));
                (Some((decode(name), decode(password))), address)
            }
            None => (None, rest),
        };
        let (host, port) =
            split_port(address).with_context(|| format!("\"{address}\" is not a host and port"))?;
        if host.is_empty() {
            bail!("\"{url}\" has no host");
        }
        if kind == Kind::Socks5
            && user
                .as_ref()
                .is_some_and(|(n, p)| n.len() > 255 || p.len() > 255)
        {
            bail!("SOCKS5 takes a user name and password of at most 255 bytes each");
        }
        let bypass = settings
            .bypass
            .split(|c: char| c == ',' || c.is_whitespace())
            .map(|h| h.trim().trim_start_matches("*.").trim_start_matches('.'))
            .filter(|h| !h.is_empty())
            .map(|h| {
                h.trim_start_matches('[')
                    .trim_end_matches(']')
                    .to_ascii_lowercase()
            })
            .collect();
        Ok(Some(Self {
            kind,
            host: host.to_owned(),
            port: port.unwrap_or(default_port),
            user,
            bypass,
        }))
    }

    /// Whether `host`, with no port, is reached through the relay.
    pub fn carries(&self, host: &str) -> bool {
        let host = host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_ascii_lowercase();
        !self.bypass.iter().any(|entry| {
            entry == "*"
                || host == *entry
                || (host.len() > entry.len()
                    && host.ends_with(entry.as_str())
                    && host.as_bytes()[host.len() - entry.len() - 1] == b'.')
        })
    }

    /// `Proxy-Authorization`'s value, if a user is given.
    pub fn basic(&self) -> Option<String> {
        self.user.as_ref().map(|(name, password)| {
            format!("Basic {}", STANDARD.encode(format!("{name}:{password}")))
        })
    }

    pub fn address(&self) -> String {
        if self.host.contains(':') {
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }

    /// A connection to the relay itself.
    pub async fn connect(&self) -> Result<TcpStream, String> {
        let tcp = TcpStream::connect((self.host.as_str(), self.port))
            .await
            .map_err(|e| format!("connect to the relay {}: {e}", self.address()))?;
        let _ = tcp.set_nodelay(true);
        Ok(tcp)
    }

    /// A tunnel through the relay to `host:port`, over which anything can go.
    pub async fn tunnel(&self, host: &str, port: u16) -> Result<TcpStream, String> {
        let mut tcp = self.connect().await?;
        let made = tokio::time::timeout(ANSWER_WAIT, async {
            match self.kind {
                Kind::Http => self.http_connect(&mut tcp, host, port).await,
                Kind::Socks5 => self.socks5_connect(&mut tcp, host, port).await,
            }
        })
        .await;
        match made {
            Ok(Ok(())) => Ok(tcp),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(format!(
                "the relay {} did not answer within {} s",
                self.address(),
                ANSWER_WAIT.as_secs()
            )),
        }
    }

    async fn http_connect(&self, tcp: &mut TcpStream, host: &str, port: u16) -> Result<(), String> {
        let target = if host.contains(':') {
            format!("[{host}]:{port}")
        } else {
            format!("{host}:{port}")
        };
        let mut ask = format!("CONNECT {target} HTTP/1.1\r\nHost: {target}\r\n");
        if let Some(basic) = self.basic() {
            ask.push_str(&format!("Proxy-Authorization: {basic}\r\n"));
        }
        ask.push_str("\r\n");
        let io = |e: std::io::Error| format!("the relay {}: {e}", self.address());
        tcp.write_all(ask.as_bytes()).await.map_err(io)?;
        // Byte by byte, so that nothing of what comes after is taken.
        let mut answer = Vec::new();
        while !answer.ends_with(b"\r\n\r\n") {
            if answer.len() >= ANSWER_MAX {
                return Err(format!("the relay {} answered too much", self.address()));
            }
            let mut byte = [0];
            if tcp.read(&mut byte).await.map_err(io)? == 0 {
                return Err(format!(
                    "the relay {} closed the connection",
                    self.address()
                ));
            }
            answer.push(byte[0]);
        }
        let answer = String::from_utf8_lossy(&answer);
        let line = answer.lines().next().unwrap_or_default();
        let status: u16 = line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        match status {
            200..=299 => Ok(()),
            407 if self.user.is_none() => Err(format!(
                "the relay {} asks for a user name and password",
                self.address()
            )),
            407 => Err(format!(
                "the relay {} refused the user name and password",
                self.address()
            )),
            _ => Err(format!(
                "the relay {} would not reach {target}: {line}",
                self.address()
            )),
        }
    }

    async fn socks5_connect(
        &self,
        tcp: &mut TcpStream,
        host: &str,
        port: u16,
    ) -> Result<(), String> {
        let io = |e: std::io::Error| format!("the SOCKS5 relay {}: {e}", self.address());
        let offer: &[u8] = if self.user.is_some() {
            &[5, 2, 0, 2]
        } else {
            &[5, 1, 0]
        };
        tcp.write_all(offer).await.map_err(io)?;
        let mut chosen = [0u8; 2];
        tcp.read_exact(&mut chosen).await.map_err(io)?;
        match (chosen, &self.user) {
            ([5, 0], _) => {}
            ([5, 2], Some((name, password))) => {
                let mut auth = vec![1, name.len() as u8];
                auth.extend_from_slice(name.as_bytes());
                auth.push(password.len() as u8);
                auth.extend_from_slice(password.as_bytes());
                tcp.write_all(&auth).await.map_err(io)?;
                let mut said = [0u8; 2];
                tcp.read_exact(&mut said).await.map_err(io)?;
                if said[1] != 0 {
                    return Err(format!(
                        "the SOCKS5 relay {} refused the user name and password",
                        self.address()
                    ));
                }
            }
            ([5, 0xff], None) => {
                return Err(format!(
                    "the SOCKS5 relay {} asks for a user name and password",
                    self.address()
                ));
            }
            _ => {
                return Err(format!(
                    "the SOCKS5 relay {} offers no way to go on that tracewing knows",
                    self.address()
                ));
            }
        }
        let mut ask = vec![5, 1, 0];
        match host.parse::<std::net::IpAddr>() {
            Ok(std::net::IpAddr::V4(ip)) => {
                ask.push(1);
                ask.extend_from_slice(&ip.octets());
            }
            Ok(std::net::IpAddr::V6(ip)) => {
                ask.push(4);
                ask.extend_from_slice(&ip.octets());
            }
            Err(_) => {
                if host.len() > 255 {
                    return Err(format!("\"{host}\" is too long a name for SOCKS5"));
                }
                ask.push(3);
                ask.push(host.len() as u8);
                ask.extend_from_slice(host.as_bytes());
            }
        }
        ask.extend_from_slice(&port.to_be_bytes());
        tcp.write_all(&ask).await.map_err(io)?;
        let mut head = [0u8; 4];
        tcp.read_exact(&mut head).await.map_err(io)?;
        if head[1] != 0 {
            let why = match head[1] {
                1 => "it failed",
                2 => "its rules do not allow it",
                3 => "the network cannot be reached",
                4 => "the host cannot be reached",
                5 => "the host refused the connection",
                6 => "it took too long",
                _ => "it cannot",
            };
            return Err(format!(
                "the SOCKS5 relay {} would not reach {host}:{port}: {why}",
                self.address()
            ));
        }
        // The address it bound, which is of no use here, and its port.
        let rest = match head[3] {
            1 => 4 + 2,
            4 => 16 + 2,
            3 => {
                let mut len = [0u8; 1];
                tcp.read_exact(&mut len).await.map_err(io)?;
                len[0] as usize + 2
            }
            _ => {
                return Err(format!(
                    "the SOCKS5 relay {} answered oddly",
                    self.address()
                ));
            }
        };
        let mut skip = vec![0u8; rest];
        tcp.read_exact(&mut skip).await.map_err(io)?;
        Ok(())
    }
}

/// The relay in use, changed while the proxy runs.
#[derive(Debug, Default)]
pub struct Relay {
    route: RwLock<Option<Arc<Route>>>,
}

impl Relay {
    pub fn new(settings: &RelaySettings) -> Result<Self> {
        let relay = Self::default();
        relay.set(settings)?;
        Ok(relay)
    }

    /// Applies to the next connection. One that cannot be used changes nothing.
    pub fn set(&self, settings: &RelaySettings) -> Result<()> {
        let route = Route::parse(settings)?.map(Arc::new);
        *self.route.write().unwrap_or_else(|e| e.into_inner()) = route;
        Ok(())
    }

    /// The relay to reach `host` through, if any.
    pub fn route_for(&self, host: &str) -> Option<Arc<Route>> {
        self.route
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .filter(|route| route.carries(host))
            .cloned()
    }
}

/// `host:port`, `host`, `[v6]:port` or `[v6]`, the port if given.
fn split_port(address: &str) -> Option<(&str, Option<u16>)> {
    if let Some(rest) = address.strip_prefix('[') {
        let (host, after) = rest.split_once(']')?;
        return match after.strip_prefix(':') {
            Some(port) => Some((host, Some(port.parse().ok()?))),
            None if after.is_empty() => Some((host, None)),
            None => None,
        };
    }
    match address.rsplit_once(':') {
        Some((host, port)) => Some((host, Some(port.parse().ok()?))),
        None => Some((address, None)),
    }
}

/// `%40` as `@`, and so on: a password written in a URL.
fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(byte) = text
                .get(i + 1..i + 3)
                .and_then(|hex| u8::from_str_radix(hex, 16).ok())
        {
            out.push(byte);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(proxy: &str, bypass: &str) -> Route {
        Route::parse(&RelaySettings {
            proxy: proxy.to_owned(),
            bypass: bypass.to_owned(),
        })
        .unwrap()
        .unwrap()
    }

    #[test]
    fn relays_are_read_from_their_url() {
        let http = route("http://proxy.corp:3128", "");
        assert_eq!(
            (http.kind, http.host.as_str(), http.port),
            (Kind::Http, "proxy.corp", 3128)
        );
        assert_eq!(http.user, None);
        assert_eq!(http.basic(), None);
        let with_user = route("http://an%40corp:p%3Ass@10.0.0.1/", "");
        assert_eq!(
            with_user.user,
            Some(("an@corp".to_owned(), "p:ss".to_owned()))
        );
        assert_eq!(with_user.port, 8080, "http's usual");
        assert_eq!(with_user.basic().unwrap(), "Basic YW5AY29ycDpwOnNz");
        let socks = route("SOCKS5H://[::1]:9050", "");
        assert_eq!(
            (socks.kind, socks.host.as_str(), socks.port),
            (Kind::Socks5, "::1", 9050)
        );
        assert_eq!(socks.address(), "[::1]:9050");
        assert_eq!(route("socks5://s", "").port, 1080);

        let none = RelaySettings::default();
        assert_eq!(Route::parse(&none).unwrap(), None);
        for bad in [
            "proxy.corp:8080",
            "ftp://x",
            "https://p:8443",
            "http://:80",
            "http://p:port",
        ] {
            let settings = RelaySettings {
                proxy: bad.to_owned(),
                ..RelaySettings::default()
            };
            assert!(Route::parse(&settings).is_err(), "{bad}");
        }
    }

    #[test]
    fn hosts_are_passed_by_as_no_proxy_says() {
        let r = route(
            "http://p",
            "localhost, 127.0.0.1 ::1\n*.corp.local, .intra, example.com",
        );
        for straight in [
            "localhost",
            "127.0.0.1",
            "[::1]",
            "build.corp.local",
            "corp.local",
            "a.b.intra",
            "example.com",
            "API.Example.com",
        ] {
            assert!(!r.carries(straight), "{straight} goes straight");
        }
        for through in ["example.org", "notexample.com", "intranet", "10.0.0.2"] {
            assert!(r.carries(through), "{through} goes through");
        }
        assert!(!route("http://p", "*").carries("anything"));
        assert!(
            route("http://p", "").carries("localhost"),
            "nothing passed by"
        );

        let relay = Relay::new(&RelaySettings {
            proxy: "http://p:1".to_owned(),
            bypass: "localhost".to_owned(),
        })
        .unwrap();
        assert!(relay.route_for("example.com").is_some());
        assert!(relay.route_for("localhost").is_none());
        relay.set(&RelaySettings::default()).unwrap();
        assert!(relay.route_for("example.com").is_none(), "turned off");
        let bad = RelaySettings {
            proxy: "nope".to_owned(),
            ..RelaySettings::default()
        };
        assert!(relay.set(&bad).is_err());
    }

    #[test]
    fn settings_are_kept_in_a_file() {
        let dir = std::env::temp_dir().join(format!("tracewing-relay-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(RelaySettings::load(&dir).unwrap(), RelaySettings::default());
        let settings = RelaySettings {
            proxy: "socks5://u:p@proxy:1080".to_owned(),
            bypass: "localhost, *.corp".to_owned(),
        };
        settings.save(&dir).unwrap();
        assert_eq!(RelaySettings::load(&dir).unwrap(), settings);
        let bad = RelaySettings {
            proxy: "gopher://x".to_owned(),
            ..RelaySettings::default()
        };
        assert!(bad.save(&dir).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
