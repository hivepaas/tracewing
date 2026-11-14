//! Tokens, keys and hashes: what can be read of them without the secret.
//!
//! Nothing here verifies anything and nothing here decrypts. What a token
//! claims, what a key file holds, what a hash is the size of — that is what a
//! reader wants at the moment they are looking at it, and all of it is in the
//! bytes themselves.

use crate::{Confidence, Input, Reading, Shown};
use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use serde_json::{Map, Value};
use tracewing_i18n::tr;

/// An encrypted token: five parts, and only the first can be read. Told from
/// a JWT, which has three and can be read whole.
pub fn jwe(input: &Input) -> Option<Reading> {
    let text = input.text.trim_matches('"');
    let parts: Vec<&str> = text.split('.').collect();
    if parts.len() != 5 || !parts[0].starts_with("eyJ") {
        return None;
    }
    let header: Map<String, Value> =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[0]).ok()?).ok()?;
    let said = |key: &str| header.get(key).and_then(Value::as_str).map(str::to_owned);
    let mut fields = Vec::new();
    if let Some(algorithm) = said("alg") {
        fields.push((tr!("Key wrapped with"), algorithm));
    }
    // Without `enc` it is not a JWE, whatever else it is.
    fields.push((tr!("Encrypted with"), said("enc")?));
    if let Some(kid) = said("kid") {
        fields.push((tr!("Key named"), kid));
    }
    fields.push((
        tr!("The rest"),
        tr!("encrypted: it needs the key, which tracewing does not have"),
    ));
    Some(Reading::new(
        "jwe",
        tr!("JWE, an encrypted token"),
        Confidence::Certain,
        Shown::Fields(fields),
    ))
}

/// PASETO: a version, what it is for, and the payload where that is plain.
pub fn paseto(input: &Input) -> Option<Reading> {
    let text = input.text.trim_matches('"');
    let mut parts = text.split('.');
    let version = parts.next()?;
    let purpose = parts.next()?;
    let payload = parts.next()?;
    if !matches!(version, "v1" | "v2" | "v3" | "v4")
        || !matches!(purpose, "local" | "public")
        || payload.len() < 8
    {
        return None;
    }
    let mut fields = vec![
        (tr!("Version"), version.to_owned()),
        (
            tr!("What it is for"),
            match purpose {
                "local" => tr!("shared key, encrypted"),
                _ => tr!("public key, signed"),
            },
        ),
    ];
    // A public token carries its claims in the open; a local one does not.
    if purpose == "public"
        && let Some(bytes) = URL_SAFE_NO_PAD.decode(payload).ok()
        && let Some(text) = String::from_utf8(bytes).ok()
        && let Some(at) = text.rfind('}')
        && let Ok(Value::Object(claims)) = serde_json::from_str::<Value>(&text[..=at])
    {
        for (claim, value) in claims {
            fields.push((
                claim,
                match value {
                    Value::String(text) => text,
                    other => other.to_string(),
                },
            ));
        }
    } else if purpose == "local" {
        fields.push((
            tr!("The claims"),
            tr!("encrypted: it needs the key, which tracewing does not have"),
        ));
    }
    Some(Reading::new(
        "paseto",
        tr!("PASETO"),
        Confidence::Certain,
        Shown::Fields(fields),
    ))
}

/// A hash by the size it is. Which one it is cannot be told from the bytes,
/// only which ones it could be.
pub fn hash(input: &Input) -> Option<Reading> {
    let whole = input.text.trim_matches('"');
    let text = whole.trim_start_matches("0x");
    if !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    // Twenty bytes behind an `0x` are an address, whatever else they are the
    // size of: nobody writes a RIPEMD-160 that way, and reading one as a
    // hash is how an address came to be called SHA-1.
    if text.len() == 40 && whole.len() != text.len() {
        return None;
    }
    let said = match text.len() {
        32 => tr!("MD5, or MD4, or NTLM"),
        40 => tr!("SHA-1, or RIPEMD-160"),
        56 => tr!("SHA-224"),
        64 => tr!("SHA-256, or SHA3-256, or BLAKE2s"),
        96 => tr!("SHA-384"),
        128 => tr!("SHA-512, or SHA3-512, or BLAKE2b"),
        _ => return None,
    };
    let confidence = if input.field_says(&["hash", "digest", "checksum", "sum", "etag", "sig"]) {
        Confidence::Likely
    } else {
        Confidence::Possible
    };
    Some(Reading::new(
        "hash",
        tr!("A hash, by its size"),
        confidence,
        Shown::Line(said),
    ))
}

