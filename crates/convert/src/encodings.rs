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
fn decoded(id: &'static str, label: String, bytes: Vec<u8>, sure: bool) -> Option<Reading> {
    let kind = interesting(&bytes)?;
    let said = match (&kind, named_format(&bytes)) {
        (_, Some(name)) => format!("{label}: {name}"),
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
    decoded("base64", label, bytes, false)
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
    decoded("hex", tr!("Hex"), bytes, false)
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
        let png = STANDARD.encode(b"\x89PNG\r\n\x1a\n");
        let found = read(&png);
        let label = &by_id(&found, "base64").unwrap().label;
        assert!(label.ends_with("PNG"), "{label}");
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
