//! The name in a TLS `ClientHello`.
//!
//! A source that deals in packets hands a flow over as an address, because an
//! address is all a packet carries. The name the client was really going to
//! is in the first record it writes, and this reads it out of there: a flow
//! that would be called `140.82.121.4:443` is called `github.com:443`
//! instead, and the server is asked by the name it answers to rather than by
//! an address several names share.
//!
//! It parses rather than searches. A `ClientHello` is a length-prefixed
//! structure all the way down ([RFC 8446 §4.1.2]), so every field here is
//! reached by walking those lengths; nothing looks for a name-shaped run of
//! bytes anywhere in the record. A truncated record is read as far as it
//! goes, because the extension that carries the name usually comes early and
//! the alternative is no name at all — but a length that points past what
//! arrived ends the walk rather than being clamped to fit, since a structure
//! that disagrees with itself is not one this understood.
//!
//! **The name is checked before it is returned, and that check is not
//! cosmetic.** What comes back goes into a `CONNECT` line that
//! [`handover`](super::handover) builds by formatting — `CONNECT {authority}
//! HTTP/1.1\r\nHost: {authority}\r\n\r\n` — so a name holding a carriage
//! return would end that request early and let whatever followed be read as
//! a request of its own, by a proxy that trusts it because it came from
//! inside tracewing. The name also becomes a URI authority, and is looked up
//! in DNS. So only a plain DNS name is accepted: ASCII letters, digits, `-`
//! and `_`, in labels of 1 to 63 characters, 253 characters in all. Anything
//! else — a space, a colon, a slash, a newline, an empty label, a trailing
//! dot, a name that is really an address — is refused, and the flow keeps the
//! address it already had, which is no worse than before this module existed.
//!
//! [RFC 8446 §4.1.2]: https://www.rfc-editor.org/rfc/rfc8446#section-4.1.2

use std::net::IpAddr;

/// A TLS record's header: the type, two bytes of version, and the length of
/// what follows.
pub(crate) const RECORD_HEAD: usize = 5;

/// What a record of a handshake starts with. `0x16` is a handshake record and
/// `0x03` is the major version of SSL 3.x and of every TLS: a TLS 1.3 client
/// still writes `0x03 0x01` here, because the real version moved into an
/// extension.
pub(crate) const TLS_START: &[u8] = b"\x16\x03";

/// The most of the first record that is read looking for the name. A record
/// may be 16 KiB, but a `ClientHello` is a couple of KiB even with a
/// post-quantum key share in it, and this is read into a buffer kept for as
/// long as the connection: the limit is on what tracewing holds, not on what
/// the client may send. Past it the record is read no further and the flow
/// keeps its address.
pub(crate) const HELLO_MAX: usize = 8 * 1024;

/// The longest a DNS name may be.
const NAME_MAX: usize = 253;

/// The longest one label of it may be.
const LABEL_MAX: usize = 63;

/// A handshake message's type, where `1` is a `ClientHello`.
const CLIENT_HELLO: u8 = 1;

/// The `server_name` extension, [RFC 6066 §3].
///
/// [RFC 6066 §3]: https://www.rfc-editor.org/rfc/rfc6066#section-3
const SERVER_NAME: u16 = 0;

/// The one name type that extension has ever had, `host_name`.
const HOST_NAME: u8 = 0;

/// How long a record of `len` bytes is in all, held to [`HELLO_MAX`]. What
/// the caller reads: the header plus as much of the body as is worth having.
pub(crate) fn record_len(first: &[u8]) -> Option<usize> {
    let len = u16::from_be_bytes([*first.get(3)?, *first.get(4)?]) as usize;
    Some(RECORD_HEAD + len.min(HELLO_MAX))
}

