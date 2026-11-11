//! Protocol Buffers, and gRPC's frames of them, read with no `.proto`: each
//! field is named by its number, and what its bytes are is guessed from how
//! they are sent. A field sent more than once is a list.
//!
//! - A varint is a number; past `i64::MAX` it is taken as a negative one.
//! - Eight or four fixed bytes are a float if they read as a likely one, else
//!   a number.
//! - Bytes with a length are text if they are printable UTF-8, else a message
//!   if they read as one, else bytes.

use serde_json::Value;
use std::io::Read;

/// How deep messages in messages are read; deeper, they are bytes.
const DEPTH_MAX: u32 = 64;

/// The largest field number there is.
const FIELD_MAX: u64 = (1 << 29) - 1;

/// A message: its fields by number, each with all it was sent.
type Fields = Vec<(u64, Vec<Field>)>;

enum Field {
    Value(Value),
    Message(Fields),
}

/// One message as JSON, or `None` if it cannot be read as one.
pub fn message_to_json(data: &[u8]) -> Option<String> {
    let fields = message(data, 0)?;
    let mut out = String::new();
    write_fields(&fields, &mut out);
    Some(out)
}

/// gRPC frames as JSON: one message is itself, more are a list. Trailers sent
/// in the body, as gRPC-Web does, are an object under `$trailers`. `None` if
/// not even the first frame can be read. With `text`, the frames are base64,
/// as `grpc-web-text` sends them.
pub fn grpc_to_json(data: &[u8], text: bool) -> Option<String> {
    let decoded;
    let data = if text {
        decoded = from_base64(data)?;
        &decoded[..]
    } else {
        data
    };
    let mut parts = Vec::new();
    let mut rest = data;
    while !rest.is_empty() {
        let Some((flags, payload, after)) = frame(rest) else {
            if parts.is_empty() {
                return None;
            }
            parts.push(json_string(&format!(
                "<{} bytes after this could not be read>",
                rest.len()
            )));
            break;
        };
        parts.push(frame_json(flags, payload));
        rest = after;
    }
    match parts.len() {
        0 => None,
        1 => parts.pop(),
        _ => Some(format!("[{}]", parts.join(","))),
    }
}

/// The `grpc-status` in gRPC-Web's frame of trailers, if the body holds one.
pub fn grpc_web_status(data: &[u8], text: bool) -> Option<u16> {
    let decoded;
    let mut rest = if text {
        decoded = from_base64(data)?;
        &decoded[..]
    } else {
        data
    };
    while let Some((flags, payload, after)) = frame(rest) {
        if flags & 0x80 != 0 {
            return trailer_lines(payload)
                .find(|(name, _)| name == "grpc-status")
                .and_then(|(_, value)| value.parse().ok());
        }
        rest = after;
    }
    None
}

/// Trailers sent as a frame: header lines, names in lower case.
fn trailer_lines(payload: &[u8]) -> impl Iterator<Item = (String, String)> + '_ {
    std::str::from_utf8(payload)
        .unwrap_or_default()
        .split("\r\n")
        .filter(|line| !line.is_empty())
        .map(|line| {
            let (name, value) = line.split_once(':').unwrap_or((line, ""));
            (name.trim().to_ascii_lowercase(), value.trim().to_owned())
        })
}

/// A frame's flags, its payload, and what is after it.
fn frame(data: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    let (head, rest) = data.split_at_checked(5)?;
    let len = u32::from_be_bytes(head[1..5].try_into().expect("four bytes")) as usize;
    let (payload, after) = rest.split_at_checked(len)?;
    Some((head[0], payload, after))
}

