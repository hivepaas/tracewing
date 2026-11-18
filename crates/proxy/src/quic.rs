//! What a QUIC Initial says out loud, without terminating anything.
//!
//! tracewing refuses UDP port 443 on purpose, so that a client falls back to
//! HTTP/2 over TCP and its requests can be read ([`crate::datagrams`]). That
//! is the right default and it is not the whole story: an app that insists on
//! HTTP/3, or speaks QUIC that is not HTTP at all, is **invisible** -- not
//! refused loudly, just absent from the list, which is the worst way for a
//! debugging tool to fail.
//!
//! This reads the one part of a QUIC connection that is not secret from
//! somebody on the path. A client's first flight is an **Initial** packet,
//! and Initial packets are protected with keys derived from the Destination
//! Connection ID the client itself put in the clear, under a salt published
//! in RFC 9001 §5.2. So anything that sees the packet can decrypt it. That is
//! not a weakness being exploited: it is how a router tells a QUIC packet
//! from a stray datagram, and the ClientHello inside carries the server name
//! and the ALPN exactly as a TLS one does.
//!
//! **This decrypts nothing of the conversation.** The keys here open the
//! Initial and nothing after it; everything from the server's first flight
//! onwards is under keys only the endpoints have. What comes out is what a
//! flow listing needs -- who was being reached, over what protocol -- and
//! nothing that was private.
//!
//! What it does not read, said plainly so that a `None` is not mistaken for
//! "no QUIC here":
//!
//! - **Encrypted ClientHello.** When a client uses ECH the real server name
//!   is inside the encrypted half and only the public one is visible.
//! - **A ClientHello split across two datagrams**, which a long one is. Only
//!   the CRYPTO bytes in this packet are reassembled; a hello that starts at
//!   a non-zero offset, or runs past the end, is not guessed at.
//! - **A version whose keys are not derived here.** Each version has its own
//!   salt and its own labels, and a draft version had its own again; the two
//!   published ones are in [`VERSIONS`] and no third is guessed at. Such a
//!   packet is still reported as QUIC with nothing read from it, because
//!   that a program is speaking QUIC at all is the part worth seeing.

use aes_gcm::aes::cipher::{Array, BlockCipherEncrypt, KeyInit};
use aes_gcm::{AeadInOut, Aes128Gcm};
use std::{
    collections::HashMap,
    net::SocketAddr,
    time::{Duration, Instant},
};

/// What a QUIC version needs to have its Initials read: the salt the keys
/// come from, the labels they are expanded with, and the packet type that
/// means Initial -- which version 2 renumbered, so it cannot be assumed.
struct Version {
    number: u32,
    salt: [u8; 20],
    key: &'static [u8],
    iv: &'static [u8],
    hp: &'static [u8],
    initial: u8,
}

/// Version 1, with the salt RFC 9001 §5.2 publishes. Every endpoint and every
/// observer uses the same one, which is what makes an Initial readable.
const V1: Version = Version {
    number: 0x0000_0001,
    salt: [
        0x38, 0x76, 0x2c, 0xf7, 0xf5, 0x59, 0x34, 0xb3, 0x4d, 0x17, 0x9a, 0xe6, 0xa4, 0xc8, 0x0c,
        0xad, 0xcc, 0xbb, 0x7f, 0x0a,
    ],
    key: b"quic key",
    iv: b"quic iv",
    hp: b"quic hp",
    initial: 0,
};

/// Version 2, from RFC 9369 §3.3: its own salt, `quicv2` in place of `quic`
/// in each label, and the packet types moved on by one so that a middlebox
/// that learnt version 1 by heart cannot read it.
const V2: Version = Version {
    number: 0x6b33_43cf,
    salt: [
        0x0d, 0xed, 0xe3, 0xde, 0xf7, 0x00, 0xa6, 0xdb, 0x81, 0x93, 0x81, 0xbe, 0x6e, 0x26, 0x9d,
        0xcb, 0xf9, 0xbd, 0x2e, 0xd9,
    ],
    key: b"quicv2 key",
    iv: b"quicv2 iv",
    hp: b"quicv2 hp",
    initial: 1,
};

/// The versions whose Initials this opens.
pub const VERSIONS: [u32; 2] = [V1.number, V2.number];

