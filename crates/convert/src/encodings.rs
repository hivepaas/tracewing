//! What a value was wrapped in on its way here: base64, hex, the escapes a
//! URL and an HTML page need.

use crate::{Confidence, Input, Kind, MAX_OUTPUT, Reading, Shown};
use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
use tracewing_i18n::tr;

/// Whether decoded bytes are worth showing, and as what. Bytes that are
/// neither text, nor JSON, nor a format anybody names are not a reading: they
/// are what any eight characters turn into when read as base64.
pub fn interesting(bytes: &[u8]) -> Option<Kind> {
    if let Some(kind) = recognised(bytes) {
        return Some(kind);
    }
    // Formats with nothing at the front to say what they are: raw deflate,
    // Brotli, protobuf. Unpacking or parsing them is the only way to know
    // they are there, and without that answer a chain that reaches one
    // through base64 stops a step short of it -- which is every SAML
    // response, since those are always base64 of deflate.
    if crate::packed::unpacks_to_something(bytes) || crate::binary::reads_as_protobuf(bytes) {
        return Some(Kind::Binary);
    }
    None
}

/// What these bytes say they are: text, JSON, XML, or a format whose first
/// bytes name it. Nothing is unpacked or parsed to find out, so this is what
/// the formats that have to be unpacked are measured against -- without
/// that, telling deflate from deflate-inside-deflate would go on forever.
pub(crate) fn recognised(bytes: &[u8]) -> Option<Kind> {
    if bytes.len() < 4 {
        return None;
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        let trimmed = text.trim_start();
        if (trimmed.starts_with('{') || trimmed.starts_with('['))
            && serde_json::from_str::<serde_json::Value>(trimmed).is_ok()
        {
            return Some(Kind::Json);
        }
        if trimmed.starts_with("<?xml") || trimmed.starts_with("<!DOCTYPE") {
            return Some(Kind::Xml);
        }
        let printable = text
            .chars()
            .filter(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
            .count();
        // Mostly readable, and not one long run of control bytes that happens
        // to be valid UTF-8.
        if printable * 10 >= text.chars().count() * 9 {
            return Some(Kind::Text);
        }
    }
    named_format(bytes).map(|_| Kind::Binary)
}

/// What the first bytes say the whole is, where they say anything.
pub fn named_format(bytes: &[u8]) -> Option<&'static str> {
    const MAGIC: &[(&[u8], &str)] = &[
        (&[0x1f, 0x8b], "gzip"),
        (&[0x28, 0xb5, 0x2f, 0xfd], "zstd"),
        (&[0x78, 0x01], "zlib"),
        (&[0x78, 0x9c], "zlib"),
        (&[0x78, 0xda], "zlib"),
        (b"\x89PNG", "PNG"),
        (&[0xff, 0xd8, 0xff], "JPEG"),
        (b"GIF8", "GIF"),
        (b"RIFF", "WebP"),
        (b"%PDF", "PDF"),
        (b"PK\x03\x04", "ZIP"),
        (b"OggS", "Ogg"),
        (b"\x00\x61\x73\x6d", "WebAssembly"),
        (&[0x30, 0x82], "DER"),
    ];
    MAGIC
        .iter()
        .find(|(magic, _)| bytes.starts_with(magic))
        .map(|(_, name)| *name)
}

/// How large the picture these bytes hold is, where that can be read from
/// its first bytes: a reader looking at `PNG` learns more from `PNG 128×128`.
pub fn picture_size(bytes: &[u8]) -> Option<(u32, u32)> {
    let be32 = |at: usize| -> Option<u32> {
        Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
    };
    let be16 = |at: usize| -> Option<u32> {
        Some(u32::from(u16::from_be_bytes(
            bytes.get(at..at + 2)?.try_into().ok()?,
        )))
    };
    if bytes.starts_with(b"\x89PNG") {
        // The header chunk comes first, and holds both.
        if bytes.get(12..16)? != b"IHDR" {
            return None;
        }
        return Some((be32(16)?, be32(20)?));
    }
    if bytes.starts_with(b"GIF8") {
        let le16 = |at: usize| -> Option<u32> {
            Some(u32::from(u16::from_le_bytes(
                bytes.get(at..at + 2)?.try_into().ok()?,
            )))
        };
        return Some((le16(6)?, le16(8)?));
    }
    if bytes.starts_with(&[0xff, 0xd8]) {
        // Walk the segments to the one that says how large the frame is.
        let mut at = 2;
        while at + 9 < bytes.len() {
            if bytes[at] != 0xff {
                return None;
            }
            let marker = bytes[at + 1];
            let length = be16(at + 2)? as usize;
            // The frame headers, less the four that are not.
            if (0xc0..=0xcf).contains(&marker) && !matches!(marker, 0xc4 | 0xc8 | 0xcc) {
                return Some((be16(at + 7)?, be16(at + 5)?));
            }
            at += 2 + length;
        }
    }
    None
}

