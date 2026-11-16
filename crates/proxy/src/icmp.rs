//! `ping` and `tracert`, which a route into an adapter otherwise silences.
//!
//! The stack turns TCP into flows, answers names, and passes other datagrams
//! on. ICMP met none of those and so met nothing: it went to [`smoltcp`],
//! which has no socket for it and drops it. **Measured, not assumed** — an
//! echo request to `8.8.8.8` through the stack produced no packet at all,
//! which was worth checking because the interface has `set_any_ip(true)` and
//! an interface that answers for every destination could as easily have
//! answered the ping itself, as the host being pinged. That would have been
//! far worse than silence: `ping` would report every address on the internet
//! as reachable in under a millisecond.
//!
//! Silence is bad enough. While the mode is on, `ping` and `tracert` are the
//! two things somebody reaches for *when the network looks wrong* — so the
//! mode breaks the tools whose job is to tell them what broke, and it breaks
//! them in the way that looks most like a network fault.
//!
//! So an echo request is **really sent**, and what comes back is turned into
//! a packet the client will accept:
//!
//! - the host answered → an **echo reply** from the address that answered.
//! - a router dropped it because the hop limit ran out → a **time exceeded**
//!   from that router, quoting the request. That quote is the whole of how
//!   `tracert` works: it sends echo requests with a hop limit of 1, 2, 3 …
//!   and reads the address of whoever complains.
//! - nothing answered → nothing, and the client times out, which is what it
//!   would have done on a network where nothing answered.
//!
//! How an echo is really sent is **injected**, for the same reason as
//! [`crate::datagrams`]'s socket: on Windows it has to be IP Helper's
//! `IcmpSendEcho2Ex`, which is what `ping` itself uses and which needs no
//! administrator, with the source address of the interface the machine's own
//! traffic leaves by — or the echo would be routed into tracewing's own
//! adapter and arrive back here. A test gives a table of answers, so
//! everything below it is exercised on Linux.

use crate::source::{Stop, stopped};
use smoltcp::wire::{
    Icmpv4Packet, Icmpv4Repr, Icmpv6Packet, Icmpv6Repr, IpProtocol, Ipv4Packet, Ipv4Repr,
    Ipv6Packet, Ipv6Repr,
};
use std::{
    future::Future,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    pin::Pin,
    sync::Arc,
};
use tokio::sync::mpsc;

/// What carries ICMP, in the `protocol` of an IPv4 header.
const ICMP4: u8 = 1;

/// And in the `next_header` of an IPv6 one. A different number and a
/// different protocol: the checksum covers the addresses, and the message
/// types are its own.
const ICMP6: u8 = 58;

/// The least of an ICMP message: type, code, checksum, and four bytes that
/// mean something different per type. For an echo it is the identifier and
/// the sequence number.
const ICMP_HEAD: usize = 8;

/// How many echoes are waiting to be sent before the rest are dropped. A
/// `ping` sends one at a time and a `tracert` a handful, so a queue this deep
/// is already a client asking for more than it can use.
const QUEUED_MAX: usize = 64;

/// An echo request as it arrived.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ping {
    /// The client, which the answer has to go back to.
    pub from: IpAddr,
    /// Where it was going, which is what to really echo.
    pub to: IpAddr,
    pub ident: u16,
    pub seq: u16,
    /// The hop limit it was sent with. `tracert` varies this and reads who
    /// complains, so passing it on is not an optimisation -- it is the whole
    /// feature.
    pub ttl: u8,
    /// The ICMP message whole, as it arrived. Kept because a *time exceeded*
    /// has to quote the request that caused it, and a client matches the
    /// quote rather than trusting the message.
    pub original: Vec<u8>,
}

impl Ping {
    /// The echo's payload, which an answer repeats back.
    fn payload(&self) -> &[u8] {
        &self.original[ICMP_HEAD..]
    }
}

/// What came of really sending it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    /// The host answered, from this address.
    Answered { from: IpAddr, data: Vec<u8> },
    /// A router would not carry it any further, and said so. This is what
    /// `tracert` is reading.
    Expired { from: IpAddr },
    /// Nothing answered in time. The client times out, as it would have on a
    /// network where nothing answered.
    Silent,
}