/// The version a number names, where it is one of [`VERSIONS`].
fn version(number: u32) -> Option<&'static Version> {
    [&V1, &V2].into_iter().find(|v| v.number == number)
}

/// A connection ID is at most this long, and a longer one is a packet this
/// does not understand rather than a buffer to grow.
const CID_MAX: usize = 20;

/// The sample a header's protection mask is made from, and the offset it
/// starts at: four bytes past the packet number, whatever length that is.
const SAMPLE: usize = 16;
const SAMPLE_AT: usize = 4;

/// The tag AES-GCM leaves on the end.
const TAG: usize = 16;

/// What a client said in the open, before anything of its own was secret.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Hello {
    /// The server name, where the client sent one and did not encrypt it.
    pub server_name: Option<String>,
    /// What it offered to speak: `h3` for HTTP/3, and whatever else.
    pub alpn: Vec<String>,
}

/// The hello in a client's first QUIC datagram, where there is one to read.
///
/// `None` for anything that is not an Initial of a version this can open,
/// including every reason listed on the module above. It is deliberately
/// quiet about which: a datagram on this port that is not a QUIC Initial is
/// the ordinary case, not a fault.
pub fn hello_in(datagram: &[u8]) -> Option<Hello> {
    let (version, dcid, pn_at, length) = initial_parts(datagram)?;
    let keys = Keys::of(dcid, version);
    let (first, number, pn_len) = unprotect(datagram, &keys, pn_at)?;

    // The header as it was before protection, which is what the payload was
    // sealed against: the first byte and the packet number, both now in the
    // clear, and everything between them untouched.
    let mut aad = datagram.get(..pn_at + pn_len)?.to_vec();
    aad[0] = first;
    aad[pn_at..].copy_from_slice(&number[..pn_len]);

    let end = pn_at + length;
    let mut body = datagram.get(pn_at + pn_len..end)?.to_vec();
    if body.len() < TAG {
        return None;
    }
    keys.open(&number[..pn_len], &aad, &mut body)?;
    hello_in_frames(&body)
}

/// A client opening a QUIC connection, and what it said in the open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Opening {
    /// Which version it spoke, as the number on the wire.
    pub version: u32,
    /// The hello, where there was one to read. `None` is either a version
    /// whose keys are not derived here or a hello this could not read, and
    /// the module above lists both; it never means the client sent none.
    pub hello: Option<Hello>,
}

/// Whether this datagram is a client starting a QUIC connection.
///
/// Wider than [`hello_in`] on purpose. The question a flow listing has to
/// answer is "is this program speaking QUIC", and that has an answer even
/// when the hello does not: a packet whose version this cannot open is still
/// a packet nothing else would have sent.
///
/// What keeps it from saying so about a datagram that is not QUIC at all is
/// the first byte. A long header has its top two bits both set, and the
/// other things that go to this port do not: DTLS records begin with a
/// content type of 20 to 25, and STUN with a zero bit and a one.
pub fn opening(datagram: &[u8]) -> Option<Opening> {
    let first = *datagram.first()?;
    if first & 0xc0 != 0xc0 {
        return None;
    }
    let number = u32::from_be_bytes(datagram.get(1..5)?.try_into().ok()?);
    // Version 0 is a Version Negotiation packet, which only a server sends.
    if number == 0 {
        return None;
    }
    match version(number) {
        // A version with a salt: the type bits say whether this is the first
        // packet of the connection, and the hello in it can be read.
        Some(v) => ((first >> 4) & 0x03 == v.initial).then(|| Opening {
            version: number,
            hello: hello_in(datagram),
        }),
        // One whose keys are not derived here. Its type bits mean whatever
        // that version says they do, so this does not claim the packet is an
        // Initial -- only that a client is speaking QUIC to this address.
        None => Some(Opening {
            version: number,
            hello: None,
        }),
    }
}

/// A client's attempt at QUIC, as one row of a flow listing needs it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attempt {
    /// The socket the program sent from, which is what names it in the
    /// operating system's tables.
    pub client: SocketAddr,
    /// Where it was going. A QUIC packet carries no destination of its own
    /// beyond the hello, so this is the address on the datagram.
    pub to: SocketAddr,
    pub version: u32,
    pub hello: Option<Hello>,
}

