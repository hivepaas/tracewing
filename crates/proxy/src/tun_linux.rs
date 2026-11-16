//! A TUN device on Linux: `/dev/net/tun`, opened by name.
//!
//! The device this feeds is [`crate::tun`]'s pump, and the Windows sibling of
//! this file is [`crate::wintun`]. Only the four methods differ; everything
//! above them — the pump, the stack, the sieve that decides which
//! connections are kept — is the same code on both.
//!
//! ## Who may open one, measured rather than assumed
//!
//! On 2026-11-16, on this machine, with a control in the same transcript
//! because a step that succeeds is the one nobody re-reads:
//!
//! | What | Privilege | What happened |
//! |---|---|---|
//! | create a new TUN | none | `EPERM` |
//! | `ip tuntap add dev … mode tun user <uid>` | root | created, `tun persist user 1000` |
//! | `TUNSETIFF` on **that** device | **none** | **opened** |
//! | `TUNSETIFF` on a *different* name | none | `EPERM` again |
//! | `ip addr add`, `ip link set up` | none | not permitted, exit 2 |
//!
//! The fourth row is the one that makes the first three mean something: the
//! opening succeeds because the device was created persistent and owned, not
//! because the process had gained anything.
//!
//! So on Linux the **pump stays in tracewing**, unprivileged, and only the
//! setting up needs privilege — create the device owned by this user, give
//! it an address, bring it up, route into it. That is the shape
//! [`crate::super`]'s Windows side could not have: there, opening an adapter
//! needs an administrator too, so the pump itself has to be elevated.
//!
//! **And the carrier follows whoever holds it open.** With the address and
//! route in place, an unprivileged process holding this fd takes the link
//! `UP` and the route live; when it exits the link goes `DOWN` and the route
//! reads `linkdown`. Measured both ways. So the capture ends when tracewing
//! does, with nothing to undo — which is the piece that is dangerous to get
//! wrong on Windows.
//!
//! ## One holder at a time
//!
//! A TUN device is opened by exactly one process: a second `TUNSETIFF` on a
//! device somebody already holds answers `EBUSY`. Found by the live tests
//! below running in parallel on one device, which is a cheaper way to learn
//! it than a user with two tracewings open. Two things follow — the error
//! for that case says so rather than telling somebody to create a device
//! that already exists, and a tracewing that crashed leaves nothing to clean
//! up, since the kernel closes the descriptor and the device, being
//! persistent, waits for the next run.
//!
//! ## Waking a blocked read
//!
//! [`Device::read`] blocks until the kernel has a packet, and on a quiet
//! machine that is for ever; [`Device::shutdown`] has to end it or tracewing
//! will not quit. There is no "cancel" for a blocking `read(2)`, so this
//! waits on **two** descriptors — the device and an `eventfd` — and
//! `shutdown` writes to the second. No signals, no timeout to tune, and no
//! window in which a packet arriving and a stop racing can lose either.

use crate::tun::Device;
use anyhow::{Context, Result, anyhow};
use std::{
    io,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
};

/// `linux/if_tun.h`, which libc does not have.
const TUNSETIFF: libc::c_ulong = 0x4004_54ca;
/// A device carrying IP packets, not Ethernet frames.
const IFF_TUN: libc::c_short = 0x0001;
/// No four-byte header in front of each packet. The stack is handed IP
/// packets and nothing else, which is what [`crate::stack`] reads.
const IFF_NO_PI: libc::c_short = 0x1000;

/// Room for the largest packet a read can hand back. A device's MTU is
/// normally 1500, but it is the owner who sets that and a short buffer
/// **truncates** rather than failing, so this is the largest an IP packet can
/// be: a lost tail would reach the stack as a packet whose length field
/// disagrees with its bytes, which is a hard thing to recognise later.
const PACKET_MAX: usize = 65_535;

/// The name of a device, as `ip link` shows it, in the fixed-size field
/// `TUNSETIFF` takes.
const NAME_MAX: usize = libc::IFNAMSIZ - 1;

/// An open TUN device, and the way to wake a read that is waiting on it.
pub struct TunDevice {
    tun: OwnedFd,
    /// Written to by [`Device::shutdown`], waited on beside `tun`.
    wake: OwnedFd,
    name: String,
}

