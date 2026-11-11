//! A Protobuf message read by its schema, as JSON: fields by their names in
//! the `.proto`, in number order, enums by name, and the well-known types as
//! their JSON says them. Only what was sent is shown: a field left at its
//! default is not. Fields the schema does not know are under `$unknown`, by
//! number.

use prost_reflect::{DynamicMessage, Kind, MapKey, MessageDescriptor, ReflectMessage, Value};
use std::fmt::Write;

/// `data` as the message `schema` describes, or `None` if it is not one.
pub fn to_json(data: &[u8], schema: &MessageDescriptor) -> Option<String> {
    let message = DynamicMessage::decode(schema.clone(), data).ok()?;
    let mut out = String::new();
    write_message(&message, &mut out);
    Some(out)
}

fn write_message(message: &DynamicMessage, out: &mut String) {
    let name = message.descriptor().full_name().to_owned();
    match name.as_str() {
        "google.protobuf.Timestamp" => return write_timestamp(message, out),
        "google.protobuf.Duration" => return write_duration(message, out),
        "google.protobuf.Any" => return write_any(message, out),
        "google.protobuf.Struct" => {
            if let Some(field) = message.descriptor().get_field_by_name("fields") {
                return write_value(&message.get_field(&field), &field.kind(), out);
            }
        }
        "google.protobuf.ListValue" => {
            if let Some(field) = message.descriptor().get_field_by_name("values") {
                return write_value(&message.get_field(&field), &field.kind(), out);
            }
        }
        "google.protobuf.Value" => {
            return match message.fields().next() {
                Some((_, value)) if !matches!(value, Value::EnumNumber(_)) => {
                    let field = message.fields().next().expect("one").0;
                    write_value(value, &field.kind(), out)
                }
                _ => out.push_str("null"),
            };
        }
        "google.protobuf.DoubleValue"
        | "google.protobuf.FloatValue"
        | "google.protobuf.Int64Value"
        | "google.protobuf.UInt64Value"
        | "google.protobuf.Int32Value"
        | "google.protobuf.UInt32Value"
        | "google.protobuf.BoolValue"
        | "google.protobuf.StringValue"
        | "google.protobuf.BytesValue" => {
            if let Some(field) = message.descriptor().get_field(1) {
                return write_value(&message.get_field(&field), &field.kind(), out);
            }
        }
        _ => {}
    }
    write_fields(message, out, None);
}

/// The fields, after `first` if it is given, then those not in the schema.
fn write_fields(message: &DynamicMessage, out: &mut String, first: Option<(&str, &str)>) {
    out.push('{');
    let mut comma = false;
    if let Some((key, value)) = first {
        write_key(key, out);
        out.push_str(value);
        comma = true;
    }
    let mut fields: Vec<_> = message.fields().collect();
    fields.sort_by_key(|(field, _)| field.number());
    for (field, value) in fields {
        if std::mem::replace(&mut comma, true) {
            out.push(',');
        }
        write_key(field.name(), out);
        write_value(value, &field.kind(), out);
    }
    let mut unknown = Vec::new();
    for field in message.unknown_fields() {
        field.encode(&mut unknown);
    }
    if let Some(json) = crate::protobuf::numbered_json(&unknown).filter(|_| !unknown.is_empty()) {
        if comma {
            out.push(',');
        }
        write_key("$unknown", out);
        out.push_str(&json);
    }
    out.push('}');
}

fn write_key(key: &str, out: &mut String) {
    out.push_str(&serde_json::Value::String(key.to_owned()).to_string());
    out.push(':');
}

fn write_value(value: &Value, kind: &Kind, out: &mut String) {
    match value {
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::I32(n) => write!(out, "{n}").expect("a String takes any text"),
        Value::I64(n) => write!(out, "{n}").expect("a String takes any text"),
        Value::U32(n) => write!(out, "{n}").expect("a String takes any text"),
        Value::U64(n) => write!(out, "{n}").expect("a String takes any text"),
        Value::F32(f) => write_float(f.to_string().parse().unwrap_or(f64::from(*f)), out),
        Value::F64(f) => write_float(*f, out),
        Value::String(text) => out.push_str(&serde_json::Value::String(text.clone()).to_string()),
        Value::Bytes(data) => out.push_str(&crate::binjson::bytes(data).to_string()),
        Value::EnumNumber(n) => match kind
            .as_enum()
            .and_then(|e| e.get_value(*n))
            .map(|v| v.name().to_owned())
        {
            Some(name) => out.push_str(&serde_json::Value::String(name).to_string()),
            None => write!(out, "{n}").expect("a String takes any text"),
        },
        Value::Message(message) => write_message(message, out),
        Value::List(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(item, kind, out);
            }
            out.push(']');
        }
        Value::Map(entries) => {
            let value_kind = kind
                .as_message()
                .map(|entry| entry.map_entry_value_field().kind())
                .unwrap_or_else(|| kind.clone());
            let mut entries: Vec<(String, &Value)> = entries
                .iter()
                .map(|(key, value)| (map_key(key), value))
                .collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            out.push('{');
            for (i, (key, value)) in entries.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_key(&key, out);
                write_value(value, &value_kind, out);
            }
            out.push('}');
        }
    }
}

