//! The values HTTP carries about itself: URLs, query and form fields, the
//! headers whose wording is a language of its own.

use crate::{Confidence, Input, Kind, Reading, Shown, encodings, times};
use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};
use tracewing_i18n::tr;

/// A URL in its parts.
pub fn url(input: &Input) -> Option<Reading> {
    let text = input.text.trim_matches('"');
    let (scheme, rest) = text.split_once("://")?;
    if scheme.is_empty()
        || !scheme
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
    {
        return None;
    }
    let (rest, fragment) = match rest.split_once('#') {
        Some((rest, fragment)) => (rest, Some(fragment)),
        None => (rest, None),
    };
    let (rest, query) = match rest.split_once('?') {
        Some((rest, query)) => (rest, Some(query)),
        None => (rest, None),
    };
    let (authority, path) = match rest.find('/') {
        Some(at) => (&rest[..at], &rest[at..]),
        None => (rest, "/"),
    };
    if authority.is_empty() || authority.contains(' ') {
        return None;
    }
    let (user, host) = match authority.rsplit_once('@') {
        Some((user, host)) => (Some(user), host),
        None => (None, authority),
    };
    let (host, port) = match host.rsplit_once(':') {
        Some((host, port)) if port.bytes().all(|b| b.is_ascii_digit()) && !port.is_empty() => {
            (host, Some(port))
        }
        _ => (host, None),
    };
    let mut fields = vec![
        (tr!("Scheme"), scheme.to_owned()),
        (tr!("Host"), host.to_owned()),
    ];
    if let Some(port) = port {
        fields.push((tr!("Port"), port.to_owned()));
    }
    if let Some(user) = user {
        fields.push((tr!("User"), user.to_owned()));
    }
    fields.push((tr!("Path"), decoded_or(path)));
    if let Some(query) = query {
        let n = pairs(query).map(|pairs| pairs.len()).unwrap_or(0);
        fields.push((tr!("Query"), tr!("{n} fields", n = n)));
        for (name, value) in pairs(query).unwrap_or_default() {
            fields.push((format!("  {name}"), value));
        }
    }
    if let Some(fragment) = fragment {
        fields.push((tr!("Fragment"), decoded_or(fragment)));
    }
    Some(Reading::new(
        "url",
        tr!("URL"),
        Confidence::Certain,
        Shown::Fields(fields),
    ))
}

fn decoded_or(text: &str) -> String {
    encodings::percent_decode(text).unwrap_or_else(|| text.to_owned())
}

/// A query string or a form body as its fields, `+` for a space as a form
/// writes it.
fn pairs(text: &str) -> Option<Vec<(String, String)>> {
    let mut found = Vec::new();
    for part in text.split('&').filter(|part| !part.is_empty()) {
        let (name, value) = part.split_once('=').unwrap_or((part, ""));
        if name.is_empty() || name.contains(' ') {
            return None;
        }
        let plain = |text: &str| decoded_or(&text.replace('+', " "));
        found.push((plain(name), plain(value)));
    }
    (!found.is_empty()).then_some(found)
}

/// `a=1&b=2`, as a query string and a form body are written.
pub fn query(input: &Input) -> Option<Reading> {
    let text = input.text.trim_matches('"').trim_start_matches('?');
    if !text.contains('=') || text.contains("://") {
        return None;
    }
    let found = pairs(text)?;
    // One field with nothing around it is just a value with an equals sign.
    if found.len() < 2 && !text.contains('&') {
        return None;
    }
    Some(Reading::new(
        "query",
        tr!("Query fields"),
        Confidence::Certain,
        Shown::Table {
            head: vec![tr!("Field"), tr!("Value")],
            rows: found
                .into_iter()
                .map(|(name, value)| vec![name, value])
                .collect(),
        },
    ))
}

/// `Authorization: Basic …`, as the user and password it holds.
pub fn basic(input: &Input) -> Option<Reading> {
    let text = input.text.trim_matches('"');
    let token = match text.split_once(' ') {
        Some((scheme, token)) if scheme.eq_ignore_ascii_case("basic") => token.trim(),
        _ if input.field_says(&["authorization", "proxyauthorization"]) => text,
        _ => return None,
    };
    let bytes = STANDARD
        .decode(token)
        .or_else(|_| STANDARD_NO_PAD.decode(token))
        .ok()?;
    let pair = String::from_utf8(bytes).ok()?;
    let (user, password) = pair.split_once(':')?;
    if user.contains(|c: char| c.is_control()) || password.contains(|c: char| c.is_control()) {
        return None;
    }
    Some(Reading::new(
        "basic-auth",
        tr!("Basic authentication"),
        Confidence::Certain,
        Shown::Fields(vec![
            (tr!("User"), user.to_owned()),
            (tr!("Password"), password.to_owned()),
        ]),
    ))
}