/// How an echo is really sent. See the module's last paragraph for why this
/// is injected rather than written here.
pub type Echo = Arc<dyn Fn(Ping) -> Answering + Send + Sync>;

pub type Answering = Pin<Box<dyn Future<Output = Answer> + Send>>;

/// The echo request in an IP packet, where it is one.
///
/// `None` for anything that is not an ICMP echo request, and for the two
/// packets [`crate::stack::peek`] leaves alone for the same reason it does:
/// an IPv4 fragment that is not the first and an IPv6 packet with an
/// extension header both put the ICMP header somewhere this does not look.
pub fn echo_request(packet: &[u8]) -> Option<Ping> {
    match *packet.first()? >> 4 {
        4 => {
            let header = usize::from(packet[0] & 0x0f) * 4;
            if header < 20 || packet.len() < header + ICMP_HEAD || packet[9] != ICMP4 {
                return None;
            }
            let fragment = u16::from_be_bytes([packet[6], packet[7]]);
            if fragment & 0x1fff != 0 || fragment & 0x2000 != 0 {
                return None;
            }
            let icmp = Icmpv4Packet::new_checked(&packet[header..]).ok()?;
            let Icmpv4Repr::EchoRequest { ident, seq_no, .. } =
                Icmpv4Repr::parse(&icmp, &Default::default()).ok()?
            else {
                return None;
            };
            Some(Ping {
                from: Ipv4Addr::new(packet[12], packet[13], packet[14], packet[15]).into(),
                to: Ipv4Addr::new(packet[16], packet[17], packet[18], packet[19]).into(),
                ident,
                seq: seq_no,
                ttl: packet[8],
                original: packet[header..].to_vec(),
            })
        }
        6 => {
            const HEAD: usize = 40;
            if packet.len() < HEAD + ICMP_HEAD || packet[6] != ICMP6 {
                return None;
            }
            let from = Ipv6Addr::from(<[u8; 16]>::try_from(&packet[8..24]).ok()?);
            let to = Ipv6Addr::from(<[u8; 16]>::try_from(&packet[24..40]).ok()?);
            let icmp = Icmpv6Packet::new_checked(&packet[HEAD..]).ok()?;
            // The checksum covers the addresses, so parsing needs them.
            let Icmpv6Repr::EchoRequest { ident, seq_no, .. } =
                Icmpv6Repr::parse(&from, &to, &icmp, &Default::default()).ok()?
            else {
                return None;
            };
            Some(Ping {
                from: from.into(),
                to: to.into(),
                ident,
                seq: seq_no,
                // IPv6 calls it the hop limit and puts it in its own place.
                ttl: packet[7],
                original: packet[HEAD..].to_vec(),
            })
        }
        _ => None,
    }
}

