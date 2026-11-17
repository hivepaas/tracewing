//! The local way in, assembled: the adapter up, the sieve that keeps the
//! chosen programs, the way out for everything the sieve passes on, and the
//! echo that answers `ping`.
//!
//! **One function with an arm per platform**, rather than one per platform:
//! what differs is the device and how an echo leaves, and everything else --
//! which programs are kept, what happens to the rest, and the shape of the
//! answer when there is no source -- is the same question wherever it is
//! asked. A system that cannot do this says so in words rather than being
//! missing from a `cfg`.
//!
//! The pieces and why each is where it is:
//!
//! - **the device** is the platform's. On Windows that is a Wintun adapter
//!   with an address and the two halves of each address space
//!   ([`tracewing_proxy::wintun::Held`]).
//! - **the sieve** keeps the programs somebody chose and passes the rest
//!   straight on. It is shared rather than rebuilt: the list behind it is an
//!   `Arc<RwLock<..>>`, so adding a program while capture runs changes what
//!   is kept without touching the source.
//! - **the way out** is pinned, because a connection that followed the
//!   ordinary routes would go back into tracewing's own adapter. The pinning
//!   is process-wide and kept up to date by [`tracewing_proxy::roads::watch`],
//!   which this starts.
//! - **the echo** is optional, and the one of the four that is: without it
//!   `ping` goes unanswered, which is a feature missing rather than a wrong
//!   behaviour. The other three have no safe default at all.

use anyhow::{Result, bail};
use tracewing_proxy::{
    apps::Apps,
    roads,
    sieve::{self, Chosen},
    source::Source,
    upstream,
};

use crate::local_apps::{self, Local, Support};

/// How often the way out is looked at again. A machine that moves between
/// networks changes it, and nothing tells us when.
// Used by the Windows arm, and by the Linux one being built beside it: these
// are the parts of the assembly that are not the platform's. Marked rather
// than moved into the arm, because moving them would mean the next arm
// starts by taking them back out again. Same idiom as `system::launch`'s
// `cmd_script`.
#[cfg_attr(not(windows), allow(dead_code))]
const LOOK_EVERY: std::time::Duration = std::time::Duration::from_secs(2);

/// How long an echo may take before it counts as lost. `ping` itself waits
/// about this long.
#[cfg_attr(not(windows), allow(dead_code))]
const ECHO_WITHIN: std::time::Duration = std::time::Duration::from_secs(4);

/// A way in that is running, and the two handles the window keeps.
pub struct Started {
    /// For the engine's list of sources.
    pub source: Box<dyn Source>,
    /// What is kept, changeable while it runs: the tab writes through this.
    pub chosen: Chosen,
    /// Turning the mode **off** without a restart, which is what was
    /// decided. Ending the device's read is the first link of the cascade
    /// [`tracewing_proxy::tun::run`] documents: the reading thread ends, the
    /// stack ends with its channel, the writing thread ends with the stack,
    /// and the adapter's address and routes go with the last handle to it.
    pub stopper: Stopper,
}

