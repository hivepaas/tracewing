//! What a value means, said in words a reader knows.
//!
//! A flow is full of values that are faithful and unreadable: `1763000000`,
//! `eyJhbGciOiJIUzI1NiJ9…`, `H4sIAAAAAAAA…`. Each converter here takes some
//! selected text and, if it recognises it, says what it is: [`readings`] runs
//! them all and sorts what they found, best first.
//!
//! The crate is pure. It opens no file, holds no clock and reaches no network:
//! the time to read against and the reader's offset from UTC come in on
//! [`Input`], so a test in June is the test in December, and in Jakarta it is
//! the test in Berlin.
//!
//! See `docs/converting-values.md`.

use std::time::{Duration, Instant};

mod encodings;
mod ids;
mod json;
pub mod jwt;
mod net;
mod numbers;
mod packed;
mod times;
mod tracing;
mod web;

pub use times::{local_time_at, relative};

/// The most selected text a reading is tried on. Past this the answer would
/// not be a reading but a wait.
pub const MAX_INPUT: usize = 1 << 20;

/// The most any converter may give back, and the most that is unpacked.
pub const MAX_OUTPUT: usize = 16 << 20;

/// How deep one value may hold another: `base64 → gzip → JSON` is three.
pub const MAX_DEPTH: usize = 3;

/// How long [`readings`] may take. The menu it fills opens on a click.
pub const BUDGET: Duration = Duration::from_millis(12);

/// How sure a converter is that the value is what it read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    /// It fits, and so do others: ten digits that are also just a number.
    Possible,
    /// It fits, and the field it sits under agrees: `created_at` of 13 digits.
    Likely,
    /// It cannot be anything else: three parts that decode, carrying `alg`.
    Certain,
}

/// What a reading turned out to be, for a view that lays it out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Json,
    Xml,
    Text,
    Binary,
}

/// How a reading is drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shown {
    /// One line, after the value, dim.
    Line(String),
    /// Named parts, folded under the value as a token's claims are.
    Fields(Vec<(String, String)>),
    /// Rows, for what is a table: a query string, `Server-Timing`.
    Table {
        head: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    /// A body of its own, to open with Pretty, Raw and Hex. It is also fed
    /// back through the converters, which is how chains are found.
    Bytes { kind: Kind, bytes: Vec<u8> },
}

/// One reading of a value: what a converter made of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reading {
    /// Stable, and never translated: what a remembered rule and an MCP tool
    /// store. `"unix-ms"`, `"jwt"`, `"gzip"`.
    pub id: &'static str,
    /// Translated, for a menu: `"Unix milliseconds"`.
    pub label: String,
    pub confidence: Confidence,
    pub shown: Shown,
    /// The converters it went through to get here, this one last. One name
    /// for a value read at once, three for `base64 → gzip → JSON`.
    pub chain: Vec<&'static str>,
}

impl Reading {
    fn new(id: &'static str, label: String, confidence: Confidence, shown: Shown) -> Self {
        Self {
            id,
            label,
            confidence,
            shown,
            chain: Vec::new(),
        }
    }

    /// What the reading says, and nothing about how it was read.
    fn said(&self) -> String {
        let what = match &self.shown {
            Shown::Line(line) => line.clone(),
            Shown::Fields(fields) => fields
                .iter()
                .map(|(name, value)| format!("{name} {value}"))
                .collect::<Vec<_>>()
                .join(", "),
            Shown::Table { rows, .. } => tr_rows(rows.len()),
            Shown::Bytes { bytes, .. } => numbers::bytes_size(bytes.len() as u64),
        };
        cut(&what, 64)
    }

    /// The one line drawn beside the value. A guess says what read it as
    /// well: `10d 1h` beside a price must not look like a fact, and seeing
    /// `(Seconds)` is how a reader knows to pick another reading.
    pub fn beside(&self) -> String {
        match self.confidence {
            Confidence::Possible => self.summary(),
            _ => self.said(),
        }
    }