impl TunDevice {
    /// Opens the device called `name`, which has to exist already.
    ///
    /// Creating one needs `CAP_NET_ADMIN`; opening one that was created
    /// persistent and owned by this user does not. The error says so,
    /// because `EPERM` on its own reads as "tracewing needs root" when what
    /// it means is "that device is not there, or is not yours".
    pub fn open(name: &str) -> Result<Self> {
        if name.is_empty() || name.len() > NAME_MAX {
            return Err(anyhow!(
                "a device name is 1 to {NAME_MAX} characters; {name:?} is {}",
                name.len()
            ));
        }
        let tun = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/net/tun")
            .context("open /dev/net/tun")?;
        let tun = OwnedFd::from(tun);

        let mut request = Request::new(name, IFF_TUN | IFF_NO_PI);
        // SAFETY: the request is the layout `TUNSETIFF` reads, and the
        // descriptor is open for the length of the call.
        let answer = unsafe { libc::ioctl(tun.as_raw_fd(), TUNSETIFF, &mut request) };
        if answer < 0 {
            let why = io::Error::last_os_error();
            // Which failure it is decides what to tell somebody, and the two
            // want opposite advice: `EBUSY` means the device is there and
            // somebody has it, so making another would be the wrong move.
            return Err(match why.raw_os_error() {
                Some(libc::EBUSY) => anyhow!(
                    "the device {name} is open already: another tracewing, or \
                     another program, is holding it. Only one process may read \
                     a TUN device at a time."
                ),
                _ => anyhow!(
                    "open the device {name}: {why}. It has to exist already and \
                     belong to this user: `sudo ip tuntap add dev {name} mode tun \
                     user $(id -u)` makes one that survives tracewing restarting."
                ),
            });
        }

        // SAFETY: `eventfd` returns a new descriptor or -1, and this takes
        // ownership of it exactly once.
        let wake = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC) };
        if wake < 0 {
            return Err(anyhow!(
                "make the descriptor that stops the reader: {}",
                io::Error::last_os_error()
            ));
        }
        // SAFETY: just made, owned by nothing else.
        let wake = unsafe { OwnedFd::from_raw_fd(wake) };

        Ok(Self {
            tun,
            wake,
            name: name.to_owned(),
        })
    }
}

/// `struct ifreq` as far as `TUNSETIFF` reads it: the name, then the flags.
/// The union after them is not touched, and is zero.
#[repr(C)]
struct Request {
    name: [libc::c_char; libc::IFNAMSIZ],
    flags: libc::c_short,
    rest: [u8; 22],
}

impl Request {
    fn new(name: &str, flags: libc::c_short) -> Self {
        let mut field = [0 as libc::c_char; libc::IFNAMSIZ];
        for (slot, byte) in field.iter_mut().zip(name.as_bytes()) {
            *slot = *byte as libc::c_char;
        }
        Self {
            name: field,
            flags,
            rest: [0; 22],
        }
    }
}

impl Device for TunDevice {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn read(&self) -> Result<Vec<u8>> {
        loop {
            let mut waiting = [
                libc::pollfd {
                    fd: self.tun.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                },
                libc::pollfd {
                    fd: self.wake.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                },
            ];
            // SAFETY: two descriptors this owns, for as long as the call.
            let ready = unsafe { libc::poll(waiting.as_mut_ptr(), 2, -1) };
            if ready < 0 {
                let why = io::Error::last_os_error();
                if why.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(anyhow!("wait for a packet on {}: {why}", self.name));
            }
            // The stop first: asked to go, a packet waiting as well is not a
            // reason to stay.
            if waiting[1].revents != 0 {
                return Err(anyhow!("{} was asked to stop", self.name));
            }
            if waiting[0].revents & (libc::POLLERR | libc::POLLNVAL) != 0 {
                return Err(anyhow!("the device {} is gone", self.name));
            }
            if waiting[0].revents & libc::POLLIN == 0 {
                continue;
            }

            let mut packet = vec![0u8; PACKET_MAX];
            // SAFETY: the buffer is as long as it says and lives past the call.
            let read = unsafe {
                libc::read(
                    self.tun.as_raw_fd(),
                    packet.as_mut_ptr().cast(),
                    packet.len(),
                )
            };
            if read < 0 {
                let why = io::Error::last_os_error();
                if matches!(
                    why.kind(),
                    io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
                ) {
                    continue;
                }
                return Err(anyhow!("read from {}: {why}", self.name));
            }
            packet.truncate(read as usize);
            return Ok(packet);
        }
    }

    /// A packet the kernel will not take is this packet's problem, and
    /// [`crate::tun`] drops it: a full queue is ordinary under load, and the
    /// sender will send it again. Anything that is really the end of the
    /// device is said by [`Device::read`] as well.
    fn write(&self, packet: &[u8]) -> Result<()> {
        // SAFETY: the slice is borrowed for the length of the call.
        let written =
            unsafe { libc::write(self.tun.as_raw_fd(), packet.as_ptr().cast(), packet.len()) };
        if written < 0 {
            return Err(anyhow!(
                "write {} bytes to {}: {}",
                packet.len(),
                self.name,
                io::Error::last_os_error()
            ));
        }
        Ok(())
    }

