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
fn unpacked(id: &'static str, label: String, bytes: Vec<u8>) -> Option<Reading> {
    let kind = encodings::interesting(&bytes)?;
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

pub fn gzip(input: &Input) -> Option<Reading> {
    let raw = input.raw();
    if !raw.starts_with(&[0x1f, 0x8b]) {
        return None;
    }
    let bytes = unpack(flate2::read::GzDecoder::new(raw))?;
    unpacked("gzip", tr!("gzip"), bytes)
}

pub fn zlib(input: &Input) -> Option<Reading> {
    let raw = input.raw();
    if !matches!(raw.first(), Some(0x78)) {
        return None;
    }
    let bytes = unpack(flate2::read::ZlibDecoder::new(raw))?;
    unpacked("zlib", tr!("zlib"), bytes)
}

/// Deflate with nothing around it, as a WebSocket and a SAML request send it.
/// It has no magic, so it is only a reading when what comes out reads.
pub fn deflate(input: &Input) -> Option<Reading> {
    let raw = input.raw();
    if raw.len() < 8 {
        return None;
    }
    let bytes = unpack(flate2::read::DeflateDecoder::new(raw))?;
    unpacked("deflate", tr!("deflate"), bytes)
}

pub fn zstd(input: &Input) -> Option<Reading> {
    let raw = input.raw();
    if !raw.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]) {
        return None;
    }
    let bytes = unpack(zstd::stream::read::Decoder::new(raw).ok()?)?;
    unpacked("zstd", tr!("zstd"), bytes)
}

/// Brotli has no magic either: the same rule as deflate.
pub fn brotli(input: &Input) -> Option<Reading> {
    let raw = input.raw();
    if raw.len() < 8 {
        return None;
    }
    let bytes = unpack(brotli::Decompressor::new(raw, 4096))?;
    unpacked("brotli", tr!("Brotli"), bytes)
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