/// The `server_name` a `ClientHello` carries, where the record holds one and
/// the name is a name. `record` starts at the record header, and may be the
/// start of a longer read: anything after the first record is ignored.
pub(crate) fn in_client_hello(record: &[u8]) -> Option<String> {
    let mut rest = Bytes(record);
    if rest.u8()? != TLS_START[0] {
        return None;
    }
    rest.skip(2)?; // The version, which says nothing here.
    let len = rest.u16()? as usize;
    // As far as the record goes: a read cut short by `HELLO_MAX` or by a
    // client still writing can hold the whole extension anyway.
    let mut body = Bytes(rest.upto(len));

    if body.u8()? != CLIENT_HELLO {
        return None;
    }
    let len = body.u24()? as usize;
    let mut hello = Bytes(body.upto(len));

    hello.skip(2)?; // legacy_version
    hello.skip(32)?; // random
    hello.skip_u8_vec()?; // legacy_session_id
    hello.skip_u16_vec()?; // cipher_suites
    hello.skip_u8_vec()?; // legacy_compression_methods

    let len = hello.u16()? as usize;
    let mut exts = Bytes(hello.upto(len));
    while let Some(kind) = exts.peek_u16() {
        exts.skip(2)?;
        let len = exts.u16()? as usize;
        let data = exts.take(len)?;
        if kind == SERVER_NAME {
            return first_host_name(data);
        }
    }
    None
}

/// The first `host_name` in a `server_name` extension's list. The list has
/// held exactly one entry since the extension was written, but it is a list,
/// so this walks it rather than assuming.
fn first_host_name(ext: &[u8]) -> Option<String> {
    let mut rest = Bytes(ext);
    let len = rest.u16()? as usize;
    let mut list = Bytes(rest.upto(len));
    while let Some(kind) = list.u8() {
        let len = list.u16()? as usize;
        let name = list.take(len)?;
        if kind == HOST_NAME {
            let name = std::str::from_utf8(name).ok()?;
            return dns_name(name);
        }
    }
    None
}

/// `name` lowercased, if it is a DNS name at all. Lowercased because a
/// hostname is case-insensitive while the string it becomes is not: it ends
/// up in a URI, in a `Host` header and in what the user reads, and a client
/// that writes `GitHub.com` means the same host as one that writes
/// `github.com`.
fn dns_name(name: &str) -> Option<String> {
    if name.is_empty() || name.len() > NAME_MAX {
        return None;
    }
    // An address is not a name, and RFC 6066 says a client must not put one
    // here. One that arrived anyway would be looked up in DNS as if it were a
    // name, which fails, so the flow is better off keeping the address it was
    // handed over with.
    if name.parse::<IpAddr>().is_ok() {
        return None;
    }
    // A trailing dot is a legal way to write an absolute name, and is refused
    // rather than trimmed: it would make an empty last label here, and the
    // name is about to be formatted into places that have no reading for it.
    let ok = name.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= LABEL_MAX
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    });
    ok.then(|| name.to_ascii_lowercase())
}

/// A walk through bytes that runs out rather than panicking. Every length in
/// a `ClientHello` comes from the client, so every one of them is checked
/// against what is really there; `None` from any step ends the walk.
struct Bytes<'a>(&'a [u8]);

impl<'a> Bytes<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        if self.0.len() < n {
            return None;
        }
        let (took, rest) = self.0.split_at(n);
        self.0 = rest;
        Some(took)
    }

    /// Up to `n` bytes: what a length says is there, or the rest of a record
    /// that was cut short. Used where reading less is better than reading
    /// nothing, and never where a length is being trusted.
    fn upto(&mut self, n: usize) -> &'a [u8] {
        let n = n.min(self.0.len());
        let (took, rest) = self.0.split_at(n);
        self.0 = rest;
        took
    }

    fn skip(&mut self, n: usize) -> Option<()> {
        self.take(n).map(|_| ())
    }

    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }

    fn u16(&mut self) -> Option<u16> {
        let two = self.take(2)?;
        Some(u16::from_be_bytes([two[0], two[1]]))
    }

    fn u24(&mut self) -> Option<u32> {
        let three = self.take(3)?;
        Some(u32::from_be_bytes([0, three[0], three[1], three[2]]))
    }

    fn peek_u16(&self) -> Option<u16> {
        Some(u16::from_be_bytes([*self.0.first()?, *self.0.get(1)?]))
    }

    /// A vector whose length is one byte, skipped whole.
    fn skip_u8_vec(&mut self) -> Option<()> {
        let len = self.u8()? as usize;
        self.skip(len)
    }

    /// A vector whose length is two bytes, skipped whole.
    fn skip_u16_vec(&mut self) -> Option<()> {
        let len = self.u16()? as usize;
        self.skip(len)
    }
}

