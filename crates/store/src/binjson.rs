//! MessagePack and CBOR, which hold JSON's values as bytes, turned into JSON
//! to be read, folded and searched as JSON is. What JSON has no word for, such
//! as bytes, a CBOR tag or an integer past 64 bits, is said in a string or an
//! object that tells what it was. Protocol Buffers and gRPC are read the same
//! way, by [`crate::protobuf`].

use serde_json::{Map, Number, Value};
use std::fmt::Write;

/// Past this, a body is not turned into JSON: all of it is held in memory as
/// it is, and as JSON.
pub const DECODE_MAX: u64 = 64 << 20;

/// How many bytes of a byte string are shown, in hex.
const BYTES_SHOWN: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    MessagePack,
    Cbor,
    Protobuf,
    /// Protobuf messages in gRPC's frames, as gRPC and gRPC-Web send them.
    Grpc,
    GrpcWeb,
    /// gRPC-Web's frames in base64.
    GrpcWebText,
}

impl Format {
    /// What it is called.
    pub fn name(self) -> &'static str {
        match self {
            Self::MessagePack => "MessagePack",
            Self::Cbor => "CBOR",
            Self::Protobuf => "Protobuf",
            Self::Grpc => "gRPC",
            Self::GrpcWeb | Self::GrpcWebText => "gRPC-Web",
        }
    }

    /// Whether its fields are named by number, with no schema to name them.
    pub fn numbered(self) -> bool {
        !matches!(self, Self::MessagePack | Self::Cbor)
    }
}

/// The format a body's type names, if it is one of these.
pub fn format_of(content_type: Option<&str>) -> Option<Format> {
    let essence = content_type?.split(';').next()?.trim().to_ascii_lowercase();
    let subtype = essence.split('/').nth(1)?;
    // gRPC names what its messages are after a `+`: only Protobuf is read.
    let (base, suffix) = subtype.split_once('+').unwrap_or((subtype, "proto"));
    let grpc = match base {
        "grpc" => Some(Format::Grpc),
        "grpc-web" => Some(Format::GrpcWeb),
        "grpc-web-text" => Some(Format::GrpcWebText),
        _ => None,
    };
    if let Some(grpc) = grpc {
        return (suffix == "proto").then_some(grpc);
    }
    let name = subtype.rsplit('+').next().unwrap_or(subtype);
    match name {
        "msgpack" | "x-msgpack" | "vnd.msgpack" => Some(Format::MessagePack),
        "cbor" | "x-cbor" => Some(Format::Cbor),
        "protobuf" | "x-protobuf" | "vnd.google.protobuf" | "x-google-protobuf" | "proto" => {
            Some(Format::Protobuf)
        }
        _ => None,
    }
}

/// `data` as JSON, or `None` if not even its first value can be read. Values
/// one after another make an array; bytes after the last that can be read
/// are said at its end.
pub fn to_json(data: &[u8], format: Format) -> Option<String> {
    to_json_as(data, format, None)
}

/// As [`to_json`], with Protobuf's fields named by a message from a schema.
pub fn to_json_as(
    data: &[u8],
    format: Format,
    schema: Option<&prost_reflect::MessageDescriptor>,
) -> Option<String> {
    use crate::protobuf::{grpc_to_json, message_to_json};
    match format {
        Format::Protobuf => return message_to_json(data, schema),
        Format::Grpc | Format::GrpcWeb => return grpc_to_json(data, false, schema),
        Format::GrpcWebText => return grpc_to_json(data, true, schema),
        Format::MessagePack | Format::Cbor => {}
    }
    let mut rest = data;
    let mut values = Vec::new();
    while !rest.is_empty() {
        // A value that cannot be read still takes bytes: what is left is
        // counted from before it.
        let mut cursor = rest;
        let value = match format {
            Format::MessagePack => rmpv::decode::read_value(&mut cursor).ok().map(from_msgpack),
            Format::Cbor => ciborium::de::from_reader::<ciborium::Value, _>(&mut cursor)
                .ok()
                .map(from_cbor),
            _ => unreachable!("read above"),
        };
        match value {
            Some(value) if cursor.len() < rest.len() => {
                values.push(value);
                rest = cursor;
            }
            _ => {
                if values.is_empty() {
                    return None;
                }
                values.push(Value::String(format!(
                    "<{} bytes after this could not be read>",
                    rest.len()
                )));
                break;
            }
        }
    }
    let json = match values.len() {
        0 => return None,
        1 => values.pop().expect("one"),
        _ => Value::Array(values),
    };
    serde_json::to_string(&json).ok()
}