/// How long one client's packets to one server count as the same attempt.
///
/// A refused Initial is sent again several times before a client gives up on
/// QUIC and tries TCP, and every retry is the same attempt: without this a
/// listing fills with five rows saying one thing. A minute rather than a few
/// seconds because a client that keeps a socket open and keeps trying is
/// still telling us nothing new.
const SAME_FOR: Duration = Duration::from_secs(60);

/// How many attempts are remembered at once.
const REMEMBERED_MAX: usize = 1_024;

/// The attempts already told about, so that retries are one row.
///
/// A new connection comes from a new source port, so it is told about
/// however recently the last one was: what this folds together is one
/// socket trying again, which is the thing that is not news.
#[derive(Default)]
pub struct Attempts(HashMap<(SocketAddr, SocketAddr), Instant>);

impl Attempts {
    /// Whether this attempt is news, and remembers that it was told.
    pub fn fresh(&mut self, client: SocketAddr, to: SocketAddr, now: Instant) -> bool {
        if self.0.len() >= REMEMBERED_MAX {
            self.0
                .retain(|_, told| now.duration_since(*told) < SAME_FOR);
            if self.0.len() >= REMEMBERED_MAX {
                // A thousand sockets trying QUIC inside a minute is a flood,
                // not a session to keep notes on. Forgetting costs a
                // duplicate row at worst, and holding the notes would cost
                // memory nothing frees.
                self.0.clear();
            }
        }
        match self.0.get(&(client, to)) {
            Some(told) if now.duration_since(*told) < SAME_FOR => false,
            _ => {
                self.0.insert((client, to), now);
                true
            }
        }
    }
}

/// The parts of an Initial header that are never protected: which version it
/// is, the connection ID the keys come from, where the packet number starts
/// and how long the rest is.
fn initial_parts(packet: &[u8]) -> Option<(&'static Version, &[u8], usize, usize)> {
    let first = *packet.first()?;
    // Long header with the fixed bit set. A short header is a packet from a
    // connection already under way, whose keys nobody on the path has.
    if first & 0xc0 != 0xc0 {
        return None;
    }
    let version = version(u32::from_be_bytes(packet.get(1..5)?.try_into().ok()?))?;
    // The type that means Initial, which each version numbers for itself.
    if (first >> 4) & 0x03 != version.initial {
        return None;
    }
    let dcid_len = *packet.get(5)? as usize;
    if dcid_len > CID_MAX {
        return None;
    }
    let dcid = packet.get(6..6 + dcid_len)?;
    let mut at = 6 + dcid_len;
    let scid_len = *packet.get(at)? as usize;
    if scid_len > CID_MAX {
        return None;
    }
    at += 1 + scid_len;
    let (token_len, used) = varint(packet.get(at..)?)?;
    at += used + usize::try_from(token_len).ok()?;
    let (length, used) = varint(packet.get(at..)?)?;
    at += used;
    Some((version, dcid, at, usize::try_from(length).ok()?))
}

/// A QUIC variable-length integer, and how many bytes it took.
fn varint(bytes: &[u8]) -> Option<(u64, usize)> {
    let first = *bytes.first()?;
    let len = 1usize << (first >> 6);
    let mut value = u64::from(first & 0x3f);
    for byte in bytes.get(1..len)? {
        value = (value << 8) | u64::from(*byte);
    }
    Some((value, len))
}

/// The keys an Initial is protected with, which anybody may derive.
struct Keys {
    key: [u8; 16],
    iv: [u8; 12],
    hp: [u8; 16],
}

impl Keys {
    fn of(dcid: &[u8], version: &Version) -> Self {
        let initial = hkdf::Hkdf::<sha2::Sha256>::new(Some(&version.salt), dcid);
        let mut client = [0u8; 32];
        // `client in` is the one label every version shares.
        expand(&initial, b"client in", &mut client);
        let client = hkdf::Hkdf::<sha2::Sha256>::from_prk(&client).expect("32 bytes is a key");
        let (mut key, mut iv, mut hp) = ([0u8; 16], [0u8; 12], [0u8; 16]);
        expand(&client, version.key, &mut key);
        expand(&client, version.iv, &mut iv);
        expand(&client, version.hp, &mut hp);
        Self { key, iv, hp }
    }