/// `0000  7b 22 61 22 3a 31 7d                              {"a":1}`
pub fn hex_dump(bytes: &[u8]) -> String {
    bytes
        .chunks(16)
        .enumerate()
        .map(|(row, chunk)| {
            let hex: Vec<String> = chunk.iter().map(|b| format!("{b:02x}")).collect();
            let text: String = chunk
                .iter()
                .map(|&b| {
                    if b.is_ascii_graphic() || b == b' ' {
                        b as char
                    } else {
                        '.'
                    }
                })
                .collect();
            format!("{:04x}  {:<47}  {text}", row * 16, hex.join(" "))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Bytes as a reading, named by what they turned out to hold.
fn decoded(
    id: &'static str,
    label: String,
    bytes: Vec<u8>,
    sure: bool,
    asked: bool,
) -> Option<Reading> {
    // Asked for by name, it is done whatever comes out: a reader reaching
    // for "read as base64" wants the bytes, recognisable or not.
    let kind = match interesting(&bytes) {
        Some(kind) => kind,
        None if asked => Kind::Binary,
        None => return None,
    };
    let said = match (&kind, named_format(&bytes)) {
        (_, Some(name)) => match picture_size(&bytes) {
            Some((wide, high)) => format!("{label}: {name} {wide}×{high}"),
            None => format!("{label}: {name}"),
        },
        (Kind::Json, _) => format!("{label}: JSON"),
        (Kind::Xml, _) => format!("{label}: XML"),
        _ => label,
    };
    let confidence = if sure || kind != Kind::Text {
        Confidence::Certain
    } else {
        Confidence::Likely
    };
    Some(Reading::new(
        id,
        said,
        confidence,
        Shown::Bytes { kind, bytes },
    ))
}

/// base64, in either alphabet and padded or not.
pub fn base64(input: &Input) -> Option<Reading> {
    let text = input.text.trim_matches('"');
    if text.len() < 8 || text.len() > MAX_OUTPUT {
        return None;
    }
    let url_safe = text.contains('-') || text.contains('_');
    if !text
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'-' | b'_' | b'='))
    {
        return None;
    }
    let bytes = if url_safe {
        URL_SAFE
            .decode(text)
            .or_else(|_| URL_SAFE_NO_PAD.decode(text))
    } else {
        STANDARD
            .decode(text)
            .or_else(|_| STANDARD_NO_PAD.decode(text))
    }
    .ok()?;
    let label = if url_safe {
        tr!("base64url")
    } else {
        tr!("base64")
    };
    decoded("base64", label, bytes, false, input.asked)
}

/// Hex, as a hash, a key or a dumped body is written.
pub fn hex(input: &Input) -> Option<Reading> {
    let text = input.text.trim_matches('"').trim_start_matches("0x");
    if text.len() < 8
        || !text.len().is_multiple_of(2)
        || !text.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return None;
    }
    let bytes: Vec<u8> = text
        .as_bytes()
        .chunks(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap_or("zz"), 16))
        .collect::<Result<_, _>>()
        .ok()?;
    decoded("hex", tr!("Hex"), bytes, false, input.asked)
}

/// `%E2%9C%93`, as a URL and a form carry what is not plain.
pub fn percent(input: &Input) -> Option<Reading> {
    let text = input.text;
    if !text.contains('%') {
        return None;
    }
    let plain = percent_decode(text)?;
    if plain == text {
        return None;
    }
    Some(Reading::new(
        "percent",
        tr!("Percent-encoded"),
        Confidence::Certain,
        Shown::Line(plain),
    ))
}

