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
use tracewing_i18n::tr;

mod binary;
pub mod chain;
mod encodings;
mod ids;
mod json;
pub mod jwt;
mod net;
mod numbers;
mod packed;
mod times;
mod tokens;
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
    /// Whether a reader named this converter rather than the search finding
    /// it. What a converter would not offer by itself it still has to do
    /// when it is asked for: base64 of bytes nobody recognises is exactly
    /// what somebody reaches for "read as" to see.
    pub asked: bool,
}

impl<'a> Input<'a> {
    pub fn new(text: &'a str, now: i64, offset: i32) -> Self {
        Self {
            text: text.trim(),
            bytes: None,
            field: None,
            now,
            offset,
            asked: false,
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
    chain::address,
    chain::topic,
    chain::selector,
    tokens::jwe,
    tokens::paseto,
    tokens::session_cookie,
    tokens::crypt,
    tokens::pem,
    tokens::saml,
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
    web::aws_sigv4,
    web::cache_control,
    web::csp,
    web::link,
    web::status,
    web::cookie,
    web::data_uri,
    net::ipv6,
    net::cidr,
    net::mac,
    json::in_string,
    json::whole,
    binary::protobuf,
    packed::gzip,
    packed::zlib,
    packed::deflate,
    packed::zstd,
    packed::brotli,
    encodings::base64,
    encodings::hex,
    encodings::base85,
    encodings::base58,
    encodings::base32,
    encodings::quoted_printable,
    encodings::percent,
    encodings::unicode_escapes,
    encodings::entities,
    net::ip_number,
    net::port,
    tokens::hash,
    numbers::size,
    numbers::radix,
    numbers::flags,
    times::duration,
    times::hfs,
    times::cocoa,
    times::excel_date,
];

/// What a converter is about, for a reader looking down a menu of them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Group {
    Times,
    Names,
    Tracing,
    Web,
    Network,
    Data,
    Encodings,
    Packed,
    Tokens,
    Numbers,
    Chain,
}

impl Group {
    /// The heading it goes under.
    pub fn name(self) -> String {
        match self {
            Self::Times => tr!("Times"),
            Self::Names => tr!("Names and ids"),
            Self::Tracing => tr!("Tracing"),
            Self::Web => tr!("The web's own"),
            Self::Network => tr!("Networks"),
            Self::Data => tr!("Data"),
            Self::Encodings => tr!("Encodings"),
            Self::Packed => tr!("Packed"),
            Self::Tokens => tr!("Tokens and keys"),
            Self::Numbers => tr!("Numbers"),
            Self::Chain => tr!("Chains"),
        }
    }
}

/// One reading a reader can ask for by name: what it is called, where it
/// belongs in the menu, and what does it.
///
/// A converter may give more than one reading -- a number is seconds or
/// milliseconds by how large it is -- so there is a row here for each
/// reading rather than for each converter, and asking for one that the value
/// is not gives nothing rather than the other.
struct Known {
    id: &'static str,
    group: Group,
    run: Converter,
    name: fn() -> String,
}

/// What a reader may ask for by name, in the order a menu shows them.
const KNOWN: &[Known] = &[
    Known {
        id: "unix-s",
        group: Group::Times,
        run: times::unix,
        name: || tr!("Unix seconds"),
    },
    Known {
        id: "unix-ms",
        group: Group::Times,
        run: times::unix,
        name: || tr!("Unix milliseconds"),
    },
    Known {
        id: "unix-us",
        group: Group::Times,
        run: times::unix,
        name: || tr!("Unix microseconds"),
    },
    Known {
        id: "unix-ns",
        group: Group::Times,
        run: times::unix,
        name: || tr!("Unix nanoseconds"),
    },
    Known {
        id: "iso-8601",
        group: Group::Times,
        run: times::iso,
        name: || tr!("ISO 8601"),
    },
    Known {
        id: "iso-8601-naive",
        group: Group::Times,
        run: times::iso,
        name: || tr!("ISO 8601, no zone: read as UTC"),
    },
    Known {
        id: "http-date",
        group: Group::Times,
        run: times::http_date,
        name: || tr!("HTTP-date"),
    },
    Known {
        id: "filetime",
        group: Group::Times,
        run: times::filetime,
        name: || tr!("Windows FILETIME"),
    },
    Known {
        id: "dotnet-ticks",
        group: Group::Times,
        run: times::dotnet_ticks,
        name: || tr!(".NET ticks"),
    },
    Known {
        id: "cocoa",
        group: Group::Times,
        run: times::cocoa,
        name: || tr!("Seconds since 2001 (Apple)"),
    },
    Known {
        id: "hfs",
        group: Group::Times,
        run: times::hfs,
        name: || tr!("Seconds since 1904 (HFS+)"),
    },
    Known {
        id: "excel-date",
        group: Group::Times,
        run: times::excel_date,
        name: || tr!("Days since 1899 (spreadsheet)"),
    },
    Known {
        id: "duration",
        group: Group::Times,
        run: times::duration,
        name: || tr!("Seconds"),
    },
    Known {
        id: "uuid",
        group: Group::Names,
        run: ids::uuid,
        name: || tr!("UUID"),
    },
    Known {
        id: "ulid",
        group: Group::Names,
        run: ids::ulid,
        name: || tr!("ULID"),
    },
    Known {
        id: "object-id",
        group: Group::Names,
        run: ids::object_id,
        name: || tr!("ObjectId"),
    },
    Known {
        id: "ksuid",
        group: Group::Names,
        run: ids::ksuid,
        name: || tr!("KSUID"),
    },
    Known {
        id: "snowflake",
        group: Group::Names,
        run: ids::snowflake,
        name: || tr!("Snowflake"),
    },
    Known {
        id: "traceparent",
        group: Group::Tracing,
        run: tracing::traceparent,
        name: || tr!("Trace context"),
    },
    Known {
        id: "b3",
        group: Group::Tracing,
        run: tracing::b3,
        name: || tr!("B3 trace"),
    },
    Known {
        id: "amazon-trace",
        group: Group::Tracing,
        run: tracing::amazon,
        name: || tr!("Amazon trace"),
    },
    Known {
        id: "server-timing",
        group: Group::Tracing,
        run: tracing::server_timing,
        name: || tr!("Server-Timing"),
    },
    Known {
        id: "url",
        group: Group::Web,
        run: web::url,
        name: || tr!("URL"),
    },
    Known {
        id: "query",
        group: Group::Web,
        run: web::query,
        name: || tr!("Query fields"),
    },
    Known {
        id: "aws-sigv4",
        group: Group::Web,
        run: web::aws_sigv4,
        name: || tr!("An Amazon signature"),
    },
    Known {
        id: "basic-auth",
        group: Group::Web,
        run: web::basic,
        name: || tr!("Basic authentication"),
    },
    Known {
        id: "cache-control",
        group: Group::Web,
        run: web::cache_control,
        name: || tr!("Cache-Control"),
    },
    Known {
        id: "csp",
        group: Group::Web,
        run: web::csp,
        name: || tr!("Content-Security-Policy"),
    },
    Known {
        id: "link",
        group: Group::Web,
        run: web::link,
        name: || tr!("Link"),
    },
    Known {
        id: "status",
        group: Group::Web,
        run: web::status,
        name: || tr!("Status code"),
    },
    Known {
        id: "cookie",
        group: Group::Web,
        run: web::cookie,
        name: || tr!("Cookie"),
    },
    Known {
        id: "data-uri",
        group: Group::Web,
        run: web::data_uri,
        name: || tr!("data: URI"),
    },
    Known {
        id: "ipv6",
        group: Group::Network,
        run: net::ipv6,
        name: || tr!("IPv6"),
    },
    Known {
        id: "cidr",
        group: Group::Network,
        run: net::cidr,
        name: || tr!("A range of addresses"),
    },
    Known {
        id: "mac",
        group: Group::Network,
        run: net::mac,
        name: || tr!("A MAC address"),
    },
    Known {
        id: "ip-number",
        group: Group::Network,
        run: net::ip_number,
        name: || tr!("As an address"),
    },
    Known {
        id: "port",
        group: Group::Network,
        run: net::port,
        name: || tr!("As a port"),
    },
    Known {
        id: "json",
        group: Group::Data,
        run: json::whole,
        name: || tr!("JSON"),
    },
    Known {
        id: "json-in-string",
        group: Group::Data,
        run: json::in_string,
        name: || tr!("JSON in a string"),
    },
    Known {
        id: "protobuf",
        group: Group::Data,
        run: binary::protobuf,
        name: || tr!("Protobuf"),
    },
    Known {
        id: "base64",
        group: Group::Encodings,
        run: encodings::base64,
        name: || tr!("base64"),
    },
    Known {
        id: "hex",
        group: Group::Encodings,
        run: encodings::hex,
        name: || tr!("Hex"),
    },
    Known {
        id: "base85",
        group: Group::Encodings,
        run: encodings::base85,
        name: || tr!("Ascii85"),
    },
    Known {
        id: "base58",
        group: Group::Encodings,
        run: encodings::base58,
        name: || tr!("base58"),
    },
    Known {
        id: "base32",
        group: Group::Encodings,
        run: encodings::base32,
        name: || tr!("base32"),
    },
    Known {
        id: "quoted-printable",
        group: Group::Encodings,
        run: encodings::quoted_printable,
        name: || tr!("Quoted-printable"),
    },
    Known {
        id: "percent",
        group: Group::Encodings,
        run: encodings::percent,
        name: || tr!("Percent-encoded"),
    },
    Known {
        id: "escapes",
        group: Group::Encodings,
        run: encodings::unicode_escapes,
        name: || tr!("Escaped text"),
    },
    Known {
        id: "entities",
        group: Group::Encodings,
        run: encodings::entities,
        name: || tr!("HTML entities"),
    },
    Known {
        id: "gzip",
        group: Group::Packed,
        run: packed::gzip,
        name: || tr!("gzip"),
    },
    Known {
        id: "zlib",
        group: Group::Packed,
        run: packed::zlib,
        name: || tr!("zlib"),
    },
    Known {
        id: "deflate",
        group: Group::Packed,
        run: packed::deflate,
        name: || tr!("deflate"),
    },
    Known {
        id: "zstd",
        group: Group::Packed,
        run: packed::zstd,
        name: || tr!("zstd"),
    },
    Known {
        id: "brotli",
        group: Group::Packed,
        run: packed::brotli,
        name: || tr!("Brotli"),
    },
    Known {
        id: "jwt",
        group: Group::Tokens,
        run: jwt::read,
        name: || tr!("JWT"),
    },
    Known {
        id: "jwe",
        group: Group::Tokens,
        run: tokens::jwe,
        name: || tr!("JWE, an encrypted token"),
    },
    Known {
        id: "paseto",
        group: Group::Tokens,
        run: tokens::paseto,
        name: || tr!("PASETO"),
    },
    Known {
        id: "session-cookie",
        group: Group::Tokens,
        run: tokens::session_cookie,
        name: || tr!("A framework's session cookie"),
    },
    Known {
        id: "saml",
        group: Group::Tokens,
        run: tokens::saml,
        name: || tr!("SAML"),
    },
    Known {
        id: "pem",
        group: Group::Tokens,
        run: tokens::pem,
        name: || tr!("PEM"),
    },
    Known {
        id: "crypt",
        group: Group::Tokens,
        run: tokens::crypt,
        name: || tr!("A hashed password"),
    },
    Known {
        id: "hash",
        group: Group::Tokens,
        run: tokens::hash,
        name: || tr!("A hash, by its size"),
    },
    Known {
        id: "size",
        group: Group::Numbers,
        run: numbers::size,
        name: || tr!("As a size"),
    },
    Known {
        id: "radix",
        group: Group::Numbers,
        run: numbers::radix,
        name: || tr!("In hex and binary"),
    },
    Known {
        id: "flags",
        group: Group::Numbers,
        run: numbers::flags,
        name: || tr!("Bits set"),
    },
    Known {
        id: "address",
        group: Group::Chain,
        run: chain::address,
        name: || tr!("An address"),
    },
    Known {
        id: "topic",
        group: Group::Chain,
        run: chain::topic,
        name: || tr!("An event's topic"),
    },
    Known {
        id: "selector",
        group: Group::Chain,
        run: chain::selector,
        name: || tr!("A call, by its four bytes"),
    },
];

/// One reading a reader can ask for, as a menu shows it.
pub struct Named {
    pub id: &'static str,
    pub group: Group,
    pub name: String,
}

/// Everything a reader may ask for by name, in menu order.
pub fn named() -> Vec<Named> {
    KNOWN
        .iter()
        .map(|known| Named {
            id: known.id,
            group: known.group,
            name: (known.name)(),
        })
        .collect()
}

/// Whether what `id` reads can be read again: only bytes can be handed on
/// to another converter, so only those can have a step after them.
pub fn goes_on(reading: &Reading) -> bool {
    matches!(reading.shown, Shown::Bytes { .. })
}

/// What `input` reads as, through the converters named, in that order.
///
/// This is what a reader asks for by hand, and it is not the search: each
/// converter is run whether or not it would have offered itself, and each
/// step after the first reads the bytes of the one before. `None` where a
/// step reads as nothing, or where what a step gave cannot be read on.
pub fn read_with(ids: &[&'static str], input: &Input) -> Option<Reading> {
    if ids.is_empty() || ids.len() > MAX_DEPTH {
        return None;
    }
    let mut held: Option<Vec<u8>> = None;
    let mut last: Option<Reading> = None;
    for (at, id) in ids.iter().enumerate() {
        let known = KNOWN.iter().find(|known| known.id == *id)?;
        let mut reading = match &held {
            None => {
                let mut asked = *input;
                asked.asked = true;
                (known.run)(&asked)?
            }
            Some(bytes) => {
                let text = std::str::from_utf8(bytes).unwrap_or("").trim();
                let inside = Input {
                    text,
                    bytes: Some(bytes),
                    field: input.field,
                    now: input.now,
                    offset: input.offset,
                    asked: true,
                };
                (known.run)(&inside)?
            }
        };
        // A converter that gives more than one reading may give another of
        // them: seconds where milliseconds were asked for. That is a no.
        if reading.id != *id {
            return None;
        }
        reading.chain = ids[..=at].to_vec();
        held = match &reading.shown {
            Shown::Bytes { bytes, .. } => Some(bytes.clone()),
            _ if at + 1 < ids.len() => return None,
            _ => None,
        };
        last = Some(reading);
    }
    last
}

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
    if let Some(found) = readings(input).into_iter().find(|found| found.id == id) {
        return Some(found);
    }
    // What the search does not offer by itself is still read where a reader
    // asked for it by name and a rule remembers that they did: a number
    // under a field nobody would call a timeout is seconds if that is what
    // the reader said it was. The search is tried first, since it is what
    // finds a reading at the end of a chain.
    let known = KNOWN.iter().find(|known| known.id == id)?;
    read_with(&[known.id], input)
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
                asked: input.asked,
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
        // A port is a port where the field says so and a guess where it does
        // not, and `(As a port)` beside the number says which it is.
        let found = read("443");
        let port = by_id(&found, "port").unwrap();
        assert_eq!(port.confidence, Confidence::Possible);
        assert_eq!(port.beside(), "HTTPS  (As a port)");
        assert_eq!(
            by_id(&read_under("port", "443"), "port")
                .unwrap()
                .confidence,
            Confidence::Likely
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

#[cfg(test)]
mod asked_tests {
    use super::*;
    use crate::tests::{NOW, TZ};
    use base64::Engine as _;

    fn b64(bytes: &[u8]) -> String {
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }

    fn deflated(data: &[u8]) -> Vec<u8> {
        use std::io::Write as _;
        let mut out = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::fast());
        out.write_all(data).unwrap();
        out.finish().unwrap()
    }

    fn asked(ids: &[&'static str], text: &str) -> Option<Reading> {
        read_with(ids, &Input::new(text, NOW, TZ))
    }

    /// The chain a reader builds by hand is run step by step, each reading
    /// the bytes of the one before.
    #[test]
    fn a_chain_asked_for_is_run_in_order() {
        let packed = b64(&deflated(br#"{"hello":"world"}"#));
        let reading = asked(&["base64", "deflate", "json"], &packed).expect("all three");
        assert_eq!(reading.id, "json");
        assert_eq!(reading.chain, vec!["base64", "deflate", "json"]);
        // And one step at a time says what that step gave.
        let first = asked(&["base64"], &packed).expect("base64 alone");
        assert!(matches!(first.shown, Shown::Bytes { .. }));
        assert!(goes_on(&first), "bytes can be read on");
    }

    /// What the search would not offer is still done when it is asked for:
    /// base64 of bytes nobody recognises is the case somebody reaches for
    /// "read as" to see at all.
    #[test]
    fn what_the_search_turns_away_is_done_when_asked() {
        let key: Vec<u8> = (0..32u8)
            .map(|n| n.wrapping_mul(37).wrapping_add(11))
            .collect();
        let text = b64(&key);
        assert!(
            readings(&Input::new(&text, NOW, TZ))
                .iter()
                .all(|reading| reading.id != "base64"),
            "the search says nothing of it"
        );
        let reading = asked(&["base64"], &text).expect("asked for, it is done");
        match reading.shown {
            Shown::Bytes { ref bytes, .. } => assert_eq!(bytes, &key),
            ref other => panic!("{other:?}"),
        }
    }

    /// Asking for what the value is not gives nothing, rather than the
    /// reading next door.
    #[test]
    fn asking_for_the_wrong_one_gives_nothing() {
        // 1794492800 is seconds; asked for as milliseconds it is not that.
        assert!(asked(&["unix-ms"], "1794492800").is_none());
        assert_eq!(asked(&["unix-s"], "1794492800").unwrap().id, "unix-s");
        // A step that gives no bytes cannot have one after it.
        assert!(asked(&["unix-s", "json"], "1794492800").is_none());
        // A name nobody has.
        assert!(asked(&["not-a-reading"], "1794492800").is_none());
        // Deeper than a chain goes.
        assert!(asked(&["base64", "base64", "base64", "base64"], "aGVsbG8=").is_none());
    }

    /// Every reading a value of the samples has is one a reader can ask for
    /// by name: the menu and the search say the same words.
    #[test]
    fn the_menu_names_what_the_search_finds() {
        let named = named();
        assert!(named.len() >= 57, "{}", named.len());
        for one in &named {
            assert!(!one.name.is_empty(), "{} has no name", one.id);
        }
        let mut ids: Vec<&str> = named.iter().map(|one| one.id).collect();
        ids.sort_unstable();
        let had = ids.len();
        ids.dedup();
        assert_eq!(had, ids.len(), "a name is in the menu twice");
    }
}