    /// AES-ECB of one block, which is what header protection is.
    fn mask(&self, sample: &[u8]) -> [u8; 16] {
        let mut block = Array::<u8, aes_gcm::aes::cipher::consts::U16>::try_from(sample)
            .expect("a sample is one block");
        aes_gcm::aes::Aes128::new(&Array::from(self.hp)).encrypt_block(&mut block);
        block.into()
    }

    /// Opens the payload in place, leaving the plaintext at the front.
    fn open(&self, number: &[u8], aad: &[u8], body: &mut Vec<u8>) -> Option<()> {
        // The nonce is the IV with the packet number xored into its right.
        let mut nonce = self.iv;
        for (at, byte) in number.iter().rev().enumerate() {
            nonce[11 - at] ^= byte;
        }
        Aes128Gcm::new(&Array::from(self.key))
            .decrypt_in_place(&Array::from(nonce), aad, body)
            .ok()
    }
}

/// HKDF-Expand-Label as TLS 1.3 defines it and QUIC uses it: the label with
/// `tls13 ` in front, no context, and the length in front of both.
fn expand(from: &hkdf::Hkdf<sha2::Sha256>, label: &[u8], out: &mut [u8]) {
    let mut info = Vec::with_capacity(label.len() + 10);
    info.extend_from_slice(&(out.len() as u16).to_be_bytes());
    info.push(6 + label.len() as u8);
    info.extend_from_slice(b"tls13 ");
    info.extend_from_slice(label);
    info.push(0);
    from.expand(&info, out).expect("a length HKDF allows");
}

/// Takes the protection off the first byte and the packet number.
fn unprotect(packet: &[u8], keys: &Keys, pn_at: usize) -> Option<(u8, [u8; 4], usize)> {
    let sample = packet.get(pn_at + SAMPLE_AT..pn_at + SAMPLE_AT + SAMPLE)?;
    let mask = keys.mask(sample);
    let first = packet.first()? ^ (mask[0] & 0x0f);
    let pn_len = usize::from(first & 0x03) + 1;
    let mut number = [0u8; 4];
    for (at, byte) in packet.get(pn_at..pn_at + pn_len)?.iter().enumerate() {
        number[at] = byte ^ mask[1 + at];
    }
    Some((first, number, pn_len))
}

/// The hello in the CRYPTO frames of an opened payload.
///
/// Only what this packet carries, and only from offset zero: a hello that
/// begins elsewhere is the second half of one, and the first half is in a
/// datagram this never saw.
fn hello_in_frames(payload: &[u8]) -> Option<Hello> {
    let mut at = 0;
    let mut crypto: Vec<u8> = Vec::new();
    while at < payload.len() {
        let (kind, used) = varint(payload.get(at..)?)?;
        at += used;
        match kind {
            // PADDING and PING carry nothing.
            0x00 | 0x01 => {}
            0x06 => {
                let (offset, used) = varint(payload.get(at..)?)?;
                at += used;
                let (len, used) = varint(payload.get(at..)?)?;
                at += used;
                let len = usize::try_from(len).ok()?;
                let data = payload.get(at..at + len)?;
                at += len;
                let offset = usize::try_from(offset).ok()?;
                if offset > crypto.len() {
                    // A hole: the bytes before it were in another datagram.
                    return None;
                }
                crypto.resize(crypto.len().max(offset + len), 0);
                crypto[offset..offset + len].copy_from_slice(data);
            }
            // Anything else in a client's Initial is not something this
            // needs, and a frame of unknown length cannot be stepped over.
            _ => break,
        }
    }
    hello_in_crypto(&crypto)
}

