//! Addresses and ports, said the way a reader looks them up.

use crate::{Confidence, Input, Reading, Shown};
use tracewing_i18n::tr;

/// The ports worth naming: what a flow on this machine is likely to meet.
const SERVICES: &[(u16, &str)] = &[
    (20, "FTP data"),
    (21, "FTP"),
    (22, "SSH"),
    (23, "Telnet"),
    (25, "SMTP"),
    (53, "DNS"),
    (80, "HTTP"),
    (110, "POP3"),
    (123, "NTP"),
    (143, "IMAP"),
    (389, "LDAP"),
    (443, "HTTPS"),
    (445, "SMB"),
    (465, "SMTPS"),
    (587, "SMTP submission"),
    (636, "LDAPS"),
    (853, "DNS over TLS"),
    (993, "IMAPS"),
    (995, "POP3S"),
    (1080, "SOCKS"),
    (1433, "SQL Server"),
    (1521, "Oracle"),
    (3000, "Node and friends"),
    (3306, "MySQL"),
    (3389, "RDP"),
    (5432, "PostgreSQL"),
    (5672, "AMQP"),
    (6379, "Redis"),
    (8080, "HTTP, another"),
    (8443, "HTTPS, another"),
    (9000, "PHP-FPM and friends"),
    (9092, "Kafka"),
    (9200, "Elasticsearch"),
    (11211, "memcached"),
    (27017, "MongoDB"),
];

