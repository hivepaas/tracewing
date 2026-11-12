//! The headers that tie one request to the next: W3C trace context, B3,
//! Amazon's own, and the timings a server reports about itself.

use crate::{Confidence, Input, Reading, Shown, times};
use tracewing_i18n::tr;

fn is_hex(text: &str, len: usize) -> bool {
    text.len() == len && text.bytes().all(|b| b.is_ascii_hexdigit())
}

/// `00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01`
pub fn traceparent(input: &Input) -> Option<Reading> {
    let text = input.text.trim_matches('"');
    let parts: Vec<&str> = text.split('-').collect();
    if parts.len() != 4
        || !is_hex(parts[0], 2)
        || !is_hex(parts[1], 32)
        || !is_hex(parts[2], 16)
        || !is_hex(parts[3], 2)
    {
        return None;
    }
    let flags = u8::from_str_radix(parts[3], 16).ok()?;
    Some(Reading::new(
        "traceparent",
        tr!("Trace context"),
        Confidence::Certain,
        Shown::Fields(vec![
            (tr!("Version"), parts[0].to_owned()),
            (tr!("Trace"), parts[1].to_owned()),
            (tr!("Span"), parts[2].to_owned()),
            (
                tr!("Sampled"),
                if flags & 1 == 1 {
                    tr!("yes")
                } else {
                    tr!("no")
                },
            ),
        ]),
    ))
}

/// B3, as Zipkin and its many clients send it: trace, span, whether it was
/// sampled, and the span it came from.
pub fn b3(input: &Input) -> Option<Reading> {
    let text = input.text.trim_matches('"');
    let parts: Vec<&str> = text.split('-').collect();
    if parts.len() < 2 || parts.len() > 4 {
        return None;
    }
    if (!is_hex(parts[0], 32) && !is_hex(parts[0], 16)) || !is_hex(parts[1], 16) {
        return None;
    }
    let mut fields = vec![
        (tr!("Trace"), parts[0].to_owned()),
        (tr!("Span"), parts[1].to_owned()),
    ];
    if let Some(sampled) = parts.get(2) {
        let said = match *sampled {
            "1" | "d" => tr!("yes"),
            "0" => tr!("no"),
            other => other.to_owned(),
        };
        fields.push((tr!("Sampled"), said));
    }
    if let Some(parent) = parts.get(3).filter(|parent| is_hex(parent, 16)) {
        fields.push((tr!("Parent span"), (*parent).to_owned()));
    }
    Some(Reading::new(
        "b3",
        tr!("B3 trace"),
        Confidence::Certain,
        Shown::Fields(fields),
    ))
}

/// `Root=1-67891233-abcdef012345678912345678;Parent=…;Sampled=1`, whose root
/// carries the time the trace began.
pub fn amazon(input: &Input) -> Option<Reading> {
    let text = input.text.trim_matches('"');
    if !text.starts_with("Root=") {
        return None;
    }
    let mut fields = Vec::new();
    for part in text.split(';').filter(|part| !part.is_empty()) {
        let (name, value) = part.split_once('=')?;
        fields.push((name.to_owned(), value.to_owned()));
        if name == "Root" {
            let seconds = value.split('-').nth(1)?;
            let at = i64::from_str_radix(seconds, 16).ok()?;
            fields.push((
                tr!("Began"),
                format!(
                    "{}, {}",
                    times::local_time_at(at, input.offset),
                    times::relative(at, input.now)
                ),
            ));
        }
    }
    Some(Reading::new(
        "amazon-trace",
        tr!("Amazon trace"),
        Confidence::Certain,
        Shown::Fields(fields),
    ))
}

/// `db;dur=53, app;dur=47.2;desc="render"`, as a server reports where its
/// time went.
pub fn server_timing(input: &Input) -> Option<Reading> {
    let text = input.text.trim_matches('"');
    if !text.contains("dur=") {
        return None;
    }
    let mut rows = Vec::new();
    for entry in text.split(',').map(str::trim).filter(|e| !e.is_empty()) {
        let mut parts = entry.split(';').map(str::trim);
        let name = parts.next()?.to_owned();
        let mut duration = String::new();
        let mut about = String::new();
        for part in parts {
            if let Some(value) = part.strip_prefix("dur=") {
                let millis: f64 = value.parse().ok()?;
                duration = format!("{millis} ms");
            } else if let Some(value) = part.strip_prefix("desc=") {
                about = value.trim_matches('"').to_owned();
            }
        }
        if duration.is_empty() {
            continue;
        }
        rows.push(vec![name, duration, about]);
    }
    if rows.is_empty() {
        return None;
    }
    Some(Reading::new(
        "server-timing",
        tr!("Server-Timing"),
        Confidence::Certain,
        Shown::Table {
            head: vec![tr!("Name"), tr!("Took"), tr!("About")],
            rows,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{by_id, read};

    fn fields(text: &str, id: &str) -> Vec<(String, String)> {
        let found = read(text);
        match by_id(&found, id).map(|r| &r.shown) {
            Some(Shown::Fields(fields)) => fields.clone(),
            other => panic!("no fields for {id}: {other:?}"),
        }
    }

    #[test]
    fn a_traceparent_comes_apart() {
        let found = fields(
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
            "traceparent",
        );
        assert_eq!(found[1].1, "4bf92f3577b34da6a3ce929d0e0e4736");
        assert_eq!(found[2].1, "00f067aa0ba902b7");
        assert_eq!(found[3].1, "yes");
        let found = fields(
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-00",
            "traceparent",
        );
        assert_eq!(found[3].1, "no");
        assert!(by_id(&read("00-nope-00f067aa0ba902b7-01"), "traceparent").is_none());
    }

    #[test]
    fn b3_is_read_in_its_shapes() {
        let found = fields(
            "80f198ee56343ba864fe8b2a57d3eff7-e457b5a2e4d86bd1-1-05e3ac9a4f6e3b90",
            "b3",
        );
        assert_eq!(found[2].1, "yes");
        assert_eq!(found[3].1, "05e3ac9a4f6e3b90");
        assert!(
            by_id(
                &read("80f198ee56343ba864fe8b2a57d3eff7-e457b5a2e4d86bd1"),
                "b3"
            )
            .is_some()
        );
    }

    #[test]
    fn an_amazon_trace_says_when_it_began() {
        let found = fields(
            "Root=1-67891233-abcdef012345678912345678;Parent=53995c3f42cd8ad8;Sampled=1",
            "amazon-trace",
        );
        assert!(
            found
                .iter()
                .any(|(name, value)| name == "Began" && value.starts_with("2025-01-16"))
        );
        assert!(
            found
                .iter()
                .any(|(name, value)| name == "Sampled" && value == "1")
        );
    }

    #[test]
    fn server_timing_comes_out_as_rows() {
        let found = read(r#"db;dur=53, app;dur=47.2;desc="render", cache;desc="hit""#);
        match &by_id(&found, "server-timing").unwrap().shown {
            Shown::Table { rows, .. } => {
                assert_eq!(rows.len(), 2, "what has no duration is left out");
                assert_eq!(rows[0], vec!["db", "53 ms", ""]);
                assert_eq!(rows[1], vec!["app", "47.2 ms", "render"]);
            }
            other => panic!("{other:?}"),
        }
    }
}
