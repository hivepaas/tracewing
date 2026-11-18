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
//! - **Versions other than 1.** Version 2 (RFC 9369) uses a different salt
//!   and different labels, and draft versions used their own; each would be a
//!   table entry here and none is guessed.

use aes_gcm::aes::cipher::{Array, BlockCipherEncrypt, KeyInit};
use aes_gcm::{AeadInOut, Aes128Gcm};

/// QUIC version 1, the one this reads.
const V1: u32 = 0x0000_0001;

/// The salt RFC 9001 §5.2 publishes for version 1. Every endpoint and every
/// observer uses the same one, which is what makes an Initial readable.
const V1_SALT: [u8; 20] = [
    0x38, 0x76, 0x2c, 0xf7, 0xf5, 0x59, 0x34, 0xb3, 0x4d, 0x17, 0x9a, 0xe6, 0xa4, 0xc8, 0x0c, 0xad,
    0xcc, 0xbb, 0x7f, 0x0a,
];

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
/// `None` for anything that is not a version 1 Initial this can open,
/// including every reason listed on the module above. It is deliberately
/// quiet about which: a datagram on this port that is not a QUIC Initial is
/// the ordinary case, not a fault.
pub fn hello_in(datagram: &[u8]) -> Option<Hello> {
    let (header, dcid, pn_at, length) = initial_parts(datagram)?;
    let keys = Keys::of(dcid);
    let (first, number, pn_len) = unprotect(datagram, &keys, pn_at)?;

    // The header as it was before protection, which is what the payload was
    // sealed against: the first byte and the packet number, both now in the
    // clear, and everything between them untouched.
    let mut aad = datagram.get(..pn_at + pn_len)?.to_vec();
    aad[0] = first;
    aad[pn_at..].copy_from_slice(&number[..pn_len]);
    let _ = header;

    let end = pn_at + length;
    let mut body = datagram.get(pn_at + pn_len..end)?.to_vec();
    if body.len() < TAG {
        return None;
    }
    keys.open(&number[..pn_len], &aad, &mut body)?;
    hello_in_frames(&body)
}

/// The parts of an Initial header that are never protected: where the packet
/// number starts, how long the rest is, and the connection ID the keys come
/// from.
fn initial_parts(packet: &[u8]) -> Option<(usize, &[u8], usize, usize)> {
    let first = *packet.first()?;
    // Long header, fixed bit set, type Initial. A short header is a packet
    // from a connection already under way, whose keys nobody on the path has.
    if first & 0xc0 != 0xc0 || (first >> 4) & 0x03 != 0 {
        return None;
    }
    if u32::from_be_bytes(packet.get(1..5)?.try_into().ok()?) != V1 {
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
    Some((at, dcid, at, usize::try_from(length).ok()?))
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
    fn of(dcid: &[u8]) -> Self {
        let initial = hkdf::Hkdf::<sha2::Sha256>::new(Some(&V1_SALT), dcid);
        let mut client = [0u8; 32];
        expand(&initial, b"client in", &mut client);
        let client = hkdf::Hkdf::<sha2::Sha256>::from_prk(&client).expect("32 bytes is a key");
        let (mut key, mut iv, mut hp) = ([0u8; 16], [0u8; 12], [0u8; 16]);
        expand(&client, b"quic key", &mut key);
        expand(&client, b"quic iv", &mut iv);
        expand(&client, b"quic hp", &mut hp);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Hex as the RFCs print it, spaces and all.
    fn bytes(hex: &str) -> Vec<u8> {
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
    const RFC_9001_A2: &str = "
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

    #[test]
    fn the_rfcs_own_client_initial_says_where_it_was_going() {
        let said = hello_in(&bytes(RFC_9001_A2)).expect("a version 1 Initial this can open");
        assert_eq!(said.server_name.as_deref(), Some("example.com"));
        assert_eq!(said.alpn, ["alpn"]);
    }

    /// Everything that is not a hello to read is quiet about it, because a
    /// datagram on this port that is not an Initial is the ordinary case.
    #[test]
    fn anything_else_is_no_hello_rather_than_a_guess() {
        assert_eq!(hello_in(&[]), None, "nothing at all");
        assert_eq!(hello_in(&[0x40, 0, 0, 0]), None, "a short header");
        let mut other = bytes(RFC_9001_A2);
        other[1] = 0x6b; // version 2, whose salt and labels differ
        assert_eq!(hello_in(&other), None, "a version this does not read");
        let mut cut = bytes(RFC_9001_A2);
        cut.truncate(200);
        assert_eq!(hello_in(&cut), None, "a packet that ends mid-payload");
        let mut torn = bytes(RFC_9001_A2);
        let last = torn.len() - 1;
        torn[last] ^= 0xff;
        assert_eq!(hello_in(&torn), None, "a tag that does not check out");
    }
}
