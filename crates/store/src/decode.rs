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
                | "application/ndjson"
                | "application/jsonl"
                | "application/x-jsonl"
                | "application/jsonlines"
                | "application/x-jsonlines"
                | "application/json-seq"
                | "application/stream+json"
                | "application/x-json-stream"
        )
}

/// Text from bytes that may stop inside a character or not be text at all.
/// `%41` as `A` and `+` as a space, as in a query or a form. Bytes that are not
/// UTF-8 come out as replacement characters.
pub fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| (b as char).to_digit(16);
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => match (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                (Some(high), Some(low)) => {
                    out.push((high * 16 + low) as u8);
                    i += 2;
                }
                _ => out.push(b'%'),
            },
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The `name=value` pairs of a query or a form, decoded.
pub fn form_pairs(text: &str) -> Vec<(String, String)> {
    text.split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            (percent_decode(name), percent_decode(value))
        })
        .collect()
}

pub fn lossy_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// What a packed stream unpacks to, a frame at a time, for a stream that is
/// read as it comes: its frames are cut wherever the server flushed them.
/// It stops for good at an error, and past `max` bytes in all, which a few
/// KB packed can come to.
pub struct Unpacker {
    state: UnpackState,
    max: u64,
    out: u64,
}

enum UnpackState {
    /// "deflate" is zlib or raw, which only the first two bytes tell.
    Deflate(Vec<u8>),
    Gzip(flate2::write::MultiGzDecoder<Vec<u8>>),
    Zlib(flate2::write::ZlibDecoder<Vec<u8>>),
    Raw(flate2::write::DeflateDecoder<Vec<u8>>),
    Brotli(Box<brotli::DecompressorWriter<Vec<u8>>>),
    Zstd(zstd::stream::write::Decoder<'static, Vec<u8>>),
    Stopped,
}

impl Unpacker {
    /// `None` for what is not packed, or packed in a way it cannot read.
    pub fn new(encoding: Encoding, max: u64) -> Option<Self> {
        let state = match encoding {
            Encoding::Gzip => UnpackState::Gzip(flate2::write::MultiGzDecoder::new(Vec::new())),
            Encoding::Deflate => UnpackState::Deflate(Vec::new()),
            Encoding::Brotli => {
                UnpackState::Brotli(Box::new(brotli::DecompressorWriter::new(Vec::new(), 4096)))
            }
            Encoding::Zstd => {
                UnpackState::Zstd(zstd::stream::write::Decoder::new(Vec::new()).ok()?)
            }
            Encoding::Identity | Encoding::Unknown => return None,
        };
        Some(Self { state, max, out: 0 })
    }

    /// What `data` unpacks to, as far as it goes.
    pub fn feed(&mut self, data: &[u8]) -> Vec<u8> {
        if let UnpackState::Deflate(head) = &mut self.state {
            head.extend_from_slice(data);
            if head.len() < 2 {
                return Vec::new();
            }
            let head = std::mem::take(head);
            let zlib =
                head[0] & 0x0f == 8 && u16::from_be_bytes([head[0], head[1]]).is_multiple_of(31);
            self.state = if zlib {
                UnpackState::Zlib(flate2::write::ZlibDecoder::new(Vec::new()))
            } else {
                UnpackState::Raw(flate2::write::DeflateDecoder::new(Vec::new()))
            };
            return self.feed(&head);
        }
        let written = match &mut self.state {
            UnpackState::Gzip(d) => d.write_all(data).and_then(|()| d.flush()),
            UnpackState::Zlib(d) => d.write_all(data).and_then(|()| d.flush()),
            UnpackState::Raw(d) => d.write_all(data).and_then(|()| d.flush()),
            UnpackState::Brotli(d) => d.write_all(data).and_then(|()| d.flush()),
            UnpackState::Zstd(d) => d.write_all(data).and_then(|()| d.flush()),
            UnpackState::Deflate(_) | UnpackState::Stopped => return Vec::new(),
        };
        let out = self.take();
        if written.is_err() {
            self.state = UnpackState::Stopped;
        }
        out
    }

    /// What is unpacked so far and not yet given, within `max`.
    fn take(&mut self) -> Vec<u8> {
        let buffer = match &mut self.state {
            UnpackState::Gzip(d) => d.get_mut(),
            UnpackState::Zlib(d) => d.get_mut(),
            UnpackState::Raw(d) => d.get_mut(),
            UnpackState::Brotli(d) => d.get_mut(),
            UnpackState::Zstd(d) => d.get_mut(),
            UnpackState::Deflate(_) | UnpackState::Stopped => return Vec::new(),
        };
        let mut out = std::mem::take(buffer);
        let room = self.max.saturating_sub(self.out);
        if out.len() as u64 >= room {
            out.truncate(room as usize);
            self.state = UnpackState::Stopped;
        }
        self.out += out.len() as u64;
        out
    }
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
    fn a_stream_unpacks_as_it_comes_wherever_it_is_cut() {
        let zlib = {
            let mut out = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(6));
            out.write_all(TEXT).unwrap();
            out.finish().unwrap()
        };
        for (encoding, packed) in [
            (Encoding::Gzip, gzip(TEXT)),
            (Encoding::Deflate, zlib),
            (Encoding::Brotli, brotli_of(TEXT)),
            (Encoding::Zstd, zstd::encode_all(TEXT, 1).unwrap()),
        ] {
            for cut in [1, 7, 4096] {
                let mut unpacker = Unpacker::new(encoding, 1 << 20).unwrap();
                let mut out = Vec::new();
                for frame in packed.chunks(cut) {
                    out.extend(unpacker.feed(frame));
                }
                assert_eq!(out, TEXT, "{encoding:?} cut every {cut}");
            }
        }
        assert!(Unpacker::new(Encoding::Identity, 1).is_none());
    }

    #[test]
    fn a_stream_flushed_part_way_gives_what_came_before() {
        let mut packer = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::new(6));
        packer.write_all(b"data: one\n\n").unwrap();
        packer.flush().unwrap();
        let first = std::mem::take(packer.get_mut());
        let mut unpacker = Unpacker::new(Encoding::Gzip, 1 << 20).unwrap();
        assert_eq!(
            unpacker.feed(&first),
            b"data: one\n\n",
            "before the stream ends"
        );
    }

    #[test]
    fn a_packed_stream_stops_at_its_limit_and_at_an_error() {
        let bomb = gzip(&vec![0u8; 4 << 20]);
        let mut unpacker = Unpacker::new(Encoding::Gzip, 1000).unwrap();
        let out: usize = bomb.chunks(512).map(|f| unpacker.feed(f).len()).sum();
        assert_eq!(out, 1000);
        let mut broken = Unpacker::new(Encoding::Gzip, 1000).unwrap();
        assert!(broken.feed(b"not gzip at all").is_empty());
        assert!(broken.feed(&gzip(b"x")).is_empty(), "stopped for good");
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
