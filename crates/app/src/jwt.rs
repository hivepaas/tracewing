//! JSON Web Tokens in headers and cookies, decoded to read. Nothing is verified:
//! that needs the key, and what is wanted here is what the token claims.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Map, Value};
use tracewing_i18n::tr;

#[derive(Clone, Debug, PartialEq)]
pub struct Jwt {
    pub header: Map<String, Value>,
    pub payload: Map<String, Value>,
}

/// The claims that are times, and what to call them.
const TIMES: [(&str, &str); 3] = [("exp", "Expires"), ("iat", "Issued"), ("nbf", "Not before")];

impl Jwt {
    /// The first token in `text`, which may be `Bearer …` or a cookie's value.
    pub fn find(text: &str) -> Option<Self> {
        let mut rest = text;
        while let Some(at) = rest.find("eyJ") {
            let candidate = &rest[at..];
            let end = candidate
                .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '=')))
                .unwrap_or(candidate.len());
            if let Some(jwt) = Self::decode(&candidate[..end]) {
                return Some(jwt);
            }
            rest = &candidate[3..];
        }
        None
    }

    fn decode(token: &str) -> Option<Self> {
        let parts: Vec<&str> = token.split('.').collect();
        // Five parts is an encrypted token, which cannot be read.
        if parts.len() != 3 {
            return None;
        }
        let object = |part: &str| -> Option<Map<String, Value>> {
            let bytes = URL_SAFE_NO_PAD.decode(part.trim_end_matches('=')).ok()?;
            match serde_json::from_slice(&bytes).ok()? {
                Value::Object(map) => Some(map),
                _ => None,
            }
        };
        let header = object(parts[0])?;
        header.get("alg")?;
        Some(Self {
            header,
            payload: object(parts[1])?,
        })
    }

    pub fn algorithm(&self) -> Option<&str> {
        self.header.get("alg").and_then(Value::as_str)
    }

    /// `Expires`, its time in seconds since 1970, for each time claim present.
    pub fn times(&self) -> Vec<(&'static str, i64)> {
        TIMES
            .iter()
            .filter_map(|(claim, label)| {
                let at = self.payload.get(*claim)?;
                let at = at.as_i64().or_else(|| at.as_f64().map(|f| f as i64))?;
                Some((*label, at))
            })
            .collect()
    }

    /// Whether it had expired at `now`, in seconds since 1970.
    pub fn expired(&self, now: i64) -> bool {
        self.times()
            .iter()
            .any(|(label, at)| *label == "Expires" && *at <= now)
    }

    pub fn header_text(&self) -> String {
        serde_json::to_string(&self.header).unwrap_or_default()
    }

    pub fn payload_text(&self) -> String {
        serde_json::to_string_pretty(&self.payload).unwrap_or_default()
    }
}

/// What to show for a time claim's label from [`Jwt::times`].
pub fn time_label(label: &str) -> String {
    match label {
        "Expires" => tr!("Expires"),
        "Issued" => tr!("Issued"),
        "Not before" => tr!("Not before"),
        other => other.to_owned(),
    }
}

/// `in 2 h`, `5 min ago`.
pub fn relative(at: i64, now: i64) -> String {
    let delta = at - now;
    let span = delta.unsigned_abs();
    let amount = match span {
        0..60 => format!("{span} s"),
        60..3_600 => format!("{} min", span / 60),
        3_600..86_400 => format!("{} h", span / 3_600),
        86_400..31_536_000 => format!("{} d", span / 86_400),
        _ => format!("{} y", span / 31_536_000),
    };
    if delta >= 0 {
        tr!("in {amount}", amount = amount)
    } else {
        tr!("{amount} ago", amount = amount)
    }
}

/// `2026-11-07 18:00:00` in local time.
pub fn local_time(at: i64) -> String {
    chrono::DateTime::from_timestamp(at, 0)
        .map(|t| {
            t.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
        .unwrap_or_else(|| at.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(header: &str, payload: &str) -> String {
        format!(
            "{}.{}.c2ln",
            URL_SAFE_NO_PAD.encode(header),
            URL_SAFE_NO_PAD.encode(payload)
        )
    }

    #[test]
    fn a_bearer_token_reads_as_its_claims() {
        let jwt = token(
            r#"{"alg":"HS256","typ":"JWT"}"#,
            r#"{"sub":"42","exp":2000,"iat":1000.5}"#,
        );
        let found = Jwt::find(&format!("Bearer {jwt}")).unwrap();
        assert_eq!(found.algorithm(), Some("HS256"));
        assert_eq!(found.payload["sub"], "42");
        assert_eq!(found.times(), [("Expires", 2000), ("Issued", 1000)]);
        assert!(!found.expired(1999));
        assert!(found.expired(2000));
        assert!(found.payload_text().contains("\"sub\": \"42\""));
    }

    #[test]
    fn it_is_found_inside_other_text() {
        let jwt = token(r#"{"alg":"none"}"#, r#"{"a":1}"#);
        assert!(Jwt::find(&format!("session={jwt}; Path=/")).is_some());
        assert!(Jwt::find(&format!("eyJnot-a-token and then {jwt}")).is_some());
    }

    #[test]
    fn what_is_not_a_token_is_not_one() {
        assert_eq!(Jwt::find("Basic dXNlcjpwYXNz"), None);
        assert_eq!(Jwt::find("eyJhbGciOiJIUzI1NiJ9"), None);
        // No `alg`: some other base64 JSON.
        assert_eq!(Jwt::find(&token(r#"{"x":1}"#, r#"{"a":1}"#)), None);
        let jwe = format!("{}.a.b.c.d", URL_SAFE_NO_PAD.encode(r#"{"alg":"RSA"}"#));
        assert_eq!(Jwt::find(&jwe), None);
    }

    #[test]
    fn times_read_from_now() {
        assert_eq!(relative(1_090, 1_000), "in 1 min");
        assert_eq!(relative(1_000 - 7_200, 1_000), "2 h ago");
        assert_eq!(relative(1_000 + 3 * 86_400, 1_000), "in 3 d");
        assert_eq!(relative(1_000, 1_000), "in 0 s");
        assert_eq!(relative(1_000 + 800 * 86_400, 1_000), "in 2 y");
    }
}