/// The server name and the ALPN in a TLS ClientHello.
fn hello_in_crypto(crypto: &[u8]) -> Option<Hello> {
    // Handshake header: type 1 for a ClientHello, then a 24-bit length.
    if *crypto.first()? != 0x01 {
        return None;
    }
    let len = u32::from_be_bytes([0, crypto[1], crypto[2], crypto[3]]) as usize;
    let body = crypto.get(4..4 + len)?;
    // Legacy version and random.
    let mut at = 2 + 32;
    at += 1 + usize::from(*body.get(at)?); // session id
    let suites = usize::from(u16::from_be_bytes(body.get(at..at + 2)?.try_into().ok()?));
    at += 2 + suites;
    at += 1 + usize::from(*body.get(at)?); // compression methods
    let total = usize::from(u16::from_be_bytes(body.get(at..at + 2)?.try_into().ok()?));
    at += 2;
    let extensions = body.get(at..at + total)?;
    let mut said = Hello::default();
    let mut at = 0;
    while at + 4 <= extensions.len() {
        let kind = u16::from_be_bytes(extensions.get(at..at + 2)?.try_into().ok()?);
        let len = usize::from(u16::from_be_bytes(
            extensions.get(at + 2..at + 4)?.try_into().ok()?,
        ));
        let body = extensions.get(at + 4..at + 4 + len)?;
        at += 4 + len;
        match kind {
            // server_name: a list, of which the only kind defined is a host.
            0 => {
                let kind = *body.get(2)?;
                let len = usize::from(u16::from_be_bytes(body.get(3..5)?.try_into().ok()?));
                if kind == 0 {
                    said.server_name = String::from_utf8(body.get(5..5 + len)?.to_vec()).ok();
                }
            }
            // application_layer_protocol_negotiation: a list of names, each
            // with its length in front.
            16 => {
                let mut at = 2;
                while at < body.len() {
                    let len = usize::from(*body.get(at)?);
                    at += 1;
                    if let Ok(name) = String::from_utf8(body.get(at..at + len)?.to_vec()) {
                        said.alpn.push(name);
                    }
                    at += len;
                }
            }
            _ => {}
        }
    }
    Some(said)
}