/// `Cache-Control`, as a sentence rather than a list of words.
pub fn cache_control(input: &Input) -> Option<Reading> {
    let text = input.text.trim_matches('"').to_ascii_lowercase();
    let mut said = Vec::new();
    let mut known = false;
    let directives: Vec<&str> = text.split(',').map(str::trim).collect();
    let value_of = |name: &str| -> Option<u64> {
        directives
            .iter()
            .find_map(|d| d.strip_prefix(name)?.strip_prefix('=')?.trim().parse().ok())
    };
    if directives.contains(&"no-store") {
        said.push(tr!("never kept"));
        known = true;
    } else if let Some(seconds) = value_of("max-age") {
        said.push(tr!("kept {span}", span = times::span(seconds)));
        known = true;
    }
    if let Some(seconds) = value_of("s-maxage") {
        said.push(tr!(
            "kept {span} by shared caches",
            span = times::span(seconds)
        ));
        known = true;
    }
    for (directive, words) in [
        ("private", tr!("by the browser only")),
        ("public", tr!("by anyone")),
        ("no-cache", tr!("checked every time")),
        ("must-revalidate", tr!("not used once stale")),
        ("immutable", tr!("never checked again")),
    ] {
        if directives.contains(&directive) {
            said.push(words);
            known = true;
        }
    }
    if !known {
        return None;
    }
    Some(Reading::new(
        "cache-control",
        tr!("Cache-Control"),
        Confidence::Certain,
        Shown::Line(said.join(", ")),
    ))
}

/// A status code by what it means.
pub fn status(input: &Input) -> Option<Reading> {
    let text = input.text.trim_matches('"');
    if text.len() != 3 || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let code: u16 = text.parse().ok()?;
    let reason = http::StatusCode::from_u16(code)
        .ok()?
        .canonical_reason()?
        .to_owned();
    let confidence = if input.field_says(&["status", "code", "statuscode"]) {
        Confidence::Likely
    } else {
        Confidence::Possible
    };
    Some(Reading::new(
        "status",
        tr!("Status code"),
        confidence,
        Shown::Line(reason),
    ))
}

/// A `Set-Cookie` in its parts, with its life said as a span.
pub fn cookie(input: &Input) -> Option<Reading> {
    const ATTRIBUTES: [&str; 8] = [
        "path",
        "domain",
        "expires",
        "max-age",
        "secure",
        "httponly",
        "samesite",
        "partitioned",
    ];
    let text = input.text.trim_matches('"');
    if !text.contains('=') || !text.contains(';') {
        return None;
    }
    let mut parts = text.split(';').map(str::trim);
    let (name, value) = parts.next()?.split_once('=')?;
    if name.is_empty() || name.contains(' ') {
        return None;
    }
    let mut fields = vec![(name.to_owned(), value.to_owned())];
    let mut known = 0;
    for part in parts.filter(|part| !part.is_empty()) {
        let (key, attribute) = part.split_once('=').unwrap_or((part, ""));
        let lower = key.trim().to_ascii_lowercase();
        if !ATTRIBUTES.contains(&lower.as_str()) {
            continue;
        }
        known += 1;
        let said = match lower.as_str() {
            "max-age" => match attribute.trim().parse() {
                Ok(seconds) => times::span(seconds),
                Err(_) => attribute.trim().to_owned(),
            },
            _ if attribute.is_empty() => tr!("yes"),
            _ => attribute.trim().to_owned(),
        };
        fields.push((key.trim().to_owned(), said));
    }
    if known == 0 {
        return None;
    }
    Some(Reading::new(
        "cookie",
        tr!("Cookie"),
        Confidence::Certain,
        Shown::Fields(fields),
    ))
}