    /// The one line a menu shows: what it says, and what read it.
    pub fn summary(&self) -> String {
        format!("{}  ({})", self.said(), self.label)
    }

    /// The text `Copy reading` puts on the clipboard.
    pub fn text(&self) -> String {
        match &self.shown {
            Shown::Line(line) => line.clone(),
            Shown::Fields(fields) => fields
                .iter()
                .map(|(name, value)| format!("{name}: {value}"))
                .collect::<Vec<_>>()
                .join("\n"),
            Shown::Table { head, rows } => std::iter::once(head.join("\t"))
                .chain(rows.iter().map(|row| row.join("\t")))
                .collect::<Vec<_>>()
                .join("\n"),
            Shown::Bytes { bytes, .. } => match std::str::from_utf8(bytes) {
                Ok(text) => text.to_owned(),
                Err(_) => encodings::hex_dump(bytes),
            },
        }
    }
}

fn tr_rows(n: usize) -> String {
    tracewing_i18n::tr_n!(n as i64, "{n} row", "{n} rows")
}

/// `abc…` where text is longer than `max` characters.
fn cut(text: &str, max: usize) -> String {
    let one_line = text.replace(['\n', '\r', '\t'], " ");
    match one_line.char_indices().nth(max) {
        Some((at, _)) => format!("{}…", &one_line[..at]),
        None => one_line,
    }
}

/// Everything a converter may know. Nothing here comes from a global.
#[derive(Clone, Copy, Debug)]
pub struct Input<'a> {
    /// The selected text, trimmed of the space around it.
    pub text: &'a str,
    /// Its bytes, where the selection came from Hex or from a step before.
    pub bytes: Option<&'a [u8]>,
    /// The name the value sits under, where the view knows it: the JSON key,
    /// the header's name, the query field. It turns a guess into a reading.
    pub field: Option<&'a str>,
    /// Seconds since 1970 to read a time against, for `4 minutes ago`.
    pub now: i64,
    /// The reader's offset from UTC, in seconds east of it.
    pub offset: i32,
}

impl<'a> Input<'a> {
    pub fn new(text: &'a str, now: i64, offset: i32) -> Self {
        Self {
            text: text.trim(),
            bytes: None,
            field: None,
            now,
            offset,
        }
    }

    pub fn under(mut self, field: Option<&'a str>) -> Self {
        self.field = field;
        self
    }

    pub fn with_bytes(mut self, bytes: &'a [u8]) -> Self {
        self.bytes = Some(bytes);
        self
    }

    /// The bytes to read, whether they came as bytes or as text.
    pub fn raw(&self) -> &'a [u8] {
        self.bytes.unwrap_or(self.text.as_bytes())
    }

    /// The field's name with the spelling taken out of it: `created_at`,
    /// `createdAt` and `X-Created-At` all come out as `createdat`.
    fn flat_field(&self) -> Option<String> {
        Some(
            self.field?
                .chars()
                .filter(|c| c.is_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect(),
        )
    }

    /// Whether the field's name holds any of `words`.
    pub fn field_says(&self, words: &[&str]) -> bool {
        let Some(flat) = self.flat_field() else {
            return false;
        };
        words.iter().any(|word| flat.contains(word))
    }

    /// Whether the field's name ends in any of `words`. For the short ones:
    /// `at` in the middle of a name is `status` and `latency`, not a time.
    pub fn field_ends(&self, words: &[&str]) -> bool {
        let Some(flat) = self.flat_field() else {
            return false;
        };
        words.iter().any(|word| flat.ends_with(word))
    }
}

/// A converter: some text in, at most one reading out.
pub type Converter = fn(&Input) -> Option<Reading>;

