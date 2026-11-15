//! How a flow taken outside tracewing is handed to it.
//!
//! macOS's network extension (`crates/netext`) takes a flow from an app that
//! was never told a proxy exists, connects to a port tracewing keeps for it,
//! and says a few lines before the flow's own bytes: where the flow was going
//! and which app made it. tracewing answers, and from then on the connection
//! is the app's conversation, as if the app had made it to tracewing.
//!
//! ```text
//! TRACEWING-HANDOFF 1
//! to: example.com:443
//! app: com.apple.curl
//! pid: 4242
//! nonce: 9f2c…(64 hex)
//! proof: 51ab…(64 hex)
//!
//! ```
//!
//! answered by `OK <proof>` or `NO <why>`, each one line.
//!
//! **Both ends prove they hold one key**, which tracewing made and gave the
//! extension in its configuration, and which neither ever sends. The
//! extension's `proof` is an HMAC of everything it said before it, so the
//! lines are the extension's and nobody else's: a program on this computer
//! that is not the extension cannot hand tracewing a flow under another app's
//! name. tracewing's answer is an HMAC of the nonce, and that one matters
//! more: while tracewing is not running any program may listen on its port,
//! and an extension that took the answer on trust would hand that program the
//! traffic of every app it was told to take. Each `\r\n` is part of what is
//! signed, so the lines are compared as written.

use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::fmt;

/// The first line, and the version of what follows it.
pub const HELLO: &str = "TRACEWING-HANDOFF 1";
/// The most the lines before the flow may be. A name is at most 253 bytes, so
/// anything near this is not the extension.
pub const HEAD_MAX: usize = 2048;
/// The most the answer line may be.
pub const ANSWER_MAX: usize = 256;

const PROOF_LEN: usize = 32;

/// The key both ends hold.
#[derive(Clone, PartialEq, Eq)]
pub struct Key([u8; 32]);

impl Key {
    /// A new one, from the operating system's randomness.
    pub fn new() -> Result<Self, getrandom::Error> {
        let mut key = [0u8; 32];
        getrandom::fill(&mut key)?;
        Ok(Self(key))
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The key as 64 hex digits, as it is kept and given to the extension.
    pub fn to_hex(&self) -> String {
        hex(&self.0)
    }

    pub fn from_hex(text: &str) -> Option<Self> {
        Some(Self(unhex(text.trim())?.try_into().ok()?))
    }

    fn mac(&self) -> Hmac<Sha256> {
        Hmac::<Sha256>::new_from_slice(&self.0).expect("HMAC takes a key of any length")
    }
}

impl fmt::Debug for Key {
    /// Never the key itself, in a log or a panic.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Key(…)")
    }
}

/// A flow, as the extension describes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hello {
    /// Where it was going: the name the app asked for, or the address it
    /// connected to (an IPv6 address without brackets).
    pub host: String,
    pub port: u16,
    /// The app's signing id.
    pub app: String,
    /// The app's process, where its audit token said.
    pub pid: Option<u32>,
}

/// A nonce: what tracewing's answer is made from, new for every flow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Nonce([u8; 32]);

impl Nonce {
    pub fn new() -> Result<Self, getrandom::Error> {
        let mut nonce = [0u8; 32];
        getrandom::fill(&mut nonce)?;
        Ok(Self(nonce))
    }
}

/// Why a hello was turned down. Said back to the extension, which logs it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    /// Not the lines of this protocol, or not of this version.
    NotAHello,
    /// A line that should be there is not, or is there twice.
    Missing(&'static str),
    /// A value that is not what it should be.
    Bad(&'static str),
    /// The proof does not match: the key is not tracewing's.
    WrongKey,
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAHello => f.write_str("not a hand-off"),
            Self::Missing(line) => write!(f, "no single {line} line"),
            Self::Bad(line) => write!(f, "the {line} line is not one"),
            Self::WrongKey => f.write_str("the proof is not tracewing's key"),
        }
    }
}