/// Bytes as a string that says how many there are, and the first in hex.
pub(crate) fn bytes(data: &[u8]) -> Value {
    let mut text = format!("<{} bytes: ", data.len());
    for b in data.iter().take(BYTES_SHOWN) {
        write!(text, "{b:02x}").expect("a String takes any text");
    }
    if data.len() > BYTES_SHOWN {
        text.push('…');
    }
    text.push('>');
    Value::String(text)
}

fn float(f: f64) -> Value {
    Number::from_f64(f).map_or_else(|| Value::String(f.to_string()), Value::Number)
}

/// A map's key as JSON's, which must be a string: a key that is not one is
/// written as the JSON it would be.
fn key(key: Value) -> String {
    match key {
        Value::String(text) => text,
        other => other.to_string(),
    }
}

fn from_msgpack(value: rmpv::Value) -> Value {
    use rmpv::Value as M;
    match value {
        M::Nil => Value::Null,
        M::Boolean(b) => Value::Bool(b),
        M::Integer(n) => n
            .as_u64()
            .map(Value::from)
            .or_else(|| n.as_i64().map(Value::from))
            .unwrap_or_else(|| Value::String(n.to_string())),
        M::F32(f) => float(f64::from(f)),
        M::F64(f) => float(f),
        M::String(text) => match text.into_str() {
            Some(text) => Value::String(text),
            None => Value::String("<text that is not UTF-8>".to_owned()),
        },
        M::Binary(data) => bytes(&data),
        M::Array(items) => Value::Array(items.into_iter().map(from_msgpack).collect()),
        M::Map(pairs) => Value::Object(
            pairs
                .into_iter()
                .map(|(k, v)| (key(from_msgpack(k)), from_msgpack(v)))
                .collect::<Map<_, _>>(),
        ),
        M::Ext(kind, data) => {
            let mut ext = Map::new();
            ext.insert("$ext".to_owned(), Value::from(kind));
            ext.insert("data".to_owned(), bytes(&data));
            Value::Object(ext)
        }
    }
}

