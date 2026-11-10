//! What a HAR leaves out when it is to be shared: anything that would let
//! whoever reads it act as the user. Headers and fields are told by their
//! names; a JWT is told by its look wherever it is.
//!
//! It errs on the side of leaving out: a field named `code` or `key` that held
//! nothing secret costs a reader little, and one that did would cost the user.

use serde_json::Value;

/// What a value left out reads as.
pub const REMOVED: &str = "(removed)";

/// Parts of a name, without `-` and `_`, that say it holds a secret.
const SECRET_PARTS: [&str; 13] = [
    "authorization",
    "cookie",
    "token",
    "secret",
    "password",
    "passwd",
    "apikey",
    "accesskey",
    "session",
    "signature",
    "credential",
    "privatekey",
    "clientsecret",
];

/// Whole names that hold a secret, though they say less.
const SECRET_NAMES: [&str; 6] = ["key", "sig", "code", "auth", "pwd", "otp"];

/// Whether a header's, a field's or a key's name says that its value is a
/// secret: `Authorization`, `X-Api-Key`, `access_token`, `client_secret`,
/// `?key=`.
pub fn is_secret_name(name: &str) -> bool {
    let plain: String = name
        .chars()
        .filter(|c| *c != '-' && *c != '_')
        .flat_map(char::to_lowercase)
        .collect();
    SECRET_NAMES.contains(&plain.as_str()) || SECRET_PARTS.iter().any(|part| plain.contains(part))
}

/// Headers whose value is a URL, whose query may carry a secret: an OAuth
/// redirect hands its code over in `Location`.
const URL_HEADERS: [&str; 4] = ["location", "referer", "content-location", "refresh"];

/// A header's value as it goes in a shared HAR.
pub fn header_value(name: &str, value: &str) -> String {
    if is_secret_name(name) {
        REMOVED.to_owned()
    } else if URL_HEADERS.contains(&name.to_ascii_lowercase().as_str()) {
        url(value)
    } else {
        without_jwts(value)
    }
}

/// A query or a form, `a=1&b=2`, with the values of secret fields left out
/// and the rest kept as they were written.
pub fn query(query: &str) -> String {
    query
        .split('&')
        .map(|pair| match pair.split_once('=') {
            Some((name, _)) if is_secret_name(&crate::decode::percent_decode(name)) => {
                format!("{name}={REMOVED}")
            }
            _ => without_jwts(pair),
        })
        .collect::<Vec<_>>()
        .join("&")
}

/// A URL with its query's secrets left out. The rest is as it was.
pub fn url(url: &str) -> String {
    let (before, fragment) = match url.split_once('#') {
        Some((before, fragment)) => (before, Some(fragment)),
        None => (url, None),
    };
    let mut out = match before.split_once('?') {
        Some((path, q)) => format!("{}?{}", without_jwts(path), query(q)),
        None => without_jwts(before),
    };
    if let Some(fragment) = fragment {
        out.push('#');
        // An OAuth implicit flow hands its token over in the fragment.
        out.push_str(&query(fragment));
    }
    out
}

/// JSON with the values of secret keys left out, at any depth.
pub fn json(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, value) in map.iter_mut() {
                if is_secret_name(key) && !value.is_object() && !value.is_array() {
                    *value = Value::String(REMOVED.to_owned());
                } else {
                    json(value);
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(json),
        Value::String(text) => {
            let clean = without_jwts(text);
            if clean != *text {
                *text = clean;
            }
        }
        _ => {}
    }
}

/// A body as it goes in a shared HAR: JSON by its keys, a form by its
/// fields, any other text by the JWTs in it. Bytes that are not text stay.
pub fn body(bytes: &[u8], mime: Option<&str>) -> Vec<u8> {
    let essence = mime
        .unwrap_or_default()
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let Ok(text) = std::str::from_utf8(bytes) else {
        return bytes.to_vec();
    };
    if essence == "application/x-www-form-urlencoded" {
        return query(text).into_bytes();
    }
    if let Ok(mut value) = serde_json::from_str::<Value>(text)
        && (value.is_object() || value.is_array())
    {
        json(&mut value);
        return serde_json::to_vec(&value).unwrap_or_else(|_| bytes.to_vec());
    }
    // JSON a line, as a stream sends it.
    if text.lines().count() > 1
        && text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .all(|line| serde_json::from_str::<Value>(line).is_ok())
    {
        let lines: Vec<String> = text
            .lines()
            .map(|line| match serde_json::from_str::<Value>(line) {
                Ok(mut value) => {
                    json(&mut value);
                    value.to_string()
                }
                Err(_) => line.to_owned(),
            })
            .collect();
        return (lines.join("\n") + if text.ends_with('\n') { "\n" } else { "" }).into_bytes();
    }
    without_jwts(text).into_bytes()
}

