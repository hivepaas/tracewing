# Converting values

The design, and where it stands. Built: `crates/convert` — the registry, the
phase one converters and their tests — the readings in the body view's
right-click menu, and the reading chosen drawn beside the value it was read
from. What is not built yet is marked **to come** below.

A response says `1763000000`, or `eyJhbGciOiJIUzI1NiJ9…`, or `H4sIAAAAAAAA…`.
Pretty shows all three faithfully and the reader learns nothing from any of them.
This is the feature that reads them: select a value, and tracewing says what it
is in human terms, next to where it stands.

One converter exists already. [`jwt.rs`](../crates/app/src/jwt.rs) decodes a
token without verifying it, names its time claims and says when they fall, and
[`inspector.rs`](../crates/app/src/inspector.rs) draws the result folded under
the header or cookie that carried it. What follows generalises that one case:
the same idea, for every value in a body, a header, a cookie, a query or a form
field, reached the same way everywhere.

## How it behaves

### Detect first, menu second

The obvious shape — right-click, `Datetime ▸ Unix ▸ Milliseconds` — is four
choices deep and asks the reader to already know the answer. They do not: not
knowing is why they selected the value.

So the registry runs on the selection before the menu opens, and the menu's
first item is the answer:

```
  Copy
  Copy all
  ─────────────────────────────────────────────
  2026-11-12 21:13:20  (Unix milliseconds)     ← one click, draws it
  342 KiB  (As a size)
  Copy reading                                 ← the best one, whole
  Read as                                      ← the ones ranked lower
  0x53e30  0b1010001111100110000  (In hex and binary)
  Clear what was read
```

`Read as` holds the readings the registry found but did not rank first: four
are offered above it and six under it, which is as many as a menu can hold
without becoming a list to read. It is a heading in the one menu rather than a
submenu, because the menu this app draws has no second level and one is not
worth building for ten items.

**To come:** the keyboard cycling readings on a selection, so a wrong guess
costs one key rather than another trip through the menu, and
`Always read this field this way ▸` for the rules below.

### Where the reading is drawn

Four shapes, by what the converter returned:

| Shape | Drawn as | |
|---|---|---|
| One line (a time, a size, a decoded integer) | after the value, dim, on the same row | built |
| Named fields (a JWT's claims, a traceparent's parts) | folded under the row, as JWTs are drawn now | to come: for now the fields are said on one line after the value, cut to what a line can spare |
| A body (JSON out of base64, protobuf out of gzip) | its own pane, with Pretty, Raw and Hex, folds and all | to come: for now the size and what it holds are said after the value |
| An image, or bytes | a thumbnail, or a hex dump, in that pane | to come |

What is drawn is kept by the piece it was read from, and a click on it takes it
away again, as does `Clear what was read`. Another mode is another text, so
Pretty, Raw and Hex each start with nothing drawn.

A reading is never part of the body. It is not in `selected_text()`, not in
`Copy all`, not in a saved body, not in Copy as cURL. The menu carries
`Copy reading` for when it is wanted.

### Saying that a reading exists

A reader who does not know the feature is there gets nothing from it. So values
with a certain or likely reading carry a dim dotted underline, worked out only
for the lines on screen, and switched off by a setting for anyone who finds it
noisy.

### Remembering the choice

Without this the feature is a toy: every request of the same endpoint would be
read by hand again.

After a reading is chosen, the menu offers to keep it:

- this value, for as long as the flow is open;
- every `created_at` from this host;
- every `created_at`, everywhere.

The key is the field's name and the host, not a JSON path. A body is indexed by
line so that 50 MB opens without freezing ([`body.rs`](../crates/app/src/body.rs)),
and a line index carries no path; the name the value sits under is right there in
the line's tokens. Field name also generalises the way a reader expects — the
same `expires_at` on six endpoints is the same thing — where a path does not. A
path prefix can narrow a rule later if one name turns out to mean two things.

Rules are listed in Settings and removable there, like the ABI and `.proto`
choices.

### Chains

Most real cases are not one step. `base64 → gzip → JSON → a Unix time inside
it` is the normal shape of a logging or webhook payload. So the registry runs
again on what a converter produced, three deep at most, and the pane says which
chain it took:

```
  base64 → gzip → JSON   (3.1 KB)
```

Each step is a menu of its own, so a wrong middle step can be changed without
starting over.

## What gets converted

### Phase one — built

The values that come up daily. Every one of these is pure, offline and cheap.

| Group | Readings |
|---|---|
| Times | Unix seconds, milliseconds, microseconds, nanoseconds, told apart by magnitude and shown as both when truly ambiguous; ISO 8601 and RFC 3339 as local time and as "4 minutes ago"; HTTP-date, as `Date`, `Expires` and `Last-Modified` carry it |
| Durations | Seconds as `2h 30m`, for `max-age`, `expires_in`, timeouts and retries |
| Encodings | base64, base64url, hex, percent-encoding, `\uXXXX` escapes, HTML entities |
| JSON in a string | Unescaped and laid out — the most common of all of these |
| Compression | gzip, Brotli, zstd, deflate, over a base64 or hex blob |
| Identifiers | UUID version and variant, with the time inside a v1, v6 or v7; ULID; Snowflake (Twitter, Discord); MongoDB ObjectId; KSUID |
| Numbers | An integer in hex and binary; a bitmask as the flags it sets; a byte count as `1 MiB` |
| HTTP's own | A URL in parts; a query string or a form body as a table; `Authorization: Basic` as its user and password; `Cache-Control` as a sentence; a status code by meaning; `Retry-After`; a cookie's attributes; a `data:` URI as what it holds |
| Tracing | `traceparent` as trace id, span id and flags; B3; `X-Amzn-Trace-Id`; `Server-Timing` |

### Phase two — to come

Narrower, and worth it for anyone debugging a backend.

| Group | Readings |
|---|---|
| Binary formats | MessagePack, CBOR, BSON, and protobuf off the wire with no `.proto`: field numbers and wire types. The store already reads protobuf and CBOR, so this is reuse, not new code |
| Tokens and keys | JWS and JWE told apart; PASETO; SAML (base64, deflate, XML); DER and PEM as a certificate or a public key; a hash named by its length; bcrypt and argon2 parameters out of their prefix |
| With a secret the reader types | Verify a JWT's HMAC; open a framework's signed cookie (Rails, Django, Laravel, Flask); decrypt with a key given by hand |
| Network | An IP as an integer; IPv6 shortened and spelled out; IPv4-mapped; a CIDR as its range, and whether it is private; a port by service; a MAC by vendor, from an OUI table in the build |
| Other epochs | Windows FILETIME, .NET ticks, Cocoa, HFS+, Excel serial |
| Bytes as a thing | A base64 image as a thumbnail; magic bytes as `PNG, 128×128`; a file type named |

### Phase three — to come

Taken one at a time, when asked for.

Minor units as money, with the currency from a neighbouring field · ISO 3166,
639 and 4217 codes by name · `Accept-Language` as an ordered list · E.164 as a
formatted number and a country · geohash as a position · a hex colour as a
swatch · a cron line as "every day at 03:00" · semver compared · a Unicode
codepoint by name · CRC and checksums · IBAN, ISBN and EAN checked · SQL in a
string laid out · Markdown or HTML in a string as a preview · gRPC status codes,
`errno`, Windows `HRESULT`.

## Shape in code

A crate of its own, `crates/convert`, because this must stay pure: no GPUI, no
files, no clock, no network. That keeps its tests instant and lets
[`mcp.rs`](../crates/app/src/mcp.rs) offer the same readings as a tool, so an
agent reading a flow sees what the reader sees.