impl std::error::Error for Refused {}

impl Hello {
    /// The lines the extension says, ending in the blank line.
    pub fn write(&self, key: &Key, nonce: Nonce) -> Vec<u8> {
        let mut said = format!(
            "{HELLO}\r\nto: {}\r\napp: {}\r\n",
            authority(&self.host, self.port),
            clean_app(&self.app),
        );
        if let Some(pid) = self.pid {
            said.push_str(&format!("pid: {pid}\r\n"));
        }
        said.push_str(&format!("nonce: {}\r\n", hex(&nonce.0)));
        let mut mac = key.mac();
        mac.update(said.as_bytes());
        said.push_str(&format!(
            "proof: {}\r\n\r\n",
            hex(&mac.finalize().into_bytes())
        ));
        said.into_bytes()
    }

    /// What the extension said, up to and with the blank line, if it is a
    /// hello signed with `key`.
    pub fn read(head: &[u8], key: &Key) -> Result<(Hello, Nonce), Refused> {
        let head = std::str::from_utf8(head).map_err(|_| Refused::NotAHello)?;
        let body = head.strip_suffix("\r\n\r\n").ok_or(Refused::NotAHello)?;
        let proof_at = body.rfind("\r\nproof: ").ok_or(Refused::Missing("proof"))? + 2;
        let (signed, proof) = (&head[..proof_at], &body[proof_at + "proof: ".len()..]);
        let proof = unhex(proof)
            .filter(|p| p.len() == PROOF_LEN)
            .ok_or(Refused::Bad("proof"))?;
        let mut mac = key.mac();
        mac.update(signed.as_bytes());
        mac.verify_slice(&proof).map_err(|_| Refused::WrongKey)?;

        let mut lines = signed.strip_suffix("\r\n").unwrap_or(signed).split("\r\n");
        if lines.next() != Some(HELLO) {
            return Err(Refused::NotAHello);
        }
        let (mut to, mut app, mut pid, mut nonce) = (None, None, None, None);
        for line in lines {
            let (name, value) = line.split_once(": ").ok_or(Refused::NotAHello)?;
            let (slot, name) = match name {
                "to" => (&mut to, "to"),
                "app" => (&mut app, "app"),
                "pid" => (&mut pid, "pid"),
                "nonce" => (&mut nonce, "nonce"),
                // A line of a later version is signed, and not this one's to read.
                _ => continue,
            };
            if slot.replace(value).is_some() {
                return Err(Refused::Missing(name));
            }
        }
        let (host, port) =
            split_authority(to.ok_or(Refused::Missing("to"))?).ok_or(Refused::Bad("to"))?;
        let app = app.ok_or(Refused::Missing("app"))?;
        if app.is_empty() || clean_app(app) != app {
            return Err(Refused::Bad("app"));
        }
        let pid = match pid {
            None => None,
            Some(pid) => Some(pid.parse().map_err(|_| Refused::Bad("pid"))?),
        };
        let nonce = unhex(nonce.ok_or(Refused::Missing("nonce"))?)
            .and_then(|n| <[u8; 32]>::try_from(n).ok())
            .ok_or(Refused::Bad("nonce"))?;
        Ok((
            Hello {
                host: host.to_owned(),
                port,
                app: app.to_owned(),
                pid,
            },
            Nonce(nonce),
        ))
    }
}

fn answer_proof(key: &Key, nonce: Nonce) -> Hmac<Sha256> {
    let mut mac = key.mac();
    mac.update(b"tracewing answers ");
    mac.update(&nonce.0);
    mac
}

/// tracewing's answer to a hello it takes: the flow's bytes follow it.
pub fn answer(key: &Key, nonce: Nonce) -> Vec<u8> {
    format!(
        "OK {}\r\n",
        hex(&answer_proof(key, nonce).finalize().into_bytes())
    )
    .into_bytes()
}