/// A `ClientHello` built by hand, so a test can leave a piece out or put a
/// wrong length in. The test this module rests on does not use it: a hello
/// written by rustls is the check that the shape read here is not the same
/// mistake twice. [`handover`](super::handover) uses it to drive a whole
/// handover, which is where the name has to arrive to be worth anything.
#[cfg(test)]
pub(crate) fn hello(exts: &[(u16, Vec<u8>)]) -> Vec<u8> {
    let mut body = vec![CLIENT_HELLO, 0, 0, 0]; // Type, length filled in below.
    body.extend_from_slice(&[0x03, 0x03]); // legacy_version
    body.extend_from_slice(&[0x11; 32]); // random
    body.push(0); // legacy_session_id, empty
    body.extend_from_slice(&[0, 2, 0x13, 0x01]); // one cipher suite
    body.extend_from_slice(&[1, 0]); // one compression method, null

    let mut written = Vec::new();
    for (kind, data) in exts {
        written.extend_from_slice(&kind.to_be_bytes());
        written.extend_from_slice(&(data.len() as u16).to_be_bytes());
        written.extend_from_slice(data);
    }
    body.extend_from_slice(&(written.len() as u16).to_be_bytes());
    body.extend_from_slice(&written);

    let len = (body.len() - 4) as u32;
    body[1..4].copy_from_slice(&len.to_be_bytes()[1..]);

    let mut record = vec![TLS_START[0], 0x03, 0x01];
    record.extend_from_slice(&(body.len() as u16).to_be_bytes());
    record.extend_from_slice(&body);
    record
}