/// A password that was hashed to be kept: what did it, and how hard it was
/// made to do again.
pub fn crypt(input: &Input) -> Option<Reading> {
    let text = input.text.trim_matches('"');
    let parts: Vec<&str> = text.split('$').collect();
    if parts.len() < 4 || !parts[0].is_empty() {
        return None;
    }
    let mut fields = Vec::new();
    match parts[1] {
        "2a" | "2b" | "2x" | "2y" => {
            fields.push((tr!("Made by"), tr!("bcrypt")));
            fields.push((tr!("Rounds"), tr!("2 to the {cost}", cost = parts[2])));
        }
        name if name.starts_with("argon2") => {
            fields.push((tr!("Made by"), name.to_owned()));
            for part in parts.iter().skip(2) {
                for setting in part.split(',') {
                    match setting.split_once('=') {
                        Some(("m", kib)) => fields.push((tr!("Memory"), format!("{kib} KiB"))),
                        Some(("t", times)) => fields.push((tr!("Passes"), times.to_owned())),
                        Some(("p", lanes)) => fields.push((tr!("In parallel"), lanes.to_owned())),
                        _ => {}
                    }
                }
            }
        }
        "5" => fields.push((tr!("Made by"), tr!("SHA-256 crypt"))),
        "6" => fields.push((tr!("Made by"), tr!("SHA-512 crypt"))),
        "pbkdf2-sha256" | "pbkdf2-sha512" | "pbkdf2" => {
            fields.push((tr!("Made by"), parts[1].to_owned()));
            fields.push((tr!("Passes"), parts[2].to_owned()));
        }
        _ => return None,
    }
    Some(Reading::new(
        "crypt",
        tr!("A hashed password"),
        Confidence::Certain,
        Shown::Fields(fields),
    ))
}

/// A PEM file: what it says it holds, and how much of it there is.
pub fn pem(input: &Input) -> Option<Reading> {
    let text = input.text.trim();
    let start = text.find("-----BEGIN ")? + "-----BEGIN ".len();
    let label = text[start..].split("-----").next()?.trim();
    if label.is_empty() || label.len() > 64 {
        return None;
    }
    let body: String = text[start + label.len()..]
        .trim_start_matches("-----")
        .lines()
        .take_while(|line| !line.starts_with("-----END"))
        .flat_map(str::chars)
        .filter(|c| !c.is_whitespace())
        .collect();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&body)
        .ok()?;
    if bytes.is_empty() {
        return None;
    }
    Some(Reading::new(
        "pem",
        tr!("PEM"),
        Confidence::Certain,
        Shown::Fields(vec![
            (tr!("What it holds"), label.to_owned()),
            (
                tr!("How much"),
                tr!(
                    "{size} of DER",
                    size = crate::numbers::bytes_size(bytes.len() as u64)
                ),
            ),
        ]),
    ))
}

/// SAML, once it is out of its base64 and its deflate: which message it is,
/// who sent it and where it was going.
pub fn saml(input: &Input) -> Option<Reading> {
    let text = input.text;
    if !text.contains("urn:oasis:names:tc:SAML") {
        return None;
    }
    let tag = [
        "AuthnRequest",
        "LogoutRequest",
        "Response",
        "LogoutResponse",
    ]
    .into_iter()
    .find(|tag| text.contains(&format!("{tag} ")) || text.contains(&format!(":{tag}")))?;
    let attribute = |name: &str| -> Option<String> {
        let at = text.find(&format!("{name}=\""))? + name.len() + 2;
        Some(text[at..].split('"').next()?.to_owned())
    };
    let between = |tag: &str| -> Option<String> {
        let open = text.find(&format!("{tag}>"))? + tag.len() + 1;
        let rest = &text[open..];
        Some(rest[..rest.find('<')?].to_owned())
    };
    let mut fields = vec![(tr!("Which message"), tag.to_owned())];
    for (name, found) in [
        (
            tr!("From"),
            between("Issuer").or_else(|| attribute("Issuer")),
        ),
        (tr!("To"), attribute("Destination")),
        (tr!("Made"), attribute("IssueInstant")),
        (tr!("Named"), attribute("ID")),
    ] {
        if let Some(value) = found.filter(|value| !value.is_empty() && value.len() < 200) {
            fields.push((name, value));
        }
    }
    Some(Reading::new(
        "saml",
        tr!("SAML"),
        Confidence::Certain,
        Shown::Fields(fields),
    ))
}

/// The frameworks whose session cookies can be told apart by their shape
/// alone.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Framework {
    Flask,
    Django,
    RailsSigned,
    RailsEncrypted,
}

impl Framework {
    fn said(self) -> String {
        match self {
            Self::Flask => tr!("Flask, signed by itsdangerous"),
            Self::Django => tr!("Django"),
            Self::RailsSigned => tr!("Rails, signed"),
            Self::RailsEncrypted => tr!("Rails, encrypted"),
        }
    }
}

/// What a session cookie came apart into: the session itself where it can be
/// had, when it was signed where that is written down, and the signature.
struct Session {
    framework: Framework,
    /// None where the session is encrypted rather than only signed.
    payload: Option<Vec<u8>>,
    at: Option<i64>,
    signature: String,
}

