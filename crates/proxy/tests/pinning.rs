//! What a socket of tracewing's own really leaves by while it holds an
//! adapter.
//!
//! **A process of its own, which is the whole reason this is a file here
//! rather than a `mod tests`.** The answer lives in process-wide statics:
//! [`upstream::leave_by`] writes them and every socket tracewing makes reads
//! them, so a unit test that set them would pin every other test in the same
//! binary that happened to be connecting at that moment. `upstream.rs` says
//! so itself, where it takes the decision apart from the statics: "shared
//! state read by several of them is how a suite starts failing one time in
//! ten." An integration test is one binary and one process, so what this
//! sets reaches nothing else.
//!
//! What is covered elsewhere, so that this does not read as more than it is:
//! `roads::watch`'s decisions, with fakes, in `roads.rs`; `out_from`, the
//! decision itself, in `upstream.rs`; and `leave_socket_by`, that the option
//! lands on a socket of either kind and either family, in `upstream.rs`'s
//! own `pinning`. This is the one join none of those reaches -- that the
//! statics and a real socket are wired together at all.
//!
//! What **nothing** covers is written down on `local_source::Pinned`: the
//! wiring that puts the real routing table on one end of this and the real
//! `leave_by` on the other.
//!
//! The other way out, [`upstream::datagrams_out`], is the one that *does*
//! fall back to an ordinary socket when nothing is held, because the way out
//! over QUIC runs whether or not an adapter is: every `quic_upstream` test
//! makes one that way with nothing pinned, so that arm is covered by them.

#![cfg(target_os = "linux")]

use std::os::fd::AsRawFd;
use tracewing_proxy::upstream::{self, OutBy};

/// Loopback, which every Linux machine numbers 1 and has.
///
/// Also the harmless choice if this ever stops being a process of its own:
/// everything a test reaches is on the loopback anyway.
const SOME_INTERFACE: u32 = 1;

/// `SO_BINDTOIFINDEX` as the socket holds it, which is the only thing that
/// says where it will really leave by. `0` is "nothing forced".
fn read_back(socket: &impl AsRawFd) -> u32 {
    let mut value: libc::c_int = -1;
    let mut len = std::mem::size_of::<libc::c_int>() as libc::socklen_t;
    // SAFETY: four bytes of ours, with four given as their length.
    let got = unsafe {
        libc::getsockopt(
            socket.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_BINDTOIFINDEX,
            std::ptr::addr_of_mut!(value).cast(),
            &mut len,
        )
    };
    assert_eq!(
        got,
        0,
        "read the option back: {}",
        std::io::Error::last_os_error()
    );
    value as u32
}

/// The three states a socket can be made in, in one test because they share
/// the statics and running them beside each other is the race this file
/// exists to avoid.
#[tokio::test]
async fn a_socket_leaves_by_what_is_pinned_and_fails_when_that_is_nowhere() {
    let bind = upstream::bound_datagrams();

    // **No adapter held: there is no socket at all**, and that was worth
    // finding out by asking rather than assuming -- I expected a fallback
    // to an ordinary socket and this is not that. The way out for a
    // passed-on datagram exists only while an adapter is held, so one made
    // with nothing pinned could only be a socket following the ordinary
    // routes, which now lead into our own adapter. Refusing is the right
    // answer and this says so.
    let none = bind(false).await;
    assert!(
        none.is_err(),
        "a way out was made with nothing pinned: {:?}",
        none.map(|s| s.local_addr())
    );

    // An adapter held, and a way out that is not it.
    upstream::leave_by(Some(OutBy {
        v4: Some(SOME_INTERFACE),
        v6: Some(SOME_INTERFACE),
    }));
    let pinned = bind(false)
        .await
        .expect("a socket while an adapter is held");
    assert_eq!(
        read_back(&pinned),
        SOME_INTERFACE,
        "out by the one interface that is not ours"
    );

    // An adapter held and **no** way out: no socket, and the reason says
    // which of the two refusals it is. Asserting only that it failed is not
    // enough and that was measured: turning the `Nowhere` arm into
    // `AsAlways` leaves a bare `is_err` green, because both refuse -- one
    // saying nothing is pinned, which is a bug's message, and one saying
    // this family has no way out that is not our own adapter, which is this
    // machine's state and the thing a reader needs.
    upstream::leave_by(Some(OutBy { v4: None, v6: None }));
    let nowhere = bind(false).await;
    let why = match nowhere {
        Err(e) => e.to_string(),
        Ok(socket) => panic!("a socket was made with nowhere to send it: {socket:?}"),
    };
    assert!(
        why.contains("IPv4") && why.contains("no way out"),
        "the refusal has to say what the machine's state is: {why}"
    );

    // And let go, which is the first state again: no adapter, no way out
    // for a datagram nobody answers here.
    upstream::leave_by(None);
    assert!(
        bind(false).await.is_err(),
        "a way out outlived the adapter it was for"
    );
}
