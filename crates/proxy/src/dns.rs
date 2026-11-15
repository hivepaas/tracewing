//! Answering the names a phone asks for.
//!
//! A phone on the WireGuard tunnel sends everything through it, DNS included,
//! and DNS is UDP. The stack speaks TCP and nothing else, so those datagrams
//! were dropped — which does not look like a DNS fault from the phone, it
//! looks like the whole network is broken, because nothing can be reached by
//! name and that is how a phone reaches everything.
//!
//! So this answers them. Not by forwarding to a resolver — that would mean
//! choosing one on the user's behalf, writing its address into tracewing, and
//! sending every name the phone looks up to whoever that is. Instead the name
//! is resolved **the way this machine resolves names**, through the operating
//! system, and the answer is written here. Whatever the machine is configured
//! to use — a company's resolver, a Pi-hole, systemd-resolved, a VPN's — the
//! phone gets the same answers the desktop gets, and tracewing holds no
//! opinion about DNS at all.
//!
//! What is answered: `A` and `AAAA`, which is what a phone asks for to open a
//! connection. Anything else gets an empty answer rather than silence — a
//! query with no reply is retried until it times out, and a phone waiting on
//! a timeout looks exactly like a phone with no network.
//!
//! What is not here: caching (the phone has one, and the operating system has
//! one), DNSSEC (which the resolver behind the operating system either does
//! or does not), and TCP DNS (a phone falls back to it only for answers too
//! large for a datagram, which an `A` record never is).

use std::net::IpAddr;

/// The header of a DNS message, which is the same length in a query and an
/// answer: id, flags, and four counts.
const HEAD: usize = 12;

/// The longest name this will read. A DNS name is at most 255 bytes on the
/// wire, and a query holding more than that is not one.
const NAME_MAX: usize = 255;

/// How long the phone may keep an answer. Short, because the answer is only
/// as good as the moment it was asked for: a name that moves — a CDN, a
/// service that fails over — should not be remembered by the phone for
/// longer than the desktop would remember it.
const TTL: u32 = 30;

/// What the record types this answers are, on the wire.
const A: u16 = 1;
const AAAA: u16 = 28;

/// The internet class, which is the only one anything asks for now.
const IN: u16 = 1;

/// A question worth answering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ask {
    /// The query's own id, which the answer has to carry back.
    pub id: u16,
    /// The name asked about, in the usual dotted form and lowercased.
    pub name: String,
    /// `A` or `AAAA`.
    pub kind: u16,
    /// Where the question ends, so the answer can quote it back exactly as
    /// it came rather than writing it again.
    pub question_ends: usize,
    /// Whether the asker wants recursion. Copied into the answer because a
    /// resolver that drops the bit looks broken to some clients.
    pub recursive: bool,
}

impl Ask {
    /// Whether this is a kind of question with an address for an answer.
    pub fn wants_an_address(&self) -> bool {
        self.kind == A || self.kind == AAAA
    }

    /// The addresses out of `found` that answer this question: a question
    /// about `A` is not answered with an IPv6 address, however many the
    /// machine knows.
    pub fn fitting<'a>(&self, found: &'a [IpAddr]) -> Vec<&'a IpAddr> {
        found
            .iter()
            .filter(|ip| match self.kind {
                A => ip.is_ipv4(),
                AAAA => ip.is_ipv6(),
                _ => false,
            })
            .collect()
    }
}

/// The question in `query`, if it is one question this can answer at all.
///
/// Every length here comes from the other end, so every one is checked
/// against what really arrived. `None` means "not a query this will answer",
/// and the caller leaves it alone rather than guessing.
pub fn ask(query: &[u8]) -> Option<Ask> {
    if query.len() < HEAD {
        return None;
    }
    let id = u16::from_be_bytes([query[0], query[1]]);
    let flags = u16::from_be_bytes([query[2], query[3]]);
    // A query, not an answer somebody sent us; and a standard query, not an
    // update or a zone transfer.
    if flags & 0x8000 != 0 || (flags >> 11) & 0x0f != 0 {
        return None;
    }
    // Exactly one question. A query with none is nothing to answer, and the
    // handful of clients that ask several in one message are not phones.
    if u16::from_be_bytes([query[4], query[5]]) != 1 {
        return None;
    }

    let mut at = HEAD;
    let mut name = String::new();
    loop {
        let len = usize::from(*query.get(at)?);
        // A pointer in a question is not a thing: there is nothing before it
        // to point at.
        if len & 0xc0 != 0 {
            return None;
        }
        at += 1;
        if len == 0 {
            break;
        }
        let label = query.get(at..at + len)?;
        if !name.is_empty() {
            name.push('.');
        }
        name.push_str(&String::from_utf8(label.to_vec()).ok()?);
        at += len;
        if name.len() > NAME_MAX {
            return None;
        }
    }
    let kind = u16::from_be_bytes([*query.get(at)?, *query.get(at + 1)?]);
    let class = u16::from_be_bytes([*query.get(at + 2)?, *query.get(at + 3)?]);
    if class != IN {
        return None;
    }
    Some(Ask {
        id,
        name: name.to_ascii_lowercase(),
        kind,
        question_ends: at + 4,
        recursive: flags & 0x0100 != 0,
    })
}