/// The RFCs' own sample packets, kept where the stack's test of the
/// wiring can reach them as well: a packet this file encoded and then
/// decoded would agree with itself whatever both halves got wrong.
#[cfg(test)]
pub(crate) mod vectors {
    /// Hex as the RFCs print it, spaces and all.
    pub fn bytes(hex: &str) -> Vec<u8> {
        let hex: String = hex.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        (0..hex.len() / 2)
            .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).expect("hex"))
            .collect()
    }

    /// The client Initial from RFC 9001 Appendix A.2, verbatim.
    ///
    /// **An outside vector on purpose.** A packet this file encoded and then
    /// decoded would agree with itself whatever both halves got wrong; this
    /// one was written by the people who defined the protection, and the
    /// hello inside it says `example.com` and offers `alpn`, neither of which
    /// anything here chose.
    pub const RFC_9001_A2: &str = "
        c000000001088394c8f03e5157080000 449e7b9aec34d1b1c98dd7689fb8ec11
        d242b123dc9bd8bab936b47d92ec356c 0bab7df5976d27cd449f63300099f399
        1c260ec4c60d17b31f8429157bb35a12 82a643a8d2262cad67500cadb8e7378c
        8eb7539ec4d4905fed1bee1fc8aafba1 7c750e2c7ace01e6005f80fcb7df6212
        30c83711b39343fa028cea7f7fb5ff89 eac2308249a02252155e2347b63d58c5
        457afd84d05dfffdb20392844ae81215 4682e9cf012f9021a6f0be17ddd0c208
        4dce25ff9b06cde535d0f920a2db1bf3 62c23e596d11a4f5a6cf3948838a3aec
        4e15daf8500a6ef69ec4e3feb6b1d98e 610ac8b7ec3faf6ad760b7bad1db4ba3
        485e8a94dc250ae3fdb41ed15fb6a8e5 eba0fc3dd60bc8e30c5c4287e53805db
        059ae0648db2f64264ed5e39be2e20d8 2df566da8dd5998ccabdae053060ae6c
        7b4378e846d29f37ed7b4ea9ec5d82e7 961b7f25a9323851f681d582363aa5f8
        9937f5a67258bf63ad6f1a0b1d96dbd4 faddfcefc5266ba6611722395c906556
        be52afe3f565636ad1b17d508b73d874 3eeb524be22b3dcbc2c7468d54119c74
        68449a13d8e3b95811a198f3491de3e7 fe942b330407abf82a4ed7c1b311663a
        c69890f4157015853d91e923037c227a 33cdd5ec281ca3f79c44546b9d90ca00
        f064c99e3dd97911d39fe9c5d0b23a22 9a234cb36186c4819e8b9c5927726632
        291d6a418211cc2962e20fe47feb3edf 330f2c603a9d48c0fcb5699dbfe58964
        25c5bac4aee82e57a85aaf4e2513e4f0 5796b07ba2ee47d80506f8d2c25e50fd
        14de71e6c418559302f939b0e1abd576 f279c4b2e0feb85c1f28ff18f58891ff
        ef132eef2fa09346aee33c28eb130ff2 8f5b766953334113211996d20011a198
        e3fc433f9f2541010ae17c1bf202580f 6047472fb36857fe843b19f5984009dd
        c324044e847a4f4a0ab34f719595de37 252d6235365e9b84392b061085349d73
        203a4a13e96f5432ec0fd4a1ee65accd d5e3904df54c1da510b0ff20dcc0c77f
        cb2c0e0eb605cb0504db87632cf3d8b4 dae6e705769d1de354270123cb11450e
        fc60ac47683d7b8d0f811365565fd98c 4c8eb936bcab8d069fc33bd801b03ade
        a2e1fbc5aa463d08ca19896d2bf59a07 1b851e6c239052172f296bfb5e724047
        90a2181014f3b94a4e97d117b4381303 68cc39dbb2d198065ae3986547926cd2
        162f40a29f0c3c8745c0f50fba3852e5 66d44575c29d39a03f0cda721984b6f4
        40591f355e12d439ff150aab7613499d bd49adabc8676eef023b15b65bfc5ca0
        6948109f23f350db82123535eb8a7433 bdabcb909271a6ecbcb58b936a88cd4e
        8f2e6ff5800175f113253d8fa9ca8885 c2f552e657dc603f252e1a8e308f76f0
        be79e2fb8f5d5fbbe2e30ecadd220723 c8c0aea8078cdfcb3868263ff8f09400
        54da48781893a7e49ad5aff4af300cd8 04a6b6279ab3ff3afb64491c85194aab
        760d58a606654f9f4400e8b38591356f bf6425aca26dc85244259ff2b19c41b9
        f96f3ca9ec1dde434da7d2d392b905dd f3d1f9af93d1af5950bd493f5aa731b4
        056df31bd267b6b90a079831aaf579be 0a39013137aac6d404f518cfd4684064
        7e78bfe706ca4cf5e9c5453e9f7cfd2b 8b4c8d169a44e55c88d4a9a7f9474241
        e221af44860018ab0856972e194cd934";

    /// The client Initial from RFC 9369 Appendix A.2, verbatim: the same
    /// hello as above, under version 2's salt, labels and packet type. It is
    /// here because version 2 exists to break a reader that learnt version 1
    /// by heart, and nothing but a packet of its own can show that this one
    /// did not.
    pub const RFC_9369_A2: &str = "
        d76b3343cf088394c8f03e5157080000 449ea0c95e82ffe67b6abcdb4298b485
        dd04de806071bf03dceebfa162e75d6c 96058bdbfb127cdfcbf903388e99ad04
        9f9a3dd4425ae4d0992cfff18ecf0fdb 5a842d09747052f17ac2053d21f57c5d
        250f2c4f0e0202b70785b7946e992e58 a59ac52dea6774d4f03b55545243cf1a
        12834e3f249a78d395e0d18f4d766004 f1a2674802a747eaa901c3f10cda5500
        cb9122faa9f1df66c392079a1b40f0de 1c6054196a11cbea40afb6ef5253cd68
        18f6625efce3b6def6ba7e4b37a40f77 32e093daa7d52190935b8da58976ff33
        12ae50b187c1433c0f028edcc4c2838b 6a9bfc226ca4b4530e7a4ccee1bfa2a3
        d396ae5a3fb512384b2fdd851f784a65 e03f2c4fbe11a53c7777c023462239dd
        6f7521a3f6c7d5dd3ec9b3f233773d4b 46d23cc375eb198c63301c21801f6520
        bcfb7966fc49b393f0061d974a2706df 8c4a9449f11d7f3d2dcbb90c6b877045
        636e7c0c0fe4eb0f697545460c806910 d2c355f1d253bc9d2452aaa549e27a1f
        ac7cf4ed77f322e8fa894b6a83810a34 b361901751a6f5eb65a0326e07de7c12
        16ccce2d0193f958bb3850a833f7ae43 2b65bc5a53975c155aa4bcb4f7b2c4e5
        4df16efaf6ddea94e2c50b4cd1dfe060 17e0e9d02900cffe1935e0491d77ffb4
        fdf85290fdd893d577b1131a610ef6a5 c32b2ee0293617a37cbb08b847741c3b
        8017c25ca9052ca1079d8b78aebd4787 6d330a30f6a8c6d61dd1ab5589329de7
        14d19d61370f8149748c72f132f0fc99 f34d766c6938597040d8f9e2bb522ff9
        9c63a344d6a2ae8aa8e51b7b90a4a806 105fcbca31506c446151adfeceb51b91
        abfe43960977c87471cf9ad4074d30e1 0d6a7f03c63bd5d4317f68ff325ba3bd
        80bf4dc8b52a0ba031758022eb025cdd 770b44d6d6cf0670f4e990b22347a7db
        848265e3e5eb72dfe8299ad7481a4083 22cac55786e52f633b2fb6b614eaed18
        d703dd84045a274ae8bfa73379661388 d6991fe39b0d93debb41700b41f90a15
        c4d526250235ddcd6776fc77bc97e7a4 17ebcb31600d01e57f32162a8560cacc
        7e27a096d37a1a86952ec71bd89a3e9a 30a2a26162984d7740f81193e8238e61
        f6b5b984d4d3dfa033c1bb7e4f0037fe bf406d91c0dccf32acf423cfa1e70710
        10d3f270121b493ce85054ef58bada42 310138fe081adb04e2bd901f2f13458b
        3d6758158197107c14ebb193230cd115 7380aa79cae1374a7c1e5bbcb80ee23e
        06ebfde206bfb0fcbc0edc4ebec30966 1bdd908d532eb0c6adc38b7ca7331dce
        8dfce39ab71e7c32d318d136b6100671 a1ae6a6600e3899f31f0eed19e3417d1
        34b90c9058f8632c798d4490da498730 7cba922d61c39805d072b589bd52fdf1
        e86215c2d54e6670e07383a27bbffb5a ddf47d66aa85a0c6f9f32e59d85a44dd
        5d3b22dc2be80919b490437ae4f36a0a e55edf1d0b5cb4e9a3ecabee93dfc6e3
        8d209d0fa6536d27a5d6fbb17641cde2 7525d61093f1b28072d111b2b4ae5f89
        d5974ee12e5cf7d5da4d6a31123041f3 3e61407e76cffcdcfd7e19ba58cf4b53
        6f4c4938ae79324dc402894b44faf8af bab35282ab659d13c93f70412e85cb19
        9a37ddec600545473cfb5a05e08d0b20 9973b2172b4d21fb69745a262ccde96b
        a18b2faa745b6fe189cf772a9f84cbfc";
}