fn frame_json(flags: u8, payload: &[u8]) -> String {
    // Trailers, as header lines.
    if flags & 0x80 != 0 {
        let trailers: serde_json::Map<_, _> = trailer_lines(payload)
            .map(|(name, value)| (name, Value::String(value)))
            .collect();
        let mut out = serde_json::Map::new();
        out.insert("$trailers".to_owned(), Value::Object(trailers));
        return Value::Object(out).to_string();
    }
    let unpacked;
    let payload = if flags & 1 != 0 {
        match unpack(payload) {
            Some(data) => {
                unpacked = data;
                &unpacked[..]
            }
            None => {
                let mut out = serde_json::Map::new();
                out.insert("$compressed".to_owned(), crate::binjson::bytes(payload));
                return Value::Object(out).to_string();
            }
        }
    } else {
        payload
    };
    message_to_json(payload).unwrap_or_else(|| crate::binjson::bytes(payload).to_string())
}

/// A compressed message, unpacked by what its first bytes say it is.
fn unpack(data: &[u8]) -> Option<Vec<u8>> {
    let max = crate::binjson::DECODE_MAX;
    let mut out = Vec::new();
    let read = match data {
        [0x1f, 0x8b, ..] => flate2::read::GzDecoder::new(data)
            .take(max)
            .read_to_end(&mut out),
        [0x28, 0xb5, 0x2f, 0xfd, ..] => zstd::stream::read::Decoder::new(data)
            .ok()?
            .take(max)
            .read_to_end(&mut out),
        [0x78, ..] => flate2::read::ZlibDecoder::new(data)
            .take(max)
            .read_to_end(&mut out),
        _ => return None,
    };
    read.ok().map(|_| out)
}

/// Base64 sent in pieces, each with its own padding.
fn from_base64(data: &[u8]) -> Option<Vec<u8>> {
    use base64::Engine;
    let clean: Vec<u8> = data
        .iter()
        .copied()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();
    let mut out = Vec::new();
    let mut start = 0;
    for (i, quad) in clean.chunks(4).enumerate() {
        let end = i * 4 + quad.len();
        if quad.contains(&b'=') || end == clean.len() {
            base64::engine::general_purpose::STANDARD
                .decode_vec(&clean[start..end], &mut out)
                .ok()?;
            start = end;
        }
    }
    Some(out)
}

/// Reads a whole message, or `None` if its bytes are not one.
fn message(data: &[u8], depth: u32) -> Option<Fields> {
    let mut cursor = data;
    let fields = fields(&mut cursor, depth, None)?;
    Some(fields)
}

/// Fields up to the end of `data`, or up to the end of the group numbered
/// `group`.
fn fields(data: &mut &[u8], depth: u32, group: Option<u64>) -> Option<Fields> {
    let mut fields: Fields = Vec::new();
    loop {
        if data.is_empty() {
            // A group must end with its own end.
            return group.is_none().then(|| sorted(fields));
        }
        let tag = varint(data)?;
        let number = tag >> 3;
        if number == 0 || number > FIELD_MAX {
            return None;
        }
        let field = match tag & 7 {
            0 => Field::Value(integer(varint(data)?)),
            1 => Field::Value(fixed64(u64::from_le_bytes(take(data, 8)?.try_into().ok()?))),
            2 => {
                let len = usize::try_from(varint(data)?).ok()?;
                length_delimited(take(data, len)?, depth)
            }
            3 if depth < DEPTH_MAX => Field::Message(fields_in_group(data, depth, number)?),
            4 => return (group == Some(number)).then(|| sorted(fields)),
            5 => Field::Value(fixed32(u32::from_le_bytes(take(data, 4)?.try_into().ok()?))),
            _ => return None,
        };
        match fields.iter_mut().find(|(n, _)| *n == number) {
            Some((_, sent)) => sent.push(field),
            None => fields.push((number, vec![field])),
        }
    }
}

fn fields_in_group(data: &mut &[u8], depth: u32, number: u64) -> Option<Fields> {
    fields(data, depth + 1, Some(number))
}

/// Fields in number order; what one field was sent keeps its order.
fn sorted(mut fields: Fields) -> Fields {
    fields.sort_by_key(|(number, _)| *number);
    fields
}

fn take<'a>(data: &mut &'a [u8], len: usize) -> Option<&'a [u8]> {
    let (taken, rest) = data.split_at_checked(len)?;
    *data = rest;
    Some(taken)
}

