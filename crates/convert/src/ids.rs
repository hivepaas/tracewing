//! Identifiers, and the time most of them carry without saying so.

use crate::{Confidence, Input, Reading, Shown, times};
use tracewing_i18n::tr;

/// A moment, as a field under an identifier.
fn when(at: i64, input: &Input) -> (String, String) {
    (
        tr!("Time"),
        format!(
            "{}, {}",
            times::local_time_at(at, input.offset),
            times::relative(at, input.now)
        ),
    )
}

fn hex_bytes(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    text.as_bytes()
        .chunks(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect()
}

/// A UUID: which version made it, and the time the ones that carry one hold.
pub fn uuid(input: &Input) -> Option<Reading> {
    let text = input
        .text
        .trim_matches('"')
        .trim_matches(|c| c == '{' || c == '}');
    let flat: String = text.chars().filter(|c| *c != '-').collect();
    if flat.len() != 32 || (text.len() != 32 && text.len() != 36) {
        return None;
    }
    if text.len() == 36 {
        let dashes: Vec<usize> = text.match_indices('-').map(|(at, _)| at).collect();
        if dashes != [8, 13, 18, 23] {
            return None;
        }
    }
    let bytes = hex_bytes(&flat)?;
    let version = bytes[6] >> 4;
    let variant = match bytes[8] >> 6 {
        0b10 => "RFC 4122",
        0b00 => "NCS",
        0b110 => "Microsoft",
        _ => "reserved",
    };
    let mut fields = vec![
        (tr!("Version"), version.to_string()),
        (tr!("Variant"), variant.to_owned()),
    ];
    // v1 and v6 count 100 ns since 1582; v7 counts milliseconds since 1970.
    let hundreds = |high: u64, mid: u64, low: u64| high << 48 | mid << 32 | low;
    let ticks = match version {
        1 => Some(hundreds(
            u64::from_be_bytes([0, 0, 0, 0, 0, 0, bytes[6] & 0x0f, bytes[7]]),
            u64::from(u16::from_be_bytes([bytes[4], bytes[5]])),
            u64::from(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])),
        )),
        6 => Some(
            u64::from_be_bytes([
                0, 0, bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5],
            ]) << 12
                | u64::from(u16::from_be_bytes([bytes[6] & 0x0f, bytes[7]])),
        ),
        _ => None,
    };
    if let Some(ticks) = ticks {
        // 1582-10-15 to 1970, in seconds.
        let at = ticks as i64 / 10_000_000 - 12_219_292_800;
        fields.push(when(at, input));
    } else if version == 7 {
        let millis = u64::from_be_bytes([
            0, 0, bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5],
        ]);
        fields.push(when(millis as i64 / 1_000, input));
    }
    Some(Reading::new(
        "uuid",
        tr!("UUID"),
        Confidence::Certain,
        Shown::Fields(fields),
    ))
}

/// Crockford's base32, as a ULID is written.
fn crockford(text: &str) -> Option<u128> {
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let mut n: u128 = 0;
    for byte in text.bytes() {
        let upper = byte.to_ascii_uppercase();
        let digit = ALPHABET.iter().position(|c| *c == upper)?;
        n = n.checked_mul(32)?.checked_add(digit as u128)?;
    }
    Some(n)
}

/// A ULID: 26 characters, the first ten of them a time.
pub fn ulid(input: &Input) -> Option<Reading> {
    let text = input.text.trim_matches('"');
    if text.len() != 26 || !text.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return None;
    }
    let millis = crockford(&text[..10])?;
    let at = (millis / 1_000) as i64;
    // Outside this it is 26 characters of something else.
    if !(1_000_000_000..4_102_444_800).contains(&at) {
        return None;
    }
    crockford(&text[10..])?;
    Some(Reading::new(
        "ulid",
        tr!("ULID"),
        Confidence::Certain,
        Shown::Fields(vec![when(at, input)]),
    ))
}