fn map_key(key: &MapKey) -> String {
    match key {
        MapKey::Bool(b) => b.to_string(),
        MapKey::I32(n) => n.to_string(),
        MapKey::I64(n) => n.to_string(),
        MapKey::U32(n) => n.to_string(),
        MapKey::U64(n) => n.to_string(),
        MapKey::String(text) => text.clone(),
    }
}

fn write_float(f: f64, out: &mut String) {
    match serde_json::Number::from_f64(f) {
        Some(n) => write!(out, "{n}").expect("a String takes any text"),
        None => out.push_str(&serde_json::Value::String(f.to_string()).to_string()),
    }
}

fn integer(message: &DynamicMessage, name: &str) -> i64 {
    match message.get_field_by_name(name).as_deref() {
        Some(Value::I64(n)) => *n,
        Some(Value::I32(n)) => i64::from(*n),
        _ => 0,
    }
}

/// As RFC 3339, in UTC, with as many digits of the second as it has.
fn write_timestamp(message: &DynamicMessage, out: &mut String) {
    use time::{OffsetDateTime, format_description::well_known::Rfc3339};
    let seconds = integer(message, "seconds");
    let nanos = integer(message, "nanos");
    let text = OffsetDateTime::from_unix_timestamp(seconds)
        .ok()
        .and_then(|t| {
            t.replace_nanosecond(nanos.clamp(0, 999_999_999) as u32)
                .ok()
        })
        .and_then(|t| t.format(&Rfc3339).ok())
        .unwrap_or_else(|| format!("{seconds}s"));
    out.push_str(&serde_json::Value::String(text).to_string());
}

/// As `1.5s`.
fn write_duration(message: &DynamicMessage, out: &mut String) {
    let seconds = integer(message, "seconds");
    let nanos = integer(message, "nanos");
    let mut text = if nanos == 0 {
        format!("{seconds}")
    } else {
        let sign = if seconds < 0 || nanos < 0 { "-" } else { "" };
        let fraction = format!("{:09}", nanos.unsigned_abs());
        format!(
            "{sign}{}.{}",
            seconds.unsigned_abs(),
            fraction.trim_end_matches('0')
        )
    };
    text.push('s');
    out.push_str(&serde_json::Value::String(text).to_string());
}