#[cfg(test)]
mod tests {
    use super::{vectors::*, *};

    #[test]
    fn the_rfcs_own_client_initial_says_where_it_was_going() {
        let said = hello_in(&bytes(RFC_9001_A2)).expect("a version 1 Initial this can open");
        assert_eq!(said.server_name.as_deref(), Some("example.com"));
        assert_eq!(said.alpn, ["alpn"]);
    }

    #[test]
    fn so_does_version_twos() {
        let said = hello_in(&bytes(RFC_9369_A2)).expect("a version 2 Initial this can open");
        assert_eq!(said.server_name.as_deref(), Some("example.com"));
        assert_eq!(said.alpn, ["alpn"]);
    }

    /// Both are seen as a client opening a connection, and each names its own
    /// version: the hello is the bonus, and the version is what says QUIC.
    #[test]
    fn both_are_a_client_opening_a_connection() {
        for (packet, version) in [(RFC_9001_A2, V1.number), (RFC_9369_A2, V2.number)] {
            let opening = opening(&bytes(packet)).expect("a client opening a connection");
            assert_eq!(opening.version, version);
            assert_eq!(
                opening.hello.and_then(|said| said.server_name).as_deref(),
                Some("example.com")
            );
        }
    }

    /// A version whose salt is not here is still QUIC, and said to be, with
    /// nothing read from it. This is the half of [`opening`] that [`hello_in`]
    /// cannot do: a program on an unreadable version would otherwise be
    /// absent from the list rather than visible and unread.
    #[test]
    fn a_version_this_cannot_open_is_still_quic() {
        let mut draft = bytes(RFC_9001_A2);
        draft[1..5].copy_from_slice(&0xff00_001du32.to_be_bytes()); // draft-29
        let opening = opening(&draft).expect("a long header with a version of its own");
        assert_eq!(opening.version, 0xff00_001d);
        assert_eq!(opening.hello, None, "its keys are not derived here");
        assert_eq!(hello_in(&draft), None, "and nothing of it is read");
    }