/// tracewing's answer to a hello it turns down; the connection closes after.
pub fn refusal(why: &Refused) -> Vec<u8> {
    format!("NO {why}\r\n").into_bytes()
}

/// Whether the answer line, `\r\n` and all, is tracewing's, holding `key`,
/// to the hello that sent `nonce`.
pub fn answered(line: &[u8], key: &Key, nonce: Nonce) -> Result<(), String> {
    let line = std::str::from_utf8(line).map_err(|_| "an answer that is not text".to_owned())?;
    let line = line.strip_suffix("\r\n").ok_or("an answer with no end")?;
    if let Some(why) = line.strip_prefix("NO ") {
        return Err(format!("turned down: {why}"));
    }
    let proof = line
        .strip_prefix("OK ")
        .and_then(unhex)
        .filter(|p| p.len() == PROOF_LEN)
        .ok_or_else(|| "an answer that is not tracewing's".to_owned())?;
    answer_proof(key, nonce)
        .verify_slice(&proof)
        .map_err(|_| "an answer from something that does not hold tracewing's key".to_owned())
}

/// Whether `name` may be named in a hello: a DNS name or an address, and
/// nothing that could end a line early.
pub fn is_host(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 253
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._:%".contains(&b))
}

/// `host:port`, an IPv6 address in brackets.
pub fn authority(host: &str, port: u16) -> String {
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

fn split_authority(text: &str) -> Option<(&str, u16)> {
    let (host, port) = match text.strip_prefix('[') {
        Some(rest) => {
            let (host, port) = rest.split_once("]:")?;
            host.contains(':').then_some((host, port))?
        }
        None => {
            let (host, port) = text.rsplit_once(':')?;
            (!host.contains(':')).then_some((host, port))?
        }
    };
    let port: u16 = port.parse().ok().filter(|&p| p != 0)?;
    is_host(host).then_some((host, port))
}

/// A signing id as it goes in a line: a reverse-DNS name, so anything else
/// is left out rather than let into one.
fn clean_app(app: &str) -> String {
    app.chars()
        .filter(|c| c.is_ascii_alphanumeric() || "-._".contains(*c))
        .take(255)
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> Key {
        Key::from_bytes([7; 32])
    }

    fn hello() -> Hello {
        Hello {
            host: "example.com".into(),
            port: 443,
            app: "com.apple.curl".into(),
            pid: Some(4242),
        }
    }

    #[test]
    fn a_hello_is_read_as_it_was_written() {
        let nonce = Nonce::new().unwrap();
        let said = hello().write(&key(), nonce);
        let text = String::from_utf8(said.clone()).unwrap();
        assert!(text.starts_with("TRACEWING-HANDOFF 1\r\nto: example.com:443\r\napp: com.apple.curl\r\npid: 4242\r\nnonce: "));
        assert!(text.ends_with("\r\n\r\n"));
        assert!(said.len() < HEAD_MAX);
        assert_eq!(Hello::read(&said, &key()), Ok((hello(), nonce)));
    }

    #[test]
    fn an_address_and_no_pid_are_read_too() {
        let v6 = Hello {
            host: "2606:4700::6810:85e5".into(),
            port: 80,
            app: "a.out".into(),
            pid: None,
        };
        let nonce = Nonce::new().unwrap();
        let said = v6.write(&key(), nonce);
        assert!(String::from_utf8_lossy(&said).contains("to: [2606:4700::6810:85e5]:80\r\n"));
        assert_eq!(Hello::read(&said, &key()), Ok((v6, nonce)));
    }

    #[test]
    fn another_key_is_turned_down() {
        let said = hello().write(&Key::from_bytes([8; 32]), Nonce::new().unwrap());
        assert_eq!(Hello::read(&said, &key()), Err(Refused::WrongKey));
    }

    #[test]
    fn a_line_changed_on_the_way_is_turned_down() {
        let said = hello().write(&key(), Nonce::new().unwrap());
        let changed = String::from_utf8(said)
            .unwrap()
            .replace("app: com.apple.curl", "app: com.apple.Safar");
        assert_eq!(
            Hello::read(changed.as_bytes(), &key()),
            Err(Refused::WrongKey)
        );
    }

    #[test]
    fn what_is_not_a_hello_is_turned_down() {
        assert_eq!(
            Hello::read(b"GET / HTTP/1.1\r\n\r\n", &key()),
            Err(Refused::Missing("proof"))
        );
        assert_eq!(
            Hello::read(b"\x16\x03\x01", &key()),
            Err(Refused::NotAHello)
        );
        assert_eq!(Hello::read(b"", &key()), Err(Refused::NotAHello));
    }

    #[test]
    fn values_are_checked_even_when_signed() {
        // Signed with the right key, as only the extension can, and still read
        // with care: a host that could end the line, a port of nothing.
        let sign = |lines: &str| {
            let signed = format!("{HELLO}\r\n{lines}nonce: {}\r\n", "ab".repeat(32));
            let mut mac = key().mac();
            mac.update(signed.as_bytes());
            format!(
                "{signed}proof: {}\r\n\r\n",
                hex(&mac.finalize().into_bytes())
            )
            .into_bytes()
        };
        let read = |lines: &str| Hello::read(&sign(lines), &key()).map(|(h, _)| h);
        assert!(read("to: example.com:443\r\napp: com.apple.curl\r\n").is_ok());
        assert_eq!(
            read("to: example.com:0\r\napp: x\r\n"),
            Err(Refused::Bad("to"))
        );
        assert_eq!(
            read("to: exa mple.com:443\r\napp: x\r\n"),
            Err(Refused::Bad("to"))
        );
        assert_eq!(read("to: ::1:443\r\napp: x\r\n"), Err(Refused::Bad("to")));
        assert_eq!(
            read("to: example.com:443\r\napp: a b\r\n"),
            Err(Refused::Bad("app"))
        );
        assert_eq!(
            read("to: example.com:443\r\napp: x\r\npid: me\r\n"),
            Err(Refused::Bad("pid"))
        );
        assert_eq!(read("app: x\r\n"), Err(Refused::Missing("to")));
        assert_eq!(
            read("to: a.com:1\r\nto: b.com:1\r\napp: x\r\n"),
            Err(Refused::Missing("to")),
            "said twice"
        );
        assert_eq!(
            read("to: a.com:1\r\napp: x\r\nlater: what a later version says\r\n").map(|h| h.host),
            Ok("a.com".to_owned())
        );
    }

    #[test]
    fn only_tracewings_answer_is_taken() {
        let nonce = Nonce::new().unwrap();
        assert_eq!(answered(&answer(&key(), nonce), &key(), nonce), Ok(()));
        // Another key, another hello, or something else listening.
        assert!(answered(&answer(&Key::from_bytes([8; 32]), nonce), &key(), nonce).is_err());
        assert!(answered(&answer(&key(), Nonce::new().unwrap()), &key(), nonce).is_err());
        assert!(answered(b"HTTP/1.1 400 Bad Request\r\n", &key(), nonce).is_err());
        assert!(answered(b"OK \r\n", &key(), nonce).is_err());
        let no = answered(&refusal(&Refused::WrongKey), &key(), nonce).unwrap_err();
        assert!(no.contains("not tracewing's key"), "{no}");
    }

    #[test]
    fn a_key_is_kept_as_hex_and_never_shown() {
        let key = Key::new().unwrap();
        assert_eq!(key.to_hex().len(), 64);
        assert_eq!(Key::from_hex(&key.to_hex()), Some(key.clone()));
        assert_eq!(Key::from_hex("abc"), None);
        assert_eq!(format!("{key:?}"), "Key(…)");
    }
}