/// Whether a count of seconds is near enough to now to have been written by
/// a framework signing a cookie: 2011 to 2096. A number outside that is some
/// other number.
fn plausible(at: &i64) -> bool {
    (1_300_000_000..4_000_000_000).contains(at)
}

fn base64url(part: &str) -> Option<Vec<u8>> {
    let part = part.trim_end_matches('=');
    if part.is_empty()
        || !part
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return None;
    }
    URL_SAFE_NO_PAD.decode(part).ok()
}

/// The payload of itsdangerous and of Django alike: base64url, and zlib
/// under a leading dot where packing it came out shorter.
fn payload_bytes(part: &str) -> Option<Vec<u8>> {
    match part.strip_prefix('.') {
        Some(packed) => {
            use std::io::Read as _;
            let bytes = base64url(packed)?;
            let mut out = Vec::new();
            flate2::read::ZlibDecoder::new(bytes.as_slice())
                .take(crate::MAX_INPUT as u64)
                .read_to_end(&mut out)
                .ok()?;
            (!out.is_empty()).then_some(out)
        }
        None => base64url(part),
    }
}

/// itsdangerous writes the time as the fewest big-endian bytes that hold it,
/// then base64url.
fn stamped(part: &str) -> Option<i64> {
    let bytes = base64url(part)?;
    if bytes.is_empty() || bytes.len() > 8 {
        return None;
    }
    Some(
        bytes
            .iter()
            .fold(0i64, |at, byte| at << 8 | i64::from(*byte)),
    )
}

/// Django writes the time in base62, digits before uppercase before lower.
fn base62(part: &str) -> Option<i64> {
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    if part.is_empty() || part.len() > 11 {
        return None;
    }
    part.bytes().try_fold(0i64, |at, byte| {
        let digit = ALPHABET.iter().position(|a| *a == byte)? as i64;
        at.checked_mul(62)?.checked_add(digit)
    })
}

/// A signature at the end of a cookie, as base64url: long enough to be one.
fn signed(part: &str) -> bool {
    part.len() >= 20 && base64url(part).is_some()
}

/// `payload.timestamp.signature`, which is itsdangerous and so is Flask.
fn flask(text: &str) -> Option<Session> {
    let mut parts = text.rsplitn(3, '.');
    let signature = parts.next()?;
    let at = stamped(parts.next()?).filter(plausible)?;
    let payload = payload_bytes(parts.next()?)?;
    signed(signature).then(|| Session {
        framework: Framework::Flask,
        payload: Some(payload),
        at: Some(at),
        signature: signature.to_owned(),
    })
}

/// `payload:timestamp:signature`, which is `django.core.signing`. The time
/// has to read as a time: two colons in a value are easy to come by, and the
/// rest of the shape is the same as everything else base64.
fn django(text: &str) -> Option<Session> {
    let mut parts = text.rsplitn(3, ':');
    let signature = parts.next()?;
    let at = base62(parts.next()?).filter(plausible)?;
    let payload = payload_bytes(parts.next()?)?;
    signed(signature).then(|| Session {
        framework: Framework::Django,
        payload: Some(payload),
        at: Some(at),
        signature: signature.to_owned(),
    })
}

/// Rails, where `--` separates. Two parts and a hex HMAC is the signed
/// cookie of Rails 4, whose payload can be read; three parts is the
/// encrypted one of Rails 5.2 on, whose payload cannot.
fn rails(text: &str) -> Option<Session> {
    let parts: Vec<&str> = text.split("--").collect();
    match parts.as_slice() {
        [body, signature]
            if signature.len() == 40 && signature.bytes().all(|b| b.is_ascii_hexdigit()) =>
        {
            Some(Session {
                framework: Framework::RailsSigned,
                payload: Some(STANDARD.decode(body).ok()?),
                at: None,
                signature: (*signature).to_owned(),
            })
        }
        // The initialisation vector is 12 bytes and the tag 16, each written
        // base64: 16 characters and 24, which is what tells this from a
        // signed cookie that happens to hold two dashes.
        [body, iv, tag]
            if body.len() >= 16
                && iv.len() == 16
                && tag.len() == 24
                && STANDARD.decode(iv).is_ok()
                && STANDARD.decode(tag).is_ok() =>
        {
            Some(Session {
                framework: Framework::RailsEncrypted,
                payload: None,
                at: None,
                signature: (*tag).to_owned(),
            })
        }
        _ => None,
    }
}