    /// What must not become a row saying a program spoke QUIC.
    #[test]
    fn what_is_not_a_client_opening_a_connection() {
        assert_eq!(opening(&[]), None, "nothing at all");
        assert_eq!(opening(&[0x40, 0, 0, 0, 1]), None, "a short header");
        assert_eq!(
            opening(&[0x16, 0xfe, 0xfd, 0, 0]),
            None,
            "a DTLS handshake record, which also goes to this port"
        );
        assert_eq!(opening(&[0x00, 0x01, 0, 8]), None, "a STUN binding request");
        let mut negotiation = bytes(RFC_9001_A2);
        negotiation[1..5].copy_from_slice(&[0; 4]);
        assert_eq!(
            opening(&negotiation),
            None,
            "version negotiation, which only a server sends"
        );
        let mut handshake = bytes(RFC_9001_A2);
        handshake[0] |= 0x30; // the type that means Handshake in version 1
        assert_eq!(
            opening(&handshake),
            None,
            "a later packet of a connection already under way"
        );
    }

    /// One socket trying again is one attempt; another socket is another.
    #[test]
    fn retries_of_one_attempt_are_told_about_once() {
        let mut told = Attempts::default();
        let now = Instant::now();
        let client: SocketAddr = "10.0.0.7:51000".parse().unwrap();
        let other: SocketAddr = "10.0.0.7:51001".parse().unwrap();
        let to: SocketAddr = "93.184.216.34:443".parse().unwrap();
        let elsewhere: SocketAddr = "93.184.216.35:443".parse().unwrap();

        assert!(told.fresh(client, to, now), "the first is news");
        assert!(!told.fresh(client, to, now), "the retry is not");
        assert!(
            !told.fresh(client, to, now + SAME_FOR - Duration::from_millis(1)),
            "nor is one just inside the minute"
        );
        assert!(
            told.fresh(client, to, now + SAME_FOR),
            "a socket still trying after a minute is worth saying again"
        );
        assert!(
            told.fresh(other, to, now),
            "another socket is another attempt"
        );
        assert!(
            told.fresh(client, elsewhere, now),
            "and so is the same socket to another server"
        );
    }

    /// A flood is forgotten rather than remembered, because the notes would
    /// be memory nothing frees.
    #[test]
    fn a_thousand_attempts_at_once_do_not_grow_without_end() {
        let mut told = Attempts::default();
        let now = Instant::now();
        let to: SocketAddr = "93.184.216.34:443".parse().unwrap();
        for port in 1..=(REMEMBERED_MAX as u16 + 500) {
            let client = SocketAddr::new("10.0.0.7".parse().unwrap(), port);
            assert!(told.fresh(client, to, now));
        }
        assert!(
            told.0.len() <= REMEMBERED_MAX,
            "{} remembered, which is more than the cap",
            told.0.len()
        );
    }

    /// Everything that is not a hello to read is quiet about it, because a
    /// datagram on this port that is not an Initial is the ordinary case.
    #[test]
    fn anything_else_is_no_hello_rather_than_a_guess() {
        assert_eq!(hello_in(&[]), None, "nothing at all");
        assert_eq!(hello_in(&[0x40, 0, 0, 0]), None, "a short header");
        let mut draft = bytes(RFC_9001_A2);
        draft[1] = 0xff; // a draft version, whose salt is its own
        assert_eq!(hello_in(&draft), None, "a version this does not read");
        let mut two = bytes(RFC_9001_A2);
        two[1..5].copy_from_slice(&V2.number.to_be_bytes());
        assert_eq!(
            hello_in(&two),
            None,
            "version 2's number on a version 1 packet: the type is no longer              Initial there, and its salt would not open this payload either"
        );
        let mut cut = bytes(RFC_9001_A2);
        cut.truncate(200);
        assert_eq!(hello_in(&cut), None, "a packet that ends mid-payload");
        let mut torn = bytes(RFC_9001_A2);
        let last = torn.len() - 1;
        torn[last] ^= 0xff;
        assert_eq!(hello_in(&torn), None, "a tag that does not check out");
    }
}