/// `text` with each JWT in it left out: three base64url parts, the first
/// two of them JSON, which always starts `eyJ`.
pub fn without_jwts(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("eyJ") {
        let (before, from) = rest.split_at(at);
        out.push_str(before);
        let len = jwt_len(from);
        if len > 0 {
            out.push_str(REMOVED);
            rest = &from[len..];
        } else {
            out.push_str("eyJ");
            rest = &from[3..];
        }
    }
    out.push_str(rest);
    out
}

/// How long the JWT at the start of `text` is, or 0 if there is none.
fn jwt_len(text: &str) -> usize {
    let b64 = |c: u8| c.is_ascii_alphanumeric() || c == b'-' || c == b'_';
    let bytes = text.as_bytes();
    let mut at = 0;
    for part in 0..3 {
        let start = at;
        while at < bytes.len() && b64(bytes[at]) {
            at += 1;
        }
        // The header and the claims are never short; the signature may be
        // empty, for an unsigned token.
        if part < 2 && at - start < 8 {
            return 0;
        }
        if part < 2 {
            if bytes.get(at) != Some(&b'.') {
                return 0;
            }
            at += 1;
        }
    }
    at
}

#[cfg(test)]
mod tests {
    use super::*;

    const JWT: &str = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiI0MiJ9.c2lnbmF0dXJl";

    #[test]
    fn names_say_which_values_are_secret() {
        for name in [
            "Authorization",
            "X-Api-Key",
            "x-auth-token",
            "X-Amz-Security-Token",
            "access_token",
            "refresh_token",
            "client_secret",
            "password",
            "api_key",
            "apiKey",
            "sessionId",
            "key",
            "code",
            "Set-Cookie",
        ] {
            assert!(is_secret_name(name), "{name}");
        }
        for name in [
            "content-type",
            "user-agent",
            "page",
            "id",
            "keyword",
            "authority",
            "monkey",
        ] {
            assert!(!is_secret_name(name), "{name}");
        }
    }

    #[test]
    fn a_url_keeps_all_but_its_secrets() {
        assert_eq!(
            url("https://a.dev/v1/me?page=2&access_token=abc&key=k1#state=s&id_token=t"),
            "https://a.dev/v1/me?page=2&access_token=(removed)&key=(removed)#state=s&id_token=(removed)"
        );
        assert_eq!(url("https://a.dev/x"), "https://a.dev/x");
        assert_eq!(
            url(&format!("https://a.dev/r/{JWT}/x")),
            "https://a.dev/r/(removed)/x"
        );
    }

    #[test]
    fn a_body_loses_its_secrets_and_keeps_the_rest() {
        let body = |text: &str, mime: &str| {
            String::from_utf8(super::body(text.as_bytes(), Some(mime))).unwrap()
        };
        assert_eq!(
            body(
                r#"{"user":"ana","password":"p","tokens":{"access_token":"a","list":[1]},"items":[{"apiKey":"k"}]}"#,
                "application/json"
            ),
            r#"{"items":[{"apiKey":"(removed)"}],"password":"(removed)","tokens":{"access_token":"(removed)","list":[1]},"user":"ana"}"#
        );
        assert_eq!(
            body(
                "user=ana&password=p&remember=1",
                "application/x-www-form-urlencoded"
            ),
            "user=ana&password=(removed)&remember=1"
        );
        assert_eq!(
            body(&format!("<p>{JWT}</p>"), "text/html"),
            "<p>(removed)</p>"
        );
        assert_eq!(
            body("{\"token\":\"a\"}\n{\"n\":1}\n", "application/x-ndjson"),
            "{\"token\":\"(removed)\"}\n{\"n\":1}\n"
        );
        assert_eq!(super::body(&[0xff, 0x00], Some("image/png")), [0xff, 0x00]);
    }

    #[test]
    fn a_jwt_is_told_by_its_look_wherever_it_is() {
        assert_eq!(
            header_value("x-custom", &format!("Bearer {JWT}")),
            "Bearer (removed)"
        );
        assert_eq!(header_value("X-Api-Key", "abc"), "(removed)");
        assert_eq!(
            header_value("accept", "application/json"),
            "application/json"
        );
        assert_eq!(
            header_value("Location", "https://app.test/cb?code=C1&state=s"),
            "https://app.test/cb?code=(removed)&state=s"
        );
        assert_eq!(
            without_jwts("eyJ is not one. eyJabc.def"),
            "eyJ is not one. eyJabc.def"
        );
        assert_eq!(
            without_jwts("eyJhbGciOiJub25lIn0.eyJzdWIiOiI0MiJ9."),
            "(removed)",
            "unsigned"
        );
    }
}