/// `data:image/png;base64,…`, as the bytes it holds.
pub fn data_uri(input: &Input) -> Option<Reading> {
    let rest = input.text.trim_matches('"').strip_prefix("data:")?;
    let (head, payload) = rest.split_once(',')?;
    let (media, is_base64) = match head.strip_suffix(";base64") {
        Some(media) => (media, true),
        None => (head, false),
    };
    let bytes = if is_base64 {
        STANDARD
            .decode(payload)
            .or_else(|_| STANDARD_NO_PAD.decode(payload))
            .ok()?
    } else {
        encodings::percent_decode(payload)
            .unwrap_or_else(|| payload.to_owned())
            .into_bytes()
    };
    let media = if media.is_empty() {
        "text/plain"
    } else {
        media
    };
    let kind = encodings::interesting(&bytes).unwrap_or(Kind::Binary);
    Some(Reading::new(
        "data-uri",
        tr!("data: URI, {media}", media = media),
        Confidence::Certain,
        Shown::Bytes { kind, bytes },
    ))
}

/// What an `Authorization` of Amazon's says, which is a great deal: the key
/// it was signed with, the day, the region, the service, and which headers
/// were part of the signature.
///
/// Nothing else in a request to a cloud is so long and so unreadable, and a
/// request refused for the wrong region or a stale day is a morning gone.
pub fn aws_sigv4(input: &Input) -> Option<Reading> {
    let text = input.text.trim().trim_matches('"');
    let rest = text
        .strip_prefix("AWS4-HMAC-SHA256")
        .map(str::trim_start)
        .filter(|rest| rest.starts_with("Credential="));
    // A presigned URL carries the same thing in a query field, with no
    // algorithm in front of it.
    let credential = match rest {
        Some(rest) => rest
            .split(',')
            .find_map(|part| part.trim().strip_prefix("Credential="))?,
        None if text.ends_with("/aws4_request") => text,
        None => return None,
    };
    let parts: Vec<&str> = credential.split('/').collect();
    let [key, day, region, service, scope] = parts.as_slice() else {
        return None;
    };
    if *scope != "aws4_request" || day.len() != 8 || !day.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut fields = vec![
        (tr!("Key"), (*key).to_owned()),
        (
            tr!("Day"),
            format!("{}-{}-{}", &day[..4], &day[4..6], &day[6..]),
        ),
        (tr!("Region"), (*region).to_owned()),
        (tr!("Service"), (*service).to_owned()),
    ];
    if let Some(rest) = rest {
        if let Some(headers) = rest
            .split(',')
            .find_map(|part| part.trim().strip_prefix("SignedHeaders="))
        {
            let named: Vec<&str> = headers.split(';').filter(|h| !h.is_empty()).collect();
            fields.push((tr!("Headers signed"), named.join(", ")));
        }
        if let Some(signature) = rest
            .split(',')
            .find_map(|part| part.trim().strip_prefix("Signature="))
        {
            fields.push((tr!("Signature"), signature.trim().to_owned()));
        }
    }
    Some(Reading::new(
        "aws-sigv4",
        tr!("An Amazon signature"),
        Confidence::Certain,
        Shown::Fields(fields),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{by_id, line, read, read_under};

    fn fields(text: &str, id: &str) -> Vec<(String, String)> {
        let found = read(text);
        match by_id(&found, id).map(|r| &r.shown) {
            Some(Shown::Fields(fields)) => fields.clone(),
            other => panic!("no fields for {id}: {other:?}"),
        }
    }

    #[test]
    fn a_url_comes_apart() {
        let found = fields(
            "https://api.example.com:8443/v1/orders?page=2&q=caf%C3%A9#top",
            "url",
        );
        let said: Vec<(&str, &str)> = found
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        assert!(said.contains(&("Scheme", "https")));
        assert!(said.contains(&("Host", "api.example.com")));
        assert!(said.contains(&("Port", "8443")));
        assert!(said.contains(&("Path", "/v1/orders")));
        assert!(said.contains(&("  q", "café")));
        assert!(said.contains(&("Fragment", "top")));
    }

    #[test]
    fn what_is_not_a_url_is_not_one() {
        assert!(by_id(&read("not://"), "url").is_none());
        assert!(by_id(&read("https://"), "url").is_none());
        assert!(by_id(&read("a://b c"), "url").is_none());
    }

    #[test]
    fn query_fields_come_out_as_rows() {
        let found = read("name=Jos%C3%A9+Mar%C3%ADa&page=2");
        match &by_id(&found, "query").unwrap().shown {
            Shown::Table { rows, .. } => {
                assert_eq!(rows[0], vec!["name", "José María"]);
                assert_eq!(rows[1], vec!["page", "2"]);
            }
            other => panic!("{other:?}"),
        }
        // One pair alone is a value with an equals sign in it.
        assert!(by_id(&read("a=1"), "query").is_none());
    }

    #[test]
    fn an_amazon_signature_comes_apart() {
        let found = fields(
            "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20150830/us-east-1/iam/aws4_request, \
             SignedHeaders=content-type;host;x-amz-date, \
             Signature=5d672d79c15b13162d9279b0855cfba6789a8edb4c82c400e06b5924a6f2b5d7",
            "aws-sigv4",
        );
        let said: Vec<(&str, &str)> = found
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        assert!(said.contains(&("Key", "AKIDEXAMPLE")));
        assert!(said.contains(&("Day", "2015-08-30")));
        assert!(said.contains(&("Region", "us-east-1")));
        assert!(said.contains(&("Service", "iam")));
        assert!(said.contains(&("Headers signed", "content-type, host, x-amz-date")));
        assert!(said.contains(&(
            "Signature",
            "5d672d79c15b13162d9279b0855cfba6789a8edb4c82c400e06b5924a6f2b5d7"
        )));
    }

    #[test]
    fn a_presigned_credential_is_enough() {
        let found = fields(
            "AKIDEXAMPLE/20150830/eu-west-1/s3/aws4_request",
            "aws-sigv4",
        );
        let said: Vec<&str> = found.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(said, vec!["Key", "Day", "Region", "Service"]);
        assert_eq!(found[1].1, "2015-08-30");
    }

    #[test]
    fn what_is_not_an_amazon_signature_is_not_one() {
        // The algorithm alone, with nothing signed.
        assert!(by_id(&read("AWS4-HMAC-SHA256"), "aws-sigv4").is_none());
        // A path that happens to end the same way.
        assert!(by_id(&read("/one/two/three/four/aws4_request"), "aws-sigv4").is_none());
        // A day that is not a day.
        assert!(
            by_id(
                &read("AKIDEXAMPLE/last-week/us-east-1/iam/aws4_request"),
                "aws-sigv4"
            )
            .is_none()
        );
        // The older scheme, which says nothing of regions or services.
        assert!(by_id(&read("AWS4-HMAC-SHA256 Signature=abc"), "aws-sigv4").is_none());
    }

    #[test]
    fn basic_authentication_says_who() {
        let found = fields("Basic dXNlcjpwYXNzd29yZA==", "basic-auth");
        assert_eq!(found[0].1, "user");
        assert_eq!(found[1].1, "password");
        // Under its own header the scheme may be left out.
        let found = read_under("authorization", "dXNlcjpwYXNzd29yZA==");
        assert!(by_id(&found, "basic-auth").is_some());
    }

    #[test]
    fn cache_control_is_said_as_a_sentence() {
        assert_eq!(
            line(&read("public, max-age=9000"), "cache-control"),
            "kept 2h 30m, by anyone"
        );
        assert_eq!(line(&read("no-store"), "cache-control"), "never kept");
        assert_eq!(
            line(
                &read("private, max-age=0, must-revalidate"),
                "cache-control"
            ),
            "kept 0s, by the browser only, not used once stale"
        );
        assert!(by_id(&read("whatever"), "cache-control").is_none());
    }

    #[test]
    fn a_status_code_says_what_it_means() {
        assert_eq!(line(&read("422"), "status"), "Unprocessable Entity");
        assert_eq!(
            by_id(&read_under("status", "404"), "status")
                .unwrap()
                .confidence,
            Confidence::Likely
        );
        assert!(by_id(&read("999"), "status").is_none());
    }

    #[test]
    fn a_cookie_comes_apart_and_says_its_life() {
        let found = fields(
            "session=abc123; Path=/; Max-Age=86400; HttpOnly; SameSite=Lax",
            "cookie",
        );
        assert_eq!(found[0], ("session".to_owned(), "abc123".to_owned()));
        assert!(
            found
                .iter()
                .any(|(name, value)| name == "Max-Age" && value == "1d")
        );
        assert!(
            found
                .iter()
                .any(|(name, value)| name == "HttpOnly" && value == "yes")
        );
    }

    #[test]
    fn a_data_uri_gives_up_its_bytes() {
        let found = read("data:text/plain;base64,SGVsbG8sIHdvcmxkIQ==");
        let reading = by_id(&found, "data-uri").unwrap();
        assert!(reading.label.contains("text/plain"));
        match &reading.shown {
            Shown::Bytes { bytes, .. } => assert_eq!(bytes, b"Hello, world!"),
            other => panic!("{other:?}"),
        }
    }
}