/// Every converter, in the order a tie is broken.
const CONVERTERS: &[Converter] = &[
    jwt::read,
    times::unix,
    times::iso,
    times::http_date,
    times::filetime,
    times::dotnet_ticks,
    ids::uuid,
    ids::ulid,
    ids::object_id,
    ids::ksuid,
    ids::snowflake,
    tracing::traceparent,
    tracing::b3,
    tracing::amazon,
    tracing::server_timing,
    web::url,
    web::query,
    web::basic,
    web::cache_control,
    web::status,
    web::cookie,
    web::data_uri,
    net::ipv6,
    net::cidr,
    net::mac,
    json::in_string,
    json::whole,
    packed::gzip,
    packed::zlib,
    packed::deflate,
    packed::zstd,
    packed::brotli,
    encodings::base64,
    encodings::hex,
    encodings::percent,
    encodings::unicode_escapes,
    encodings::entities,
    net::ip_number,
    net::port,
    numbers::size,
    numbers::radix,
    numbers::flags,
    times::duration,
    times::hfs,
    times::cocoa,
    times::excel_date,
];

/// Every reading of `input`, best first, within [`BUDGET`].
pub fn readings(input: &Input) -> Vec<Reading> {
    readings_within(input, BUDGET)
}

/// The picture these bytes hold, if they hold one. A reading that turns out
/// to be a picture is worth looking at rather than reading.
pub fn picture(bytes: &[u8]) -> Option<&'static str> {
    encodings::named_format(bytes).filter(|name| matches!(*name, "PNG" | "JPEG" | "GIF" | "WebP"))
}

/// Whether `text` is worth running the converters over, by its shape alone.
///
/// A view with a thousand values on it asks this of each before it reads any,
/// so it has to be cheap and it has to let through everything a converter
/// could want: prose is what it turns away. It is a sieve, not an answer —
/// plenty of what passes reads as nothing.
pub fn could_read(text: &str) -> bool {
    let text = text.trim().trim_matches('"');
    if !(8..=MAX_INPUT).contains(&text.len()) {
        return false;
    }
    // Spaces mean words, except in the one time format written with them.
    !text.contains(char::is_whitespace) || text.ends_with(" GMT")
}

/// The reading `id` makes of `input`, if that is still what the value is.
///
/// A remembered rule names a reading rather than keeping it, so this finds it
/// again for the next value of that field. Where the value no longer reads
/// that way — a field that held milliseconds now holding a word — there is
/// nothing to draw, which is the honest answer and not a stale one.
pub fn read_as(id: &str, input: &Input) -> Option<Reading> {
    readings(input).into_iter().find(|found| found.id == id)
}

/// Every reading of `input`, best first, giving up after `budget`. A chain is
/// followed as far as [`MAX_DEPTH`], and no converter runs twice in one chain.
pub fn readings_within(input: &Input, budget: Duration) -> Vec<Reading> {
    if input.text.len() > MAX_INPUT || input.raw().len() > MAX_INPUT {
        return Vec::new();
    }
    let mut found = Vec::new();
    collect(input, &mut Vec::new(), &mut found, Instant::now() + budget);
    // Best first: what is certain over what is merely possible, and what was
    // read at once over what took three steps to reach.
    found.sort_by_key(|r| (std::cmp::Reverse(r.confidence), r.chain.len()));
    found.dedup_by(|a, b| a.id == b.id && a.shown == b.shown);
    found
}

