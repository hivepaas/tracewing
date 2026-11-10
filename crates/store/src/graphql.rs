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

/// One operation a request sends: its text, if it sent one (a persisted
/// query sends only a hash), its name, and its variables.
#[derive(Clone, Debug, PartialEq)]
pub struct Sent {
    pub name: Option<String>,
    pub query: Option<String>,
    pub variables: Option<Value>,
}

/// What a GraphQL request sends, each operation of a batch on its own. Empty
/// when it is not GraphQL.
pub fn sent(method: &str, path: &str, content_type: Option<&str>, body: &[u8]) -> Vec<Sent> {
    if method.eq_ignore_ascii_case("GET") {
        let Some((_, query)) = path.split_once('?') else {
            return Vec::new();
        };
        let mut one = Sent {
            name: None,
            query: None,
            variables: None,
        };
        for pair in query.split('&') {
            match pair.split_once('=') {
                Some(("query", value)) => one.query = Some(decode::percent_decode(value)),
                Some(("operationName", value)) => {
                    one.name = Some(decode::percent_decode(value)).filter(|n| !n.is_empty())
                }
                Some(("variables", value)) => {
                    one.variables = serde_json::from_str(&decode::percent_decode(value)).ok()
                }
                _ => {}
            }
        }
        return if one.query.is_some() || one.name.is_some() {
            vec![one]
        } else {
            Vec::new()
        };
    }
    let essence = content_type
        .unwrap_or_default()
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if essence == "application/graphql" {
        return std::str::from_utf8(body).map_or(Vec::new(), |text| {
            vec![Sent {
                name: None,
                query: Some(text.to_owned()),
                variables: None,
            }]
        });
    }
    if !essence.ends_with("json") {
        return Vec::new();
    }
    let read = |value: &Value| {
        let query = value
            .get("query")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let name = value
            .get("operationName")
            .and_then(Value::as_str)
            .filter(|n| !n.is_empty())
            .map(str::to_owned);
        (query.is_some() || name.is_some()).then(|| Sent {
            name,
            query,
            variables: value.get("variables").filter(|v| !v.is_null()).cloned(),
        })
    };
    match serde_json::from_slice::<Value>(body) {
        Ok(Value::Array(batch)) => batch.iter().filter_map(read).collect(),
        Ok(value) => read(&value).into_iter().collect(),
        Err(_) => Vec::new(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Piece {
    Punct,
    Spread,
    Name,
    Value,
    Comment,
}

/// A query's text, a token at a time, with what kind each is.
fn pieces(text: &str) -> Vec<(Piece, &str)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        let piece = match bytes[i] {
            b if b.is_ascii_whitespace() || b == b',' => {
                i += 1;
                continue;
            }
            b'#' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                Piece::Comment
            }
            b'"' => {
                i = skip_string(bytes, i);
                Piece::Value
            }
            b'.' if bytes[i..].starts_with(b"...") => {
                i += 3;
                Piece::Spread
            }
            b if is_name_start(b) => {
                i += name_at(bytes, i).len();
                Piece::Name
            }
            b if b.is_ascii_digit() || b == b'-' => {
                i += 1;
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'.' | b'+' | b'-'))
                {
                    i += 1;
                }
                Piece::Value
            }
            _ => {
                // One character, whole: a punctuator, or something stray.
                i += text[i..].chars().next().map_or(1, char::len_utf8);
                Piece::Punct
            }
        };
        out.push((piece, &text[start..i.min(bytes.len())]));
    }
    out
}

