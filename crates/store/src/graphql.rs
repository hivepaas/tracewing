//! Which GraphQL operation a request asks for, to name it in the list: every
//! request goes to the same `/graphql`, so its path says nothing.

use crate::decode;
use serde_json::Value;

/// `query GetUser`, `mutation`, `GetUser` for a persisted query, `GetA +2` for a
/// batch. `None` when it is not GraphQL.
pub fn operation(
    method: &str,
    path: &str,
    content_type: Option<&str>,
    body: &[u8],
) -> Option<String> {
    if method.eq_ignore_ascii_case("GET") {
        let (_, query) = path.split_once('?')?;
        let mut text = None;
        let mut name = None;
        for pair in query.split('&') {
            match pair.split_once('=') {
                Some(("query", value)) => text = Some(decode::percent_decode(value)),
                Some(("operationName", value)) => name = Some(decode::percent_decode(value)),
                _ => {}
            }
        }
        return named(
            text.as_deref(),
            name.as_deref().filter(|n| !n.is_empty()),
            false,
        );
    }
    let essence = content_type?
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if essence == "application/graphql" {
        return named(Some(std::str::from_utf8(body).ok()?), None, false);
    }
    if !essence.ends_with("json") {
        return None;
    }
    match serde_json::from_slice::<Value>(body).ok()? {
        Value::Array(batch) => {
            let first = batch.iter().find_map(one)?;
            Some(match batch.len() {
                1 => first,
                n => format!("{first} +{}", n - 1),
            })
        }
        value => one(&value),
    }
}

fn one(value: &Value) -> Option<String> {
    let text = value.get("query").and_then(Value::as_str);
    let name = value
        .get("operationName")
        .and_then(Value::as_str)
        .filter(|n| !n.is_empty());
    let persisted = value
        .get("extensions")
        .and_then(|e| e.get("persistedQuery"))
        .is_some();
    named(text, name, persisted)
}

fn named(text: Option<&str>, name: Option<&str>, persisted: bool) -> Option<String> {
    let Some(text) = text else {
        // A persisted query sends only its hash, and its name if it has one.
        return name.filter(|_| persisted).map(str::to_owned);
    };
    let definitions = definitions(text);
    let chosen = match name {
        Some(name) => definitions
            .iter()
            .find(|(_, n)| n.as_deref() == Some(name))
            .or(definitions.first())?,
        None => definitions.first()?,
    };
    Some(match (&chosen.1, name) {
        (Some(own), _) => format!("{} {own}", chosen.0),
        (None, Some(name)) => format!("{} {name}", chosen.0),
        (None, None) => chosen.0.to_owned(),
    })
}

/// The operations in a document, as kind and name. Fragments are skipped.
fn definitions(text: &str) -> Vec<(&'static str, Option<String>)> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut depth = 0usize;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'#' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            b'"' => {
                i = skip_string(bytes, i);
                continue;
            }
            b'{' | b'(' | b'[' => {
                if depth == 0 && bytes[i] == b'{' {
                    // A document that is only a selection is an anonymous query.
                    found.push(("query", None));
                }
                depth += 1;
            }
            b'}' | b')' | b']' => depth = depth.saturating_sub(1),
            b if depth == 0 && is_name_start(b) => {
                let word = name_at(bytes, i);
                i += word.len();
                let kind = match word {
                    "query" => "query",
                    "mutation" => "mutation",
                    "subscription" => "subscription",
                    "fragment" => {
                        skip_to_body(bytes, &mut i, &mut depth);
                        continue;
                    }
                    _ => return Vec::new(),
                };
                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                let name = (i < bytes.len() && is_name_start(bytes[i]))
                    .then(|| name_at(bytes, i).to_owned());
                found.push((kind, name));
                skip_to_body(bytes, &mut i, &mut depth);
                continue;
            }
            b if b.is_ascii_whitespace() || b == b',' => {}
            _ if depth == 0 => return Vec::new(),
            _ => {}
        }
        i += 1;
    }
    found
}

