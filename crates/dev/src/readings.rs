//! Values for the window to read: one of every kind it can make something of.
//!
//! The window's Read offers what a value can be understood as -- a number as
//! a time, base64 as the JSON inside it, a token as its claims. Trying that
//! by hand meant finding traffic that happened to carry each kind. `/readings`
//! carries all of them at once, in headers and in a body, each under a field
//! named as the real thing would be, since the name is part of what the
//! window goes on.
//!
//! The test beside this says every kind the window knows is here, and that
//! each value really does read as the kind it is filed under.

use base64::Engine as _;
use std::{fmt::Write as _, sync::OnceLock};

/// Where a sample is carried.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum At {
    /// A header of the answer, where the field is the header's name.
    Header,
    /// A field of the answer's JSON body.
    Body,
    /// A body of its own at `/readings/<name>`, for a value that a JSON
    /// string cannot carry: a PEM block is its newlines.
    Alone,
}

/// One value, and what the window should make of it.
pub struct Sample {
    /// The reading it is here for, as `tracewing_convert` names it.
    pub id: &'static str,
    pub at: At,
    /// The header or field it is under: the window reads the name too.
    pub name: &'static str,
    pub value: String,
    /// What there is to see, for the page that lists them.
    pub what: &'static str,
}

/// The token the chain samples are of: USDC on mainnet, as a node writes it.
const TOKEN: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";

/// The time the samples are written around, so that what the window says of
/// them is the same from one run to the next: 2026-11-12 14:13:20 UTC.
const WHEN: i64 = 1_794_492_800;

/// Everything `/readings` carries, built once.
pub fn samples() -> &'static [Sample] {
    static SAMPLES: OnceLock<Vec<Sample>> = OnceLock::new();
    SAMPLES.get_or_init(build)
}

/// The sample served on its own at `/readings/<name>`.
pub fn alone(name: &str) -> Option<&'static Sample> {
    samples()
        .iter()
        .find(|sample| sample.at == At::Alone && sample.name == name)
}

/// The headers of the answer, as names and values.
pub fn headers() -> Vec<(&'static str, String)> {
    samples()
        .iter()
        .filter(|sample| sample.at == At::Header)
        .map(|sample| (sample.name, sample.value.clone()))
        .collect()
}

/// The answer's body: every sample that is not a header, as JSON written out
/// a field to a line, so that each value is a line of its own to read.
pub fn body() -> String {
    let mut out = String::from("{\n");
    let fields: Vec<&Sample> = samples()
        .iter()
        .filter(|sample| sample.at == At::Body)
        .collect();
    for (at, sample) in fields.iter().enumerate() {
        let comma = if at + 1 == fields.len() { "" } else { "," };
        let _ = writeln!(
            out,
            "  \"{}\": \"{}\"{comma}",
            sample.name,
            escape(&sample.value)
        );
    }
    out.push_str("}\n");
    out
}

fn escape(text: &str) -> String {
    text.replace('\\', r"\\")
        .replace('"', "\\\"")
        .replace('\n', r"\n")
        .replace('\r', r"\r")
        .replace('\t', r"\t")
}