/// Rails wraps its own payload once more, to say what the message is for and
/// when it runs out. What is inside that is the session.
fn unwrapped(bytes: &[u8], fields: &mut Vec<(String, String)>) -> Option<Vec<u8>> {
    let Ok(Value::Object(outer)) = serde_json::from_slice::<Value>(bytes) else {
        return None;
    };
    let wrapper = outer.get("_rails")?.as_object()?;
    if let Some(purpose) = wrapper.get("pur").and_then(Value::as_str) {
        fields.push((tr!("What it is for"), purpose.to_owned()));
    }
    STANDARD.decode(wrapper.get("message")?.as_str()?).ok()
}

/// The session as a reader wants it: a JSON object's fields by name, and
/// anything else said for what it is.
fn laid_out(bytes: &[u8], fields: &mut Vec<(String, String)>) {
    if let Ok(Value::Object(session)) = serde_json::from_slice::<Value>(bytes) {
        for (key, value) in session {
            fields.push((
                key,
                match value {
                    Value::String(text) => text,
                    other => other.to_string(),
                },
            ));
        }
    } else if bytes.starts_with(b"\x04\x08") {
        fields.push((
            tr!("The session"),
            tr!("a Ruby Marshal, which tracewing does not unpack"),
        ));
    } else if let Ok(text) = std::str::from_utf8(bytes) {
        fields.push((tr!("The session"), text.to_owned()));
    } else {
        fields.push((
            tr!("The session"),
            tr!("{n} bytes, in no format this knows", n = bytes.len()),
        ));
    }
}

/// What a framework's session cookie carries, without the key that signed it.
///
/// Flask, Django and Rails each keep the session in the cookie rather than on
/// the server, so the whole of it travels in every request: the user id, the
/// flags, a CSRF token, when it was signed. Only changing one needs the key.
/// Reading one is a matter of taking the signature off the end and the zlib
/// off the front -- which is exactly what nobody wants to do by hand at the
/// moment they are wondering why a login did not stick.
pub fn session_cookie(input: &Input) -> Option<Reading> {
    let text = input.text.trim().trim_matches('"');
    if text.len() < 24 || text.len() > crate::MAX_INPUT || text.contains(char::is_whitespace) {
        return None;
    }
    let session = flask(text)
        .or_else(|| django(text))
        .or_else(|| rails(text))?;
    let mut fields = vec![(tr!("Framework"), session.framework.said())];
    if let Some(at) = session.at {
        fields.push((
            tr!("Signed"),
            format!(
                "{}, {}",
                crate::times::local_time_at(at, input.offset),
                crate::times::relative(at, input.now)
            ),
        ));
    }
    match &session.payload {
        Some(bytes) => {
            let inner = unwrapped(bytes, &mut fields);
            laid_out(inner.as_deref().unwrap_or(bytes), &mut fields);
        }
        None => fields.push((
            tr!("The session"),
            tr!("encrypted: it needs the key, which tracewing does not have"),
        )),
    }
    fields.push((tr!("Signature"), session.signature));
    Some(Reading::new(
        "session-cookie",
        tr!("A framework's session cookie"),
        Confidence::Certain,
        Shown::Fields(fields),
    ))
}

/// Secret fields: the private half of an asymmetric key, or the whole of a
/// symmetric one. A set published so the world can check signatures has no
/// business carrying any of them.
const SECRET: [&str; 7] = ["d", "p", "q", "dp", "dq", "qi", "k"];

/// A JWK, or a whole JWKS, as what the keys are.
///
/// The JSON around them is already readable, and that is the trouble: what a
/// reader cannot see is that `n` -- three hundred characters of base64url --
/// says 2048-bit, or that a one-letter `d` has turned a public set into a
/// published private key. Both are a line here.
pub fn jwk(input: &Input) -> Option<Reading> {
    let value: Value = serde_json::from_str(input.text.trim()).ok()?;
    let object = value.as_object()?;
    // A set is read to find one key in it, so a row each.
    if let Some(keys) = object.get("keys").and_then(Value::as_array) {
        let rows: Vec<Vec<String>> = keys
            .iter()
            .filter_map(Value::as_object)
            .map(|key| {
                vec![
                    said(key, "kid"),
                    what_it_is(key),
                    what_for(key),
                    said(key, "alg"),
                ]
            })
            .filter(|row| !row[1].is_empty())
            .collect();
        if rows.is_empty() {
            return None;
        }
        return Some(Reading::new(
            "jwk",
            tr!("JWKS, a set of keys"),
            Confidence::Certain,
            Shown::Table {
                head: vec![
                    tr!("Key id"),
                    tr!("What it is"),
                    tr!("For"),
                    tr!("Algorithm"),
                ],
                rows,
            },
        ));
    }
    let kind = what_it_is(object);
    if kind.is_empty() {
        return None;
    }
    let mut fields = vec![(tr!("What it is"), kind)];
    for (label, text) in [
        (tr!("For"), what_for(object)),
        (tr!("Algorithm"), said(object, "alg")),
        (tr!("Key id"), said(object, "kid")),
    ] {
        if !text.is_empty() {
            fields.push((label, text));
        }
    }
    if let Some(certificates) = object.get("x5c").and_then(Value::as_array) {
        fields.push((tr!("Certificates"), certificates.len().to_string()));
    }
    // Said plainly and not only marked, because this is the one thing here
    // that is somebody's morning: a table cell has room for a word, and one
    // key has room for the sentence.
    if object.contains_key("k") {
        fields.push((tr!("Careful"), tr!("the key itself is here")));
    } else if SECRET.iter().any(|name| object.contains_key(*name)) {
        fields.push((
            tr!("Careful"),
            tr!("the private half is here, not only the public one"),
        ));
    }
    Some(Reading::new(
        "jwk",
        tr!("JWK, a key as JSON"),
        Confidence::Certain,
        Shown::Fields(fields),
    ))
}