/// A query laid out as a formatter would: a field a line, two spaces deeper
/// in each selection, arguments and input values on the field's line. It
/// only moves whitespace and commas, so what it does with a query that is
/// not quite GraphQL is still that query.
pub fn pretty(text: &str) -> String {
    let pieces = pieces(text);
    let mut out = String::new();
    let mut line = String::new();
    let mut depth = 0usize;
    // Inside `(` or `[`, or an input object, everything stays on the line.
    let mut inline = 0usize;
    // Braces opened inline, which close inline.
    let mut inline_braces: Vec<bool> = Vec::new();
    let mut previous: Option<(Piece, &str)> = None;
    let flush = |out: &mut String, line: &mut String, depth: usize| {
        if !line.trim().is_empty() {
            out.push_str(&"  ".repeat(depth));
            out.push_str(line.trim());
            out.push('\n');
        }
        line.clear();
    };
    for &(piece, token) in &pieces {
        let before = previous.map(|(_, t)| t);
        let ends_value = matches!(
            previous,
            Some((Piece::Name | Piece::Value, _)) | Some((Piece::Punct, "]" | "}" | "!"))
        );
        match (piece, token) {
            (Piece::Comment, _) => {
                flush(&mut out, &mut line, depth);
                line.push_str(token);
                flush(&mut out, &mut line, depth);
            }
            (Piece::Punct, "{") if inline > 0 => {
                if ends_value && !matches!(before, Some(":" | "(" | "[" | "{" | "=")) {
                    line.push_str(", ");
                }
                line.push('{');
                inline += 1;
                inline_braces.push(true);
            }
            (Piece::Punct, "{") => {
                line.push_str(" {");
                flush(&mut out, &mut line, depth);
                depth += 1;
                inline_braces.push(false);
            }
            (Piece::Punct, "}") => {
                if inline_braces.pop() == Some(true) {
                    line.push('}');
                    inline = inline.saturating_sub(1);
                } else {
                    flush(&mut out, &mut line, depth);
                    depth = depth.saturating_sub(1);
                    line.push('}');
                    flush(&mut out, &mut line, depth);
                }
            }
            (Piece::Punct, "(" | "[") => {
                if inline > 0
                    && ends_value
                    && token == "["
                    && !matches!(before, Some(":" | "(" | "[" | "="))
                {
                    line.push_str(", ");
                }
                line.push_str(token);
                inline += 1;
            }
            (Piece::Punct, ")" | "]") => {
                line.push_str(token);
                inline = inline.saturating_sub(1);
            }
            (Piece::Punct, ":") => line.push_str(": "),
            (Piece::Punct, "!") => line.push('!'),
            (Piece::Punct, "=") => line.push_str(" = "),
            (Piece::Punct, "@") => {
                line.push_str(" @");
            }
            (Piece::Punct, "$") => {
                if inline > 0 && ends_value && !matches!(before, Some(":" | "(" | "[" | "{" | "="))
                {
                    line.push_str(", ");
                } else if !line.is_empty() && !line.ends_with(['(', '[', '{', ' ']) {
                    line.push(' ');
                }
                line.push('$');
            }
            (Piece::Punct, other) => {
                line.push_str(other);
            }
            (Piece::Spread, _) => {
                if depth > 0 && inline == 0 {
                    flush(&mut out, &mut line, depth);
                }
                line.push_str("...");
            }
            (Piece::Name | Piece::Value, _) => {
                let after_spread = matches!(previous, Some((Piece::Spread, _)));
                let joined = matches!(before, Some("$" | "@" | "(" | "[" | "{"))
                    || (after_spread && token != "on")
                    || line.ends_with(": ")
                    || line.ends_with(" = ");
                let field_start = depth > 0
                    && inline == 0
                    && !joined
                    && !after_spread
                    && before != Some("on")
                    && !matches!(before, Some(":" | "@"));
                if field_start {
                    flush(&mut out, &mut line, depth);
                } else if inline > 0 && ends_value && !joined {
                    line.push_str(", ");
                } else if !joined && !line.is_empty() && !line.ends_with(' ') {
                    line.push(' ');
                }
                line.push_str(token);
            }
        }
        previous = Some((piece, token));
    }
    flush(&mut out, &mut line, depth);
    out.trim_end().to_owned()
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
    fn a_minified_query_comes_out_laid_out() {
        let text = r#"query GetUser($id: ID!, $first: Int = 10) { user(id: $id) { id name friends(first: $first, where: {age: {gt: 18}, tags: ["a", "b"]}) { edges { node { ...Friend @include(if: true) } } } } } fragment Friend on User { name ... on Admin { level } }"#;
        assert_eq!(
            pretty(text),
            "query GetUser($id: ID!, $first: Int = 10) {
  user(id: $id) {
    id
    name
    friends(first: $first, where: {age: {gt: 18}, tags: [\"a\", \"b\"]}) {
      edges {
        node {
          ...Friend @include(if: true)
        }
      }
    }
  }
}
fragment Friend on User {
  name
  ... on Admin {
    level
  }
}"
        );
    }

    #[test]
    fn laid_out_twice_is_laid_out_once() {
        let text = "mutation { a: addUser(input: {name: \"x y\"}) { id } } # done";
        let once = pretty(text);
        assert_eq!(pretty(&once), once);
        assert!(
            once.contains("a: addUser(input: {name: \"x y\"}) {"),
            "{once}"
        );
        assert!(once.ends_with("# done"), "{once}");
    }

    #[test]
    fn what_a_request_sends_is_read_whole() {
        let body = r#"[{"operationName":"A","query":"query A { a }","variables":{"x":1}},
                       {"operationName":"B","extensions":{"persistedQuery":{"sha256Hash":"ab"}}}]"#;
        let sent = sent(
            "POST",
            "/graphql",
            Some("application/json"),
            body.as_bytes(),
        );
        assert_eq!(sent.len(), 2);
        assert_eq!(sent[0].name.as_deref(), Some("A"));
        assert_eq!(sent[0].variables, Some(serde_json::json!({"x": 1})));
        assert_eq!(sent[1].query, None, "persisted: the server has the text");
        let got = super::sent(
            "GET",
            "/graphql?query=%7Bme%7Bid%7D%7D&variables=%7B%22a%22%3A2%7D",
            None,
            b"",
        );
        assert_eq!(got[0].query.as_deref(), Some("{me{id}}"));
        assert_eq!(got[0].variables, Some(serde_json::json!({"a": 2})));
        assert!(super::sent("POST", "/x", Some("text/plain"), b"{}").is_empty());
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
