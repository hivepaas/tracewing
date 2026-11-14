# Converting values

The design, and where it stands. Built: `crates/convert` — the registry, the
phase one converters and their tests — the readings in the right-click menu of
a body and of the headers, cookies, query and form fields, the reading chosen
drawn where the value is, a reading that holds a body opened as one, and rules
remembered per host and field. What is not
built yet is marked **to come** below.

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

`mod+shift+R` goes through the readings of the selection without the menu:
the next one, then the next, and round again to none. A wrong guess then costs
one key rather than another trip through the menu.

### Where the reading is drawn

Four shapes, by what the converter returned:

| Shape | Drawn as | |
|---|---|---|
| One line (a time, a size, a decoded integer) | after the value, dim, on the same row | built |
| Named fields (a JWT's claims, a traceparent's parts) | folded under the row, as JWTs are drawn now | built under a header, a cookie and a query or form field; in a body, where a line is a line, they are said on one line after the value, cut to what it can spare |
| A body (JSON out of base64, protobuf out of gzip) | its own pane, with Pretty, Raw and Hex, folds and all | built |
| An image, or bytes | the picture itself, or a hex dump, in that pane | built: bytes that start as a PNG, JPEG, GIF or WebP are looked at rather than read, and the rest open as a body, where Hex shows them |

A reading that holds a body is not drawn beside the value at all: it opens in
the body pane, over the body it came out of, with a line above it saying what
read it and where from, and the way back.

```
  READ FROM BLOB   base64 → json                      Back to the body
  1  ▾ {
  2      "a": 1,
  3      "b": 2
  4    }
```

It is a body like any other, because it is one: the bytes are written into a
file in the data folder and opened as a body is opened, so Pretty, Raw, Hex,
folds, search and copying all work on them. The switch above the pane is about
what is in the pane, not about the body behind it, and the file goes when the
pane closes or another flow is chosen.

What is drawn in a body is kept by the piece it was read from, and a click on
it takes it away again, as does `Clear what was read`. Another mode is another
text, so Pretty, Raw and Hex each start with nothing drawn.

Under a field it is kept by the field and the value instead of by where they
sat, because rows move as a flow is chosen or a side is switched. That also
means the same header in the next flow says itself again, which is the small
version of the remembered rules below. What was unpacked is not kept whole:
one line says what it holds until there is a pane to open it in.

A reading is never part of the body. It is not in `selected_text()`, not in
`Copy all`, not in a saved body, not in Copy as cURL. The menu carries
`Copy reading` for when it is wanted.

### Saying that a reading exists — built

A reader who does not know the feature is there gets nothing from it. So a
value with a reading behind it carries a faint underline, in a body and under
a field alike:

```
  content-length   869
  date             Thu, 12 Nov 2026 16:11:41 GMT
                   ─────────────────────────────
```

Three things keep it honest and cheap:

- **Only what is more than a guess.** A `Possible` reading is not marked: every
  number is a size and a mask of bits and a number in hex, and underlining
  every number says nothing at all. A time, a token, a URL, a cookie — those
  are marked.
- **A sieve before the readings.** `could_read` turns away prose by its shape
  alone — too short, or it has spaces and is not an HTTP-date — so most lines
  never reach a converter.
- **Worked out once.** Each answer is kept by the piece it was found for, and
  dropped once there are more than a reader could have looked at (4096 lines in
  a body, 512 values under fields). A frame looks things up.

Where the reading itself is already shown, the underline is not: it has
nothing left to say.

Settings has a switch for it — **Reading values · Underline what can be read**
— for anyone who finds it noisy. It takes hold at once: the views read the
switch as they draw, so nothing has to be told about it.

### Remembering the choice — built

Without this the feature is a toy: every request of the same endpoint would be
read by hand again.