/// Its type under `@type`, and the message it holds if the schema has it.
fn write_any(message: &DynamicMessage, out: &mut String) {
    let type_url = match message.get_field_by_name("type_url").as_deref() {
        Some(Value::String(url)) => url.clone(),
        _ => String::new(),
    };
    let data = match message.get_field_by_name("value").as_deref() {
        Some(Value::Bytes(data)) => data.clone(),
        _ => Default::default(),
    };
    let type_json = serde_json::Value::String(type_url.clone()).to_string();
    let name = type_url.rsplit('/').next().unwrap_or_default();
    let inner = message
        .descriptor()
        .parent_pool()
        .get_message_by_name(name)
        .and_then(|schema| DynamicMessage::decode(schema, &data[..]).ok());
    match inner {
        Some(inner) => write_fields(&inner, out, Some(("@type", &type_json))),
        None => {
            out.push('{');
            write_key("@type", out);
            out.push_str(&type_json);
            out.push(',');
            write_key("value", out);
            out.push_str(&crate::binjson::bytes(&data).to_string());
            out.push('}');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost_reflect::DescriptorPool;

    const PROTO: &str = r#"
syntax = "proto3";
package t;
import "google/protobuf/timestamp.proto";
import "google/protobuf/duration.proto";
import "google/protobuf/wrappers.proto";
import "google/protobuf/struct.proto";
import "google/protobuf/any.proto";

enum State { STATE_UNSPECIFIED = 0; PAID = 1; }
message Item { string sku = 1; uint32 qty = 2; }
message Order {
  uint64 id = 1;
  string customer = 2;
  repeated Item items = 3;
  State state = 4;
  google.protobuf.Timestamp paid_at = 5;
  google.protobuf.Duration took = 6;
  google.protobuf.StringValue note = 7;
  google.protobuf.Struct extra = 8;
  google.protobuf.Any detail = 9;
  map<string, int32> counts = 10;
  bytes sig = 11;
  double price = 12;
  int64 change = 13;
  float rate = 14;
}
"#;

    fn pool() -> DescriptorPool {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("tracewing-protojson-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("t.proto"), PROTO).unwrap();
        let mut compiler = protox::Compiler::new([&dir]).unwrap();
        compiler.open_file("t.proto").unwrap();
        let pool = compiler.descriptor_pool();
        std::fs::remove_dir_all(&dir).unwrap();
        pool
    }

    fn json(text: &str) -> serde_json::Value {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn a_message_reads_by_its_schema() {
        use prost::Message;
        let pool = pool();
        let order = pool.get_message_by_name("t.Order").unwrap();
        let item = pool.get_message_by_name("t.Item").unwrap();
        let mut message = DynamicMessage::new(order.clone());
        let set = |m: &mut DynamicMessage, name: &str, value: Value| {
            m.set_field_by_name(name, value);
        };
        set(&mut message, "id", Value::U64(42));
        set(
            &mut message,
            "customer",
            Value::String("Nguyễn Văn A".into()),
        );
        let mut a = DynamicMessage::new(item.clone());
        set(&mut a, "sku", Value::String("A-1".into()));
        set(&mut a, "qty", Value::U32(2));
        set(&mut message, "items", Value::List(vec![Value::Message(a)]));
        set(&mut message, "state", Value::EnumNumber(1));
        let mut paid = DynamicMessage::new(
            pool.get_message_by_name("google.protobuf.Timestamp")
                .unwrap(),
        );
        set(&mut paid, "seconds", Value::I64(1_700_000_000));
        set(&mut paid, "nanos", Value::I32(500_000_000));
        set(&mut message, "paid_at", Value::Message(paid));
        let mut took = DynamicMessage::new(
            pool.get_message_by_name("google.protobuf.Duration")
                .unwrap(),
        );
        set(&mut took, "seconds", Value::I64(1));
        set(&mut took, "nanos", Value::I32(250_000_000));
        set(&mut message, "took", Value::Message(took));
        let mut note = DynamicMessage::new(
            pool.get_message_by_name("google.protobuf.StringValue")
                .unwrap(),
        );
        set(&mut note, "value", Value::String("gift".into()));
        set(&mut message, "note", Value::Message(note));
        let mut any = DynamicMessage::new(pool.get_message_by_name("google.protobuf.Any").unwrap());
        let mut b = DynamicMessage::new(item);
        set(&mut b, "sku", Value::String("B-7".into()));
        set(
            &mut any,
            "type_url",
            Value::String("type.googleapis.com/t.Item".into()),
        );
        set(&mut any, "value", Value::Bytes(b.encode_to_vec().into()));
        set(&mut message, "detail", Value::Message(any));
        set(
            &mut message,
            "counts",
            Value::Map(
                [
                    (MapKey::String("b".into()), Value::I32(2)),
                    (MapKey::String("a".into()), Value::I32(1)),
                ]
                .into(),
            ),
        );
        set(&mut message, "sig", Value::Bytes(vec![0xca, 0xfe].into()));
        set(&mut message, "price", Value::F64(12.5));
        set(&mut message, "change", Value::I64(-3));
        set(&mut message, "rate", Value::F32(0.1));
        let mut data = message.encode_to_vec();
        // A field the schema has no word for.
        data.extend_from_slice(&[0xa0, 0x06, 0x07]);
        assert_eq!(message.descriptor().full_name(), "t.Order");

        let read = to_json(&data, &order).unwrap();
        assert!(read.starts_with(r#"{"id":42,"customer":"#), "{read}");
        assert!(read.ends_with(r#""$unknown":{"100":7}}"#), "{read}");
        assert_eq!(
            json(&read),
            json(
                r#"{"id":42,"customer":"Nguyễn Văn A","items":[{"sku":"A-1","qty":2}],
                    "state":"PAID","paid_at":"2023-11-14T22:13:20.5Z","took":"1.25s",
                    "note":"gift","detail":{"@type":"type.googleapis.com/t.Item","sku":"B-7"},
                    "counts":{"a":1,"b":2},"sig":"<2 bytes: cafe>","price":12.5,
                    "change":-3,"rate":0.1,"$unknown":{"100":7}}"#
            )
        );
    }

    #[test]
    fn what_does_not_fit_is_none() {
        let pool = pool();
        let order = pool.get_message_by_name("t.Order").unwrap();
        // Field 2 is a string, sent here as a varint.
        assert_eq!(to_json(b"\x10\x01", &order), None);
        assert_eq!(to_json(b"", &order).as_deref(), Some("{}"));
    }
}