/// What ends the capture, held by the window rather than by the source.
pub struct Stopper(#[cfg(windows)] std::sync::Arc<tracewing_proxy::wintun::WintunAdapter>);

impl Stopper {
    /// Ends it. Safe to call twice: the second shutdown of a session that has
    /// already gone is what the live test
    /// `a_shutdown_ends_the_reading_loop` leaves behind.
    pub fn stop(&self) {
        #[cfg(windows)]
        {
            use tracewing_proxy::tun::Device;
            self.0.shutdown();
        }
    }
}

/// Starts it, or says why there is none.
///
/// `Ok(None)` is "nothing to start and nothing wrong": the switch is off, or
/// nothing was chosen. An `Err` is for the window to show, and every one of
/// them says what somebody could do about it.
///
/// `ports` is every port of tracewing's own that programs on this machine
/// connect to, which is what [`Apps`] needs to tell their sockets apart.
pub fn start(
    local: &Local,
    ports: Vec<u16>,
    data_dir: &std::path::Path,
) -> Result<Option<Started>> {
    if !local.on {
        return Ok(None);
    }
    match Support::here() {
        Support::NotOnThisSystem => bail!(
            "this system has no way to capture by route yet, so there is nothing for the \
             switch to turn on"
        ),
        // The window offers the relaunch rather than reaching this, so this
        // is the case where a switch was left on by a run that was elevated
        // and the next one is not. Said, because the alternative is a switch
        // that looks on with nothing behind it.
        Support::NeedsAdministrator => bail!(
            "capturing this machine's traffic needs an administrator: tracewing has to be \
             started again as one"
        ),
        Support::Ready => {}
    }
    let _ = (ports, data_dir, &local.programs);
    #[cfg(windows)]
    {
        return windows::start(local, ports, data_dir).map(Some);
    }
    #[cfg(not(windows))]
    unreachable!("Support::here() answers NotOnThisSystem off Windows")
}

/// The sieve, shared with the window so that what is kept can change while
/// the capture runs.
///
/// `Apps` is a second one: the proxy builds its own inside `run`, and this
/// source is built before the proxy starts, so there is no asking it for its.
/// What that costs is roughly one extra lookup per kept connection, and the
/// shape that does not duplicate it is one `Apps` handed to both -- see the
/// design doc, which names it as the follow-up rather than leaving it to be
/// discovered.
#[cfg_attr(not(windows), allow(dead_code))]
fn sieving(programs: &[local_apps::Picked], ports: Vec<u16>) -> Result<(sieve::Sieve, Chosen)> {
    let apps = Apps::spawn(ports)?;
    let chosen = Chosen::of(local_apps::programs_of(programs));
    let sieve = sieve::by_programs(apps, chosen.clone(), upstream::straight_to());
    Ok((sieve, chosen))
}

/// Keeps the pinning up to date, and gives the echo a way to ask where to
/// leave from.
///
/// Started here rather than by the engine because it is this mode that makes
/// the question matter: with no adapter held there is nothing to pin away
/// from, and `leave_by(None)` is that state.
#[cfg_attr(not(windows), allow(dead_code))]
fn watching(ours: u64, stop: tracewing_proxy::source::Stop) -> Result<()> {
    roads::watch(ours, LOOK_EVERY, roads::roads_now, upstream::leave_by, stop)
}

#[cfg(windows)]
mod windows {
    use super::*;
    use tracewing_proxy::{icmp, tun::Tun, wintun::Held};

    pub(super) fn start(
        local: &Local,
        ports: Vec<u16>,
        data_dir: &std::path::Path,
    ) -> Result<Started> {
        let held = Held::up(&dll(data_dir))?;
        let ours = held.luid()?;
        let stopper = Stopper(held.adapter());
        let (sieve, chosen) = sieving(&local.programs, ports)?;
        let source = Tun::new(held, sieve, upstream::bound_datagrams()).answering(echo(ours));
        Ok(Started {
            source: Box::new(source),
            chosen,
            stopper,
        })
    }

    /// Wintun's own library, beside tracewing's program.
    ///
    /// By a path rather than by a name, and the reason is in
    /// [`tracewing_proxy::wintun::WintunAdapter::open`]: a `wintun.dll` found
    /// on the search path would be somebody else's choice of code to run with
    /// an administrator behind it. `WINTUN_DLL` overrides it, which is what
    /// the live tests use.
    fn dll(data_dir: &std::path::Path) -> std::path::PathBuf {
        if let Some(said) = std::env::var_os("WINTUN_DLL") {
            return said.into();
        }
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|dir| dir.join("wintun.dll")))
            .unwrap_or_else(|| data_dir.join("wintun.dll"))
    }

    /// Where an echo leaves from: an address of the interface the machine's
    /// own traffic leaves by, never one of the adapter's.
    ///
    /// Both tables are read per echo. That is two reads for something that
    /// happens once a second during a `ping` and thirty times over a
    /// `tracert`, which is cheap enough to prefer over a cached answer that
    /// could be wrong the moment a machine changes network -- and being
    /// wrong here means an echo routed into tracewing's own adapter.
    fn echo(ours: u64) -> icmp::Echo {
        icmp::by_ip_helper(
            Arc::new(move |v6| {
                let roads = roads::roads_now().ok()?;
                let out = roads::out_by(&roads, ours, v6)?;
                let luid = roads
                    .iter()
                    .find(|road| road.interface == out)
                    .map(|road| road.luid)?;
                roads::source_on(&roads::addresses_now().ok()?, luid, v6)
            }),
            ECHO_WITHIN,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two answers that are not a source, and the difference between
    /// them: one is nothing to do and the other is something to say.
    #[test]
    fn a_switch_that_is_off_is_not_a_problem() {
        let dir = std::path::Path::new("/nowhere");
        let off = Local::default();
        assert!(
            matches!(start(&off, vec![9090], dir), Ok(None)),
            "off is nothing to start and nothing wrong"
        );
        let on = Local {
            on: true,
            ..Local::default()
        };
        let said = match start(&on, vec![9090], dir) {
            Err(why) => why.to_string(),
            Ok(_) => panic!("on, where nothing captures by route, is worth saying"),
        };
        match cfg!(windows) {
            // Elevated or not, it has something else to say.
            true => assert!(!said.contains("no way to capture by route"), "{said}"),
            false => assert!(said.contains("no way to capture by route"), "{said}"),
        }
    }
}