Once a reading is drawn, the menu offers `Always read created_at this way`, and
afterwards `Forget how created_at is read`. A rule is a host, a field and the
**name** of the reading — `unix-ms`, `http-date`, `base64>gzip>json` — never the
reading itself: the next value is read again from scratch
([`read_as`](../crates/convert/src/lib.rs)). Where a field stops reading that
way, nothing is drawn, which is the honest answer rather than a stale one.

The key is the field's name and the host, not a JSON path. A body is indexed by
line so that 50 MB opens without freezing ([`body.rs`](../crates/app/src/body.rs)),
and a line index carries no path; the name the value sits under is right there in
the line's tokens. Field name also generalises the way a reader expects — the
same `expires_at` on six endpoints is the same thing — where a path does not. A
path prefix can narrow a rule later if one name turns out to mean two things.

The rules live in `readings.toml` in the data folder, beside `devices.toml`,
and are shared by the body view and the inspector, so remembering in one is
remembered in the other:

```toml
[[rule]]
host = "api.example.com"
field = "createdat"
read_as = "unix-ms"
```

A field's name is kept with its spelling taken out, as the converters match it:
`Created-At`, `created_at` and `createdAt` are one field, and `X-Created-At` is
another. A rule with an empty host would apply everywhere; nothing writes one
yet, and the menu offers the host it is on.

What a rule reads is worked out once per value and kept until the rules change,
which a version counter on the rules says. A frame looks things up; it does not
read them again.

Settings lists them under **Fields always read a certain way**, with what each
is read as, the host it applies to, and a `Forget` beside it, since a rule
remembered from a value should not have to be found again from that same
value.

**To come:** the third scope — every host — which the store already
understands and reads, but which nothing writes yet.

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

A step is only taken where the bytes before it read as something, and three
formats say nothing about themselves at the front: raw deflate, Brotli and
protobuf have no bytes at the start that name them. For those the only way to
know is to unpack or parse and see whether what comes out is worth reading,
which is what `encodings::interesting` does once the simpler tests have said
nothing. It matters for more than neatness: a SAML response is always base64
of deflate, so without it that converter could not be reached by a real value
at all. What is not packed still has to fail — a key or a hash must not read
as deflate — so what comes out is measured against what bytes say for
themselves, never against another round of unpacking.

## What gets converted

### Phase one — built

The values that come up daily. Every one of these is pure, offline and cheap.

| Group | Readings |
|---|---|
| Times | Unix seconds, milliseconds, microseconds, nanoseconds, told apart by magnitude and shown as both when truly ambiguous; ISO 8601 and RFC 3339 as local time and as "4 minutes ago"; HTTP-date, as `Date`, `Expires` and `Last-Modified` carry it |
| Durations | Seconds as `2h 30m`, where the field says the number is one: `max-age`, `expires_in`, `retry-after`, a name ending in `age`, `secs` or `took`. Nothing about a number says it is seconds, so a bare one is read as a span only when a reader asks for it |
| Encodings | base64, base64url, hex, percent-encoding, `\uXXXX` escapes, HTML entities |
| JSON in a string | Unescaped and laid out — the most common of all of these |
| Compression | gzip, Brotli, zstd, deflate, over a base64 or hex blob |
| Identifiers | UUID version and variant, with the time inside a v1, v6 or v7; ULID; Snowflake (Twitter, Discord); MongoDB ObjectId; KSUID |
| Numbers | An integer in hex and binary; a bitmask as the flags it sets; a byte count as `1 MiB` |
| HTTP's own | A URL in parts; a query string or a form body as a table; `Authorization: Basic` as its user and password; `Cache-Control` as a sentence; a status code by meaning; `Retry-After`; a cookie's attributes; a `data:` URI as what it holds; an AWS SigV4 `Authorization` as the key, day, region, service and headers it was signed over; a `Content-Security-Policy` as a row for each directive |
| Tracing | `traceparent` as trace id, span id and flags; B3; `X-Amzn-Trace-Id`; `Server-Timing` |

### Phase two — partly built

Narrower, and worth it for anyone debugging a backend.

