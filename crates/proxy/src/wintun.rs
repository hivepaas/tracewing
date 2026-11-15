//! Windows: a Wintun adapter, which is a [`crate::tun::Device`].
//!
//! Wintun is WireGuard's layer-3 TUN driver for Windows. The operating
//! system routes packets into the adapter, Wintun hands them over through a
//! ring buffer, and what comes out is what [`crate::stack`] takes: IP
//! packets, which it turns into flows.
//!
//! ## Why this rather than WinDivert
//!
//! [capture-modes.md](../../../docs/design/capture-modes.md) planned
//! WinDivert, and the reasons against it are not about its code:
//!
//! - **There is no signed ARM64 driver, and nobody has agreed to make one.**
//!   On a Windows-on-ARM machine that capture mode would not exist at all.
//!   Wintun ships signed binaries for both x86-64 and ARM64.
//! - **Its signing has no owner.** The project froze v2.2.2 as its long-term
//!   version in 2022 and says it will change only for security; the attempt
//!   to get the driver signed again was abandoned in 2026 after a
//!   certificate expired mid-process. Wintun's signature is kept alive
//!   because WireGuard for Windows ships it, which is somebody else's reason
//!   and does not depend on a favour.
//! - **Antivirus does block it.** BitDefender quarantined `WinDivert64.sys`
//!   in 2025 with a clean VirusTotal. Wintun's driver is already installed on
//!   every machine that runs WireGuard.
//!
//! What is given up is real and belongs here rather than in a commit
//! message: WinDivert takes packets **by process**, leaving the rest of the
//! machine alone, and hands over the owning process with them. A TUN adapter
//! takes whatever is routed into it. Choosing one program is then a decision
//! made in tracewing after the packet is already ours, and the program
//! behind a flow has to be looked up by its address — which is what
//! [`crate::apps`] does for every other way in.
//!
//! ## What is here and what is not
//!
//! Here: the adapter, its session, and the two halves of
//! [`crate::tun::Device`] over Wintun's ring.
//!
//! Not here, and nothing captures anything until it is: **an address on the
//! adapter and a route into it**, and the matching exclusion that keeps
//! tracewing's own connections out of the adapter it is reading. Without the
//! second, every flow tracewing makes on a captured program's behalf is
//! routed back into tracewing. That is the dangerous half, and it is a
//! change in the outbound path rather than here, so it is its own piece of
//! work.
//!
//! The address setters the `wintun` crate offers are deliberately unused:
//! they shell out to `netsh`. Addressing and routing will be done through
//! `windows-sys` with the rest of this program's Windows calls.

use crate::tun::Device;
use anyhow::{Result, anyhow};
use std::{path::Path, sync::Arc};

/// How much of the ring Wintun keeps for packets on their way in. It takes a
/// power of two, and this is large enough to hold a burst while the stack is
/// busy without taking memory that a capture holds for its whole life.
const RING_BYTES: u32 = 4 * 1024 * 1024;

/// What Windows calls the adapter, and the kind it says it is. Both are shown
/// in the network settings, so they say tracewing rather than anything about
/// tunnels.
const ADAPTER_NAME: &str = "tracewing";
const ADAPTER_TYPE: &str = "tracewing";

/// The adapter's identity, fixed rather than fresh each time.
///
/// Windows keys a network profile — and the firewall answers that go with it
/// — on the adapter's GUID. A new one on every run would mean a new
/// "unidentified network", and a new set of prompts, every time tracewing
/// started. Generated once; changing it is changing the adapter's identity on
/// every machine that has already answered for it.
const ADAPTER_GUID: u128 = 0x9f2b_4d61_8e03_47a5_b1c6_2f7d_0a93_5e48;

/// An open Wintun adapter, with a session running on it.
pub struct WintunAdapter {
    /// The field order is the drop order, and it is the point: a session must
    /// not outlive the adapter it came from, and neither may outlive the
    /// library whose code they are still inside. The `wintun` crate may well
    /// hold its own references for both — that could not be verified from its
    /// documentation, so these are held rather than assumed.
    session: Arc<::wintun::Session>,
    _adapter: Arc<::wintun::Adapter>,
    _library: ::wintun::Wintun,
}

impl WintunAdapter {
    /// Loads Wintun from `dll` and creates the adapter.
    ///
    /// Needs Administrator: Wintun installs its driver on the first call and
    /// fails without it.
    pub fn open(dll: &Path) -> Result<Self> {
        // SAFETY: this loads and runs the code in `dll`. The caller passes a
        // path — ours, beside our own executable — rather than a name for
        // Windows to go looking for, because a `wintun.dll` found on the
        // search path would be somebody else's choice of code to run with
        // Administrator behind it.
        let library = unsafe { ::wintun::load_from_path(dll) }
            .map_err(|e| anyhow!("load Wintun from {}: {e}", dll.display()))?;
        // The errors are carried by hand rather than with `?`: whether
        // `wintun::Error` is one `anyhow` will take could not be checked
        // without a Windows machine to build on, and a message loses nothing.
        let adapter =
            ::wintun::Adapter::create(&library, ADAPTER_NAME, ADAPTER_TYPE, Some(ADAPTER_GUID))
                .map_err(|e| {
                    anyhow!(
                        "create the {ADAPTER_NAME} adapter: {e}. Wintun needs Administrator, \
                         and wintun.dll beside the program"
                    )
                })?;
        let session = Arc::new(
            adapter
                .start_session(RING_BYTES)
                .map_err(|e| anyhow!("start a Wintun session: {e}"))?,
        );
        Ok(Self {
            session,
            _adapter: adapter,
            _library: library,
        })
    }
}

impl Device for WintunAdapter {
    fn name(&self) -> String {
        "wintun".to_owned()
    }

    fn read(&self) -> Result<Vec<u8>> {
        let packet = self
            .session
            .receive_blocking()
            .map_err(|e| anyhow!("read from the Wintun adapter: {e}"))?;
        // Copied out rather than held: the packet borrows the ring until it
        // is dropped, and the stack is handed bytes of its own.
        Ok(packet.bytes().to_vec())
    }

    fn write(&self, packet: &[u8]) -> Result<()> {
        let size = u16::try_from(packet.len()).map_err(|_| {
            anyhow!(
                "a packet of {} bytes is longer than Wintun takes",
                packet.len()
            )
        })?;
        let mut out = self
            .session
            .allocate_send_packet(size)
            .map_err(|e| anyhow!("room in the Wintun ring for {size} bytes: {e}"))?;
        out.bytes_mut().copy_from_slice(packet);
        self.session.send_packet(out);
        Ok(())
    }

    fn shutdown(&self) {
        // Unlike the two above, this failing is worth saying: it is what ends
        // the reading thread, and a stop that did not take would be a
        // tracewing that will not quit.
        if let Err(e) = self.session.shutdown() {
            tracing::warn!("asking the Wintun session to stop: {e}");
        }
    }
}