/// What a key is, in the words a reader wants: the family, the size where
/// the family does not already say it, and whether the secret came too.
fn what_it_is(key: &Map<String, Value>) -> String {
    let kind = key.get("kty").and_then(Value::as_str).unwrap_or_default();
    let mut what = match kind {
        // The curve names the size, so `P-256` is the whole answer.
        "EC" | "OKP" => match key.get("crv").and_then(Value::as_str) {
            Some(curve) => format!("{kind} {curve}"),
            None => kind.to_owned(),
        },
        // `n` is the modulus, and the modulus is the key's size.
        "RSA" => sized(kind, modulus_bits(key, "n")),
        "oct" => sized(kind, bits(key, "k")),
        "" => return String::new(),
        other => other.to_owned(),
    };
    if SECRET.iter().any(|name| key.contains_key(*name)) {
        what.push_str(", ");
        what.push_str(&tr!("private"));
    }
    what
}

fn sized(kind: &str, bits: Option<usize>) -> String {
    match bits {
        Some(bits) => format!("{kind} {}", tr!("{bits}-bit", bits = bits.to_string())),
        None => kind.to_owned(),
    }
}

/// The size of a base64url field, in bits. Every octet counts: for a
/// symmetric key a leading zero is key material like any other byte, and
/// dropping it would understate the key by eight bits a time.
fn bits(key: &Map<String, Value>, name: &str) -> Option<usize> {
    match URL_SAFE_NO_PAD.decode(key.get(name)?.as_str()?).ok()?.len() {
        0 => None,
        bytes => Some(bytes * 8),
    }
}

/// The same for a modulus, where one leading zero byte may have come along
/// to keep the number positive. RFC 7518 says it should not be there at all
/// -- `n` is a Base64urlUInt and those carry no leading zero -- but
/// producers emit one, and counting it would call every such key 2056-bit.
/// Exactly one is dropped: past that, the bytes are the number.
fn modulus_bits(key: &Map<String, Value>, name: &str) -> Option<usize> {
    let bytes = URL_SAFE_NO_PAD.decode(key.get(name)?.as_str()?).ok()?;
    let bytes = match bytes.split_first() {
        Some((0, rest)) if !rest.is_empty() => rest,
        _ => bytes.as_slice(),
    };
    match bytes.len() {
        0 => None,
        bytes => Some(bytes * 8),
    }
}

/// What the key is for: `use`, or the operations where a key says those
/// instead.
fn what_for(key: &Map<String, Value>) -> String {
    if let Some(what) = key.get("use").and_then(Value::as_str) {
        return match what {
            "sig" => tr!("signing"),
            "enc" => tr!("encryption"),
            other => other.to_owned(),
        };
    }
    key.get("key_ops")
        .and_then(Value::as_array)
        .map(|ops| {
            ops.iter()
                .filter_map(Value::as_str)
                .collect::<Vec<&str>>()
                .join(", ")
        })
        .unwrap_or_default()
}