| Group | Readings |
|---|---|
| Binary formats — **partly built** | Protobuf off the wire with no `.proto`: every field by number, how it was written, text where it is text, and a count for a message inside a message; a gRPC frame taken off first. MessagePack, CBOR and BSON are not here on purpose — `tracewing_store::binjson` already reads all three for a whole body, and the right move is to let a value reuse that rather than to write it twice; where `binjson` should live is a question for its own change |
| Tokens and keys — **built** | JWE told from a JWS by its five parts, saying what wrapped the key and what encrypted the rest; PASETO, with a public token's claims read and a local one's said to need the key; SAML out of the chain that unpacks it, saying which message, from whom and to where; PEM, saying what it holds and how much DER; a hash named by the sizes it could be; bcrypt, argon2, SHA-crypt and PBKDF2 by their parameters; a framework's session cookie taken apart into the session it carries -- Flask's as itsdangerous writes it, Django's with the zlib off the front, Rails signed with its own wrapper off as well, and Rails encrypted said to need `secret_key_base` -- with the signature shown rather than checked. Reading a DER structure itself is still to come: it wants an ASN.1 parser, and `x509-parser` is the app's dependency, not this crate's |
| With a secret the reader types | Verify a JWT's HMAC; verify a framework's signed cookie, whose payload is already read without the key under Tokens and keys; decrypt with a key given by hand, which is what Rails 5.2's encrypted cookie and Laravel's need |
| Network — **built** | An IP as an integer, with what sort of address it is; IPv6 shortened and spelled out, with the IPv4 inside it; a CIDR as the range it holds; a port by what listens there; a MAC, with what its own bits say. A vendor from an OUI table is to come: the table is a megabyte of data to carry |
| Other epochs — **built** | Windows FILETIME and .NET ticks, which nothing else is large enough to be; Cocoa, HFS+ and a spreadsheet's serial, which a Unix time could be too, and so are only offered where the field says it is a time — or, for HFS+, past where a Unix time can reach |
| Bytes as a thing — **built** | A picture shown as itself in the pane, and named with its size: `PNG 128×128`, read from the first bytes of a PNG, GIF or JPEG |

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

`Selection` now carries the field each piece is the value of, so a header's
own name reaches the converters: a `Date` is read as an HTTP-date because the
field says `date`, not because the text was guessed at. The same thing is what
the underline and the remembered rules will ask.

Phase one is now built in full, menu to keyboard. What is left is the phase
two and three converters below, and the one scope the rules do not write yet.

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

- Settled: remembered rules belong to the app, in `readings.toml`, as the ABI
  and `.proto` choices do. Whether a session may override one is still open,
  and nothing needs it yet.
- A reading a rule asks for is drawn but cannot be clicked away, since the next
  draw would bring it back; only forgetting the rule removes it. A reader who
  wants it gone once has to forget the rule and remember it again.
- Does a `Fields` reading that runs long belong folded under the row, or in the
  inspector's own pane? Folding a 40-claim token under one line will read badly.
- A `Content-Security-Policy` is a row for each directive, with the sources
  kept as they were written and only the five that are words rather than
  places -- `'self'`, `'none'`, the two unsafe ones, `*` -- put into words. A
  nonce or a hash is kept to the character, because a reader opens the policy
  to compare one against a tag. The sources are joined with commas rather than
  the spaces the policy uses, since `this origin inline code eval` does not
  read as three things; that is not the `Cache-Control` trap below, because
  these are short noun phrases and a list of those translates, where a
  sentence assembled from fragments does not.
- `Cache-Control` is said by joining fragments with commas — `kept 2h 30m`,
  `by anyone`, `checked every time`. That is English word order with English
  punctuation, and it came out stiff in every other language it was translated
  into. Either each shape becomes a message of its own, or the reading goes
  back to being a list of directives with their values read.
- Which of these, if any, is Pro. The secret-holding ones are the only group
  that looks like it; this is parked with the rest of the licensing.