/// The packet to send back to the client, for what came of the echo.
///
/// `None` for [`Answer::Silent`], and for an answer from the wrong family --
/// a reply has to look as though it came over the same kind of network the
/// request went out on, or the client will not have it.
pub fn answer_for(ping: &Ping, answer: &Answer) -> Option<Vec<u8>> {
    // The family is decided first and only one message is built. Building
    // both and picking afterwards is what the first version did, and the two
    // `?`s inside the unused one turned every IPv4 answer into `None`.
    let source = match answer {
        Answer::Silent => return None,
        Answer::Answered { from, .. } | Answer::Expired { from } => *from,
    };
    // At least eight bytes of the request, which is what a client matches a
    // complaint against: the type, the identifier and the sequence number.
    //
    // The quoted header's length field is set to what is actually quoted
    // rather than to the request's own length. RFC 792 has a router quote its
    // header **verbatim**, so a real one says 36 bytes while 8 follow -- but
    // nothing reads that field to match a probe, and a header that claims
    // more than it carries is one a strict parser refuses outright
    // (`smoltcp`'s does, which is how this was found). Self-consistent and
    // readable beats verbatim and rejected.
    let quote = &ping.original[..ICMP_HEAD.min(ping.original.len())];
    match (source, ping.from, ping.to) {
        (IpAddr::V4(src), IpAddr::V4(dst), IpAddr::V4(was_going_to)) => {
            let icmp = match answer {
                Answer::Answered { data, .. } => Icmpv4Repr::EchoReply {
                    ident: ping.ident,
                    seq_no: ping.seq,
                    // What the host sent back, where it sent anything; the
                    // request's own payload otherwise, which is what a
                    // client compares against.
                    data: if data.is_empty() {
                        ping.payload()
                    } else {
                        data
                    },
                },
                Answer::Expired { .. } => Icmpv4Repr::TimeExceeded {
                    reason: smoltcp::wire::Icmpv4TimeExceeded::TtlExpired,
                    header: Ipv4Repr {
                        src_addr: dst,
                        dst_addr: was_going_to,
                        next_header: IpProtocol::Icmp,
                        payload_len: quote.len(),
                        hop_limit: ping.ttl,
                    },
                    data: quote,
                },
                Answer::Silent => return None,
            };
            let ip = Ipv4Repr {
                src_addr: src,
                dst_addr: dst,
                next_header: IpProtocol::Icmp,
                payload_len: icmp.buffer_len(),
                hop_limit: 64,
            };
            let head = ip.buffer_len();
            let mut packet = vec![0u8; head + icmp.buffer_len()];
            ip.emit(
                &mut Ipv4Packet::new_unchecked(&mut packet),
                &Default::default(),
            );
            icmp.emit(
                &mut Icmpv4Packet::new_unchecked(&mut packet[head..]),
                &Default::default(),
            );
            Some(packet)
        }
        (IpAddr::V6(src), IpAddr::V6(dst), IpAddr::V6(was_going_to)) => {
            let icmp = match answer {
                Answer::Answered { data, .. } => Icmpv6Repr::EchoReply {
                    ident: ping.ident,
                    seq_no: ping.seq,
                    data: if data.is_empty() {
                        ping.payload()
                    } else {
                        data
                    },
                },
                Answer::Expired { .. } => Icmpv6Repr::TimeExceeded {
                    reason: smoltcp::wire::Icmpv6TimeExceeded::HopLimitExceeded,
                    header: Ipv6Repr {
                        src_addr: dst,
                        dst_addr: was_going_to,
                        next_header: IpProtocol::Icmpv6,
                        payload_len: quote.len(),
                        hop_limit: ping.ttl,
                    },
                    data: quote,
                },
                Answer::Silent => return None,
            };
            let ip = Ipv6Repr {
                src_addr: src,
                dst_addr: dst,
                next_header: IpProtocol::Icmpv6,
                payload_len: icmp.buffer_len(),
                hop_limit: 64,
            };
            let head = ip.buffer_len();
            let mut packet = vec![0u8; head + icmp.buffer_len()];
            ip.emit(&mut Ipv6Packet::new_unchecked(&mut packet));
            icmp.emit(
                &src,
                &dst,
                &mut Icmpv6Packet::new_unchecked(&mut packet[head..]),
                &Default::default(),
            );
            Some(packet)
        }
        // An answer from the other family is no answer to this request.
        _ => None,
    }
}

