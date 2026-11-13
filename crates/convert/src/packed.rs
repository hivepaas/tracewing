//! What was packed for the wire, unpacked: gzip, zlib, raw deflate, zstd,
//! Brotli. Each stops at [`MAX_OUTPUT`], so a small value that swells to a
//! gigabyte is refused rather than waited for.

use crate::{Confidence, Input, MAX_OUTPUT, Reading, Shown, encodings};
use std::io::Read;
use tracewing_i18n::tr;

/// Read out, up to one byte past what is allowed, so going over is seen.
fn unpack(reader: impl Read) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    reader
        .take(MAX_OUTPUT as u64 + 1)
        .read_to_end(&mut out)
        .ok()?;
    (out.len() <= MAX_OUTPUT && !out.is_empty()).then_some(out)
}

/// The reading an unpacked value makes, if it holds anything worth showing.
fn unpacked(id: &'static str, label: String, bytes: Vec<u8>, asked: bool) -> Option<Reading> {
    // Asked for by name, it is done whatever comes out.
    let kind = match encodings::interesting(&bytes) {
        Some(kind) => kind,
        None if asked => crate::Kind::Binary,
        None => return None,
    };
    let said = format!(
        "{label}, {}",
        crate::numbers::bytes_size(bytes.len() as u64)
    );
    Some(Reading::new(
        id,
        said,
        Confidence::Certain,
        Shown::Bytes { kind, bytes },
    ))
}

/// Whether raw deflate or Brotli is in here, told by unpacking it and seeing
/// whether what comes out is something a reader would want. Neither format
/// puts anything at the front to say what it is, so unpacking is the only
/// way to know, and what comes out is measured against what bytes say for
/// themselves rather than against this, which would not end.
pub(crate) fn unpacks_to_something(bytes: &[u8]) -> bool {
    if !(8..=crate::MAX_INPUT).contains(&bytes.len()) {
        return false;
    }
    let worth = |out: Vec<u8>| encodings::recognised(&out).is_some();
    unpack(flate2::read::DeflateDecoder::new(bytes)).is_some_and(worth)
        || unpack(brotli::Decompressor::new(bytes, 4096)).is_some_and(worth)
}

pub fn gzip(input: &Input) -> Option<Reading> {
    let raw = input.raw();
    if !raw.starts_with(&[0x1f, 0x8b]) {
        return None;
    }
    let bytes = unpack(flate2::read::GzDecoder::new(raw))?;
    unpacked("gzip", tr!("gzip"), bytes, input.asked)
}

pub fn zlib(input: &Input) -> Option<Reading> {
    let raw = input.raw();
    if !matches!(raw.first(), Some(0x78)) {
        return None;
    }
    let bytes = unpack(flate2::read::ZlibDecoder::new(raw))?;
    unpacked("zlib", tr!("zlib"), bytes, input.asked)
}

/// Deflate with nothing around it, as a WebSocket and a SAML request send it.
/// It has no magic, so it is only a reading when what comes out reads.
pub fn deflate(input: &Input) -> Option<Reading> {
    let raw = input.raw();
    if raw.len() < 8 {
        return None;
    }
    let bytes = unpack(flate2::read::DeflateDecoder::new(raw))?;
    unpacked("deflate", tr!("deflate"), bytes, input.asked)
}

pub fn zstd(input: &Input) -> Option<Reading> {
    let raw = input.raw();
    if !raw.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]) {
        return None;
    }
    let bytes = unpack(zstd::stream::read::Decoder::new(raw).ok()?)?;
    unpacked("zstd", tr!("zstd"), bytes, input.asked)
}

/// Brotli has no magic either: the same rule as deflate.
pub fn brotli(input: &Input) -> Option<Reading> {
    let raw = input.raw();
    if raw.len() < 8 {
        return None;
    }
    let bytes = unpack(brotli::Decompressor::new(raw, 4096))?;
    unpacked("brotli", tr!("Brotli"), bytes, input.asked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{by_id, read};
    use std::io::Write;

    fn packed_json() -> Vec<u8> {
        let mut out = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        out.write_all(br#"{"hello":"world"}"#).unwrap();
        out.finish().unwrap()
    }

    fn b64(bytes: &[u8]) -> String {
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }

    fn deflated(data: &[u8]) -> Vec<u8> {
        let mut out = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::fast());
        out.write_all(data).unwrap();
        out.finish().unwrap()
    }

    fn brotlied(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut from = data;
        brotli::BrotliCompress(&mut from, &mut out, &Default::default()).unwrap();
        out
    }

    /// Raw deflate says nothing about itself at the front, so base64 of it
    /// used to read as nothing at all: the step that would have unpacked it
    /// was never reached.
    #[test]
    fn deflate_inside_base64_is_reached() {
        let found = read(&b64(&deflated(br#"{"hello":"world"}"#)));
        let reading = by_id(&found, "deflate").expect("the deflate in it");
        assert_eq!(reading.chain, vec!["base64", "deflate"]);
        let json = by_id(&found, "json").expect("and the JSON inside that");
        assert_eq!(json.chain, vec!["base64", "deflate", "json"]);
    }

    /// Brotli has no magic number either.
    #[test]
    fn brotli_inside_base64_is_reached() {
        let found = read(&b64(&brotlied(br#"{"hello":"world"}"#)));
        let reading = by_id(&found, "brotli").expect("the Brotli in it");
        assert_eq!(reading.chain, vec!["base64", "brotli"]);
    }

    /// And bytes that are neither are still nothing: unpacking has to fail
    /// for what is not packed, or every key and hash would read as deflate.
    #[test]
    fn bytes_that_are_not_packed_are_not_offered_as_packed() {
        // A 32-byte key, as a value would carry one.
        let key: Vec<u8> = (0..32u8)
            .map(|n| n.wrapping_mul(37).wrapping_add(11))
            .collect();
        let found = read(&b64(&key));
        for id in ["deflate", "brotli", "gzip", "zlib", "zstd"] {
            assert!(by_id(&found, id).is_none(), "{id}: {found:?}");
        }
    }

    #[test]
    fn gzip_is_unpacked_and_what_it_holds_is_read() {
        let packed = packed_json();
        let input = Input::new("", 0, 0).with_bytes(&packed);
        let found = crate::readings(&input);
        let reading = by_id(&found, "gzip").unwrap();
        assert!(reading.label.starts_with("gzip"), "{}", reading.label);
        assert!(by_id(&found, "json").is_some(), "the JSON inside is read");
    }

    #[test]
    fn zstd_is_unpacked() {
        let packed = zstd::encode_all(&br#"{"hello":"world"}"#[..], 1).unwrap();
        let input = Input::new("", 0, 0).with_bytes(&packed);
        let found = crate::readings(&input);
        assert!(by_id(&found, "zstd").is_some());
    }

    #[test]
    fn what_is_not_packed_is_left_alone() {
        let found = read("just some words here");
        for id in ["gzip", "zlib", "deflate", "zstd", "brotli"] {
            assert!(by_id(&found, id).is_none(), "{id} read plain text");
        }
    }
}