fn varint(data: &mut &[u8]) -> Option<u64> {
    let mut value = 0u64;
    for (i, &byte) in data.iter().enumerate().take(10) {
        value |= u64::from(byte & 0x7f) << (7 * i);
        if byte & 0x80 == 0 {
            *data = &data[i + 1..];
            return Some(value);
        }
    }
    None
}

/// Negative `int32` and `int64` are sent as ten-byte varints.
fn integer(n: u64) -> Value {
    if n > i64::MAX as u64 {
        Value::from(n as i64)
    } else {
        Value::from(n)
    }
}

fn likely_float(f: f64, max: f64) -> bool {
    f.is_normal() && (1e-6..=max).contains(&f.abs())
}

fn fixed64(n: u64) -> Value {
    let f = f64::from_bits(n);
    if likely_float(f, 1e15) {
        serde_json::Number::from_f64(f).map_or(Value::from(n), Value::Number)
    } else {
        Value::from(n)
    }
}

fn fixed32(n: u32) -> Value {
    let f = f32::from_bits(n);
    if likely_float(f64::from(f), 1e9) {
        // As short as the float is written, not as the double it widens to.
        f.to_string()
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map_or(Value::from(n), Value::Number)
    } else {
        Value::from(n)
    }
}

fn length_delimited(data: &[u8], depth: u32) -> Field {
    let text = std::str::from_utf8(data).ok();
    if let Some(text) = text
        && text
            .chars()
            .all(|c| !c.is_control() || matches!(c, '\t' | '\n' | '\r'))
    {
        return Field::Value(Value::String(text.to_owned()));
    }
    if depth < DEPTH_MAX
        && let Some(fields) = message(data, depth + 1)
    {
        return Field::Message(fields);
    }
    Field::Value(match text {
        Some(text) => Value::String(text.to_owned()),
        None => crate::binjson::bytes(data),
    })
}

fn json_string(text: &str) -> String {
    Value::String(text.to_owned()).to_string()
}

fn write_fields(fields: &Fields, out: &mut String) {
    out.push('{');
    for (i, (number, sent)) in fields.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('"');
        out.push_str(&number.to_string());
        out.push_str("\":");
        if let [one] = &sent[..] {
            write_field(one, out);
        } else {
            out.push('[');
            for (i, field) in sent.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_field(field, out);
            }
            out.push(']');
        }
    }
    out.push('}');
}