fn collect(
    input: &Input,
    chain: &mut Vec<&'static str>,
    found: &mut Vec<Reading>,
    deadline: Instant,
) {
    for convert in CONVERTERS {
        if Instant::now() >= deadline {
            return;
        }
        let Some(mut reading) = convert(input) else {
            continue;
        };
        // A chain that comes back to where it was says nothing new.
        if chain.contains(&reading.id) {
            continue;
        }
        reading.chain = chain.clone();
        reading.chain.push(reading.id);
        if let Shown::Bytes { bytes, .. } = &reading.shown
            && chain.len() + 1 < MAX_DEPTH
        {
            let text = std::str::from_utf8(bytes).unwrap_or("").trim();
            let inside = Input {
                text,
                bytes: Some(bytes),
                field: input.field,
                now: input.now,
                offset: input.offset,
            };
            chain.push(reading.id);
            collect(&inside, chain, found, deadline);
            chain.pop();
        }
        found.push(reading);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-11-12 14:13:20 UTC, and a reader seven hours east of it.
    pub const NOW: i64 = 1_794_492_800;
    pub const TZ: i32 = 7 * 3600;

    pub fn read(text: &str) -> Vec<Reading> {
        readings(&Input::new(text, NOW, TZ))
    }

    pub fn read_under(field: &str, text: &str) -> Vec<Reading> {
        readings(&Input::new(text, NOW, TZ).under(Some(field)))
    }

    /// The first reading a given converter gave, if any.
    pub fn by_id<'a>(readings: &'a [Reading], id: &str) -> Option<&'a Reading> {
        readings.iter().find(|r| r.id == id)
    }

    pub fn line(readings: &[Reading], id: &str) -> String {
        match by_id(readings, id).map(|r| &r.shown) {
            Some(Shown::Line(line)) => line.clone(),
            other => panic!("no line for {id}: {other:?}"),
        }
    }

    #[test]
    fn nothing_is_read_out_of_nothing() {
        assert!(read("").is_empty());
        assert!(read("hello").is_empty());
        assert!(read("   ").is_empty());
    }

    #[test]
    fn a_selection_too_large_is_left_alone() {
        let huge = "a".repeat(MAX_INPUT + 1);
        assert!(read(&huge).is_empty());
    }

    #[test]
    fn the_best_reading_comes_first() {
        let found = read_under("created_at", "1794492800");
        assert_eq!(found[0].id, "unix-s");
        assert_eq!(found[0].confidence, Confidence::Certain);
    }

    #[test]
    fn a_field_name_is_read_however_it_is_written() {
        let input = Input::new("1", 0, 0);
        assert!(input.under(Some("createdAt")).field_says(&["created"]));
        assert!(input.under(Some("CREATED_AT")).field_says(&["created"]));
        assert!(input.under(Some("X-Created-At")).field_says(&["created"]));
        assert!(!input.under(Some("creator")).field_says(&["created"]));
        assert!(!input.field_says(&["created"]));
        assert!(input.under(Some("created_at")).field_ends(&["at"]));
        assert!(!input.under(Some("status")).field_ends(&["at"]));
    }

    #[test]
    fn a_chain_says_every_step_it_took() {
        // `{"exp":1794492800}` gzipped, then base64: three steps to the JSON.
        let json = br#"{"exp":1794492800}"#;
        let packed = {
            use flate2::{Compression, write::GzEncoder};
            use std::io::Write;
            let mut out = GzEncoder::new(Vec::new(), Compression::fast());
            out.write_all(json).unwrap();
            out.finish().unwrap()
        };
        let text = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &packed);
        let found = read(&text);
        let json = found
            .iter()
            .find(|r| r.id == "json")
            .expect("the JSON inside is read");
        assert_eq!(json.chain, ["base64", "gzip", "json"]);
    }

    #[test]
    fn a_chain_stops_before_it_goes_round() {
        // Hex of hex of hex of … is not a reading worth having.
        let found = read("36383635");
        assert!(found.iter().all(|r| r.chain.len() <= MAX_DEPTH));
        assert!(found.iter().all(|r| {
            let mut seen = r.chain.clone();
            seen.sort_unstable();
            let was = seen.len();
            seen.dedup();
            seen.len() == was
        }));
    }

    #[test]
    fn a_reading_says_itself_beside_the_value_and_in_a_menu() {
        let found = read_under("exp", "1794492800");
        let reading = by_id(&found, "unix-s").unwrap();
        assert_eq!(reading.beside(), "2026-11-12 21:13:20 (+07:00), in 0 s");
        assert_eq!(
            reading.summary(),
            "2026-11-12 21:13:20 (+07:00), in 0 s  (Unix seconds)"
        );
        // What runs long is cut to the width a line can spare.
        let token = read(&format!("eyJhbGciOiJIUzI1NiJ9.{}.sig", "eyJzdWIiOiJhIn0"));
        if let Some(jwt) = by_id(&token, "jwt") {
            assert!(jwt.beside().chars().count() <= 65, "{}", jwt.beside());
        }
    }

    #[test]
    fn a_guess_drawn_beside_a_value_says_it_is_a_guess() {
        // A price is not a span of seconds, and `10d 1h` beside it would read
        // as though it were.
        let found = read("868617");
        let span = by_id(&found, "duration").unwrap();
        assert_eq!(span.confidence, Confidence::Possible);
        assert_eq!(span.beside(), "10d 1h  (Seconds)");
        assert_ne!(
            found[0].id, "duration",
            "a guess about a span leads nothing"
        );
        // What is certain says itself and no more.
        let certain = read_under("created_at", "1794492800");
        let time = by_id(&certain, "unix-s").unwrap();
        assert_eq!(time.confidence, Confidence::Certain);
        assert!(!time.beside().contains("(Unix seconds)"));
    }

    #[test]
    fn a_picture_is_known_from_its_first_bytes() {
        assert_eq!(picture(b"\x89PNG\r\n\x1a\n"), Some("PNG"));
        assert_eq!(picture(&[0xff, 0xd8, 0xff, 0xe0]), Some("JPEG"));
        assert_eq!(picture(b"GIF89a"), Some("GIF"));
        assert_eq!(picture(b"{\"a\":1}"), None);
        assert_eq!(picture(&[0x1f, 0x8b, 0, 0]), None, "gzip is not a picture");
        assert_eq!(picture(b""), None);
    }

    #[test]
    fn prose_is_turned_away_before_anything_is_read() {
        assert!(could_read("1794492800"));
        assert!(could_read("eyJhbGciOiJIUzI1NiJ9.eyJhIjoxfQ.sig"));
        assert!(could_read("\"2026-11-12T14:13:20Z\""));
        assert!(could_read("Thu, 12 Nov 2026 15:20:02 GMT"));
        assert!(!could_read("42"));
        assert!(!could_read("Chi Pham"));
        assert!(!could_read("the quick brown fox"));
        assert!(!could_read(""));
        assert!(!could_read(&"a".repeat(MAX_INPUT + 1)));
        // What it lets through is not promised to read as anything.
        assert!(could_read("aaaaaaaaaa"));
    }

    #[test]
    fn a_remembered_reading_is_found_again() {
        let input = Input::new("1794492800000", NOW, TZ).under(Some("created_at"));
        let found = read_as("unix-ms", &input).unwrap();
        assert_eq!(found.beside(), "2026-11-12 21:13:20 (+07:00), in 0 s");
        // A chain is named by where it ends, and found again whole.
        let packed = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            br#"{"a":1,"b":2}"#,
        );
        let inside = Input::new(&packed, NOW, TZ);
        let json = read_as("json", &inside).unwrap();
        assert_eq!(json.chain, ["base64", "json"]);
        // And what the value is not reads as nothing at all.
        assert!(read_as("unix-ms", &Input::new("a word", NOW, TZ)).is_none());
        assert!(read_as("no-such-reading", &input).is_none());
    }

    #[test]
    fn a_reading_copies_as_text() {
        let found = read_under("exp", "1794492800");
        let reading = by_id(&found, "unix-s").unwrap();
        assert_eq!(reading.text(), "2026-11-12 21:13:20 (+07:00), in 0 s");
        assert!(reading.summary().ends_with("(Unix seconds)"));
    }
}