fn from_cbor(value: ciborium::Value) -> Value {
    use ciborium::Value as C;
    match value {
        C::Null => Value::Null,
        C::Bool(b) => Value::Bool(b),
        C::Integer(n) => {
            let n = i128::from(n);
            u64::try_from(n)
                .map(Value::from)
                .or_else(|_| i64::try_from(n).map(Value::from))
                .unwrap_or_else(|_| Value::String(n.to_string()))
        }
        C::Float(f) => float(f),
        C::Text(text) => Value::String(text),
        C::Bytes(data) => bytes(&data),
        C::Array(items) => Value::Array(items.into_iter().map(from_cbor).collect()),
        C::Map(pairs) => Value::Object(
            pairs
                .into_iter()
                .map(|(k, v)| (key(from_cbor(k)), from_cbor(v)))
                .collect::<Map<_, _>>(),
        ),
        C::Tag(tag, inner) => {
            let mut tagged = Map::new();
            tagged.insert("$tag".to_owned(), Value::from(tag));
            tagged.insert("value".to_owned(), from_cbor(*inner));
            Value::Object(tagged)
        }
        _ => Value::String("<a CBOR value with no JSON>".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn the_type_names_the_format() {
        assert_eq!(
            format_of(Some("application/msgpack")),
            Some(Format::MessagePack)
        );
        assert_eq!(
            format_of(Some("application/x-msgpack; q=1")),
            Some(Format::MessagePack)
        );
        assert_eq!(
            format_of(Some("application/vnd.api+msgpack")),
            Some(Format::MessagePack)
        );
        assert_eq!(format_of(Some("Application/CBOR")), Some(Format::Cbor));
        assert_eq!(
            format_of(Some("application/x-protobuf; messageType=\"shop.Order\"")),
            Some(Format::Protobuf)
        );
        assert_eq!(
            format_of(Some("application/vnd.shop+protobuf")),
            Some(Format::Protobuf)
        );
        assert_eq!(format_of(Some("application/grpc")), Some(Format::Grpc));
        assert_eq!(
            format_of(Some("application/grpc+proto")),
            Some(Format::Grpc)
        );
        assert_eq!(format_of(Some("application/grpc+json")), None);
        assert_eq!(
            format_of(Some("application/grpc-web+proto")),
            Some(Format::GrpcWeb)
        );
        assert_eq!(
            format_of(Some("application/grpc-web-text")),
            Some(Format::GrpcWebText)
        );
        assert_eq!(format_of(Some("application/json")), None);
        assert_eq!(format_of(None), None);
    }

    #[test]
    fn messagepack_reads_as_json() {
        let value = rmpv::Value::Map(vec![
            ("id".into(), 42.into()),
            ("name".into(), "Áo thun".into()),
            ("price".into(), 1.5.into()),
            (
                "tags".into(),
                rmpv::Value::Array(vec!["a".into(), rmpv::Value::Nil]),
            ),
            ("raw".into(), rmpv::Value::Binary(vec![0xde, 0xad])),
            (7.into(), true.into()),
            ("when".into(), rmpv::Value::Ext(-1, vec![0, 0, 0, 1])),
        ]);
        let mut data = Vec::new();
        rmpv::encode::write_value(&mut data, &value).unwrap();
        let read = json(&to_json(&data, Format::MessagePack).unwrap());
        assert_eq!(
            read,
            json(
                r#"{"id":42,"name":"Áo thun","price":1.5,"tags":["a",null],
                    "raw":"<2 bytes: dead>","7":true,
                    "when":{"$ext":-1,"data":"<4 bytes: 00000001>"}}"#
            )
        );
    }

    #[test]
    fn cbor_reads_as_json() {
        let value = ciborium::Value::Map(vec![
            ("n".into(), ciborium::Value::Integer(u64::MAX.into())),
            ("neg".into(), ciborium::Value::Integer((-5).into())),
            (
                "at".into(),
                ciborium::Value::Tag(1, Box::new(ciborium::Value::Integer(1_700_000_000.into()))),
            ),
            ("nan".into(), ciborium::Value::Float(f64::NAN)),
        ]);
        let mut data = Vec::new();
        ciborium::ser::into_writer(&value, &mut data).unwrap();
        let read = json(&to_json(&data, Format::Cbor).unwrap());
        assert_eq!(
            read,
            json(
                r#"{"n":18446744073709551615,"neg":-5,
                    "at":{"$tag":1,"value":1700000000},"nan":"NaN"}"#
            )
        );
    }

    #[test]
    fn values_one_after_another_are_an_array_and_junk_is_said() {
        let mut data = Vec::new();
        rmpv::encode::write_value(&mut data, &1.into()).unwrap();
        rmpv::encode::write_value(&mut data, &"two".into()).unwrap();
        assert_eq!(
            json(&to_json(&data, Format::MessagePack).unwrap()),
            json(r#"[1,"two"]"#)
        );
        // A string that says it is longer than what is left.
        data.extend_from_slice(&[0xd9, 0xff, b'x']);
        let read = json(&to_json(&data, Format::MessagePack).unwrap());
        assert_eq!(read[2], json(r#""<3 bytes after this could not be read>""#));
        assert_eq!(
            to_json(&[0xd9], Format::MessagePack),
            None,
            "a string with no length"
        );
        assert_eq!(to_json(b"", Format::Cbor), None);
    }
}
