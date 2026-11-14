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
///
/// Only where the field says the number is a span. Nothing about a number
/// says it is seconds -- every number is seconds if nobody says otherwise --
/// so offering this for all of them put `7m 23s` beside the `443` of a
/// `host:443` and beside every id, count and status in the body. A reader
/// who means it still has it by name, under `Read as`.
pub fn duration(input: &Input) -> Option<Reading> {
    let n = whole(input.text)?;
    // Past ten years it is not a timeout anybody set.
    if n == 0 || n > 315_360_000 {
        return None;
    }
    let said = input.field_says(SAYS_SPAN) || input.field_ends(ENDS_SPAN);
    if !said && !input.asked {
        return None;
    }
    Some(Reading::new(
        "duration",
        tr!("Seconds"),
        // Asked for by name, it is read whatever the field is called; the
        // confidence still says which of the two happened.
        match said {
            true => Confidence::Likely,
            false => Confidence::Possible,
        },
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

/// An ISO 8601 duration -- `PT1H30M`, `P3D`, `P1Y2M10DT2H30M` -- as a span a
/// reader can take in. A configuration value written this way is common and
/// unreadable in equal measure, and `PT5400S` saying `1h 30m` is the whole
/// point of reading it at all.
///
/// `M` is months before the `T` and minutes after it, which is the mistake
/// this format is famous for, so the reading says `1mo` for one and `1m` for
/// the other and never silently turns one into the other.
///
/// Years and months are left as years and months. A month is not a number of
/// seconds -- how many depends on which month, and on where the duration
/// starts, which a value in isolation does not say -- so only the part below
/// days is added up. The rest of the format is exact: a week is seven days
/// and a day is 86400 seconds, whatever the calendar does around them.
pub fn iso_duration(input: &Input) -> Option<Reading> {
    let span = Iso8601Duration::parse(input.text.trim().trim_matches('"'))?;
    Some(Reading::new(
        "iso-8601-duration",
        tr!("ISO 8601 duration"),
        Confidence::Certain,
        Shown::Line(span.said()),
    ))
}

/// The parts of one, kept as they were written: years and months apart from
/// the seconds, because they cannot be added to them.
struct Iso8601Duration {
    negative: bool,
    months: f64,
    seconds: f64,
}

impl Iso8601Duration {
    fn parse(text: &str) -> Option<Self> {
        // The format has no letters but its own, so one case is every case.
        let text = text.to_ascii_uppercase();
        let (negative, rest) = match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text.strip_prefix('+').unwrap_or(&text)),
        };
        let rest = rest.strip_prefix('P')?;
        let (date, time) = match rest.split_once('T') {
            Some((date, time)) => (date, time),
            None => (rest, ""),
        };
        let date = pairs(date, "YMWD")?;
        let time = pairs(time, "HMS")?;
        // `P` on its own, and `PT` on its own, are not durations.
        if date.is_empty() && time.is_empty() {
            return None;
        }
        let mut months = 0.0;
        let mut seconds = 0.0;
        for (unit, many) in date {
            match unit {
                'Y' => months += many * 12.0,
                'M' => months += many,
                'W' => seconds += many * 7.0 * 86_400.0,
                _ => seconds += many * 86_400.0,
            }
        }
        for (unit, many) in time {
            match unit {
                'H' => seconds += many * 3_600.0,
                'M' => seconds += many * 60.0,
                _ => seconds += many,
            }
        }
        Some(Self {
            negative,
            months,
            seconds,
        })
    }

    /// The span in words, the date part as itself and the rest added up.
    fn said(&self) -> String {
        let mut said = Vec::new();
        let years = (self.months / 12.0).floor();
        if years > 0.0 {
            said.push(format!("{}y", rounded(years)));
        }
        let months = self.months - years * 12.0;
        if months > 0.0 {
            said.push(format!("{}mo", rounded(months)));
        }
        let mut left = self.seconds;
        for (size, unit) in [(86_400.0, "d"), (3_600.0, "h"), (60.0, "m")] {
            let many = (left / size).floor();
            if many > 0.0 {
                said.push(format!("{}{unit}", rounded(many)));
                left -= many * size;
            }
        }
        // Seconds last, and the only place a fraction is left: `PT0.5S` is a
        // real thing to write and rounding it away would be a lie. Nothing
        // at all becomes `0s`, since a duration of zero is still a duration.
        if left > 0.0 || said.is_empty() {
            said.push(format!("{}s", rounded(left)));
        }
        match self.negative {
            true => format!("-{}", said.join(" ")),
            false => said.join(" "),
        }
    }
}

/// Three decimal places at most, and no trailing zeros: `0.5`, not `0.500`.
fn rounded(many: f64) -> String {
    let rounded = (many * 1_000.0).round() / 1_000.0;
    format!("{rounded}")
}

/// The `number designator` pairs of one half of a duration, left to right.
///
/// A designator may be used once and never after one that comes later in
/// `order`, so `P1M2Y` and `P1Y2Y` are both refused. That, with a number
/// required in front of every designator and a designator after every
/// number, is what keeps a word beginning with P out of here.
fn pairs(text: &str, order: &str) -> Option<Vec<(char, f64)>> {
    let mut found: Vec<(char, f64)> = Vec::new();
    let mut least = 0;
    let mut rest = text;
    while !rest.is_empty() {
        // A number with no designator after it ends the string, which `find`
        // reports as nothing found; a designator with no number is at zero.
        let end = rest
            .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == ','))
            .filter(|end| *end > 0)?;
        // A comma is a decimal point in ISO 8601, and some writers use it.
        let many: f64 = rest[..end].replace(',', ".").parse().ok()?;
        if !many.is_finite() || many < 0.0 {
            return None;
        }
        let unit = rest[end..].chars().next()?;
        let at = order.find(unit).filter(|at| *at >= least)?;
        least = at + 1;
        found.push((unit, many));
        rest = &rest[end + unit.len_utf8()..];
    }
    Some(found)
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
    fn a_span_is_read_where_the_field_says_so_and_nowhere_else() {
        // Nothing about a number says it is seconds, so a bare one is not
        // offered as a span at all: `7m 23s` beside the 443 of a host and
        // beside every id and count in a body is noise, not a reading.
        assert!(by_id(&read("3600"), "duration").is_none());
        assert_eq!(
            by_id(&read_under("expires_in", "3600"), "duration")
                .unwrap()
                .confidence,
            Confidence::Likely
        );
        assert_eq!(
            by_id(&read_under("took_secs", "3600"), "duration")
                .unwrap()
                .confidence,
            Confidence::Likely
        );
        // Asked for by name -- from the pane, or by a rule that remembers a
        // reader asking once -- it is read whatever the field is called, and
        // says that it is a guess.
        let input = crate::Input::new("3600", crate::tests::NOW, crate::tests::TZ);
        let asked = crate::read_as("duration", &input).expect("asked for, it is read");
        assert_eq!(asked.confidence, Confidence::Possible);
        assert_eq!(line(&[asked], "duration"), "1h");
    }
    fn iso_span(text: &str) -> String {
        line(&read(text), "iso-8601-duration")
    }

    /// The reason to read one at all: the same length of time can be written
    /// several ways and only one of them is readable.
    #[test]
    fn an_iso_duration_is_said_as_a_span() {
        assert_eq!(iso_span("PT1H30M"), "1h 30m");
        assert_eq!(iso_span("PT5400S"), "1h 30m");
        assert_eq!(iso_span("PT90M"), "1h 30m");
        assert_eq!(iso_span("P1Y2M10DT2H30M"), "1y 2mo 10d 2h 30m");
        // A week is seven days exactly, whatever the calendar does.
        assert_eq!(iso_span("P1W"), "7d");
        // Lower case, a sign, a comma for a decimal point, and a zero that
        // is still a duration.
        assert_eq!(iso_span("pt1h"), "1h");
        assert_eq!(iso_span("-PT1H"), "-1h");
        assert_eq!(iso_span("PT0,5S"), "0.5s");
        assert_eq!(iso_span("PT0S"), "0s");
    }

    /// `M` is months before the `T` and minutes after it, which is the one
    /// thing this format is famous for getting wrong.
    #[test]
    fn a_month_and_a_minute_are_never_confused() {
        assert_eq!(iso_span("P1M"), "1mo");
        assert_eq!(iso_span("PT1M"), "1m");
        // And a month stays a month: how many seconds it is depends on which
        // month and on where the duration starts, neither of which a value
        // on its own says.
        assert_eq!(iso_span("P14M"), "1y 2mo");
    }

    #[test]
    fn what_is_not_an_iso_duration_is_not_read() {
        for text in [
            // A P and nothing to go with it.
            "P", "PT", "PASSWORD", "P2P",
            // A number with no designator, and a designator with no number.
            "P1", "PY", "P1D2", // A designator that is not one of this half's.
            "PT1X", "P1H", // Out of order, and said twice.
            "P1M2Y", "P1Y2Y", "PT1M2H",
        ] {
            assert!(
                by_id(&read(text), "iso-8601-duration").is_none(),
                "read {text} as a duration"
            );
        }
    }
}
