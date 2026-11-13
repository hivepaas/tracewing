//! Bytes that are a message rather than text, read with no schema to say what
//! the fields are called.

use crate::{Confidence, Input, MAX_INPUT, Reading, Shown};
use tracewing_i18n::tr;

/// One field off the wire: its number, how it was written, and what it held.
struct Field {
    number: u64,
    wire: u8,
    said: String,
}

/// A varint, and where it ends.
fn varint(bytes: &[u8], at: usize) -> Option<(u64, usize)> {
    let mut value = 0u64;
    let mut shift = 0;
    let mut at = at;
    loop {
        let byte = *bytes.get(at)?;
        // Ten bytes is the most a 64-bit varint takes.
        if shift > 63 {
            return None;
        }
        value |= u64::from(byte & 0x7f) << shift;
        at += 1;
        if byte & 0x80 == 0 {
            return Some((value, at));
        }
        shift += 7;
    }
}

/// Protobuf's own wire format, as far as it can be read with no `.proto`:
/// which field numbers are there, how each was written, and what it holds.
/// gRPC's five-byte frame is taken off first, where there is one.
/// Whether these bytes read as a protobuf message, every byte of them. For
/// telling that a value holds one where nothing at the front says so: a
/// message has no magic number, so the only test is whether it parses.
pub(crate) fn reads_as_protobuf(bytes: &[u8]) -> bool {
    if !(2..=MAX_INPUT).contains(&bytes.len()) || std::str::from_utf8(bytes).is_ok() {
        return false;
    }
    read_fields(bytes).is_some_and(|fields| !fields.is_empty() && fields.len() <= 256)
}

pub fn protobuf(input: &Input) -> Option<Reading> {
    let mut raw = input.raw();
    if !(2..=MAX_INPUT).contains(&raw.len()) {
        return None;
    }
    // Text is text, whatever else it could be read as.
    if std::str::from_utf8(raw).is_ok() {
        return None;
    }
    let mut framed = false;
    // A gRPC message: one byte saying whether it is packed, then its length.
    if raw[0] == 0 && raw.len() > 5 {
        let length = u32::from_be_bytes([raw[1], raw[2], raw[3], raw[4]]) as usize;
        if length == raw.len() - 5 {
            raw = &raw[5..];
            framed = true;
        }
    }
    let fields = read_fields(raw)?;
    if fields.is_empty() || fields.len() > 256 {
        return None;
    }
    let said = fields
        .iter()
        .map(|field| {
            (
                tr!(
                    "Field {number}, {wire}",
                    number = field.number,
                    wire = wire_name(field.wire)
                ),
                field.said.clone(),
            )
        })
        .collect();
    let label = if framed {
        tr!("gRPC, with no .proto to name its fields")
    } else {
        tr!("Protobuf, with no .proto to name its fields")
    };
    let confidence = if framed || input.field_says(&["proto", "grpc", "pb"]) {
        Confidence::Likely
    } else {
        Confidence::Possible
    };
    Some(Reading::new(
        "protobuf",
        label,
        confidence,
        Shown::Fields(said),
    ))
}

fn wire_name(wire: u8) -> String {
    match wire {
        0 => tr!("a number"),
        1 => tr!("eight bytes"),
        2 => tr!("bytes"),
        5 => tr!("four bytes"),
        _ => tr!("unknown"),
    }
}

/// Every field of a message, or nothing at all: a run of bytes that does not
/// read to its very end is not a message.
fn read_fields(bytes: &[u8]) -> Option<Vec<Field>> {
    let mut fields = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let (key, next) = varint(bytes, at)?;
        let (number, wire) = (key >> 3, (key & 7) as u8);
        // Field numbers start at one, and junk read as protobuf tends to give
        // enormous ones.
        if number == 0 || number > 100_000 {
            return None;
        }
        at = next;
        let said = match wire {
            0 => {
                let (value, next) = varint(bytes, at)?;
                at = next;
                value.to_string()
            }
            1 => {
                let eight = bytes.get(at..at + 8)?;
                at += 8;
                format!("0x{}", hex_of(eight))
            }
            2 => {
                let (length, next) = varint(bytes, at)?;
                let length = usize::try_from(length).ok()?;
                let held = bytes.get(next..next + length)?;
                at = next + length;
                said_of(held)
            }
            5 => {
                let four = bytes.get(at..at + 4)?;
                at += 4;
                format!("0x{}", hex_of(four))
            }
            // Groups: gone from the language since 2001.
            _ => return None,
        };
        fields.push(Field { number, wire, said });
    }
    Some(fields)
}

fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// What a length-delimited field holds: text where it is text, a message
/// where it reads as one, and otherwise its size.
fn said_of(held: &[u8]) -> String {
    if let Ok(text) = std::str::from_utf8(held)
        && !text.is_empty()
        && text.chars().all(|c| !c.is_control() || c == '\n')
    {
        return format!("\"{}\"", crate::cut(text, 48));
    }
    match read_fields(held) {
        Some(inside) if !inside.is_empty() => tr_fields(inside.len()),
        _ => crate::numbers::bytes_size(held.len() as u64),
    }
}

fn tr_fields(n: usize) -> String {
    tracewing_i18n::tr_n!(
        n as i64,
        "a message of {n} field",
        "a message of {n} fields"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Input;
    use crate::tests::{by_id, read};

    /// A message: field 1 the number 150, field 2 the text "hi".
    fn message() -> Vec<u8> {
        vec![0x08, 0x96, 0x01, 0x12, 0x02, b'h', b'i']
    }

    /// A message has no magic number, so base64 of one used to read as
    /// nothing: the step that would have parsed it was never reached.
    #[test]
    fn protobuf_inside_base64_is_reached() {
        use base64::Engine as _;
        let mut bytes = message();
        // Long enough that base64 of it is worth looking at at all.
        bytes.extend_from_slice(&[0x1a, 0x06, b'A', b'-', b'4', b'4', b'7', b'1']);
        let travelling = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let found = read(&travelling);
        let reading = by_id(&found, "protobuf").expect("the message in it");
        assert_eq!(reading.chain, vec!["base64", "protobuf"]);
    }

    fn fields_of(bytes: &[u8]) -> Vec<(String, String)> {
        let input = Input::new("", crate::tests::NOW, crate::tests::TZ).with_bytes(bytes);
        match crate::readings(&input)
            .into_iter()
            .find(|reading| reading.id == "protobuf")
            .map(|reading| reading.shown)
        {
            Some(Shown::Fields(fields)) => fields,
            other => panic!("no protobuf reading: {other:?}"),
        }
    }

    #[test]
    fn a_message_is_read_field_by_field() {
        let found = fields_of(&message());
        assert_eq!(found.len(), 2);
        assert!(found[0].0.contains('1') && found[0].0.contains("number"));
        assert_eq!(found[0].1, "150");
        assert_eq!(found[1].1, "\"hi\"");
    }

    #[test]
    fn a_grpc_frame_is_taken_off_first() {
        let mut framed = vec![0];
        framed.extend_from_slice(&(message().len() as u32).to_be_bytes());
        framed.extend_from_slice(&message());
        let input = Input::new("", crate::tests::NOW, crate::tests::TZ).with_bytes(&framed);
        let reading = crate::readings(&input)
            .into_iter()
            .find(|reading| reading.id == "protobuf")
            .unwrap();
        assert!(reading.label.starts_with("gRPC"), "{}", reading.label);
        assert_eq!(reading.confidence, Confidence::Likely);
    }

    #[test]
    fn a_message_inside_a_message_is_counted() {
        let inner = message();
        let mut outer = vec![0x1a, inner.len() as u8];
        outer.extend_from_slice(&inner);
        let found = fields_of(&outer);
        assert!(found[0].1.contains('2'), "{}", found[0].1);
    }

    #[test]
    fn what_does_not_read_to_the_end_is_not_a_message() {
        // One byte short of what the length says.
        assert!(read_fields(&[0x12, 0x05, b'h', b'i']).is_none());
        // Text is read as text, not as a message.
        assert!(by_id(&read("hello there"), "protobuf").is_none());
        // Noise with an enormous field number.
        assert!(read_fields(&[0xff, 0xff, 0xff, 0x7f, 0x01]).is_none());
    }
}