/// MongoDB's ObjectId: 24 hex characters that open with a time.
pub fn object_id(input: &Input) -> Option<Reading> {
    let text = input.text.trim_matches('"');
    if text.len() != 24 {
        return None;
    }
    let bytes = hex_bytes(text)?;
    let at = i64::from(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
    if !(1_000_000_000..4_102_444_800).contains(&at) {
        return None;
    }
    let counter = u32::from_be_bytes([0, bytes[9], bytes[10], bytes[11]]);
    Some(Reading::new(
        "object-id",
        tr!("ObjectId"),
        Confidence::Certain,
        Shown::Fields(vec![when(at, input), (tr!("Counter"), counter.to_string())]),
    ))
}

/// A KSUID: 27 characters of base62, the first four bytes a time.
pub fn ksuid(input: &Input) -> Option<Reading> {
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let text = input.text.trim_matches('"');
    if text.len() != 27 || !text.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return None;
    }
    // Twenty bytes is more than any integer here holds, so the digits are
    // multiplied into the bytes themselves.
    let mut bytes = [0u8; 20];
    for digit in text.bytes() {
        let digit = ALPHABET.iter().position(|c| *c == digit)? as u32;
        let mut carry = digit;
        for byte in bytes.iter_mut().rev() {
            let wide = u32::from(*byte) * 62 + carry;
            *byte = wide as u8;
            carry = wide >> 8;
        }
        if carry != 0 {
            return None;
        }
    }
    // KSUID counts from 2014-05-13.
    let at =
        i64::from(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])) + 1_400_000_000;
    if !(1_400_000_000..4_102_444_800).contains(&at) {
        return None;
    }
    Some(Reading::new(
        "ksuid",
        tr!("KSUID"),
        Confidence::Certain,
        Shown::Fields(vec![when(at, input)]),
    ))
}

/// A snowflake: a number whose top bits are a time, counted from an epoch its
/// maker chose. Which maker is not in the number, so both are offered.
pub fn snowflake(input: &Input) -> Option<Reading> {
    const EPOCHS: [(&str, i64); 2] = [
        ("Twitter", 1_288_834_974_657),
        ("Discord", 1_420_070_400_000),
    ];
    let text = input.text.trim_matches('"');
    if text.len() < 18 || text.len() > 20 || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let n: u64 = text.parse().ok()?;
    let millis = (n >> 22) as i64;
    let mut fields = Vec::new();
    for (maker, epoch) in EPOCHS {
        let at = (millis + epoch) / 1_000;
        if (1_262_304_000..4_102_444_800).contains(&at) {
            let (_, said) = when(at, input);
            fields.push((maker.to_owned(), said));
        }
    }
    if fields.is_empty() {
        return None;
    }
    fields.push((tr!("Worker"), ((n >> 12) & 0x3ff).to_string()));
    fields.push((tr!("Sequence"), (n & 0xfff).to_string()));
    let confidence = if input.field_says(&["id", "snowflake"]) {
        Confidence::Likely
    } else {
        Confidence::Possible
    };
    Some(Reading::new(
        "snowflake",
        tr!("Snowflake"),
        confidence,
        Shown::Fields(fields),
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

    fn field(text: &str, id: &str, name: &str) -> String {
        fields(text, id)
            .into_iter()
            .find(|(f, _)| f == name)
            .unwrap_or_else(|| panic!("no {name} in {id}"))
            .1
    }

    #[test]
    fn a_uuid_says_its_version_and_the_time_it_carries() {
        assert_eq!(
            field("018f3f1e-4c7a-7000-8000-000000000000", "uuid", "Version"),
            "7"
        );
        assert!(
            field("018f3f1e-4c7a-7000-8000-000000000000", "uuid", "Time").starts_with("2024-05-03")
        );
        // v4 carries no time at all.
        let v4 = fields("f47ac10b-58cc-4372-a567-0e02b2c3d479", "uuid");
        assert_eq!(v4[0].1, "4");
        assert!(v4.iter().all(|(name, _)| name != "Time"));
        // v1, counting from 1582.
        assert!(
            field("2c4a5c20-8a9e-11ec-9d64-0242ac120002", "uuid", "Time")
                .starts_with("2022-02-11 01:20:49")
        );
    }

    #[test]
    fn what_is_not_a_uuid_is_not_one() {
        assert!(by_id(&read("f47ac10b58cc4372a5670e02b2c3d47"), "uuid").is_none());
        assert!(by_id(&read("f47ac10b-58cc-4372-a567-0e02b2c3d47z"), "uuid").is_none());
        assert!(by_id(&read("f47ac10b58-cc4372-a567-0e02b2c3d479"), "uuid").is_none());
    }

    #[test]
    fn the_time_in_an_identifier_is_read() {
        assert!(field("01HQ3M5ZPJ0000000000000000", "ulid", "Time").starts_with("2024-02-20"));
        assert!(field("507f1f77bcf86cd799439011", "object-id", "Time").starts_with("2012-10-"));
        assert_eq!(
            field("507f1f77bcf86cd799439011", "object-id", "Counter"),
            "4427793"
        );
        assert!(field("0ujtsYcgvSTl8PAuAdqWYZLAJhJ", "ksuid", "Time").starts_with("2017-10-10"));
    }

    #[test]
    fn a_snowflake_is_read_against_both_epochs() {
        let found = fields("175928847299117063", "snowflake");
        let discord = found.iter().find(|(name, _)| name == "Discord").unwrap();
        assert!(discord.1.starts_with("2016-04-30"), "{}", discord.1);
        assert!(found.iter().any(|(name, _)| name == "Twitter"));
        assert!(found.iter().any(|(name, _)| name == "Sequence"));
    }
}