/// A string field as it was written, or nothing where it is absent.
fn said(key: &Map<String, Value>, name: &str) -> String {
    key.get(name)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
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

    fn part(json: &str) -> String {
        URL_SAFE_NO_PAD.encode(json)
    }

    #[test]
    fn an_encrypted_token_says_what_it_can_and_stops() {
        let header = part(r#"{"alg":"RSA-OAEP","enc":"A256GCM","kid":"2026-11"}"#);
        let token = format!("{header}.key.iv.cipher.tag");
        let found = fields(&token, "jwe");
        assert_eq!(found[0].1, "RSA-OAEP");
        assert_eq!(found[1].1, "A256GCM");
        assert_eq!(found[2].1, "2026-11");
        assert!(found[3].1.contains("needs the key"));
        // A token of three parts is a JWT, and is read whole.
        let jwt = format!("{}.{}.sig", part(r#"{"alg":"HS256"}"#), part(r#"{"a":1}"#));
        assert!(by_id(&read(&jwt), "jwe").is_none());
        assert!(by_id(&read(&jwt), "jwt").is_some());
        // Five parts without `enc` is something else.
        let not = format!("{}.b.c.d.e", part(r#"{"alg":"RSA-OAEP"}"#));
        assert!(by_id(&read(&not), "jwe").is_none());
    }

    #[test]
    fn paseto_says_its_version_and_what_it_holds() {
        let public = format!("v4.public.{}", part(r#"{"sub":"42"}"#));
        let found = fields(&public, "paseto");
        assert_eq!(found[0].1, "v4");
        assert!(found[1].1.contains("signed"));
        assert!(
            found
                .iter()
                .any(|(name, value)| name == "sub" && value == "42")
        );
        let local = format!("v4.local.{}", part("not readable at all"));
        let found = fields(&local, "paseto");
        assert!(found.last().unwrap().1.contains("needs the key"));
        assert!(by_id(&read("v9.local.abcdefgh"), "paseto").is_none());
    }

    /// A signature is never checked here, so a plausible one will do: 27
    /// base64url characters, as HMAC-SHA1 comes out.
    const SIG: &str = "JIIA9KIUf0_-zlSlGQad890UH18";

    fn stamp(at: u32) -> String {
        URL_SAFE_NO_PAD.encode(at.to_be_bytes())
    }

    #[test]
    fn a_flask_cookie_reads_as_its_session() {
        let payload = URL_SAFE_NO_PAD.encode(r#"{"user_id":"42","_fresh":true}"#);
        let cookie = format!("{payload}.{}.{SIG}", stamp(crate::tests::NOW as u32));
        let found = fields(&cookie, "session-cookie");
        let said: Vec<(&str, &str)> = found
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        assert!(said[0].1.contains("Flask"), "{said:?}");
        assert!(
            said.iter().any(|(name, value)| *name == "Signed"
                && value.starts_with("2026-11-12 21:13:20 (+07:00)")),
            "{said:?}"
        );
        assert!(said.contains(&("user_id", "42")));
        assert!(said.contains(&("_fresh", "true")));
        assert!(said.contains(&("Signature", SIG)));
    }

    #[test]
    fn a_django_cookie_has_its_zlib_taken_off_first() {
        use std::io::Write as _;
        let session = r#"{"_auth_user_id":"42","_session_expiry":1209600}"#;
        let mut packed = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
        packed.write_all(session.as_bytes()).unwrap();
        let payload = URL_SAFE_NO_PAD.encode(packed.finish().unwrap());
        // 1xRVYG is 1794492800 in the base62 Django writes times in.
        // 32 bytes of signature, as HMAC-SHA256 gives: 43 characters.
        let signature = URL_SAFE_NO_PAD.encode([0x5au8; 32]);
        let cookie = format!(".{payload}:1xRVYG:{signature}");
        let found = fields(&cookie, "session-cookie");
        let said: Vec<(&str, &str)> = found
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        assert_eq!(said[0].1, "Django");
        assert!(
            said.iter().any(|(name, value)| *name == "Signed"
                && value.starts_with("2026-11-12 21:13:20 (+07:00)")),
            "{said:?}"
        );
        assert!(said.contains(&("_auth_user_id", "42")));
        assert!(said.contains(&("_session_expiry", "1209600")));
    }

    #[test]
    fn a_rails_cookie_is_unwrapped_once_more() {
        let inner = STANDARD.encode(r#"{"session_id":"9f8c2b1a","user_id":42}"#);
        let wrapped = STANDARD.encode(format!(
            r#"{{"_rails":{{"message":"{inner}","exp":null,"pur":"cookie._session_id"}}}}"#
        ));
        let cookie = format!("{wrapped}--5e8da9fa1e0402217940caaf6feb89ce976a72d6");
        let found = fields(&cookie, "session-cookie");
        let said: Vec<(&str, &str)> = found
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        assert_eq!(said[0].1, "Rails, signed");
        assert!(said.contains(&("What it is for", "cookie._session_id")));
        assert!(said.contains(&("session_id", "9f8c2b1a")));
        assert!(said.contains(&("user_id", "42")));
        // Nothing here says when it was signed, so nothing is made up.
        assert!(!said.iter().any(|(name, _)| *name == "Signed"));
    }

    #[test]
    fn a_rails_encrypted_cookie_says_it_needs_the_key() {
        let cookie = format!(
            "{}--{}--{}",
            STANDARD.encode([7u8; 48]),
            STANDARD.encode([1u8; 12]),
            STANDARD.encode([2u8; 16])
        );
        let found = fields(&cookie, "session-cookie");
        assert_eq!(found[0].1, "Rails, encrypted");
        assert!(found[1].1.contains("needs the key"), "{found:?}");
    }

    #[test]
    fn a_marshal_payload_is_named_rather_than_guessed_at() {
        let cookie = format!(
            "{}--5e8da9fa1e0402217940caaf6feb89ce976a72d6",
            STANDARD.encode(b"\x04\x08o:\x17ActionDispatch")
        );
        let found = fields(&cookie, "session-cookie");
        assert!(found[1].1.contains("Marshal"), "{found:?}");
    }

    #[test]
    fn what_is_not_a_session_cookie_is_not_one() {
        let jwt = format!("{}.{}.sig", part(r#"{"alg":"HS256"}"#), part(r#"{"a":1}"#));
        assert!(by_id(&read(&jwt), "session-cookie").is_none());
        // A time that is not a time: some other value with two dots in it.
        let payload = URL_SAFE_NO_PAD.encode(r#"{"a":1}"#);
        assert!(by_id(&read(&format!("{payload}.AAAA.{SIG}")), "session-cookie").is_none());
        // A signature too short to be one.
        assert!(
            by_id(
                &read(&format!(
                    "{payload}.{}.sig",
                    stamp(crate::tests::NOW as u32)
                )),
                "session-cookie"
            )
            .is_none()
        );
        // Two dashes in a value, with no hex signature after them.
        assert!(by_id(&read("eyJhIjoxfQ--not-a-signature"), "session-cookie").is_none());
        // Base62 is not base64: an `_` in the time is not Django's.
        assert!(by_id(&read(&format!("{payload}:1xR_YG:{SIG}")), "session-cookie").is_none());
    }

    #[test]
    fn a_hash_is_known_by_its_size_and_no_more() {
        assert_eq!(
            line(&read(&"a".repeat(64)), "hash"),
            "SHA-256, or SHA3-256, or BLAKE2s"
        );
        assert_eq!(line(&read(&"b".repeat(32)), "hash"), "MD5, or MD4, or NTLM");
        assert!(by_id(&read(&"c".repeat(33)), "hash").is_none());
        assert_eq!(
            by_id(&read_under("etag", &"d".repeat(40)), "hash")
                .unwrap()
                .confidence,
            Confidence::Likely
        );
    }

    #[test]
    fn a_hashed_password_says_how_hard_it_was_made() {
        let found = fields(
            "$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj4J/HS.aW8K",
            "crypt",
        );
        assert_eq!(found[0].1, "bcrypt");
        assert_eq!(found[1].1, "2 to the 12");
        let argon = fields("$argon2id$v=19$m=65536,t=3,p=4$c29tZXNhbHQ$hash", "crypt");
        assert_eq!(argon[0].1, "argon2id");
        assert!(
            argon
                .iter()
                .any(|(name, value)| name == "Memory" && value == "65536 KiB")
        );
        assert!(
            argon
                .iter()
                .any(|(name, value)| name == "Passes" && value == "3")
        );
        assert!(by_id(&read("$nope$1$2$3"), "crypt").is_none());
    }

    #[test]
    fn a_pem_says_what_it_holds() {
        let der = base64::engine::general_purpose::STANDARD.encode([0x30, 0x82, 0, 10]);
        let pem = format!("-----BEGIN CERTIFICATE-----\n{der}\n-----END CERTIFICATE-----");
        let found = fields(&pem, "pem");
        assert_eq!(found[0].1, "CERTIFICATE");
        assert_eq!(found[1].1, "4 B of DER");
        assert!(by_id(&read("-----BEGIN -----"), "pem").is_none());
    }

    #[test]
    fn saml_says_which_message_it_is() {
        let xml = r#"<samlp:AuthnRequest xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" ID="_abc" IssueInstant="2026-11-12T14:13:20Z" Destination="https://idp.example.com/sso"><saml:Issuer>https://app.example.com</saml:Issuer></samlp:AuthnRequest>"#;
        let found = fields(xml, "saml");
        assert_eq!(found[0].1, "AuthnRequest");
        assert!(
            found
                .iter()
                .any(|(_, value)| value == "https://app.example.com")
        );
        assert!(
            found
                .iter()
                .any(|(_, value)| value == "https://idp.example.com/sso")
        );
        assert!(by_id(&read("<html><body>no saml here</body></html>"), "saml").is_none());
    }

    /// How a SAML response actually travels: deflated, then base64, in a
    /// form field. Until the chain could see deflate with nothing at its
    /// front to name it, this read as nothing at all -- the converter above
    /// was never reached by anything but a test.
    #[test]
    fn saml_is_read_the_way_it_travels() {
        use base64::Engine as _;
        use std::io::Write as _;
        let xml = concat!(
            r#"<samlp:Response xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" "#,
            r#"ID="_8e8dc5f6" Version="2.0" IssueInstant="2026-11-12T14:13:20Z" "#,
            r#"Destination="https://app.example.com/acs">"#,
            r#"<saml:Issuer>https://idp.example.org/metadata</saml:Issuer>"#,
            r#"</samlp:Response>"#
        );
        let mut packed =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::fast());
        packed.write_all(xml.as_bytes()).unwrap();
        let travelling = base64::engine::general_purpose::STANDARD.encode(packed.finish().unwrap());

        let found = read(&travelling);
        let saml = by_id(&found, "saml").expect("the SAML in it");
        assert_eq!(saml.chain, vec!["base64", "deflate", "saml"]);
        let Shown::Fields(fields) = &saml.shown else {
            panic!("fields: {saml:?}")
        };
        assert_eq!(fields[0].1, "Response");
        assert!(
            fields
                .iter()
                .any(|(_, value)| value == "https://idp.example.org/metadata"),
            "{fields:?}"
        );
    }
    fn table(text: &str, id: &str) -> Vec<Vec<String>> {
        let found = read(text);
        match by_id(&found, id).map(|r| &r.shown) {
            Some(Shown::Table { rows, .. }) => rows.clone(),
            other => panic!("no table for {id}: {other:?}"),
        }
    }

    /// `n` is what says 2048, and nothing else in the key does.
    #[test]
    fn an_rsa_key_says_the_size_its_modulus_makes_it() {
        let modulus = URL_SAFE_NO_PAD.encode([0x80u8; 256]);
        let key = format!(
            r#"{{"kty":"RSA","n":"{modulus}","e":"AQAB","alg":"RS256","use":"sig","kid":"2026-11"}}"#
        );
        let found = fields(&key, "jwk");
        let said: Vec<(&str, &str)> = found
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        assert!(said.contains(&("What it is", "RSA 2048-bit")), "{said:?}");
        assert!(said.contains(&("For", "signing")), "{said:?}");
        assert!(said.contains(&("Algorithm", "RS256")), "{said:?}");
        assert!(said.contains(&("Key id", "2026-11")), "{said:?}");

        // A leading zero byte keeps the modulus positive and is not part of
        // the number: the key is still 2048-bit, not 2056.
        let mut padded = vec![0u8];
        padded.extend_from_slice(&[0x80u8; 256]);
        let key = format!(
            r#"{{"kty":"RSA","n":"{}","e":"AQAB"}}"#,
            URL_SAFE_NO_PAD.encode(&padded)
        );
        assert_eq!(fields(&key, "jwk")[0].1, "RSA 2048-bit");
    }

    /// A curve names its own size, so there is nothing to measure.
    #[test]
    fn a_set_is_a_row_for_each_key() {
        let modulus = URL_SAFE_NO_PAD.encode([0x7fu8; 128]);
        let set = format!(
            r#"{{"keys":[
                {{"kty":"EC","crv":"P-256","x":"AA","y":"AQ","use":"sig","kid":"ec"}},
                {{"kty":"RSA","n":"{modulus}","e":"AQAB","alg":"RS256","kid":"rsa"}},
                {{"kty":"OKP","crv":"Ed25519","x":"AA","key_ops":["verify"],"kid":"ed"}}
            ]}}"#
        );
        let rows = table(&set, "jwk");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], vec!["ec", "EC P-256", "signing", ""]);
        assert_eq!(rows[1], vec!["rsa", "RSA 1024-bit", "", "RS256"]);
        // `key_ops` where a key says that instead of `use`.
        assert_eq!(rows[2], vec!["ed", "OKP Ed25519", "verify", ""]);
    }

    /// The one thing here that is somebody's morning.
    #[test]
    fn a_secret_that_came_with_a_public_key_is_said_plainly() {
        let key = r#"{"kty":"EC","crv":"P-256","x":"AA","y":"AQ","d":"c2VjcmV0"}"#;
        let found = fields(key, "jwk");
        assert_eq!(found[0].1, "EC P-256, private");
        assert!(
            found
                .iter()
                .any(|(name, value)| name == "Careful" && value.contains("private half")),
            "{found:?}"
        );
        // A symmetric key is all secret, and `k` is where it sits.
        let key = r#"{"kty":"oct","k":"AAAAAAAAAAAAAAAAAAAAAA","alg":"HS256"}"#;
        let found = fields(key, "jwk");
        assert_eq!(found[0].1, "oct 128-bit, private");
        assert!(
            found
                .iter()
                .any(|(name, value)| name == "Careful" && value == "the key itself is here"),
            "{found:?}"
        );
    }

    #[test]
    fn what_is_not_a_key_is_not_read() {
        // JSON, and readable as JSON, but nothing says it is a key.
        assert!(by_id(&read(r#"{"name":"ana","age":7}"#), "jwk").is_none());
        assert!(by_id(&read(r#"{"kty":""}"#), "jwk").is_none());
        // A set with nothing in it, and a set whose members are not keys.
        assert!(by_id(&read(r#"{"keys":[]}"#), "jwk").is_none());
        assert!(by_id(&read(r#"{"keys":[{"id":1}]}"#), "jwk").is_none());
        assert!(by_id(&read("not json at all"), "jwk").is_none());
    }
}
