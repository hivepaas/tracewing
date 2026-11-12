//! Numbers said another way: in another base, as the flags they set, as a
//! size a reader feels.

use crate::{Confidence, Input, Reading, Shown};
use tracewing_i18n::tr;

/// `512 B`, `1.5 MiB`: a count of bytes as a reader holds it.
pub fn bytes_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024. && unit + 1 < UNITS.len() {
        size /= 1024.;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else if size < 10. {
        format!("{size:.1} {}", UNITS[unit])
    } else {
        format!("{size:.0} {}", UNITS[unit])
    }
}

fn whole(text: &str) -> Option<u64> {
    let text = text.trim_matches('"');
    if text.len() > 20 || text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// The same number in hex and in binary, for a value that is really bits.
pub fn radix(input: &Input) -> Option<Reading> {
    let n = whole(input.text)?;
    // Below sixteen there is nothing to see, and past a word the binary is a
    // wall.
    if n < 16 || n > u32::MAX as u64 {
        return None;
    }
    Some(Reading::new(
        "radix",
        tr!("In hex and binary"),
        Confidence::Possible,
        Shown::Line(format!("0x{n:x}  0b{n:b}")),
    ))
}

/// Which bits a mask sets, for the fields that are flags.
pub fn flags(input: &Input) -> Option<Reading> {
    let n = whole(input.text)?;
    if n == 0 || n > u32::MAX as u64 || n.count_ones() < 2 {
        return None;
    }
    let bits: Vec<String> = (0..32)
        .filter(|b| n >> b & 1 == 1)
        .map(|b| b.to_string())
        .collect();
    let confidence = if input.field_says(&["flag", "mask", "perm", "bits", "mode", "rights"]) {
        Confidence::Likely
    } else {
        Confidence::Possible
    };
    Some(Reading::new(
        "flags",
        tr!("Bits set"),
        confidence,
        Shown::Line(format!("{} (0x{n:x})", bits.join(", "))),
    ))
}

/// A count of bytes as a size.
pub fn size(input: &Input) -> Option<Reading> {
    let n = whole(input.text)?;
    if n < 1024 {
        return None;
    }
    let confidence = if input.field_says(&["size", "bytes", "length", "len", "contentlength"]) {
        Confidence::Likely
    } else {
        Confidence::Possible
    };
    Some(Reading::new(
        "size",
        tr!("As a size"),
        confidence,
        Shown::Line(bytes_size(n)),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{by_id, line, read, read_under};

    #[test]
    fn a_size_is_said_in_the_unit_a_reader_feels() {
        assert_eq!(bytes_size(0), "0 B");
        assert_eq!(bytes_size(512), "512 B");
        assert_eq!(bytes_size(1024), "1.0 KiB");
        assert_eq!(bytes_size(1_572_864), "1.5 MiB");
        assert_eq!(bytes_size(104_857_600), "100 MiB");
        assert_eq!(
            line(&read_under("content_length", "1048576"), "size"),
            "1.0 MiB"
        );
    }

    #[test]
    fn a_number_is_offered_in_the_other_bases() {
        assert_eq!(line(&read("1234"), "radix"), "0x4d2  0b10011010010");
        assert!(by_id(&read("7"), "radix").is_none());
    }

    #[test]
    fn a_mask_says_which_bits_it_sets() {
        assert_eq!(line(&read("137"), "flags"), "0, 3, 7 (0x89)");
        // One bit is a number, not a mask.
        assert!(by_id(&read("8"), "flags").is_none());
        assert_eq!(
            by_id(&read_under("permissions", "137"), "flags")
                .unwrap()
                .confidence,
            Confidence::Likely
        );
    }
}