/// Past the operation's own `{`, so it is not taken for another one.
fn skip_to_body(bytes: &[u8], i: &mut usize, depth: &mut usize) {
    let mut inner = 0usize;
    while *i < bytes.len() {
        match bytes[*i] {
            b'"' => {
                *i = skip_string(bytes, *i);
                continue;
            }
            b'(' | b'[' => inner += 1,
            b')' | b']' => inner = inner.saturating_sub(1),
            b'{' if inner == 0 => {
                *depth += 1;
                *i += 1;
                return;
            }
            _ => {}
        }
        *i += 1;
    }
}

fn skip_string(bytes: &[u8], open: usize) -> usize {
    if bytes[open..].starts_with(b"\"\"\"") {
        let mut i = open + 3;
        while i < bytes.len() {
            if bytes[i..].starts_with(b"\"\"\"") && bytes[i - 1] != b'\\' {
                return i + 3;
            }
            i += 1;
        }
        return i;
    }
    let mut i = open + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'"' => return i + 1,
            _ => i += 1,
        }
    }
    i
}

fn is_name_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn name_at(bytes: &[u8], at: usize) -> &str {
    let end = bytes[at..]
        .iter()
        .position(|b| !(b.is_ascii_alphanumeric() || *b == b'_'))
        .map_or(bytes.len(), |n| at + n);
    std::str::from_utf8(&bytes[at..end]).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn post(body: &str) -> Option<String> {
        operation(
            "POST",
            "/graphql",
            Some("application/json"),
            body.as_bytes(),
        )
    }

    #[test]
    fn a_named_operation_says_what_it_is() {
        assert_eq!(
            post(r#"{"query":"query GetUser($id: ID!) { user(id: $id) { name } }"}"#).as_deref(),
            Some("query GetUser")
        );
        assert_eq!(
            post(r#"{"query":"mutation { addItem(name: \"{x}\") { id } }"}"#).as_deref(),
            Some("mutation")
        );
        assert_eq!(
            post(r#"{"query":"{ me { id } }"}"#).as_deref(),
            Some("query")
        );
    }

    #[test]
    fn the_operation_name_picks_one_of_several() {
        let body = r#"{"query":"fragment F on User { id } query A { a } mutation B { b }","operationName":"B"}"#;
        assert_eq!(post(body).as_deref(), Some("mutation B"));
        let anonymous = r#"{"query":"{ me { id } }","operationName":"Me"}"#;
        assert_eq!(post(anonymous).as_deref(), Some("query Me"));
    }

    #[test]
    fn persisted_queries_and_batches() {
        let persisted = r#"{"operationName":"Feed","extensions":{"persistedQuery":{"version":1,"sha256Hash":"ab"}}}"#;
        assert_eq!(post(persisted).as_deref(), Some("Feed"));
        let batch = r#"[{"query":"query A { a }"},{"query":"query B { b }"},{"query":"{ c }"}]"#;
        assert_eq!(post(batch).as_deref(), Some("query A +2"));
    }

    #[test]
    fn a_get_carries_it_in_the_url() {
        let path = "/graphql?query=query%20Q%20%7B%20a%20%7D&variables=%7B%7D";
        assert_eq!(
            operation("GET", path, None, b"").as_deref(),
            Some("query Q")
        );
        let named = "/graphql?query=%7Ba%7D&operationName=Top";
        assert_eq!(
            operation("GET", named, None, b"").as_deref(),
            Some("query Top")
        );
        assert_eq!(
            operation(
                "POST",
                "/",
                Some("application/graphql"),
                b"subscription OnMsg { m }"
            )
            .as_deref(),
            Some("subscription OnMsg")
        );
    }

    #[test]
    fn other_json_is_left_alone() {
        assert_eq!(post(r#"{"query":"red shoes"}"#), None);
        assert_eq!(post(r#"{"query":42}"#), None);
        assert_eq!(post(r#"{"name":"x"}"#), None);
        assert_eq!(post("not json"), None);
        assert_eq!(
            operation("POST", "/", Some("text/plain"), br#"{"query":"{ a }"}"#),
            None
        );
        assert_eq!(operation("GET", "/search?q=x", None, b""), None);
    }
}