/// The answer to `ask`, naming `found`. An empty `found` is a real answer
/// too: the name exists as far as this knows and has no address of that
/// kind, which is what a phone asking `AAAA` on an IPv4 network hears.
pub fn answer(query: &[u8], ask: &Ask, found: &[IpAddr]) -> Vec<u8> {
    let fitting = ask.fitting(found);
    let mut out = Vec::with_capacity(HEAD + ask.question_ends + fitting.len() * 32);
    out.extend_from_slice(&ask.id.to_be_bytes());
    // An answer, authoritative for nothing, recursion available, and the
    // asker's own recursion-desired bit handed back.
    let flags = 0x8000u16 | if ask.recursive { 0x0180 } else { 0x0080 };
    out.extend_from_slice(&flags.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes()); // one question, quoted below
    out.extend_from_slice(&(fitting.len() as u16).to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes()); // no authority records
    out.extend_from_slice(&0u16.to_be_bytes()); // no additional records
    // The question exactly as it came. Quoted rather than written again so
    // that a client comparing the two byte for byte is satisfied.
    out.extend_from_slice(&query[HEAD..ask.question_ends]);

    for ip in fitting {
        // The name, as a pointer back to the question's copy of it. Every
        // resolver writes it this way and every client reads it.
        out.extend_from_slice(&[0xc0, HEAD as u8]);
        out.extend_from_slice(&ask.kind.to_be_bytes());
        out.extend_from_slice(&IN.to_be_bytes());
        out.extend_from_slice(&TTL.to_be_bytes());
        match ip {
            IpAddr::V4(ip) => {
                out.extend_from_slice(&4u16.to_be_bytes());
                out.extend_from_slice(&ip.octets());
            }
            IpAddr::V6(ip) => {
                out.extend_from_slice(&16u16.to_be_bytes());
                out.extend_from_slice(&ip.octets());
            }
        }
    }
    out
}

/// The answer to a question this does not answer: the same question back,
/// with nothing in it and no error. Silence would be worse — a phone retries
/// an unanswered query until it times out, and a timeout looks like a broken
/// network rather than an unsupported record type.
pub fn nothing_to_say(query: &[u8], ask: &Ask) -> Vec<u8> {
    answer(query, ask, &[])
}