/// `%41` as `A`, and `+` left alone: a query field's `+` is handled where
/// query fields are.
pub fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    let mut any = false;
    while at < bytes.len() {
        if bytes[at] == b'%' && at + 2 < bytes.len() {
            let pair = std::str::from_utf8(&bytes[at + 1..at + 3]).ok()?;
            match u8::from_str_radix(pair, 16) {
                Ok(byte) => {
                    out.push(byte);
                    at += 3;
                    any = true;
                    continue;
                }
                Err(_) => return None,
            }
        }
        out.push(bytes[at]);
        at += 1;
    }
    if !any {
        return None;
    }
    String::from_utf8(out).ok()
}

/// `é` and `\n`, as JSON writes what it cannot hold plainly.
pub fn unicode_escapes(input: &Input) -> Option<Reading> {
    let text = input.text;
    if !text.contains("\\u") && !text.contains("\\n") && !text.contains("\\t") {
        return None;
    }
    let quoted = format!("\"{}\"", text.replace('"', "\\\""));
    let plain: String = serde_json::from_str(&quoted).ok()?;
    if plain == text {
        return None;
    }
    Some(Reading::new(
        "escapes",
        tr!("Escaped text"),
        Confidence::Certain,
        Shown::Line(plain),
    ))
}

/// `&amp;`, `&#233;`, as an HTML page carries what is not plain.
pub fn entities(input: &Input) -> Option<Reading> {
    const NAMED: &[(&str, char)] = &[
        ("amp", '&'),
        ("lt", '<'),
        ("gt", '>'),
        ("quot", '"'),
        ("apos", '\''),
        ("nbsp", '\u{a0}'),
        ("copy", '©'),
        ("reg", '®'),
        ("hellip", '…'),
        ("mdash", '—'),
        ("ndash", '–'),
    ];
    let text = input.text;
    if !text.contains('&') || !text.contains(';') {
        return None;
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    let mut any = false;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at + 1..];
        let Some(end) = rest.find(';').filter(|end| *end <= 8) else {
            out.push('&');
            continue;
        };
        let name = &rest[..end];
        let found = if let Some(digits) = name.strip_prefix("#x").or(name.strip_prefix("#X")) {
            u32::from_str_radix(digits, 16)
                .ok()
                .and_then(char::from_u32)
        } else if let Some(digits) = name.strip_prefix('#') {
            digits.parse().ok().and_then(char::from_u32)
        } else {
            NAMED
                .iter()
                .find(|(named, _)| *named == name)
                .map(|(_, c)| *c)
        };
        match found {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
                any = true;
            }
            None => out.push('&'),
        }
    }
    out.push_str(rest);
    if !any {
        return None;
    }
    Some(Reading::new(
        "entities",
        tr!("HTML entities"),
        Confidence::Certain,
        Shown::Line(out),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{by_id, line, read};

    fn bytes_of(readings: &[Reading], id: &str) -> Vec<u8> {
        match by_id(readings, id).map(|r| &r.shown) {
            Some(Shown::Bytes { bytes, .. }) => bytes.clone(),
            other => panic!("no bytes for {id}: {other:?}"),
        }
    }

    #[test]
    fn base64_is_read_in_either_alphabet() {
        let found = read("SGVsbG8sIHdvcmxkIQ==");
        assert_eq!(bytes_of(&found, "base64"), b"Hello, world!");
        let found = read("eyJhIjoxfQ");
        assert_eq!(bytes_of(&found, "base64"), br#"{"a":1}"#);
        // `-` and `_` say the URL's alphabet.
        let found = read("Y2Fm76-_IGF1IGxhaXQ");
        assert!(by_id(&found, "base64").is_some());
    }

    #[test]
    fn what_decodes_to_nothing_anybody_wants_is_not_a_reading() {
        // Valid base64, and six bytes of noise.
        assert!(by_id(&read("abcdefgh"), "base64").is_none());
        assert!(by_id(&read("hello"), "base64").is_none());
    }

    #[test]
    fn hex_is_read_when_it_holds_something() {
        let found = read("48656c6c6f2c20776f726c6421");
        assert_eq!(bytes_of(&found, "hex"), b"Hello, world!");
        let found = read("0x48656c6c6f2c20776f726c6421");
        assert_eq!(bytes_of(&found, "hex"), b"Hello, world!");
        // A hash is 32 bytes of nothing to read.
        assert!(by_id(&read(&"a3".repeat(16)), "hex").is_none());
    }

    #[test]
    fn a_decoded_format_is_named() {
        // Its signature and no header chunk: a PNG that cannot say its size.
        let png = STANDARD.encode(b"\x89PNG\r\n\x1a\n");
        let found = read(&png);
        let label = &by_id(&found, "base64").unwrap().label;
        assert!(label.ends_with("PNG"), "{label}");
        assert!(
            !label.contains('×'),
            "nothing to say about its size: {label}"
        );
    }

    #[test]
    fn a_picture_says_how_large_it_is() {
        // A PNG header chunk, saying 128 by 64.
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend_from_slice(&13u32.to_be_bytes());
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&128u32.to_be_bytes());
        png.extend_from_slice(&64u32.to_be_bytes());
        assert_eq!(picture_size(&png), Some((128, 64)));
        let found = read(&STANDARD.encode(&png));
        let label = &by_id(&found, "base64").unwrap().label;
        assert!(label.ends_with("PNG 128×64"), "{label}");

        // A GIF, which counts the other way round.
        let mut gif = b"GIF89a".to_vec();
        gif.extend_from_slice(&64u16.to_le_bytes());
        gif.extend_from_slice(&32u16.to_le_bytes());
        assert_eq!(picture_size(&gif), Some((64, 32)));

        // A JPEG, where it is in a segment further in.
        let mut jpeg = vec![0xff, 0xd8, 0xff, 0xe0, 0x00, 0x04, 0x00, 0x00];
        jpeg.extend_from_slice(&[0xff, 0xc0, 0x00, 0x11, 0x08]);
        jpeg.extend_from_slice(&200u16.to_be_bytes());
        jpeg.extend_from_slice(&300u16.to_be_bytes());
        jpeg.extend_from_slice(&[0; 8]);
        assert_eq!(picture_size(&jpeg), Some((300, 200)));

        assert_eq!(picture_size(b"{\"a\":1}"), None);
        assert_eq!(picture_size(b"\x89PNG"), None);
    }

    #[test]
    fn the_escapes_are_taken_off() {
        assert_eq!(
            line(&read("caf%C3%A9%20au%20lait"), "percent"),
            "café au lait"
        );
        let escaped = ["caf", "\\", "u00e9"].concat();
        assert_eq!(line(&read(&escaped), "escapes"), "café");
        assert_eq!(line(&read("a &amp; b &#233; c"), "entities"), "a & b é c");
        assert!(by_id(&read("100%"), "percent").is_none());
        assert!(by_id(&read("a & b"), "entities").is_none());
    }

    #[test]
    fn a_dump_says_the_bytes_and_what_they_spell() {
        let dump = hex_dump(br#"{"a":1}"#);
        assert_eq!(
            dump,
            r#"0000  7b 22 61 22 3a 31 7d                             {"a":1}"#
        );
    }
}

/// Ascii85, as PDF and git's binary patches write bytes, and ZeroMQ's z85.
///
/// Five characters carry four bytes, which is why it is reached for where
/// base64's four-for-three is thought wasteful. Adobe's wraps it in `<~ ~>`;
/// z85 uses an alphabet safe inside a string literal.
pub fn base85(input: &Input) -> Option<Reading> {
    let text = input.text.trim().trim_matches('"');
    let adobe = text.strip_prefix("<~").and_then(|t| t.strip_suffix("~>"));
    let (text, label) = match adobe {
        Some(inside) => (inside, tr!("Ascii85")),
        None => (text, tr!("Ascii85")),
    };
    if text.len() < 5 {
        return None;
    }
    let bytes = ascii85(text).or_else(|| z85(text))?;
    let label = if ascii85(text).is_some() {
        label
    } else {
        tr!("z85")
    };
    decoded("base85", label, bytes, false, input.asked)
}

/// The Ascii85 alphabet: `!` through `u`, with `z` for four zero bytes.
fn ascii85(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut group = Vec::with_capacity(5);
    for c in text.chars().filter(|c| !c.is_whitespace()) {
        if c == 'z' && group.is_empty() {
            out.extend_from_slice(&[0, 0, 0, 0]);
            continue;
        }
        if !('!'..='u').contains(&c) {
            return None;
        }
        group.push(c as u32 - 33);
        if group.len() == 5 {
            out.extend_from_slice(&four(&group, 5)?);
            group.clear();
        }
    }
    if !group.is_empty() {
        // A short last group is padded with the highest digit, and only as
        // many bytes as it carried are kept.
        let held = group.len();
        if held == 1 {
            return None;
        }
        let mut padded = group.clone();
        padded.resize(5, 84);
        let whole = four(&padded, 5)?;
        out.extend_from_slice(&whole[..held - 1]);
    }
    (!out.is_empty()).then_some(out)
}

/// What five base85 digits hold, as four bytes.
fn four(group: &[u32], base: u32) -> Option<[u8; 4]> {
    let _ = base;
    let mut n: u32 = 0;
    for digit in group {
        n = n.checked_mul(85)?.checked_add(*digit)?;
    }
    Some(n.to_be_bytes())
}

/// ZeroMQ's alphabet, which is five characters to four bytes as well.
fn z85(text: &str) -> Option<Vec<u8>> {
    const ALPHABET: &[u8] =
        b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ.-:+=^!/*?&<>()[]{}@%$#";
    if !text.len().is_multiple_of(5) {
        return None;
    }
    let mut out = Vec::with_capacity(text.len() / 5 * 4);
    for group in text.as_bytes().chunks(5) {
        let mut n: u32 = 0;
        for byte in group {
            let digit = ALPHABET.iter().position(|a| a == byte)? as u32;
            n = n.checked_mul(85)?.checked_add(digit)?;
        }
        out.extend_from_slice(&n.to_be_bytes());
    }
    (!out.is_empty()).then_some(out)
}

/// base58, and base58check where the four bytes at the end check out.
///
/// The alphabet leaves out `0`, `O`, `I` and `l`, so that what is read aloud
/// or copied by eye cannot turn into something else. Bitcoin addresses,
/// Solana keys and IPFS names are written in it.
pub fn base58(input: &Input) -> Option<Reading> {
    const ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let text = input.text.trim().trim_matches('"');
    if !(8..=256).contains(&text.len()) || !text.bytes().all(|b| ALPHABET.contains(&b)) {
        return None;
    }
    // Base 58 the long way: every digit multiplies what is held so far.
    let mut bytes: Vec<u8> = Vec::new();
    for c in text.bytes() {
        let mut carry = ALPHABET.iter().position(|a| *a == c)? as u32;
        for byte in bytes.iter_mut().rev() {
            let n = u32::from(*byte) * 58 + carry;
            *byte = (n & 0xff) as u8;
            carry = n >> 8;
        }
        while carry > 0 {
            bytes.insert(0, (carry & 0xff) as u8);
            carry >>= 8;
        }
    }
    // Each leading `1` is a zero byte, which the arithmetic above drops.
    let zeros = text.bytes().take_while(|b| *b == b'1').count();
    let mut whole = vec![0u8; zeros];
    whole.extend_from_slice(&bytes);

    // base58check: the last four bytes are the start of the double SHA-256
    // of the rest. Nothing else makes that true by accident, so this is the
    // one way to be sure of base58 rather than merely able to decode it.
    if whole.len() > 4 {
        use sha2::{Digest, Sha256};
        let (body, given) = whole.split_at(whole.len() - 4);
        let once = Sha256::digest(body);
        let twice = Sha256::digest(once);
        if twice[..4] == *given {
            let version = body.first().copied().unwrap_or(0);
            let what = match (version, body.len()) {
                (0x00, 21) => tr!("a Bitcoin address"),
                (0x05, 21) => tr!("a Bitcoin script address"),
                (0x80, 34) | (0x80, 33) => tr!("a Bitcoin private key"),
                _ => tr!("version {version}", version = format!("{version:#04x}")),
            };
            return Some(Reading::new(
                "base58",
                tr!("base58check"),
                Confidence::Certain,
                Shown::Fields(vec![
                    (tr!("What it is"), what),
                    (
                        tr!("How much"),
                        crate::numbers::bytes_size(body.len() as u64 - 1),
                    ),
                    (tr!("Checksum"), tr!("holds")),
                ]),
            ));
        }
    }
    decoded("base58", tr!("base58"), whole, false, input.asked)
}

/// RFC 4648's base32: a TOTP secret, an onion address, a DNS label that has
/// to survive a case-insensitive hop.
pub fn base32(input: &Input) -> Option<Reading> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let text = input.text.trim().trim_matches('"').trim_end_matches('=');
    let upper = text.to_ascii_uppercase();
    if upper.len() < 8 || !upper.bytes().all(|b| ALPHABET.contains(&b)) {
        return None;
    }
    let mut out = Vec::new();
    let (mut held, mut bits) = (0u32, 0u32);
    for c in upper.bytes() {
        let digit = ALPHABET.iter().position(|a| *a == c)? as u32;
        held = held << 5 | digit;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((held >> bits & 0xff) as u8);
        }
    }
    decoded("base32", tr!("base32"), out, false, input.asked)
}

/// Quoted-printable, as mail writes anything that is not plain ASCII: `=C3=A9`
/// for a letter, `=` at the end of a line for a line that did not end.
pub fn quoted_printable(input: &Input) -> Option<Reading> {
    let text = input.text.trim_matches('"');
    if !text.contains('=') {
        return None;
    }
    let mut out: Vec<u8> = Vec::new();
    let mut seen = false;
    let bytes = text.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'=' {
            match bytes.get(at + 1..at + 3) {
                // A line that goes on: the break is not part of the text.
                Some(b"\r\n") | Some(b"\n\r") => {
                    at += 3;
                    seen = true;
                    continue;
                }
                Some(pair) if pair[0] == b'\n' => {
                    at += 2;
                    seen = true;
                    continue;
                }
                Some(pair) => {
                    let hex = std::str::from_utf8(pair).ok()?;
                    let byte = u8::from_str_radix(hex, 16).ok()?;
                    out.push(byte);
                    at += 3;
                    seen = true;
                    continue;
                }
                None => return None,
            }
        }
        out.push(bytes[at]);
        at += 1;
    }
    if !seen {
        return None;
    }
    let plain = String::from_utf8(out).ok()?;
    if plain == text {
        return None;
    }
    Some(Reading::new(
        "quoted-printable",
        tr!("Quoted-printable"),
        Confidence::Certain,
        Shown::Line(plain),
    ))
}

#[cfg(test)]
mod alphabet_tests {
    use super::*;
    use crate::tests::{by_id, read};

    fn said(found: &[Reading], id: &str) -> String {
        match by_id(found, id).map(|reading| &reading.shown) {
            Some(Shown::Line(line)) => line.clone(),
            other => panic!("no line for {id}: {other:?}"),
        }
    }

    /// Five characters to four bytes, as a PDF or a binary patch writes it.
    #[test]
    fn ascii85_is_read_and_so_is_what_it_held() {
        // `{"hello":"world"}` in Ascii85.
        let found = read("HQm-7Ci\"#63[/cdEbo81I/");
        let inside = by_id(&found, "base85").expect("the Ascii85 in it");
        assert_eq!(inside.chain, vec!["base85"]);
        assert!(by_id(&found, "json").is_some(), "and the JSON inside it");
        // Adobe wraps it; the wrapping is not part of the text.
        assert!(by_id(&read("<~HQm-7Ci\"#63[/cdEbo81I/~>"), "base85").is_some());
    }

    /// base58 says it is base58 when the four bytes at the end check out,
    /// which is the only way to be sure rather than merely able to decode.
    #[test]
    fn a_bitcoin_address_checks_out() {
        let found = read("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa");
        let reading = by_id(&found, "base58").expect("the address");
        assert_eq!(reading.confidence, Confidence::Certain);
        let Shown::Fields(fields) = &reading.shown else {
            panic!("{reading:?}")
        };
        assert_eq!(fields[0].1, "a Bitcoin address");
        assert_eq!(fields[2].1, "holds");
    }

    /// And a word in the same alphabet, which decodes to nothing worth
    /// reading, says nothing at all.
    #[test]
    fn base58_of_nothing_in_particular_is_not_offered() {
        let found = read("PleaseReadThisCarefully");
        assert!(by_id(&found, "base58").is_none(), "{found:?}");
    }

    #[test]
    fn base32_holds_what_it_holds() {
        // `{"a":1}` in base32.
        let found = read("PMRGCIR2GF6Q====");
        assert!(by_id(&found, "base32").is_some(), "{found:?}");
    }

    #[test]
    fn quoted_printable_is_the_letters_it_stands_for() {
        let found = read("caf=C3=A9_=C4=91=E1=BA=B7t");
        assert_eq!(said(&found, "quoted-printable"), "café_đặt");
        // Text with no escapes in it is not quoted-printable, whatever
        // equals signs it holds.
        assert!(by_id(&read("a=1&b=2"), "quoted-printable").is_none());
    }
}
