//! JSON, and the JSON people put inside a JSON string.

use crate::{Confidence, Input, Kind, MAX_OUTPUT, Reading, Shown};
use serde_json::Value;
use tracewing_i18n::tr;

/// Laid out, as the body view lays a body out.
fn laid_out(value: &Value) -> Option<Vec<u8>> {
    let text = serde_json::to_string_pretty(value).ok()?;
    (text.len() <= MAX_OUTPUT).then_some(text.into_bytes())
}

/// An object or an array: a value that was already JSON, or one a step before
/// unpacked into it.
pub fn whole(input: &Input) -> Option<Reading> {
    let text = input.text;
    if !text.starts_with('{') && !text.starts_with('[') {
        return None;
    }
    let value: Value = serde_json::from_str(text).ok()?;
    let said = match &value {
        Value::Object(fields) => tr!("JSON, {n} fields", n = fields.len()),
        Value::Array(items) => tr!("JSON, {n} items", n = items.len()),
        _ => return None,
    };
    Some(Reading::new(
        "json",
        said,
        Confidence::Certain,
        Shown::Bytes {
            kind: Kind::Json,
            bytes: laid_out(&value)?,
        },
    ))
}

/// JSON escaped inside a string, as a log line, a webhook's `payload` and a
/// queue's message carry it. The commonest unreadable value there is.
pub fn in_string(input: &Input) -> Option<Reading> {
    let text = input.text;
    if !text.contains('{') && !text.contains('[') {
        return None;
    }
    // Either the selection took the quotes in, or it took the escaped text
    // alone and the quotes are ours to put back.
    let inner: String = serde_json::from_str(text).ok().or_else(|| {
        text.contains('\\')
            .then(|| serde_json::from_str(&format!("\"{text}\"")).ok())
            .flatten()
    })?;
    let inner = inner.trim();
    if !inner.starts_with('{') && !inner.starts_with('[') {
        return None;
    }
    let value: Value = serde_json::from_str(inner).ok()?;
    Some(Reading::new(
        "json-in-string",
        tr!("JSON in a string"),
        Confidence::Certain,
        Shown::Bytes {
            kind: Kind::Json,
            bytes: laid_out(&value)?,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{by_id, read};

    fn text_of(readings: &[Reading], id: &str) -> String {
        match by_id(readings, id).map(|r| &r.shown) {
            Some(Shown::Bytes { bytes, .. }) => String::from_utf8(bytes.clone()).unwrap(),
            other => panic!("no bytes for {id}: {other:?}"),
        }
    }

    #[test]
    fn json_is_laid_out() {
        let found = read(r#"{"a":1,"b":[2,3]}"#);
        assert_eq!(
            text_of(&found, "json"),
            "{\n  \"a\": 1,\n  \"b\": [\n    2,\n    3\n  ]\n}"
        );
        assert!(by_id(&found, "json").unwrap().label.contains('2'));
    }

    #[test]
    fn json_inside_a_string_is_taken_out_of_it() {
        let found = read(r#""{\"a\":1}""#);
        assert_eq!(text_of(&found, "json-in-string"), "{\n  \"a\": 1\n}");
        // The quotes left behind by the selection are put back.
        let found = read(r#"{\"a\":1}"#);
        assert_eq!(text_of(&found, "json-in-string"), "{\n  \"a\": 1\n}");
    }

    #[test]
    fn what_is_not_json_is_left_alone() {
        assert!(by_id(&read("{nope}"), "json").is_none());
        assert!(by_id(&read(r#""hello""#), "json-in-string").is_none());
        assert!(by_id(&read("42"), "json").is_none());
        assert!(by_id(&read(r#""42""#), "json-in-string").is_none());
    }
}