    fn shutdown(&self) {
        // Unlike a failed read or write, this failing is worth saying: it is
        // what ends the reading thread, and a stop that did not take leaves a
        // tracewing that will not quit.
        let one = 1u64.to_ne_bytes();
        // SAFETY: eight bytes to an `eventfd`, which is what it takes.
        let written = unsafe { libc::write(self.wake.as_raw_fd(), one.as_ptr().cast(), one.len()) };
        if written < 0 {
            tracing::warn!(
                "telling the reader of {} to stop: {}",
                self.name,
                io::Error::last_os_error()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What opening said went wrong. `unwrap_err` would want the device to
    /// be `Debug`, and a device is a pair of descriptors: nothing a test
    /// should print.
    fn problem_opening(name: &str) -> String {
        match TunDevice::open(name) {
            Ok(_) => panic!("{name} opened, and this test is about it not opening"),
            Err(e) => e.to_string(),
        }
    }

    /// A name the kernel cannot hold is refused here rather than silently
    /// truncated into somebody else's device.
    #[test]
    fn a_name_too_long_for_the_field_is_refused() {
        let long = "a".repeat(NAME_MAX + 1);
        let problem = problem_opening(&long);
        assert!(problem.contains(&format!("{NAME_MAX}")), "{problem}");
        assert!(TunDevice::open("").is_err(), "and so is no name at all");
    }

    /// Opening a device that is not there fails with the reason **and** what
    /// to do about it. `EPERM` alone reads as "tracewing needs root", which
    /// sends somebody to run the whole program as root; what it means is
    /// that the device has to exist and be theirs.
    #[test]
    fn a_device_that_is_not_there_says_how_to_make_one() {
        let name = format!("tw-absent-{}", std::process::id() % 100_000);
        let problem = problem_opening(&name);
        assert!(problem.contains(&name), "names the device: {problem}");
        assert!(
            problem.contains("ip tuntap add"),
            "says how to make one: {problem}"
        );
    }

    /// The request is the layout the kernel reads, and the name lands in it
    /// as bytes rather than as a pointer. A wrong offset here opens a device
    /// with a garbled name, which is the failure that hides best: the open
    /// succeeds and nothing routes.
    #[test]
    fn the_request_holds_the_name_and_the_flags() {
        let request = Request::new("tracewing0", IFF_TUN | IFF_NO_PI);
        let bytes: Vec<u8> = request.name.iter().map(|c| *c as u8).collect();
        assert_eq!(&bytes[..10], b"tracewing0");
        assert_eq!(bytes[10], 0, "and is terminated");
        assert_eq!(request.flags, IFF_TUN | IFF_NO_PI);
        assert_eq!(std::mem::offset_of!(Request, flags), libc::IFNAMSIZ);
    }

    /// Everything a real device can say, in **one** test, because a device
    /// takes one holder: two of these in parallel answered `EBUSY`, which is
    /// how that rule was learned.
    ///
    /// The last part is the contract the whole stop rests on: a read that is
    /// waiting returns once `shutdown` is called, and says why rather than
    /// hanging. On a quiet device it would otherwise wait for ever.
    #[test]
    #[ignore = "needs a TUN device: sudo ip tuntap add dev tracewing-test mode tun user $(id -u)"]
    fn a_real_device_opens_once_and_its_read_can_be_stopped() {
        let name = std::env::var("TRACEWING_TUN").unwrap_or_else(|_| "tracewing-test".to_owned());
        {
            let held = TunDevice::open(&name).unwrap_or_else(|e| panic!("{e:#}"));
            assert_eq!(held.name(), name);
            assert_ne!(
                held.tun.as_raw_fd(),
                held.wake.as_raw_fd(),
                "a device opened without its wake-up is a reader nothing can stop"
            );
            let second = problem_opening(&name);
            assert!(
                second.contains("open already"),
                "a second open says who has it, not how to make one: {second}"
            );
        }
        // The first is closed now, so the device is free again — which is
        // also what a tracewing that crashed leaves behind.
        let device =
            std::sync::Arc::new(TunDevice::open(&name).unwrap_or_else(|e| panic!("{e:#}")));
        let reading = std::thread::spawn({
            let device = std::sync::Arc::clone(&device);
            move || device.read()
        });
        // Long enough that the thread is inside `poll` rather than before it.
        std::thread::sleep(std::time::Duration::from_millis(200));
        device.shutdown();
        let ended = std::thread::spawn(move || reading.join());
        let started = std::time::Instant::now();
        while !ended.is_finished() {
            assert!(
                started.elapsed() < std::time::Duration::from_secs(5),
                "the read did not return: a stop that hangs is worse than one that fails"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let problem = ended.join().unwrap().unwrap().unwrap_err().to_string();
        assert!(problem.contains("stop"), "says why it ended: {problem}");
    }
}
