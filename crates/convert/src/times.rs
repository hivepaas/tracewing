//! Times, and the spans between them.

use crate::{Confidence, Input, Reading, Shown};
use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone};
use tracewing_i18n::tr;

/// 1973-03-03, and 2100-01-01: outside these a count of seconds is a count of
/// something else.
const FLOOR: i128 = 100_000_000;
const CEILING: i128 = 4_102_444_800;

/// `2026-11-12 21:13:20 (+07:00)`, `at` read at `offset` seconds east of UTC.
pub fn local_time_at(at: i64, offset: i32) -> String {
    let zone = FixedOffset::east_opt(offset).unwrap_or_else(|| FixedOffset::east_opt(0).unwrap());
    match DateTime::from_timestamp(at, 0) {
        Some(time) => time
            .with_timezone(&zone)
            .format("%Y-%m-%d %H:%M:%S (%:z)")
            .to_string(),
        None => at.to_string(),
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

/// A time, and when it falls from here.
fn moment(at: i64, input: &Input) -> Shown {
    Shown::Line(format!(
        "{}, {}",
        local_time_at(at, input.offset),
        relative(at, input.now)
    ))
}

/// The words that say a field holds a time.
const SAYS_TIME: &[&str] = &[
    "time", "date", "exp", "iat", "nbf", "since", "until", "stamp", "seen", "when",
];

/// What says it at the end of a name and nowhere else: `at` inside one is
/// `status` and `latency`.
const ENDS_TIME: &[&str] = &["at", "ts", "on"];

/// The number as a whole number, with the quotes a JSON string would put
/// around it taken off.
fn whole(text: &str) -> Option<i128> {
    let text = text.trim_matches('"');
    if text.len() > 20 || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// Seconds, milliseconds, microseconds or nanoseconds since 1970, told apart
/// by how large the number is: the four windows do not overlap.
pub fn unix(input: &Input) -> Option<Reading> {
    let n = whole(input.text)?;
    let (id, label, seconds) = if (FLOOR..CEILING).contains(&n) {
        ("unix-s", tr!("Unix seconds"), n)
    } else if (FLOOR * 1_000..CEILING * 1_000).contains(&n) {
        ("unix-ms", tr!("Unix milliseconds"), n / 1_000)
    } else if (FLOOR * 1_000_000..CEILING * 1_000_000).contains(&n) {
        ("unix-us", tr!("Unix microseconds"), n / 1_000_000)
    } else if (FLOOR * 1_000_000_000..CEILING * 1_000_000_000).contains(&n) {
        ("unix-ns", tr!("Unix nanoseconds"), n / 1_000_000_000)
    } else {
        return None;
    };
    // A field that says it is a time leaves nothing to guess.
    let confidence = if input.field_says(SAYS_TIME) || input.field_ends(ENDS_TIME) {
        Confidence::Certain
    } else {
        Confidence::Likely
    };
    Some(Reading::new(
        id,
        label,
        confidence,
        moment(seconds as i64, input),
    ))
}

/// ISO 8601 and RFC 3339, as most of the web writes a time.
pub fn iso(input: &Input) -> Option<Reading> {
    let text = input.text.trim_matches('"');
    if text.len() < 10 || !text.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    if let Ok(time) = DateTime::parse_from_rfc3339(text) {
        return Some(Reading::new(
            "iso-8601",
            tr!("ISO 8601"),
            Confidence::Certain,
            moment(time.timestamp(), input),
        ));
    }
    // No zone: it means UTC often enough to read, and to say so.
    for shape in ["%Y-%m-%dT%H:%M:%S", "%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M"] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(text, shape) {
            return Some(Reading::new(
                "iso-8601-naive",
                tr!("ISO 8601, no zone: read as UTC"),
                Confidence::Likely,
                moment(chrono::Utc.from_utc_datetime(&naive).timestamp(), input),
            ));
        }
    }
    None
}

/// `Sun, 06 Nov 1994 08:49:37 GMT`, as `Date`, `Expires` and `Last-Modified`
/// carry a time.
pub fn http_date(input: &Input) -> Option<Reading> {
    let text = input.text.trim_matches('"');
    let naive = NaiveDateTime::parse_from_str(text, "%a, %d %b %Y %H:%M:%S GMT")
        .or_else(|_| NaiveDateTime::parse_from_str(text, "%A, %d-%b-%y %H:%M:%S GMT"))
        .ok()?;
    Some(Reading::new(
        "http-date",
        tr!("HTTP-date"),
        Confidence::Certain,
        moment(chrono::Utc.from_utc_datetime(&naive).timestamp(), input),
    ))
}

/// The words that say a field holds a span, not a moment.
const SAYS_SPAN: &[&str] = &[
    "maxage",
    "expiresin",
    "ttl",
    "timeout",
    "duration",
    "retryafter",
    "interval",
    "period",
    "elapsed",
    "lifetime",
];

/// `age` and `secs` say a span where a name ends in them, not in `message`.
const ENDS_SPAN: &[&str] = &["age", "secs", "seconds", "took", "every"];

/// Seconds as a span a reader can hold in their head: `2h 30m`.
pub fn duration(input: &Input) -> Option<Reading> {
    let n = whole(input.text)?;
    // Past ten years it is not a timeout anybody set.
    if n == 0 || n > 315_360_000 {
        return None;
    }
    let confidence = if input.field_says(SAYS_SPAN) || input.field_ends(ENDS_SPAN) {
        Confidence::Likely
    } else {
        Confidence::Possible
    };
    Some(Reading::new(
        "duration",
        tr!("Seconds"),
        confidence,
        Shown::Line(span(n as u64)),
    ))
}

/// Whether the field says this value is a time, which is what lets the
/// epochs below be offered at all where they could be read as something else.
fn says_time(input: &Input) -> bool {
    input.field_says(SAYS_TIME) || input.field_ends(ENDS_TIME)
}

/// Windows counts 100-nanosecond ticks from 1601. Nothing else is this large,
/// so it needs no help from the field's name.
pub fn filetime(input: &Input) -> Option<Reading> {
    let n = whole(input.text)?;
    // 1990 to 2100, in ticks.
    if !(122_768_784_000_000_000..157_472_784_000_000_000).contains(&n) {
        return None;
    }
    let at = (n / 10_000_000 - 11_644_473_600) as i64;
    Some(Reading::new(
        "filetime",
        tr!("Windows FILETIME"),
        Confidence::Likely,
        moment(at, input),
    ))
}

/// .NET counts the same ticks from the year 1, which makes them larger still.
pub fn dotnet_ticks(input: &Input) -> Option<Reading> {
    let n = whole(input.text)?;
    // 1990 to 2100, in ticks.
    if !(627_667_488_000_000_000..662_371_488_000_000_000).contains(&n) {
        return None;
    }
    let at = (n / 10_000_000 - 62_135_596_800) as i64;
    Some(Reading::new(
        "dotnet-ticks",
        tr!(".NET ticks"),
        Confidence::Likely,
        moment(at, input),
    ))
}

/// Apple counts seconds from 2001. A number that size is a Unix time just as
/// readily, so this is only offered where the field says it is a time at all.
pub fn cocoa(input: &Input) -> Option<Reading> {
    let n = whole(input.text)?;
    if !says_time(input) || !(0..3_124_137_600).contains(&n) {
        return None;
    }
    Some(Reading::new(
        "cocoa",
        tr!("Seconds since 2001 (Apple)"),
        Confidence::Possible,
        moment(n as i64 + 978_307_200, input),
    ))
}

/// HFS+ counts seconds from 1904. Past where a Unix time can reach it needs
/// no help; inside that window it asks the field.
pub fn hfs(input: &Input) -> Option<Reading> {
    let n = whole(input.text)?;
    // 1990 to 2100, in seconds from 1904.
    if !(2_713_996_800..6_185_289_600).contains(&n) {
        return None;
    }
    if n < CEILING && !says_time(input) {
        return None;
    }
    Some(Reading::new(
        "hfs",
        tr!("Seconds since 1904 (HFS+)"),
        Confidence::Possible,
        moment((n - 2_082_844_800) as i64, input),
    ))
}

/// A spreadsheet counts days from the last day of 1899, and often with a
/// fraction for the time of day. A whole number that small is anything at
/// all, so it asks the field unless it has that fraction.
pub fn excel_date(input: &Input) -> Option<Reading> {
    let text = input.text.trim_matches('"');
    let days: f64 = text.parse().ok()?;
    // 1990 to 2100.
    if !(32_874.0..73_051.0).contains(&days) {
        return None;
    }
    let fraction = days.fract() != 0.;
    if !fraction && !says_time(input) {
        return None;
    }
    // From 1899-12-30, which is what the format counts from.
    let at = ((days - 25_569.0) * 86_400.0) as i64;
    Some(Reading::new(
        "excel-date",
        tr!("Days since 1899 (spreadsheet)"),
        Confidence::Possible,
        moment(at, input),
    ))
}

/// `45s`, `2h 30m`, `3d 4h`: the two largest units that say anything.
pub fn span(seconds: u64) -> String {
    const UNITS: [(u64, &str); 4] = [(86_400, "d"), (3_600, "h"), (60, "m"), (1, "s")];
    let mut left = seconds;
    let mut said = Vec::new();
    for (size, unit) in UNITS {
        let many = left / size;
        if many > 0 {
            said.push(format!("{many}{unit}"));
            left -= many * size;
        }
        if said.len() == 2 {
            break;
        }
    }
    if said.is_empty() {
        said.push("0s".to_owned());
    }
    said.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{line, read, read_under};
    use crate::{Confidence, tests::by_id};

    #[test]
    fn a_time_is_told_from_its_unit_by_its_size() {
        assert_eq!(
            line(&read("1794492800"), "unix-s"),
            "2026-11-12 21:13:20 (+07:00), in 0 s"
        );
        assert_eq!(
            line(&read("1794492800000"), "unix-ms"),
            "2026-11-12 21:13:20 (+07:00), in 0 s"
        );
        assert_eq!(
            line(&read("1794492800000000"), "unix-us"),
            "2026-11-12 21:13:20 (+07:00), in 0 s"
        );
        assert_eq!(
            line(&read("1794492800000000000"), "unix-ns"),
            "2026-11-12 21:13:20 (+07:00), in 0 s"
        );
    }

    #[test]
    fn what_is_not_a_time_is_not_read_as_one() {
        for text in ["42", "0", "-1794492800", "1.5", "99999999", "abc"] {
            assert!(
                by_id(&read(text), "unix-s").is_none(),
                "{text} read as a time"
            );
        }
    }

    #[test]
    fn a_field_that_says_it_is_a_time_settles_it() {
        assert_eq!(
            by_id(&read("1794492800"), "unix-s").unwrap().confidence,
            Confidence::Likely
        );
        assert_eq!(
            by_id(&read_under("createdAt", "1794492800"), "unix-s")
                .unwrap()
                .confidence,
            Confidence::Certain
        );
    }

    #[test]
    fn a_quoted_number_is_still_a_number() {
        assert!(by_id(&read("\"1763000000\""), "unix-s").is_some());
    }

    #[test]
    fn iso_and_http_dates_are_read_where_the_reader_is() {
        assert_eq!(
            line(&read("2026-11-12T14:13:20Z"), "iso-8601"),
            "2026-11-12 21:13:20 (+07:00), in 0 s"
        );
        assert_eq!(
            line(&read("2026-11-12T14:13:20+00:00"), "iso-8601"),
            "2026-11-12 21:13:20 (+07:00), in 0 s"
        );
        assert_eq!(
            line(&read("2026-11-12 14:13:20"), "iso-8601-naive"),
            "2026-11-12 21:13:20 (+07:00), in 0 s"
        );
        assert_eq!(
            line(&read("Sun, 06 Nov 1994 08:49:37 GMT"), "http-date"),
            "1994-11-06 15:49:37 (+07:00), 32 y ago"
        );
    }

    #[test]
    fn times_read_from_now() {
        assert_eq!(relative(1_090, 1_000), "in 1 min");
        assert_eq!(relative(1_000 - 7_200, 1_000), "2 h ago");
        assert_eq!(relative(1_000 + 3 * 86_400, 1_000), "in 3 d");
        assert_eq!(relative(1_000, 1_000), "in 0 s");
        assert_eq!(relative(1_000 + 800 * 86_400, 1_000), "in 2 y");
    }

    #[test]
    fn the_other_epochs_are_read_where_they_cannot_be_mistaken() {
        // Large enough to be nothing else.
        assert!(line(&read("133756428000000000"), "filetime").starts_with("2024-11-"));
        assert!(line(&read("638669100000000000"), "dotnet-ticks").starts_with("2024-11-"));
        // These could be a Unix time, so they wait to be asked.
        assert!(by_id(&read("750000000"), "cocoa").is_none());
        assert!(line(&read_under("created_at", "750000000"), "cocoa").starts_with("2024-10-"));
        assert!(by_id(&read("3500000000"), "hfs").is_none());
        assert!(line(&read_under("modified_at", "3500000000"), "hfs").starts_with("2014-"));
        // Past where a Unix time reaches, it needs no asking.
        assert!(
            by_id(&read("4000000000"), "hfs").is_none(),
            "still a Unix time"
        );
        assert!(line(&read("4200000000"), "hfs").starts_with("2037-"));
        // A fraction says a spreadsheet wrote it; a whole number asks.
        assert!(by_id(&read("45678"), "excel-date").is_none());
        assert!(line(&read("45678.5"), "excel-date").starts_with("2025-01-"));
        assert!(line(&read_under("date", "45678"), "excel-date").starts_with("2025-01-"));
    }

    #[test]
    fn a_span_says_its_two_largest_units() {
        assert_eq!(span(45), "45s");
        assert_eq!(span(9_000), "2h 30m");
        assert_eq!(span(300), "5m");
        assert_eq!(span(100_000), "1d 3h");
        assert_eq!(span(0), "0s");
        assert_eq!(line(&read_under("max_age", "9000"), "duration"), "2h 30m");
    }

    #[test]
    fn a_field_that_says_it_is_a_span_lifts_the_reading() {
        assert_eq!(
            by_id(&read("3600"), "duration").unwrap().confidence,
            Confidence::Possible
        );
        assert_eq!(
            by_id(&read_under("expires_in", "3600"), "duration")
                .unwrap()
                .confidence,
            Confidence::Likely
        );
    }
}