fn whole(text: &str) -> Option<u64> {
    let text = text.trim_matches('"');
    if text.is_empty() || text.len() > 20 || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// An address written as the number it is, as a database column and a log
/// line often hold it.
pub fn ip_number(input: &Input) -> Option<Reading> {
    let n = whole(input.text)?;
    // Below this it is a number; `0.0.4.1` is nobody's address.
    if !(16_777_216..=u64::from(u32::MAX)).contains(&n) {
        return None;
    }
    let octets = (n as u32).to_be_bytes();
    let address = std::net::Ipv4Addr::from(octets);
    let confidence = if input.field_says(&["ip", "addr", "host"]) {
        Confidence::Likely
    } else {
        Confidence::Possible
    };
    Some(Reading::new(
        "ip-number",
        tr!("As an address"),
        confidence,
        Shown::Line(format!("{address}{}", said_of(address.octets()))),
    ))
}

/// `· private`, `· loopback`, and so on, for the addresses that are special.
fn said_of(octets: [u8; 4]) -> String {
    let address = std::net::Ipv4Addr::from(octets);
    let what = if address.is_loopback() {
        tr!("this machine")
    } else if address.is_private() {
        tr!("a private network")
    } else if address.is_link_local() {
        tr!("link-local")
    } else if address.is_multicast() {
        tr!("multicast")
    } else if octets[0] == 100 && (64..128).contains(&octets[1]) {
        tr!("a carrier's network")
    } else {
        return String::new();
    };
    format!(" · {what}")
}

/// IPv6, written out in full and shortened, with the IPv4 inside it where
/// there is one.
pub fn ipv6(input: &Input) -> Option<Reading> {
    let text = input
        .text
        .trim_matches('"')
        .trim_matches(|c| c == '[' || c == ']');
    if !text.contains(':') || text.len() < 2 {
        return None;
    }
    let address: std::net::Ipv6Addr = text.parse().ok()?;
    let groups = address.segments();
    let full = groups
        .iter()
        .map(|group| format!("{group:04x}"))
        .collect::<Vec<_>>()
        .join(":");
    let mut fields = vec![(tr!("Short"), address.to_string()), (tr!("In full"), full)];
    if let Some(four) = address.to_ipv4_mapped() {
        fields.push((tr!("The IPv4 in it"), four.to_string()));
    } else if address.is_loopback() {
        fields.push((tr!("What it is"), tr!("this machine")));
    } else if address.segments()[0] & 0xfe00 == 0xfc00 {
        fields.push((tr!("What it is"), tr!("a private network")));
    }
    Some(Reading::new(
        "ipv6",
        tr!("IPv6"),
        Confidence::Certain,
        Shown::Fields(fields),
    ))
}

/// A range written as an address and a width: what it holds, from where to
/// where.
pub fn cidr(input: &Input) -> Option<Reading> {
    let text = input.text.trim_matches('"');
    let (address, width) = text.split_once('/')?;
    let width: u32 = width.parse().ok()?;
    let address: std::net::Ipv4Addr = address.parse().ok()?;
    if width > 32 {
        return None;
    }
    let base = u32::from(address) & u32::MAX.checked_shl(32 - width).unwrap_or(0);
    let last = base | u32::MAX.checked_shr(width).unwrap_or(0);
    let many = u64::from(last - base) + 1;
    Some(Reading::new(
        "cidr",
        tr!("A range of addresses"),
        Confidence::Certain,
        Shown::Fields(vec![
            (
                tr!("From"),
                format!(
                    "{}{}",
                    std::net::Ipv4Addr::from(base),
                    said_of(base.to_be_bytes())
                ),
            ),
            (tr!("To"), std::net::Ipv4Addr::from(last).to_string()),
            (tr!("How many"), crate::numbers::group_digits(many)),
        ]),
    ))
}

/// A port by what listens there, where anything well known does.
pub fn port(input: &Input) -> Option<Reading> {
    let n = whole(input.text)?;
    let port = u16::try_from(n).ok().filter(|port| *port > 0)?;
    let service = SERVICES
        .iter()
        .find(|(known, _)| *known == port)
        .map(|(_, name)| *name)?;
    let confidence = if input.field_says(&["port"]) {
        Confidence::Likely
    } else {
        Confidence::Possible
    };
    Some(Reading::new(
        "port",
        tr!("As a port"),
        confidence,
        Shown::Line(service.to_owned()),
    ))
}

/// A MAC address, and what its own bits say about it.
pub fn mac(input: &Input) -> Option<Reading> {
    let text = input.text.trim_matches('"');
    let separator = if text.contains(':') { ':' } else { '-' };
    let parts: Vec<&str> = text.split(separator).collect();
    if parts.len() != 6 || !parts.iter().all(|part| part.len() == 2) {
        return None;
    }
    let bytes: Vec<u8> = parts
        .iter()
        .map(|part| u8::from_str_radix(part, 16).ok())
        .collect::<Option<_>>()?;
    let mut said = Vec::new();
    if bytes[0] & 1 == 1 {
        said.push(tr!("to many, not to one"));
    }
    if bytes[0] & 2 == 2 {
        said.push(tr!("given by hand, not by the maker"));
    }
    Some(Reading::new(
        "mac",
        tr!("A MAC address"),
        Confidence::Certain,
        Shown::Line(if said.is_empty() {
            bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<Vec<_>>()
                .join(":")
        } else {
            said.join(", ")
        }),
    ))
}

/// RFC 3492's numbers. They are the algorithm's, not this crate's, and mean
/// nothing apart from it.
const BASE: u32 = 36;
const TMIN: u32 = 1;
const TMAX: u32 = 26;
const SKEW: u32 = 38;
const DAMP: u32 = 700;
const FIRST_BIAS: u32 = 72;
const FIRST_N: u32 = 128;

/// A name written in Punycode, as the name it stands for: `xn--bcher-kva`
/// is `bücher`.
///
/// A host in a `Host` header, a certificate or a redirect that the reader
/// cannot read is the whole reason for this. It is also why the reading says
/// which characters are not ASCII and what their code points are:
/// `xn--pypal-4ve` comes back as `pаypal`, which looks like a word a reader
/// knows until the `а` turns out to be U+0430, Cyrillic. Telling those apart
/// needs the code point, and nothing else here can show it.
///
/// What this does not do is judge. Saying a name is confusable needs the
/// Unicode confusables table, which is a different piece of work and a
/// dependency this crate does not have; what is here is the arithmetic of
/// RFC 3492 and the characters laid out for a reader to see.
pub fn punycode(input: &Input) -> Option<Reading> {
    let text = input.text.trim().trim_matches('"');
    // A host is 253 characters at the outside, and this is a host or it is
    // nothing: a sentence with `xn--` in the middle is not a name.
    if text.is_empty() || text.len() > 253 {
        return None;
    }
    let mut labels = Vec::new();
    let mut encoded = 0;
    for label in text.split('.') {
        let rest = label
            .get(..4)
            .filter(|prefix| prefix.eq_ignore_ascii_case("xn--"))
            .map(|_| &label[4..])
            .filter(|rest| !rest.is_empty());
        match rest {
            Some(rest) => {
                labels.push(decode(rest)?);
                encoded += 1;
            }
            // Every other label still has to be one, or this is prose.
            None if !label.is_empty()
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-') =>
            {
                labels.push(label.to_owned());
            }
            None => return None,
        }
    }
    if encoded == 0 {
        return None;
    }
    let name = labels.join(".");
    let mut fields = vec![(tr!("Name"), name.clone())];
    let odd = not_ascii(&name);
    if !odd.is_empty() {
        fields.push((tr!("Not ASCII"), odd));
    }
    Some(Reading::new(
        "punycode",
        tr!("Punycode"),
        Confidence::Certain,
        Shown::Fields(fields),
    ))
}

/// RFC 3492's decoder, which is arithmetic and no tables at all. `encoded`
/// is one label with its `xn--` already off.
fn decode(encoded: &str) -> Option<String> {
    // The last hyphen separates the characters that came through as
    // themselves from the digits that carry the rest. The digits are
    // alphanumeric, so a hyphen anywhere is this one.
    let (basic, digits) = match encoded.rfind('-') {
        Some(at) => (&encoded[..at], &encoded[at + 1..]),
        None => ("", encoded),
    };
    if !basic.is_ascii() {
        return None;
    }
    let mut out: Vec<char> = basic.chars().collect();
    let digits: Vec<char> = digits.chars().collect();
    let mut at = 0;
    let mut n = FIRST_N;
    let mut i: u32 = 0;
    let mut bias = FIRST_BIAS;
    while at < digits.len() {
        let was = i;
        let mut weight: u32 = 1;
        let mut k = BASE;
        loop {
            let digit = value(*digits.get(at)?)?;
            at += 1;
            i = i.checked_add(digit.checked_mul(weight)?)?;
            // `t` is `k - bias` held between the two bounds.
            let t = k.saturating_sub(bias).clamp(TMIN, TMAX);
            if digit < t {
                break;
            }
            weight = weight.checked_mul(BASE - t)?;
            k += BASE;
        }
        let length = out.len() as u32 + 1;
        bias = adapt(i - was, length, was == 0);
        n = n.checked_add(i / length)?;
        i %= length;
        out.insert(i as usize, char::from_u32(n)?);
        i += 1;
    }
    Some(out.into_iter().collect())
}

/// A digit of base 36: letters first, whatever their case, then the numerals.
fn value(c: char) -> Option<u32> {
    match c {
        'a'..='z' => Some(c as u32 - 'a' as u32),
        'A'..='Z' => Some(c as u32 - 'A' as u32),
        '0'..='9' => Some(c as u32 - '0' as u32 + 26),
        _ => None,
    }
}

/// How the bias moves after each character, so that a name of one script
/// does not pay for the distance to it more than once. RFC 3492's own.
fn adapt(delta: u32, length: u32, first: bool) -> u32 {
    let mut delta = match first {
        true => delta / DAMP,
        false => delta / 2,
    };
    delta += delta / length;
    let mut k = 0;
    while delta > ((BASE - TMIN) * TMAX) / 2 {
        delta /= BASE - TMIN;
        k += BASE;
    }
    k + (((BASE - TMIN + 1) * delta) / (delta + SKEW))
}

/// The characters that are not ASCII, each once and in the order they first
/// appear, with the code point beside them: what turns `pаypal` from a word
/// a reader thinks they know into a question they can answer.
fn not_ascii(name: &str) -> String {
    let mut seen: Vec<char> = Vec::new();
    for c in name.chars().filter(|c| !c.is_ascii()) {
        if !seen.contains(&c) {
            seen.push(c);
        }
    }
    let mut said: Vec<String> = seen
        .iter()
        .take(8)
        .map(|c| format!("{c} U+{:04X}", *c as u32))
        .collect();
    if seen.len() > 8 {
        said.push("…".to_owned());
    }
    said.join(", ")
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

    #[test]
    fn an_address_written_as_a_number_is_read_back() {
        assert_eq!(
            line(&read("3232235777"), "ip-number"),
            "192.168.1.1 · a private network"
        );
        assert_eq!(
            line(&read("2130706433"), "ip-number"),
            "127.0.0.1 · this machine"
        );
        assert_eq!(line(&read("134744072"), "ip-number"), "8.8.8.8");
        assert!(by_id(&read("1234"), "ip-number").is_none());
        assert_eq!(
            by_id(&read_under("client_ip", "134744072"), "ip-number")
                .unwrap()
                .confidence,
            Confidence::Likely
        );
    }

    #[test]
    fn ipv6_is_said_both_ways() {
        let found = fields("2001:db8::1", "ipv6");
        assert_eq!(found[0].1, "2001:db8::1");
        assert_eq!(found[1].1, "2001:0db8:0000:0000:0000:0000:0000:0001");
        let mapped = fields("::ffff:192.168.1.1", "ipv6");
        assert!(mapped.iter().any(|(_, value)| value == "192.168.1.1"));
        assert!(by_id(&read("not:an:address"), "ipv6").is_none());
    }

    #[test]
    fn a_range_says_where_it_runs() {
        let found = fields("192.168.1.0/24", "cidr");
        assert_eq!(found[0].1, "192.168.1.0 · a private network");
        assert_eq!(found[1].1, "192.168.1.255");
        assert_eq!(found[2].1, "256");
        let one = fields("10.0.0.7/32", "cidr");
        assert_eq!(one[2].1, "1");
        assert!(by_id(&read("192.168.1.0/48"), "cidr").is_none());
    }

    #[test]
    fn a_well_known_port_says_what_listens() {
        assert_eq!(line(&read_under("port", "5432"), "port"), "PostgreSQL");
        assert_eq!(line(&read("443"), "port"), "HTTPS");
        // Nothing well known listens there.
        assert!(by_id(&read("54321"), "port").is_none());
    }

    #[test]
    fn a_mac_says_what_its_bits_say() {
        assert_eq!(line(&read("00:1b:44:11:3a:b7"), "mac"), "00:1b:44:11:3a:b7");
        assert_eq!(
            line(&read("02-1b-44-11-3a-b7"), "mac"),
            "given by hand, not by the maker"
        );
        assert!(by_id(&read("00:1b:44:11:3a"), "mac").is_none());
    }
    fn name(text: &str) -> String {
        fields(text, "punycode")
            .into_iter()
            .next()
            .expect("a name")
            .1
    }

    /// The expected sides of these came out of Python's own `punycode`
    /// codec rather than from memory: an implementation of RFC 3492 that
    /// shares no code with this one, which is the only thing that makes
    /// them a test of the decoder rather than a record of its output.
    #[test]
    fn a_punycode_name_comes_back_as_itself() {
        assert_eq!(name("xn--bcher-kva"), "bücher");
        assert_eq!(name("xn--mnchen-3ya.de"), "münchen.de");
        assert_eq!(name("xn--maana-pta.example.com"), "mañana.example.com");
        assert_eq!(name("xn--wgv71a119e"), "日本語");
        assert_eq!(name("xn--3B-ww4c5e180e575a65lsy2b"), "3年B組金八先生");
        assert_eq!(
            name("xn--Proprostnemluvesky-uyb24dma41a"),
            "Pročprostěnemluvíčesky"
        );
        // The prefix is a prefix whatever its case.
        assert_eq!(name("XN--bcher-kva.example"), "bücher.example");
    }

    /// The reason a reader opened it: `pаypal` is not `paypal`, and only the
    /// code point says so.
    #[test]
    fn the_characters_that_are_not_ascii_are_named() {
        let found = fields("xn--pypal-4ve.com", "punycode");
        assert_eq!(found[0].1, "pаypal.com");
        assert_eq!(found[1].1, "а U+0430");
        // A name that is all ASCII once decoded has nothing to add, so the
        // second line is not there at all.
        let found = fields("xn--mnchen-3ya", "punycode");
        assert_eq!(found.len(), 2, "münchen has a ü: {found:?}");
        assert_eq!(found[1].1, "ü U+00FC");
    }

    #[test]
    fn what_is_not_punycode_is_not_read() {
        for text in [
            // Nothing encoded in it.
            "example.com",
            "münchen.de",
            // The prefix and nothing after it.
            "xn--",
            "xn--.com",
            // Digits that are not digits of base 36.
            "xn--!!!",
            "xn--bcher-kva!",
            // Prose with a name in the middle, and a URL rather than a name.
            "the host is xn--bcher-kva today",
            "https://xn--bcher-kva.example/path",
            // An empty label is not a label.
            "xn--bcher-kva..com",
        ] {
            assert!(
                by_id(&read(text), "punycode").is_none(),
                "read {text} as a name"
            );
        }
    }
}