fn write_field(field: &Field, out: &mut String) {
    match field {
        Field::Value(value) => out.push_str(&value.to_string()),
        Field::Message(fields) => write_fields(fields, out),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    fn tag(out: &mut Vec<u8>, number: u64, wire: u64) {
        put_varint(out, number << 3 | wire);
    }

    fn put_varint(out: &mut Vec<u8>, mut n: u64) {
        while n >= 0x80 {
            out.push(n as u8 | 0x80);
            n >>= 7;
        }
        out.push(n as u8);
    }

    fn put_bytes(out: &mut Vec<u8>, number: u64, data: &[u8]) {
        tag(out, number, 2);
        put_varint(out, data.len() as u64);
        out.extend_from_slice(data);
    }

    fn item(sku: &str, qty: u64) -> Vec<u8> {
        let mut out = Vec::new();
        put_bytes(&mut out, 1, sku.as_bytes());
        tag(&mut out, 2, 0);
        put_varint(&mut out, qty);
        out
    }

    fn order() -> Vec<u8> {
        let mut out = Vec::new();
        // Sent out of order: read in number order.
        put_bytes(&mut out, 2, "Nguyễn Văn A".as_bytes());
        tag(&mut out, 1, 0);
        put_varint(&mut out, 42);
        put_bytes(&mut out, 4, &item("A-1", 2));
        put_bytes(&mut out, 4, &item("B-7", 1));
        tag(&mut out, 5, 1);
        out.extend_from_slice(&12.5f64.to_le_bytes());
        tag(&mut out, 6, 5);
        out.extend_from_slice(&0.1f32.to_le_bytes());
        put_bytes(&mut out, 7, &[0xca, 0xfe, 0xba, 0xbe]);
        tag(&mut out, 8, 0);
        put_varint(&mut out, -3i64 as u64);
        tag(&mut out, 9, 1);
        out.extend_from_slice(&1_700_000_000_000u64.to_le_bytes());
        put_bytes(&mut out, 10, b"");
        out
    }

    #[test]
    fn a_message_reads_by_field_number() {
        let read = message_to_json(&order()).unwrap();
        assert!(read.starts_with(r#"{"1":42,"2":"#), "{read}");
        assert_eq!(
            json(&read),
            json(
                r#"{"1":42,"2":"Nguyễn Văn A","4":[{"1":"A-1","2":2},{"1":"B-7","2":1}],
                    "5":12.5,"6":0.1,"7":"<4 bytes: cafebabe>","8":-3,
                    "9":1700000000000,"10":""}"#
            )
        );
    }

    #[test]
    fn what_is_not_a_message_is_none() {
        assert_eq!(message_to_json(b"\x08"), None, "a varint cut short");
        assert_eq!(message_to_json(b"\x00\x01"), None, "field zero");
        assert_eq!(message_to_json(b"\x0f"), None, "wire type 7");
        assert_eq!(message_to_json(b"\x12\x05ab"), None, "bytes cut short");
        assert_eq!(
            message_to_json(b"\x0b\x08\x01"),
            None,
            "a group with no end"
        );
    }

    #[test]
    fn groups_are_messages() {
        // Field 1 a group holding field 2 = 7.
        let read = message_to_json(b"\x0b\x10\x07\x0c\x18\x01").unwrap();
        assert_eq!(json(&read), json(r#"{"1":{"2":7},"3":1}"#));
    }

    fn framed(flags: u8, data: &[u8]) -> Vec<u8> {
        let mut out = vec![flags];
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(data);
        out
    }

    #[test]
    fn grpc_frames_are_messages_and_trailers() {
        let one = framed(0, &item("A-1", 2));
        assert_eq!(
            json(&grpc_to_json(&one, false).unwrap()),
            json(r#"{"1":"A-1","2":2}"#)
        );

        let mut gzipped = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut gzipped, &item("B-7", 1)).unwrap();
        let mut body = one.clone();
        body.extend(framed(1, &gzipped.finish().unwrap()));
        body.extend(framed(1, b"\x01\x02"));
        body.extend(framed(0, b""));
        body.extend(framed(
            0x80,
            b"grpc-status: 0\r\nGrpc-Message: all good\r\n",
        ));
        body.extend_from_slice(b"\x00\x00");
        assert_eq!(
            json(&grpc_to_json(&body, false).unwrap()),
            json(
                r#"[{"1":"A-1","2":2},{"1":"B-7","2":1},
                    {"$compressed":"<2 bytes: 0102>"},{},
                    {"$trailers":{"grpc-status":"0","grpc-message":"all good"}},
                    "<2 bytes after this could not be read>"]"#
            )
        );
        assert_eq!(grpc_to_json(b"\x00\x00\x00\x00\x09ab", false), None);
        assert_eq!(grpc_web_status(&body, false), Some(0));
        assert_eq!(grpc_web_status(&one, false), None);
        let failed = [one, framed(0x80, b"grpc-status: 5\r\n")].concat();
        assert_eq!(grpc_web_status(&failed, false), Some(5));
    }

    #[test]
    fn grpc_web_text_is_base64_in_pieces() {
        use base64::Engine;
        let b64 = |data: &[u8]| base64::engine::general_purpose::STANDARD.encode(data);
        let mut body = b64(&framed(0, &item("A-12", 2)));
        body.push_str(&b64(&framed(0x80, b"grpc-status: 0\r\n")));
        assert!(body[..body.len() - 1].contains('='), "padding mid-way");
        assert_eq!(
            json(&grpc_to_json(body.as_bytes(), true).unwrap()),
            json(r#"[{"1":"A-12","2":2},{"$trailers":{"grpc-status":"0"}}]"#)
        );
        assert_eq!(grpc_to_json(b"!!!!", true), None);
    }
}