/// An address in a word of calldata: twelve zero bytes, then the twenty.
fn address_word(address: &str) -> String {
    format!("{:0>64}", address.trim_start_matches("0x").to_lowercase())
}

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn b64url(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// The JSON that the packed samples hold, so that unpacking one shows
/// something worth having unpacked.
const INSIDE: &str = r#"{"order":"A-4471","customer":"Nguyễn Văn A","total":350000,"paid":true}"#;

fn gzipped(data: &[u8]) -> Vec<u8> {
    use flate2::{Compression, write::GzEncoder};
    use std::io::Write as _;
    let mut out = GzEncoder::new(Vec::new(), Compression::default());
    out.write_all(data).expect("a Vec takes any bytes");
    out.finish().expect("a Vec takes any bytes")
}

fn zlibbed(data: &[u8]) -> Vec<u8> {
    use flate2::{Compression, write::ZlibEncoder};
    use std::io::Write as _;
    let mut out = ZlibEncoder::new(Vec::new(), Compression::default());
    out.write_all(data).expect("a Vec takes any bytes");
    out.finish().expect("a Vec takes any bytes")
}

fn deflated(data: &[u8]) -> Vec<u8> {
    use flate2::{Compression, write::DeflateEncoder};
    use std::io::Write as _;
    let mut out = DeflateEncoder::new(Vec::new(), Compression::default());
    out.write_all(data).expect("a Vec takes any bytes");
    out.finish().expect("a Vec takes any bytes")
}

fn zstded(data: &[u8]) -> Vec<u8> {
    zstd::encode_all(data, 3).expect("a Vec takes any bytes")
}

fn brotlied(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut from = data;
    brotli::BrotliCompress(&mut from, &mut out, &Default::default())
        .expect("a Vec takes any bytes");
    out
}

/// A protobuf message with no `.proto` to go by: a number, a string, and a
/// message inside it.
fn protobuf() -> Vec<u8> {
    let mut out = vec![0x08, 0xd2, 0x09]; // field 1, varint 1234
    out.extend_from_slice(&[0x12, 0x06]); // field 2, six bytes
    out.extend_from_slice(b"A-4471");
    let inside = [0x08, 0x01, 0x15, 0x00, 0x00, 0x48, 0x42]; // 1: 1, 2: 50.0
    out.extend_from_slice(&[0x1a, inside.len() as u8]); // field 3, a message
    out.extend_from_slice(&inside);
    out
}

/// A SAML response as it travels: deflated, then base64. Short, but it is the
/// shape that is being read, not the assertion.
fn saml() -> String {
    let xml = concat!(
        r#"<samlp:Response xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" "#,
        r#"ID="_8e8dc5f69a98cc4c1ff3427e5ce34606fd672f91e6" Version="2.0" "#,
        r#"IssueInstant="2026-11-12T14:13:20Z" Destination="https://app.example.com/acs">"#,
        r#"<saml:Issuer xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion">"#,
        r#"https://idp.example.org/metadata</saml:Issuer>"#,
        r#"<samlp:Status><samlp:StatusCode Value="urn:oasis:names:tc:SAML:2.0:status:Success"/>"#,
        r#"</samlp:Status></samlp:Response>"#
    );
    b64(&deflated(xml.as_bytes()))
}

/// A JWE, which is five parts where a JWT is three: what wrapped the key,
/// and what encrypted the rest.
fn jwe() -> String {
    let header = b64url(br#"{"alg":"RSA-OAEP-256","enc":"A256GCM","kid":"2026-11"}"#);
    format!(
        "{header}.{}.{}.{}.{}",
        b64url(&[0x9a; 32]),
        b64url(&[0x1c; 12]),
        b64url(b"this is not really encrypted"),
        b64url(&[0x7f; 16])
    )
}

/// A PASETO v4 public token: its claims are there to read, the signature is
/// only bytes.
fn paseto() -> String {
    let claims = br#"{"sub":"42","iss":"shop","exp":"2026-12-01T00:00:00Z"}"#;
    let mut payload = claims.to_vec();
    payload.extend_from_slice(&[0x5e; 64]);
    format!("v4.public.{}", b64url(&payload))
}

/// A certificate, as far as anything reading its outside is concerned.
fn pem() -> String {
    let der: Vec<u8> = (0..192).map(|n| (n * 7 % 251) as u8).collect();
    let body = b64(&der);
    let mut out = String::from("-----BEGIN CERTIFICATE-----\n");
    for line in body.as_bytes().chunks(64) {
        out.push_str(std::str::from_utf8(line).expect("base64 is text"));
        out.push('\n');
    }
    out.push_str("-----END CERTIFICATE-----\n");
    out
}

fn build() -> Vec<Sample> {
    let header = |id, name, value: String, what| Sample {
        id,
        at: At::Header,
        name,
        value,
        what,
    };
    let field = |id, name, value: String, what| Sample {
        id,
        at: At::Body,
        name,
        value,
        what,
    };
    let alone = |id, name, value: String, what| Sample {
        id,
        at: At::Alone,
        name,
        value,
        what,
    };
    let jwt = crate::benchkit::SESSION_JWT;
    vec![
        // Headers, where the name is as much of a hint as the value.
        header(
            "jwt",
            "authorization",
            format!("Bearer {jwt}"),
            "A JWT behind Bearer: its claims, and its times as times",
        ),
        header(
            "http-date",
            "last-modified",
            "Thu, 12 Nov 2026 14:13:20 GMT".to_owned(),
            "The date an HTTP header writes",
        ),
        // Not under `cache-control` itself: a header that tells a browser to
        // keep the answer for an hour is obeyed, and the next look at this
        // route never leaves the browser -- so there is nothing to catch in
        // the window, which is the one thing this route is for. The reading
        // goes by the value, not the name, so it reads here all the same.
        header(
            "cache-control",
            "x-cache-control",
            "public, max-age=3600, stale-while-revalidate=60, immutable".to_owned(),
            "What each part of it means, in words",
        ),
        header(
            "cookie",
            "set-cookie",
            format!("session={jwt}; Path=/; Max-Age=3600; HttpOnly; SameSite=Lax"),
            "A cookie's parts, and the JWT inside it as a chain",
        ),
        header(
            "basic-auth",
            "proxy-authorization",
            format!("Basic {}", b64(b"ana:open sesame")),
            "The name and the password behind Basic",
        ),
        // Not under `authorization`, which already carries the JWT above: a
        // header holds one value, and both of these are worth having. The
        // reading goes by the value, so it reads here all the same.
        header(
            "aws-sigv4",
            "x-amz-authorization",
            "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20261112/ap-southeast-1/s3/aws4_request, \
             SignedHeaders=host;x-amz-content-sha256;x-amz-date, \
             Signature=5d672d79c15b13162d9279b0855cfba6789a8edb4c82c400e06b5924a6f2b5d7"
                .to_owned(),
            "What an Amazon signature says: the key, the day, the region, the service",
        ),
        header(
            "traceparent",
            "traceparent",
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01".to_owned(),
            "W3C trace: the trace, the span, and whether it is sampled",
        ),
        header(
            "b3",
            "b3",
            "4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-1".to_owned(),
            "Zipkin's own way of writing the same thing",
        ),
        header(
            "amazon-trace",
            "x-amzn-trace-id",
            "Root=1-5759e988-bd862e3fe1be46a994272793;Parent=53995c3f42cd8ad8;Sampled=1".to_owned(),
            "Amazon's trace id, with the time inside it",
        ),
        header(
            "server-timing",
            "server-timing",
            "db;dur=53.2, cache;desc=\"hit\";dur=0.4, app;dur=47.2".to_owned(),
            "What the server spent, part by part",
        ),
        header(
            "url",
            "location",
            "https://api.example.com/v2/orders?page=2&limit=50&sort=-created_at#top".to_owned(),
            "A URL pulled apart: host, path, query, fragment",
        ),
        // Not `content-length`: a header that frames the answer cannot carry
        // a made-up number, and hyper writes the real one anyway.
        header(
            "size",
            "x-upload-size",
            "1048576".to_owned(),
            "A number of bytes as a size",
        ),
        header(
            "uuid",
            "x-request-id",
            "f47ac10b-58cc-4372-a567-0e02b2c3d479".to_owned(),
            "Which UUID it is, and when, where the version says",
        ),
        header(
            "hash",
            "etag",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned(),
            "Hex of a length that names the hash it could be",
        ),
        header(
            "unix-s",
            "x-rate-limit-reset",
            WHEN.to_string(),
            "Seconds since 1970, as a time and how far off",
        ),
        header(
            "duration",
            "retry-after",
            "120".to_owned(),
            "Seconds, said as a span",
        ),
        // Times, in a body, each kind in the field a real one would be in.
        field(
            "aws-sigv4",
            "x_amz_credential",
            "AKIDEXAMPLE/20261112/ap-southeast-1/s3/aws4_request".to_owned(),
            "The same credential as a presigned URL carries it, with no algorithm in front",
        ),
        field(
            "unix-ms",
            "created_at_ms",
            (WHEN * 1_000 + 123).to_string(),
            "Milliseconds since 1970",
        ),
        field(
            "unix-us",
            "seen_at_us",
            (WHEN * 1_000_000 + 123_456).to_string(),
            "Microseconds since 1970",
        ),
        field(
            "unix-ns",
            "event_time_ns",
            (WHEN * 1_000_000_000 + 123_456_789).to_string(),
            "Nanoseconds since 1970",
        ),
        field(
            "iso-8601",
            "expires_at",
            "2026-12-01T09:30:00+07:00".to_owned(),
            "A written time, as the reader's own and how far off",
        ),
        field(
            "iso-8601-naive",
            "run_at",
            "2026-11-12 14:13:20".to_owned(),
            "The same with no zone, read as it stands",
        ),
        field(
            "filetime",
            "last_write_time",
            "133758633200000000".to_owned(),
            "Windows FILETIME: 100 ns since 1601",
        ),
        field(
            "dotnet-ticks",
            "ticks",
            "639084007200000000".to_owned(),
            ".NET ticks: 100 ns since the year 1",
        ),
        field(
            "cocoa",
            "modified_timestamp",
            (WHEN - 978_307_200).to_string(),
            "Seconds since 2001, which Apple counts from",
        ),
        field(
            "hfs",
            "created_time",
            (WHEN + 2_082_844_800).to_string(),
            "Seconds since 1904, which HFS+ counts from",
        ),
        field(
            "excel-date",
            "due_date",
            "46339".to_owned(),
            "A spreadsheet's day number",
        ),
        // What a thing is called.
        field(
            "ulid",
            "event_id",
            "01JCGQ2M7QJ0F6T2S2M7XK9A1B".to_owned(),
            "A ULID: the time it was made, then randomness",
        ),
        field(
            "object-id",
            "_id",
            "6734f1c2a4b3d2e1f0a9b8c7".to_owned(),
            "MongoDB's ObjectId, with its time and counter",
        ),
        field(
            "ksuid",
            "ksuid",
            "2cS8m3KfGHCbWxKzMSiOP0i0xeE".to_owned(),
            "A KSUID, which also carries when it was made",
        ),
        field(
            "snowflake",
            "message_id",
            "175928847299117063".to_owned(),
            "A snowflake: time, machine and sequence",
        ),
        // The web's own strings.
        field(
            "query",
            "query_string",
            "page=2&limit=50&sort=-created_at&tags=a,b".to_owned(),
            "A query string as the pairs it holds",
        ),
        field(
            "status",
            "status_code",
            "429".to_owned(),
            "What that status code means",
        ),
        field(
            "data-uri",
            "avatar",
            format!("data:image/png;base64,{}", b64(crate::benchkit::PICTURE)),
            "A data URI: what it holds, and the picture itself",
        ),
        // What a chain writes.
        field(
            "address",
            "token_address",
            TOKEN.to_owned(),
            "An address, written as EIP-55 writes it: the capitals check it",
        ),
        field(
            "topic",
            "log_topic",
            "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef".to_owned(),
            "A log's first topic, as the event it stands for",
        ),
        field(
            "selector",
            "call_data",
            // transfer(address,uint256): to vitalik.eth, 1 USDC.
            format!(
                "0xa9059cbb{}{:064x}",
                address_word("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"),
                1_000_000u64
            ),
            "Calldata, named by the four bytes it starts with",
        ),
        // Networks.
        field(
            "ipv6",
            "peer_address",
            "2001:0db8:0000:0000:0000:ff00:0042:8329".to_owned(),
            "IPv6 shortened and spelled out",
        ),
        field(
            "cidr",
            "allowed_range",
            "192.168.100.0/22".to_owned(),
            "A CIDR as the addresses it covers",
        ),
        field(
            "mac",
            "hardware_address",
            "3c:22:fb:9a:1d:0e".to_owned(),
            "A MAC, and what its own bits say",
        ),
        field(
            "ip-number",
            "client_ip",
            "3232235777".to_owned(),
            "An address written as one number",
        ),
        field(
            "port",
            "port",
            "5432".to_owned(),
            "A port, by what usually listens there",
        ),
        // Numbers that are not quantities.
        field(
            "radix",
            "opcode",
            "48879".to_owned(),
            "The same number in hex and binary",
        ),
        field(
            "flags",
            "permission_mask",
            "420".to_owned(),
            "Which bits are set",
        ),
        // Encodings, and what is inside them.
        field(
            "base64",
            "payload",
            b64(INSIDE.as_bytes()),
            "base64, and the JSON inside it as a body of its own",
        ),
        field(
            "hex",
            "raw_hex",
            INSIDE
                .as_bytes()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            "Hex, and what it spells",
        ),
        field(
            "base85",
            "ascii85_payload",
            "HQmBKA7]d*3[-jA1c.-G+tOpW@;06r3coCbAUO".to_owned(),
            "Ascii85: five characters to four bytes, with JSON inside",
        ),
        field(
            "base58",
            "wallet",
            "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_owned(),
            "base58check: what it is, and whether its four bytes check out",
        ),
        field(
            "base32",
            "totp_payload",
            "PMRG62ZCHJ2HE5LFPU======".to_owned(),
            "base32, as a secret or an onion address is written",
        ),
        field(
            "quoted-printable",
            "subject_qp",
            "caf=C3=A9_=C4=91=E1=BA=B7t".to_owned(),
            "Quoted-printable, as mail writes what is not ASCII",
        ),
        field(
            "percent",
            "redirect_to",
            "https%3A%2F%2Fexample.com%2Ft%C3%ACm%3Fq%3Dc%C3%A0%20ph%C3%AA".to_owned(),
            "Percent-encoding, undone",
        ),
        field(
            "escapes",
            "note_escaped",
            r"caf\u00e9_\u0111\u1eb7t_h\u00e0ng".to_owned(),
            "Unicode escapes, as the letters they are",
        ),
        field(
            "entities",
            "note_html",
            "&lt;b&gt;caf&eacute;&lt;/b&gt; &amp; tr&agrave;".to_owned(),
            "HTML entities, as the letters they are",
        ),
        // Packed, each one base64 so that it can travel as text: the window
        // unpacks the chain base64 then gzip then JSON.
        field(
            "gzip",
            "body_gzip",
            b64(&gzipped(INSIDE.as_bytes())),
            "base64 to gzip to the JSON inside",
        ),
        field(
            "zlib",
            "body_zlib",
            b64(&zlibbed(INSIDE.as_bytes())),
            "The same, packed as zlib",
        ),
        field(
            "deflate",
            "body_deflate",
            b64(&deflated(INSIDE.as_bytes())),
            "The same, raw deflate",
        ),
        field(
            "zstd",
            "body_zstd",
            b64(&zstded(INSIDE.as_bytes())),
            "The same, zstd",
        ),
        field(
            "brotli",
            "body_brotli",
            b64(&brotlied(INSIDE.as_bytes())),
            "The same, brotli",
        ),
        field(
            "protobuf",
            "message_bytes",
            b64(&protobuf()),
            "Protobuf with no .proto: each field by number",
        ),
        // JSON hiding inside JSON.
        field(
            "json-in-string",
            "meta",
            r#"{"retries":2,"last_error":null,"tags":["a","b"]}"#.to_owned(),
            "A string that is itself JSON, read as a body",
        ),
        // Tokens and keys.
        field(
            "jwe",
            "encrypted_token",
            jwe(),
            "A JWE: five parts, what wrapped the key, what encrypted the rest",
        ),
        field(
            "paseto",
            "paseto_token",
            paseto(),
            "A PASETO v4 public token, claims and all",
        ),
        field(
            "saml",
            "saml_response",
            saml(),
            "base64 to deflate to the SAML inside: who it is from, and to where",
        ),
        field(
            "crypt",
            "password_hash",
            "$2b$12$R9h/cIPz0gi.URNNX3kh2OPST9/PgBkqquzi.Ss7KIUgO2t0jWMUW".to_owned(),
            "bcrypt, with the work it was given",
        ),
        // A PEM is its line breaks: inside a JSON string they are written
        // `\n`, and what the window then has is escaped text, not a PEM. It
        // gets a body of its own instead.
        alone(
            "pem",
            "certificate.pem",
            pem(),
            "PEM at /readings/certificate.pem: what it holds, and how much DER",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracewing_convert::Input;

    /// What a reader would choose on the screen: a body's value is shown as
    /// it is written in the JSON, backslashes and all, which is what the
    /// window hands to the converters.
    fn as_shown(sample: &Sample) -> String {
        match sample.at {
            At::Header | At::Alone => sample.value.clone(),
            At::Body => escape(&sample.value),
        }
    }

    fn reads(sample: &Sample) -> Vec<&'static str> {
        let shown = as_shown(sample);
        let input = Input::new(&shown, WHEN + 60, 7 * 3600).under(Some(sample.name));
        tracewing_convert::readings(&input)
            .into_iter()
            .map(|reading| reading.id)
            .collect()
    }

    /// Every sample has to read as the kind it is filed under, with the
    /// field it is carried in taken into account. All of them at once, so
    /// that one that stops reading does not hide the others.
    #[test]
    fn each_value_reads_as_what_it_is_here_for() {
        let mut amiss = Vec::new();
        for sample in samples() {
            let ids = reads(sample);
            if !ids.contains(&sample.id) {
                amiss.push(format!("{} under {}: {ids:?}", sample.id, sample.name));
            }
        }
        assert!(amiss.is_empty(), "{}", amiss.join("\n"));
    }

    /// The ones that travel inside something else have to be reached
    /// through it, chain and all: raw deflate, Brotli and protobuf say
    /// nothing about themselves at the front, so these are the samples that
    /// a chain used to stop one step short of.
    #[test]
    fn what_travels_inside_base64_is_reached_through_it() {
        for id in [
            "gzip", "zlib", "deflate", "zstd", "brotli", "protobuf", "saml",
        ] {
            let sample = samples()
                .iter()
                .find(|sample| sample.id == id)
                .unwrap_or_else(|| panic!("a sample for {id}"));
            let shown = as_shown(sample);
            let input = Input::new(&shown, WHEN + 60, 7 * 3600).under(Some(sample.name));
            let reading = tracewing_convert::readings(&input)
                .into_iter()
                .find(|reading| reading.id == id)
                .unwrap_or_else(|| panic!("{id} is not reached"));
            assert_eq!(reading.chain.first(), Some(&"base64"), "{id}: {reading:?}");
            assert!(reading.chain.contains(&id), "{id}: {reading:?}");
        }
    }

    /// And every kind the window knows is here to try. The list is the ids
    /// the converters give, which is what a reader picks from in the menu:
    /// if one is added there and not here, there is traffic nobody can make.
    #[test]
    fn every_kind_the_window_knows_is_here() {
        const KNOWN: &[&str] = &[
            "address",
            "amazon-trace",
            "aws-sigv4",
            "b3",
            "base32",
            "base58",
            "base64",
            "base85",
            "basic-auth",
            "brotli",
            "cache-control",
            "cidr",
            "cocoa",
            "cookie",
            "crypt",
            "data-uri",
            "deflate",
            "dotnet-ticks",
            "duration",
            "entities",
            "escapes",
            "excel-date",
            "filetime",
            "flags",
            "gzip",
            "hash",
            "hex",
            "hfs",
            "http-date",
            "ip-number",
            "ipv6",
            "iso-8601",
            "iso-8601-naive",
            "json-in-string",
            "jwe",
            "jwt",
            "ksuid",
            "mac",
            "object-id",
            "paseto",
            "pem",
            "percent",
            "port",
            "protobuf",
            "query",
            "quoted-printable",
            "radix",
            "saml",
            "selector",
            "server-timing",
            "size",
            "snowflake",
            "status",
            "topic",
            "traceparent",
            "ulid",
            "unix-ms",
            "unix-ns",
            "unix-s",
            "unix-us",
            "url",
            "uuid",
            "zlib",
            "zstd",
        ];
        let here: Vec<&str> = samples().iter().map(|sample| sample.id).collect();
        let missing: Vec<&&str> = KNOWN.iter().filter(|id| !here.contains(id)).collect();
        assert!(missing.is_empty(), "nothing carries {missing:?}");
    }

    /// Which values say for themselves that there is something to read.
    ///
    /// The window underlines a value only where `could_read` lets it
    /// through, which turns away anything with a space in it: a sentence is
    /// not a value. Those still read when a reader chooses them, so the
    /// samples with spaces are here on purpose -- but they are the ones
    /// nobody finds by looking.
    #[test]
    fn the_values_with_spaces_are_read_when_chosen_rather_than_offered() {
        let mut quiet = Vec::new();
        for sample in samples() {
            let shown = as_shown(sample);
            if !tracewing_convert::could_read(&shown) {
                quiet.push(sample.id);
            }
        }
        quiet.sort_unstable();
        // Spaces in them, or shorter than eight characters: both are what a
        // sieve meant to turn away prose has to turn away.
        assert_eq!(
            quiet,
            [
                "aws-sigv4",
                "basic-auth",
                "cache-control",
                "cookie",
                "duration",
                "entities",
                "excel-date",
                "flags",
                "iso-8601-naive",
                "jwt",
                "pem",
                "port",
                "radix",
                "server-timing",
                "size",
                "status",
            ]
        );
    }

    /// The body is JSON, and holds what it says it holds.
    #[test]
    fn the_body_is_json_a_window_can_read() {
        let value: serde_json::Value =
            serde_json::from_str(&body()).unwrap_or_else(|trouble| panic!("{trouble}: {}", body()));
        let object = value.as_object().expect("an object");
        for sample in samples().iter().filter(|sample| sample.at == At::Body) {
            assert_eq!(
                object.get(sample.name).and_then(|v| v.as_str()),
                Some(sample.value.as_str()),
                "{}",
                sample.name
            );
        }
    }

    /// Headers have to be headers: no line breaks, nothing a client will
    /// refuse.
    #[test]
    fn the_headers_can_be_sent() {
        for (name, value) in headers() {
            assert!(
                http::HeaderValue::from_str(&value).is_ok(),
                "{name}: {value}"
            );
        }
    }
}
