//! Turning a body as it was sent back into its text.
//!
//! A body comes from a stranger, so this assumes the worst of it: it may be cut
//! off (only the first part was kept), corrupt, or a few KB that unpack to many
//! GB. So it never asks for more than `max` bytes, and whatever came out before
//! an error still counts.

use std::io::{BufRead, BufReader, Read, Write};

/// How a stored body is packed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    Identity,
    Gzip,
    Deflate,
    Brotli,
    Zstd,
    /// Something we cannot read; the body is left out of the index.
    Unknown,
}

impl Encoding {
    /// From a `Content-Encoding` header. More than one, as in `gzip, br`, is
    /// legal and almost never seen, and counts as unknown.
    pub fn from_header(value: Option<&str>) -> Self {
        match value.map(|v| v.trim().to_ascii_lowercase()).as_deref() {
            None | Some("") | Some("identity") => Self::Identity,
            Some("gzip") | Some("x-gzip") => Self::Gzip,
            Some("deflate") => Self::Deflate,
            Some("br") => Self::Brotli,
            Some("zstd") => Self::Zstd,
            Some(_) => Self::Unknown,
        }
    }
}

/// At most `max` bytes of what `data` unpacks to. `None` if it cannot be read
/// at all; a part is better than nothing, so an error part way is not one.
pub fn unpack(data: &[u8], encoding: Encoding, max: usize) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    unpack_into(data, encoding, max as u64, &mut out)?;
    Some(out)
}

/// The same for a body too big for memory: at most `max` bytes written to `out`,
/// and how many that came to.
pub fn unpack_into(
    source: impl Read,
    encoding: Encoding,
    max: u64,
    out: &mut impl Write,
) -> Option<u64> {
    let mut source = BufReader::new(source);
    let reader: Box<dyn Read + '_> = match encoding {
        Encoding::Identity => Box::new(source),
        Encoding::Gzip => Box::new(flate2::bufread::MultiGzDecoder::new(source)),
        Encoding::Deflate => {
            // "deflate" on the web means zlib, or raw deflate from servers that
            // read the standard differently. A zlib stream shows in its header.
            let head = source.fill_buf().ok()?;
            let zlib = head.len() >= 2
                && head[0] & 0x0f == 8
                && u16::from_be_bytes([head[0], head[1]]).is_multiple_of(31);
            if zlib {
                Box::new(flate2::bufread::ZlibDecoder::new(source))
            } else {
                Box::new(flate2::bufread::DeflateDecoder::new(source))
            }
        }
        Encoding::Brotli => Box::new(brotli::Decompressor::new(source, 4096)),
        Encoding::Zstd => Box::new(zstd::stream::read::Decoder::with_buffer(source).ok()?),
        Encoding::Unknown => return None,
    };
    let mut limited = reader.take(max);
    let mut chunk = vec![0u8; 64 * 1024];
    let mut written = 0;
    loop {
        match limited.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                out.write_all(&chunk[..n]).ok()?;
                written += n as u64;
            }
            Err(_) if written > 0 => break,
            Err(_) => return None,
        }
    }
    Some(written)
}

/// Whether a content type is worth a place in the index.
pub fn is_text(content_type: Option<&str>) -> bool {
    let Some(content_type) = content_type else {
        return false;
    };
    let essence = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    essence.starts_with("text/")
        || essence.ends_with("+json")
        || essence.ends_with("+xml")
        || matches!(
            essence.as_str(),
            "application/json"
                | "application/javascript"
                | "application/xml"
                | "application/x-www-form-urlencoded"
                | "application/graphql"
                | "application/x-ndjson"
        )
}

/// Text from bytes that may stop inside a character or not be text at all.
pub fn lossy_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gzip(data: &[u8]) -> Vec<u8> {
        let mut out = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::new(6));
        out.write_all(data).unwrap();
        out.finish().unwrap()
    }

    fn brotli_of(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        brotli::CompressorReader::new(data, 4096, 5, 22)
            .read_to_end(&mut out)
            .unwrap();
        out
    }

    const TEXT: &[u8] = b"{\"name\":\"\xc4\x91\xe1\xba\xb7t h\xc3\xa0ng\"}";

    #[test]
    fn every_encoding_comes_back() {
        let zlib = {
            let mut out = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(6));
            out.write_all(TEXT).unwrap();
            out.finish().unwrap()
        };
        let raw = {
            let mut out =
                flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::new(6));
            out.write_all(TEXT).unwrap();
            out.finish().unwrap()
        };
        for (encoding, packed) in [
            (Encoding::Identity, TEXT.to_vec()),
            (Encoding::Gzip, gzip(TEXT)),
            (Encoding::Deflate, zlib),
            (Encoding::Deflate, raw),
            (Encoding::Brotli, brotli_of(TEXT)),
            (Encoding::Zstd, zstd::encode_all(TEXT, 1).unwrap()),
        ] {
            assert_eq!(
                unpack(&packed, encoding, 1024).unwrap(),
                TEXT,
                "{encoding:?}"
            );
        }
    }

    #[test]
    fn a_bomb_stops_at_the_limit() {
        let bomb = vec![0u8; 512 * 1024 * 1024];
        for (encoding, packed) in [
            (Encoding::Gzip, gzip(&bomb)),
            (Encoding::Zstd, zstd::encode_all(&bomb[..], 1).unwrap()),
        ] {
            assert!(packed.len() < 1024 * 1024, "{encoding:?}: {}", packed.len());
            let out = unpack(&packed, encoding, 64 * 1024).unwrap();
            assert_eq!(out.len(), 64 * 1024, "{encoding:?}");
        }
    }

    #[test]
    fn a_body_cut_short_gives_what_it_has() {
        let long: Vec<u8> = (0..200_000u32).flat_map(|i| i.to_le_bytes()).collect();
        for (encoding, packed) in [
            (Encoding::Gzip, gzip(&long)),
            (Encoding::Brotli, brotli_of(&long)),
            (Encoding::Zstd, zstd::encode_all(&long[..], 1).unwrap()),
        ] {
            let cut = &packed[..packed.len() / 2];
            let out = unpack(cut, encoding, usize::MAX).unwrap();
            assert!(!out.is_empty(), "{encoding:?}");
            assert_eq!(out[..], long[..out.len()], "{encoding:?}");
        }
    }

    #[test]
    fn rubbish_is_no_text_and_no_crash() {
        for encoding in [
            Encoding::Gzip,
            Encoding::Brotli,
            Encoding::Zstd,
            Encoding::Deflate,
        ] {
            let _ = unpack(b"this is not packed at all", encoding, 1024);
            let _ = unpack(b"", encoding, 1024);
        }
        assert_eq!(unpack(b"x", Encoding::Unknown, 1024), None);
    }

    #[test]
    fn headers_name_their_encodings() {
        assert_eq!(Encoding::from_header(None), Encoding::Identity);
        assert_eq!(Encoding::from_header(Some(" GZIP ")), Encoding::Gzip);
        assert_eq!(Encoding::from_header(Some("br")), Encoding::Brotli);
        assert_eq!(Encoding::from_header(Some("gzip, br")), Encoding::Unknown);
    }

    #[test]
    fn only_text_goes_in_the_index() {
        for text in [
            "application/json; charset=utf-8",
            "text/html",
            "application/problem+json",
            "APPLICATION/XML",
        ] {
            assert!(is_text(Some(text)), "{text}");
        }
        for other in ["image/webp", "font/woff2", "application/octet-stream"] {
            assert!(!is_text(Some(other)), "{other}");
        }
        assert!(!is_text(None));
    }
}