/// Starts answering echo requests, and returns where to put them.
///
/// Each one is sent in a task of its own: a `tracert` has several probes out
/// at once and answering them one after another would turn a thirty-hop trace
/// into thirty timeouts in a row.
pub fn answering(out: mpsc::Sender<Vec<u8>>, echo: Echo, stop: Stop) -> mpsc::Sender<Ping> {
    let (asked, mut asking) = mpsc::channel::<Ping>(QUEUED_MAX);
    tokio::spawn(async move {
        loop {
            tokio::select! {
                ping = asking.recv() => match ping {
                    Some(ping) => {
                        let echo = Arc::clone(&echo);
                        let out = out.clone();
                        tokio::spawn(async move {
                            let answer = echo(ping.clone()).await;
                            if let Some(packet) = answer_for(&ping, &answer)
                                && out.send(packet).await.is_err() {
                                    tracing::debug!("nobody left to take a ping's answer");
                                }
                        });
                    }
                    // The stack has let go: nothing more will arrive.
                    None => break,
                },
                _ = stopped(stop.clone()) => break,
            }
        }
    });
    asked
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(from: &str, to: &str, ttl: u8, seq: u16) -> Vec<u8> {
        let (from, to) = (from.parse().unwrap(), to.parse().unwrap());
        let icmp4 = Icmpv4Repr::EchoRequest {
            ident: 0x4321,
            seq_no: seq,
            data: b"abcdefgh",
        };
        let icmp6 = Icmpv6Repr::EchoRequest {
            ident: 0x4321,
            seq_no: seq,
            data: b"abcdefgh",
        };
        match (from, to) {
            (IpAddr::V4(src), IpAddr::V4(dst)) => {
                let ip = Ipv4Repr {
                    src_addr: src,
                    dst_addr: dst,
                    next_header: IpProtocol::Icmp,
                    payload_len: icmp4.buffer_len(),
                    hop_limit: ttl,
                };
                let mut bytes = vec![0u8; ip.buffer_len() + icmp4.buffer_len()];
                let head = ip.buffer_len();
                ip.emit(
                    &mut Ipv4Packet::new_unchecked(&mut bytes),
                    &Default::default(),
                );
                icmp4.emit(
                    &mut Icmpv4Packet::new_unchecked(&mut bytes[head..]),
                    &Default::default(),
                );
                bytes
            }
            (IpAddr::V6(src), IpAddr::V6(dst)) => {
                let ip = Ipv6Repr {
                    src_addr: src,
                    dst_addr: dst,
                    next_header: IpProtocol::Icmpv6,
                    payload_len: icmp6.buffer_len(),
                    hop_limit: ttl,
                };
                let mut bytes = vec![0u8; ip.buffer_len() + icmp6.buffer_len()];
                let head = ip.buffer_len();
                ip.emit(&mut Ipv6Packet::new_unchecked(&mut bytes));
                icmp6.emit(
                    &src,
                    &dst,
                    &mut Icmpv6Packet::new_unchecked(&mut bytes[head..]),
                    &Default::default(),
                );
                bytes
            }
            _ => panic!("one family per request"),
        }
    }

    #[test]
    fn an_echo_request_is_read_in_both_families() {
        let four = echo_request(&request("10.0.0.7", "8.8.8.8", 7, 3)).expect("a ping");
        assert_eq!(four.from, "10.0.0.7".parse::<IpAddr>().unwrap());
        assert_eq!(four.to, "8.8.8.8".parse::<IpAddr>().unwrap());
        assert_eq!((four.ident, four.seq, four.ttl), (0x4321, 3, 7));
        assert_eq!(four.payload(), b"abcdefgh");

        let six = echo_request(&request("2001:db8::7", "2001:db8::9", 5, 4)).expect("a ping");
        assert_eq!(six.to, "2001:db8::9".parse::<IpAddr>().unwrap());
        assert_eq!(
            (six.ident, six.seq, six.ttl),
            (0x4321, 4, 5),
            "IPv6 keeps the hop limit in its own place, not where IPv4 does"
        );
        assert_eq!(six.payload(), b"abcdefgh");
    }

    #[test]
    fn what_is_not_an_echo_request() {
        // An echo *reply*, which is somebody else's business.
        let mut reply = request("10.0.0.7", "8.8.8.8", 7, 1);
        reply[20] = 0; // type 0 is a reply
        assert_eq!(echo_request(&reply), None, "a reply is not a request");
        // TCP.
        let mut tcp = request("10.0.0.7", "8.8.8.8", 7, 1);
        tcp[9] = 6;
        assert_eq!(echo_request(&tcp), None);
        // A fragment that is not the first.
        let mut later = request("10.0.0.7", "8.8.8.8", 7, 1);
        later[6] = 0x00;
        later[7] = 0x01;
        assert_eq!(echo_request(&later), None);
        // An IPv6 packet with an extension header, which moves the ICMP.
        let mut six = request("2001:db8::7", "2001:db8::9", 7, 1);
        six[6] = 0;
        assert_eq!(echo_request(&six), None);
        // Cut short, and nothing at all.
        let whole = request("10.0.0.7", "8.8.8.8", 7, 1);
        assert_eq!(echo_request(&whole[..20]), None);
        assert_eq!(echo_request(&[]), None);
    }

    /// The answer a client gets for a host that answered: a reply from that
    /// host, carrying the identifier and sequence number it sent.
    #[test]
    fn a_host_that_answered_becomes_an_echo_reply() {
        let ping = echo_request(&request("10.0.0.7", "8.8.8.8", 64, 9)).unwrap();
        let packet = answer_for(
            &ping,
            &Answer::Answered {
                from: "8.8.8.8".parse().unwrap(),
                data: b"abcdefgh".to_vec(),
            },
        )
        .expect("a packet");
        let ip = Ipv4Packet::new_checked(&packet[..]).expect("an IPv4 packet");
        assert!(ip.verify_checksum(), "the IPv4 header's checksum");
        assert_eq!(ip.src_addr().octets(), [8, 8, 8, 8], "from the host pinged");
        assert_eq!(ip.dst_addr().octets(), [10, 0, 0, 7], "back to the client");
        let icmp = Icmpv4Packet::new_checked(ip.payload()).expect("an ICMP message");
        assert!(icmp.verify_checksum(), "and ICMP's own");
        match Icmpv4Repr::parse(&icmp, &Default::default()).unwrap() {
            Icmpv4Repr::EchoReply {
                ident,
                seq_no,
                data,
            } => {
                assert_eq!((ident, seq_no), (0x4321, 9), "the client matches on these");
                assert_eq!(data, b"abcdefgh");
            }
            other => panic!("an echo reply, not {other:?}"),
        }
    }

    /// And the one `tracert` lives on: a router that would not carry it,
    /// quoting the request so the client can tell which probe it answers.
    #[test]
    fn a_hop_limit_that_ran_out_becomes_a_time_exceeded_quoting_the_request() {
        let ping = echo_request(&request("10.0.0.7", "8.8.8.8", 3, 11)).unwrap();
        let packet = answer_for(
            &ping,
            &Answer::Expired {
                from: "203.0.113.1".parse().unwrap(),
            },
        )
        .expect("a packet");
        let ip = Ipv4Packet::new_checked(&packet[..]).expect("an IPv4 packet");
        assert_eq!(
            ip.src_addr().octets(),
            [203, 0, 113, 1],
            "from the router that complained, which is the address tracert prints"
        );
        assert_eq!(ip.dst_addr().octets(), [10, 0, 0, 7]);
        let icmp = Icmpv4Packet::new_checked(ip.payload()).expect("an ICMP message");
        assert!(icmp.verify_checksum());
        match Icmpv4Repr::parse(&icmp, &Default::default()).unwrap() {
            Icmpv4Repr::TimeExceeded { header, data, .. } => {
                assert_eq!(
                    header.src_addr.octets(),
                    [10, 0, 0, 7],
                    "the quote is the request"
                );
                assert_eq!(header.dst_addr.octets(), [8, 8, 8, 8]);
                assert_eq!(header.hop_limit, 3, "the hop limit that ran out");
                // The quote carries the identifier and sequence number, which
                // is how the client tells one probe from another.
                assert_eq!(u16::from_be_bytes([data[4], data[5]]), 0x4321);
                assert_eq!(u16::from_be_bytes([data[6], data[7]]), 11);
            }
            other => panic!("a time exceeded, not {other:?}"),
        }
    }

    #[test]
    fn the_same_two_answers_in_ipv6() {
        let ping = echo_request(&request("2001:db8::7", "2001:db8::9", 4, 2)).unwrap();
        for (answer, expect_reply) in [
            (
                Answer::Answered {
                    from: "2001:db8::9".parse().unwrap(),
                    data: b"abcdefgh".to_vec(),
                },
                true,
            ),
            (
                Answer::Expired {
                    from: "2001:db8::1".parse().unwrap(),
                },
                false,
            ),
        ] {
            let packet = answer_for(&ping, &answer).expect("a packet");
            let ip = Ipv6Packet::new_checked(&packet[..]).expect("an IPv6 packet");
            let icmp = Icmpv6Packet::new_checked(ip.payload()).expect("an ICMP message");
            assert!(
                icmp.verify_checksum(&ip.src_addr(), &ip.dst_addr()),
                "ICMPv6's checksum covers the addresses, so it is not optional"
            );
            let repr =
                Icmpv6Repr::parse(&ip.src_addr(), &ip.dst_addr(), &icmp, &Default::default())
                    .unwrap();
            match (repr, expect_reply) {
                (Icmpv6Repr::EchoReply { seq_no, .. }, true) => assert_eq!(seq_no, 2),
                (Icmpv6Repr::TimeExceeded { header, .. }, false) => {
                    assert_eq!(header.hop_limit, 4)
                }
                (other, _) => panic!("the wrong message: {other:?}"),
            }
        }
    }

    #[test]
    fn nothing_answering_is_nothing_sent() {
        let ping = echo_request(&request("10.0.0.7", "8.8.8.8", 64, 1)).unwrap();
        assert_eq!(
            answer_for(&ping, &Answer::Silent),
            None,
            "the client times out, as it would where nothing answered"
        );
    }

    #[test]
    fn an_answer_from_the_other_family_is_no_answer() {
        let ping = echo_request(&request("10.0.0.7", "8.8.8.8", 64, 1)).unwrap();
        assert_eq!(
            answer_for(
                &ping,
                &Answer::Answered {
                    from: "2001:db8::9".parse().unwrap(),
                    data: Vec::new(),
                }
            ),
            None
        );
    }

    /// The whole of it: a request goes in, the echoer is asked, and the
    /// answer comes out as a packet the client would accept.
    #[tokio::test]
    async fn a_request_is_echoed_and_its_answer_comes_back() {
        let (out, mut coming) = mpsc::channel(8);
        let (_stop_tx, stop) = tokio::sync::watch::channel(false);
        let (seen, mut seeing) = mpsc::unbounded_channel();
        let echo: Echo = Arc::new(move |ping: Ping| {
            let seen = seen.clone();
            Box::pin(async move {
                let _ = seen.send(ping.clone());
                // A router at hop three, as a tracert probe would meet.
                match ping.ttl {
                    3 => Answer::Expired {
                        from: "203.0.113.1".parse().unwrap(),
                    },
                    _ => Answer::Answered {
                        from: ping.to,
                        data: b"abcdefgh".to_vec(),
                    },
                }
            })
        });
        let asked = answering(out, echo, stop);

        for ttl in [64u8, 3] {
            asked
                .send(echo_request(&request("10.0.0.7", "8.8.8.8", ttl, u16::from(ttl))).unwrap())
                .await
                .unwrap();
            let ping = tokio::time::timeout(std::time::Duration::from_secs(5), seeing.recv())
                .await
                .expect("the echoer is asked")
                .expect("one ask");
            assert_eq!(
                ping.ttl, ttl,
                "the hop limit is passed on, which is tracert"
            );
            let packet = tokio::time::timeout(std::time::Duration::from_secs(5), coming.recv())
                .await
                .expect("an answer comes back")
                .expect("a packet");
            let ip = Ipv4Packet::new_checked(&packet[..]).unwrap();
            let icmp = Icmpv4Packet::new_checked(ip.payload()).unwrap();
            let repr = Icmpv4Repr::parse(&icmp, &Default::default()).unwrap();
            match (ttl, repr) {
                (64, Icmpv4Repr::EchoReply { .. }) => {}
                (3, Icmpv4Repr::TimeExceeded { .. }) => {}
                (_, other) => panic!("the wrong answer for ttl {ttl}: {other:?}"),
            }
        }
    }
}
