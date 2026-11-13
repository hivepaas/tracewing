//! Tokens, keys and hashes: what can be read of them without the secret.
//!
//! Nothing here verifies anything and nothing here decrypts. What a token
//! claims, what a key file holds, what a hash is the size of — that is what a
//! reader wants at the moment they are looking at it, and all of it is in the
//! bytes themselves.

use crate::{Confidence, Input, Reading, Shown};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
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
}