/// A `server_name` extension naming `host`.
#[cfg(test)]
pub(crate) fn sni(host: &str) -> (u16, Vec<u8>) {
    let mut entry = vec![HOST_NAME];
    entry.extend_from_slice(&(host.len() as u16).to_be_bytes());
    entry.extend_from_slice(host.as_bytes());
    let mut data = (entry.len() as u16).to_be_bytes().to_vec();
    data.extend_from_slice(&entry);
    (SERVER_NAME, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The test this module rests on: the bytes are written by rustls, which
    /// knows nothing of the parser, so a shape I guessed wrong here cannot
    /// agree with itself and pass.
    #[tokio::test]
    async fn the_name_a_real_client_hello_carries_is_the_one_read() {
        use tokio::io::AsyncReadExt;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let at = listener.local_addr().unwrap();

        // A client that will never be answered: all this needs is its first
        // record, which it writes before anything comes back.
        tokio::spawn(async move {
            let config = tokio_rustls::rustls::ClientConfig::builder()
                .with_root_certificates(tokio_rustls::rustls::RootCertStore::empty())
                .with_no_client_auth();
            let name = tokio_rustls::rustls::pki_types::ServerName::try_from("Example.COM")
                .expect("a name rustls accepts");
            let stream = tokio::net::TcpStream::connect(at).await.unwrap();
            let _ = tokio_rustls::TlsConnector::from(std::sync::Arc::new(config))
                .connect(name, stream)
                .await;
        });

        let (mut stream, _) = listener.accept().await.unwrap();
        let mut first = vec![0u8; HELLO_MAX];
        let read = stream.read(&mut first).await.unwrap();
        first.truncate(read);

        assert!(first.starts_with(TLS_START), "a handshake record");
        assert_eq!(
            in_client_hello(&first).as_deref(),
            Some("example.com"),
            "rustls wrote the name as `Example.COM`; it is read lowercased"
        );
        // Whatever rustls sent, a prefix of it must never panic: a client
        // half-way through writing is the common case, not a strange one.
        for upto in 0..first.len() {
            let _ = in_client_hello(&first[..upto]);
        }
    }

    #[test]
    fn a_hello_that_names_no_server_has_no_name() {
        assert_eq!(in_client_hello(&hello(&[])), None);
    }

    #[test]
    fn a_name_after_other_extensions_is_still_found() {
        let record = hello(&[
            (0x0a0a, vec![0; 2]), // GREASE, as Chrome writes
            (0x002b, vec![2, 0x03, 0x04]),
            sni("api.github.com"),
            (0x0033, vec![0; 8]),
        ]);
        assert_eq!(in_client_hello(&record).as_deref(), Some("api.github.com"));
    }

    #[test]
    fn a_name_is_read_from_a_record_cut_short_after_it() {
        let record = hello(&[sni("github.com"), (0x0033, vec![7; 1024])]);
        let upto = record
            .windows(10)
            .position(|w| w == b"github.com")
            .expect("the name is in there")
            + 10;
        assert_eq!(
            in_client_hello(&record[..upto]).as_deref(),
            Some("github.com"),
            "the name arrived whole, even though the record did not"
        );
    }

    #[test]
    fn a_record_cut_short_before_the_name_is_not_guessed_at() {
        let record = hello(&[(0x0033, vec![7; 256]), sni("github.com")]);
        let upto = record
            .windows(10)
            .position(|w| w == b"github.com")
            .expect("the name is in there");
        assert_eq!(in_client_hello(&record[..upto]), None);
    }

    /// The check that keeps a name out of the `CONNECT` line it would break.
    #[test]
    fn a_name_with_a_line_ending_in_it_is_refused() {
        let record = hello(&[sni("github.com\r\nX-Smuggled: yes")]);
        assert_eq!(
            in_client_hello(&record),
            None,
            "a name holding CRLF would end the CONNECT request early"
        );
    }

    #[test]
    fn a_name_that_is_really_an_address_is_refused() {
        for host in ["140.82.121.4", "::1", "2606:50c0:8000::153"] {
            assert_eq!(
                in_client_hello(&hello(&[sni(host)])),
                None,
                "{host} would be looked up in DNS as a name"
            );
        }
    }

    #[test]
    fn names_that_are_not_names_are_refused() {
        for host in [
            "",                              // nothing at all
            "github.com.",                   // a trailing dot's empty label
            "git..hub",                      // an empty label in the middle
            "git hub.com",                   // a space
            "github.com:443",                // a port, which is not part of a name
            "http://github.com",             // a URL
            "../etc/passwd",                 // a path
            &"a".repeat(LABEL_MAX + 1),      // one label too long
            &vec!["ab"; NAME_MAX].join("."), // a name too long
        ] {
            assert_eq!(in_client_hello(&hello(&[sni(host)])), None, "{host:?}");
        }
    }

    #[test]
    fn a_name_is_lowercased() {
        assert_eq!(
            in_client_hello(&hello(&[sni("GitHub.COM")])).as_deref(),
            Some("github.com")
        );
    }

    #[test]
    fn an_underscore_is_allowed_because_hosts_have_them() {
        assert_eq!(
            in_client_hello(&hello(&[sni("my_service.internal")])).as_deref(),
            Some("my_service.internal")
        );
    }

    /// A length that points past the record ends the walk. The name is in
    /// there, after an extension that claims to be longer than everything
    /// that arrived: reading it anyway would mean trusting a structure that
    /// disagrees with itself.
    #[test]
    fn an_extension_longer_than_the_record_ends_the_walk() {
        let mut record = hello(&[(0x0033, vec![0; 4]), sni("github.com")]);
        let at = record
            .windows(2)
            .position(|w| w == [0x00, 0x33])
            .expect("the extension is in there");
        record[at + 2..at + 4].copy_from_slice(&u16::MAX.to_be_bytes());
        assert_eq!(in_client_hello(&record), None);
    }

    #[test]
    fn what_is_not_a_handshake_record_is_not_read() {
        let mut data = hello(&[sni("github.com")]);
        data[0] = 0x17; // Application data, which never holds a hello.
        assert_eq!(in_client_hello(&data), None);

        let mut other = hello(&[sni("github.com")]);
        other[RECORD_HEAD] = 2; // A ServerHello, in the client's place.
        assert_eq!(in_client_hello(&other), None);

        assert_eq!(in_client_hello(b""), None);
        assert_eq!(in_client_hello(b"\x16\x03"), None);
        assert_eq!(in_client_hello(b"GET / HTTP/1.1\r\n"), None);
    }

    #[test]
    fn how_much_to_read_comes_from_the_record_and_is_capped() {
        let record = hello(&[sni("github.com")]);
        assert_eq!(record_len(&record), Some(record.len()));
        assert_eq!(record_len(b"\x16\x03\x01"), None, "no length yet");

        let mut huge = record.clone();
        huge[3..5].copy_from_slice(&(16 * 1024u16).to_be_bytes());
        assert_eq!(record_len(&huge), Some(RECORD_HEAD + HELLO_MAX));
    }
}