```rust
/// One reading of a value: what a converter made of it.
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

/// How sure a converter is. It orders the menu, worst to best as written.
pub enum Confidence { Possible, Likely, Certain }

pub enum Shown {
    Line(String),
    Fields(Vec<(String, String)>),
    Table { head: Vec<String>, rows: Vec<Vec<String>> },
    /// Fed back through the registry, which is how a chain is found, and
    /// openable as a body of its own.
    Bytes { kind: Kind, bytes: Vec<u8> },
}

/// Everything a converter may know. Nothing here comes from a global: `now`
/// and `offset` are passed in, so a test in June is the test in December, and
/// in Jakarta it is the test in Berlin.
pub struct Input<'a> {
    pub text: &'a str,
    pub bytes: Option<&'a [u8]>,
    /// The name the value sits under, where the view knows it.
    pub field: Option<&'a str>,
    pub now: i64,
    pub offset: i32,
}

/// A converter: some text in, at most one reading out. A function, not a
/// trait: no converter has state, and a `fn` makes the registry a `const`.
pub type Converter = fn(&Input) -> Option<Reading>;

/// Every reading of `input`, best first, within a budget, following a chain
/// as far as `MAX_DEPTH` and never running one converter twice in a chain.
pub fn readings(input: &Input) -> Vec<Reading>;
pub fn readings_within(input: &Input, budget: Duration) -> Vec<Reading>;
```

A reading says itself two ways: `summary()` is the one line a menu shows
(`2026-11-12 21:13:20 (+07:00), in 0 s  (Unix seconds)`), and `text()` is what
`Copy reading` puts on the clipboard.

A field's name is matched with the spelling taken out of it, so `created_at`,
`createdAt` and `X-Created-At` are one name: `field_says` looks anywhere in it,
`field_ends` only at the end, because `at` inside a name is `status` and
`latency`, not a time.

The app side is small, because the parts were there.
[`reading.rs`](../crates/app/src/reading.rs) is all of it: it gives the
converters this machine's clock and offset, and turns what they found into
items for the menu [`select.rs`](../crates/app/src/select.rs) already builds
out of `Item` and `Act`. The body view adds them to its own menu after Copy,
four at most, and reads the key off the selected line
(`"created_at": 1763000000`) to fill in `field`. `jwt.rs` moved into the crate
as the first converter; what is left in the app under that name is the clock
the crate does without.

**To come:** the pane for a `Bytes` reading — which means writing the bytes
beside the body's own file and opening them as a body, the way
`detail::json_file` already does for MessagePack and Protobuf — the dim
underline, remembered rules, the keyboard, and the same menu in headers,
cookies, query and form fields. That last one wants `Selection` to carry the
field a piece was drawn under, which is the same thing the underline and the
rules need to know, so the three go together.

## Limits, and what this will not do

- A reading never calls out. No reverse DNS, no ASN or BIN lookup, no fetching
  a key. If one of those is ever wanted it belongs in the app, behind a setting
  that says plainly what leaves the machine.
- Nothing out of a body is fetched, opened or run. A `data:` URI is drawn from
  its own bytes; a URL is taken apart, not visited.
- Decryption needs a secret the reader typed. "Encrypted" is otherwise a dead
  end and the menu says so instead of guessing.
- The registry sees at most 1 MiB of selection, decompression stops at 16 MiB
  and at a ratio that smells like a bomb, a chain is three deep, and an image is
  decoded with a pixel cap.
- Digits that look like a card number get no lookup. The menu offers to mask
  them.
- A verifier this is not: a JWT is read, not trusted, as it is read today.
- CyberChef this is not either. The reader selects a value and gets its meaning;
  there is no recipe to build and no pipeline to save.

## Testing and wording

Converters are table-driven: input, expected reading, nothing else. `now` and
`tz` come from `Input`, so no test is seasonal. Every label goes through `tr!`
like the rest of the app, and the stable `id` — never the label — is what rules
and MCP store.

## Open questions

- Do remembered rules belong to the session, like a layout, or to the app, like
  the ABI choices? Probably the app, with the session able to override.
- Does a `Fields` reading that runs long belong folded under the row, or in the
  inspector's own pane? Folding a 40-claim token under one line will read badly.
- `Cache-Control` is said by joining fragments with commas — `kept 2h 30m`,
  `by anyone`, `checked every time`. That is English word order with English
  punctuation, and it came out stiff in every other language it was translated
  into. Either each shape becomes a message of its own, or the reading goes
  back to being a list of directives with their values read.
- Which of these, if any, is Pro. The secret-holding ones are the only group
  that looks like it; this is parked with the rest of the licensing.