/// The addresses this machine would reach `name` at, asked of the operating
/// system so that however it is configured to resolve names is how the phone
/// resolves them too.
pub async fn look_up(name: &str) -> Vec<IpAddr> {
    // The port is nothing to do with the answer; `lookup_host` wants one.
    match tokio::net::lookup_host((name, 0)).await {
        Ok(found) => {
            let mut addrs: Vec<IpAddr> = found.map(|at| at.ip()).collect();
            addrs.dedup();
            addrs
        }
        // A name that does not resolve is not an error worth a line: a phone
        // asks for plenty that do not exist, and the empty answer says so.
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A query as a phone writes one.
    fn query(id: u16, name: &str, kind: u16) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&id.to_be_bytes());
        out.extend_from_slice(&0x0100u16.to_be_bytes()); // a query, recursion desired
        out.extend_from_slice(&1u16.to_be_bytes()); // one question
        out.extend_from_slice(&[0, 0, 0, 0, 0, 0]); // no answers, authority, additional
        for label in name.split('.') {
            out.push(label.len() as u8);
            out.extend_from_slice(label.as_bytes());
        }
        out.push(0);
        out.extend_from_slice(&kind.to_be_bytes());
        out.extend_from_slice(&IN.to_be_bytes());
        out
    }

    #[test]
    fn a_query_says_what_it_asks_about() {
        let asked = ask(&query(0x1234, "API.GitHub.com", A)).expect("a question");
        assert_eq!(asked.id, 0x1234);
        assert_eq!(asked.name, "api.github.com", "lowercased, as names are");
        assert_eq!(asked.kind, A);
        assert!(asked.recursive);
        assert!(asked.wants_an_address());
        assert_eq!(
            asked.question_ends,
            query(0x1234, "api.github.com", A).len(),
            "the question is the whole message"
        );
    }

    #[test]
    fn what_is_not_a_question_this_answers_is_left_alone() {
        // An answer somebody sent us.
        let mut not_a_query = query(1, "github.com", A);
        not_a_query[2] |= 0x80;
        assert_eq!(ask(&not_a_query), None);

        // An update rather than a query.
        let mut update = query(1, "github.com", A);
        update[2] = (5 << 3) & 0x78;
        assert_eq!(ask(&update), None);

        // Something other than the internet class.
        let mut chaos = query(1, "github.com", A);
        let end = chaos.len();
        chaos[end - 2..].copy_from_slice(&3u16.to_be_bytes());
        assert_eq!(ask(&chaos), None);

        // A pointer where a label belongs, which has nothing to point at.
        let mut pointer = query(1, "github.com", A);
        pointer[HEAD] = 0xc0;
        assert_eq!(ask(&pointer), None);

        // Cut short, in each of the places it can be cut.
        let whole = query(1, "github.com", A);
        for upto in 0..whole.len() {
            assert_eq!(ask(&whole[..upto]), None, "cut at {upto}");
        }
        assert_eq!(ask(&[]), None);
    }

    #[test]
    fn an_answer_carries_the_addresses_and_quotes_the_question() {
        let q = query(0xbeef, "github.com", A);
        let asked = ask(&q).unwrap();
        let out = answer(
            &q,
            &asked,
            &["140.82.121.4".parse().unwrap(), "::1".parse().unwrap()],
        );

        assert_eq!(&out[..2], &0xbeefu16.to_be_bytes(), "the query's own id");
        assert_eq!(out[2] & 0x80, 0x80, "an answer");
        assert_eq!(out[3] & 0x80, 0x80, "recursion available");
        assert_eq!(out[3] & 0x0f, 0, "no error");
        assert_eq!(
            u16::from_be_bytes([out[6], out[7]]),
            1,
            "one answer: the IPv6 address does not answer an A question"
        );
        assert_eq!(
            &out[HEAD..asked.question_ends],
            &q[HEAD..asked.question_ends],
            "the question, quoted exactly"
        );

        // The record: a pointer to the name, then A, IN, the TTL, and four
        // bytes of address.
        let rr = &out[asked.question_ends..];
        assert_eq!(&rr[..2], &[0xc0, 12], "the name as a pointer");
        assert_eq!(u16::from_be_bytes([rr[2], rr[3]]), A);
        assert_eq!(u16::from_be_bytes([rr[4], rr[5]]), IN);
        assert_eq!(u32::from_be_bytes([rr[6], rr[7], rr[8], rr[9]]), TTL);
        assert_eq!(u16::from_be_bytes([rr[10], rr[11]]), 4);
        assert_eq!(&rr[12..16], &[140, 82, 121, 4]);
        assert_eq!(rr.len(), 16, "and nothing after it");
    }

    #[test]
    fn an_aaaa_question_is_answered_with_the_ipv6_address_alone() {
        let q = query(1, "github.com", AAAA);
        let asked = ask(&q).unwrap();
        let out = answer(
            &q,
            &asked,
            &[
                "140.82.121.4".parse().unwrap(),
                "2606:50c0:8000::153".parse().unwrap(),
            ],
        );
        assert_eq!(u16::from_be_bytes([out[6], out[7]]), 1);
        let rr = &out[asked.question_ends..];
        assert_eq!(u16::from_be_bytes([rr[2], rr[3]]), AAAA);
        assert_eq!(u16::from_be_bytes([rr[10], rr[11]]), 16, "sixteen bytes");
    }

    /// The empty answer, which is a real answer and not a failure: it is
    /// what stops a phone retrying until it gives up.
    #[test]
    fn a_question_with_no_address_for_it_is_still_answered() {
        let q = query(7, "nothing.invalid", 33); // SRV, which this does not answer
        let asked = ask(&q).unwrap();
        assert!(!asked.wants_an_address());
        let out = nothing_to_say(&q, &asked);
        assert_eq!(&out[..2], &7u16.to_be_bytes());
        assert_eq!(out[2] & 0x80, 0x80, "an answer, not silence");
        assert_eq!(out[3] & 0x0f, 0, "and not an error either");
        assert_eq!(
            u16::from_be_bytes([out[6], out[7]]),
            0,
            "with nothing in it"
        );

        // The same for a name that does not resolve.
        let q = query(8, "no-such-name.invalid", A);
        let asked = ask(&q).unwrap();
        let out = answer(&q, &asked, &[]);
        assert_eq!(u16::from_be_bytes([out[6], out[7]]), 0);
    }

    /// Asked of the operating system, so this is a test of the wiring and
    /// not of any particular resolver: `localhost` is the one name every
    /// machine answers without a network.
    #[tokio::test]
    async fn a_name_this_machine_knows_is_looked_up_through_it() {
        let found = look_up("localhost").await;
        assert!(
            found.iter().any(|ip| ip.is_loopback()),
            "localhost resolves to a loopback address: {found:?}"
        );
        assert!(
            look_up("tracewing-no-such-host.invalid").await.is_empty(),
            "a name that does not resolve answers with nothing, not an error"
        );
    }
}
