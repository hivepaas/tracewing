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

use anyhow::Result as Built;
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

/// A local way in that is running, as the engine keeps it: what it keeps,
/// and what ends it. The source itself has gone into the proxy's list by
/// then, which is why this is not simply [`Started`].
pub struct Running {
    /// What is kept, changeable while it runs: the tab writes through this.
    pub chosen: Chosen,
    /// Turning the mode off without a restart.
    pub stopper: Stopper,
}

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
/// **The device itself, by trait rather than by platform**, because ending a
/// capture is `Device::shutdown` wherever there is one to end -- so an arm
/// added later hands over its own device and needs nothing changed here. It
/// was a Windows-shaped field first, which would have meant the next arm
/// beginning by widening it.
pub struct Stopper(std::sync::Arc<dyn tracewing_proxy::tun::Device>);

impl Stopper {
    /// The device whose read this ends.
    pub fn of(device: std::sync::Arc<dyn tracewing_proxy::tun::Device>) -> Self {
        Self(device)
    }

    /// Ends it. Safe to call twice: shutting down a session that has already
    /// gone is what the live test `a_shutdown_ends_the_reading_loop` leaves
    /// behind, and the device says so rather than failing.
    pub fn stop(&self) {
        self.0.shutdown();
    }
}

/// Why there is no source, where the window has something to say about it.
///
/// **Cases rather than prose, and that was measured rather than preferred:**
/// `crates/system` contains no `tr!` at all, and nor did the `bail!`s this
/// replaces -- so an error carried from there to the window is untranslated
/// English in somebody's Vietnamese or Russian tab. A case has one
/// translated sentence per arm, and each platform maps its own refusal into
/// the same one.
#[derive(Debug)]
pub enum NoSource {
    /// Nothing here captures by route.
    NotHere,
    /// It needs an administrator and tracewing is not one. The tab normally
    /// heads this off by offering the relaunch; reaching it means a switch
    /// left on by an elevated run with this one not elevated.
    NeedsAdministrator,
    /// The privileged step was refused -- a UAC dialog dismissed, a `pkexec`
    /// denied. **Nothing was done**, so the switch goes back off with this
    /// beside it rather than looking on with nothing behind it.
    Refused,
    /// Something else, which the window shows as it came and the log keeps
    /// in full. Prose, and the only case that is: an unexpected failure has
    /// no sentence written for it in fourteen languages, and inventing one
    /// that says nothing would be worse than the words that happened.
    Failed(String),
}

impl std::fmt::Display for NoSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // For the log. The window matches on the case instead.
        match self {
            Self::NotHere => write!(f, "this system does not capture by route"),
            Self::NeedsAdministrator => write!(f, "an administrator is needed"),
            Self::Refused => write!(f, "the privileged step was refused"),
            Self::Failed(why) => write!(f, "{why}"),
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
) -> Result<Option<Started>, NoSource> {
    if !local.on {
        return Ok(None);
    }
    match Support::here() {
        Support::NotOnThisSystem => return Err(NoSource::NotHere),
        Support::NeedsAdministrator => return Err(NoSource::NeedsAdministrator),
        Support::Ready => {}
    }
    platform(local, ports, data_dir)
}

/// The arm for this platform. One function per platform rather than one
/// `cfg` block inside `start`, because a block that is the tail expression
/// on one platform and a statement on another is a `return` that is needed
/// here and warned about there -- which is how this shape arrived, from a
/// Windows compiler.
#[cfg(windows)]
fn platform(
    local: &Local,
    ports: Vec<u16>,
    data_dir: &std::path::Path,
) -> Result<Option<Started>, NoSource> {
    windows::start(local, ports, data_dir)
        .map(Some)
        .map_err(|why| NoSource::Failed(format!("{why:#}")))
}

/// Nothing here captures by route, and [`Support::here`] has already said so
/// above -- so this is unreachable rather than a case. A Linux arm goes
/// beside the Windows one, not here.
#[cfg(not(windows))]
fn platform(
    _local: &Local,
    _ports: Vec<u16>,
    _data_dir: &std::path::Path,
) -> Result<Option<Started>, NoSource> {
    unreachable!("Support::here() answers NotOnThisSystem off Windows")
}

/// A source with the machine's way out pinned **before** it starts.
///
/// The order is the whole point, and it is the discipline `Tun::start`
/// already keeps for its own stack: with no pinning in place, the first
/// connection tracewing makes on a captured program's behalf follows the
/// ordinary routes -- which now lead into tracewing's own adapter, where the
/// stack answers it as the server it was going to, which makes another
/// connection, which is routed back in. A loop with nothing bounding it.
///
/// It is a wrapper rather than something inside `tun.rs` because the pump is
/// generic over its device and has no business knowing about routes, and
/// because [`roads::watch`] wants two things only `start` has: the proxy's
/// stop, and the proxy's runtime to spawn on.
#[cfg_attr(not(windows), allow(dead_code))]
struct Pinned {
    inner: Box<dyn Source>,
    /// The adapter's own LUID, which is never a way out.
    ours: u64,
}

impl Source for Pinned {
    fn name(&self) -> String {
        self.inner.name()
    }

    fn by(&self) -> tracewing_proxy::source::By {
        self.inner.by()
    }

    fn own_ports(&self) -> Vec<u16> {
        self.inner.own_ports()
    }

    fn start(
        self: Box<Self>,
        proxy: std::net::SocketAddr,
        found: tracewing_proxy::source::Found,
        stop: tracewing_proxy::source::Stop,
    ) -> Built<()> {
        let me = *self;
        watching(me.ours, stop.clone())?;
        me.inner.start(proxy, found, stop)
    }
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
fn sieving(programs: &[local_apps::Picked], ports: Vec<u16>) -> Built<(sieve::Sieve, Chosen)> {
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
fn watching(ours: u64, stop: tracewing_proxy::source::Stop) -> Built<()> {
    roads::watch(ours, LOOK_EVERY, roads::roads_now, upstream::leave_by, stop)
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::sync::Arc;
    use tracewing_proxy::{icmp, tun::Tun, wintun::Held};

    pub(super) fn start(
        local: &Local,
        ports: Vec<u16>,
        data_dir: &std::path::Path,
    ) -> Built<Started> {
        let held = Held::up(&dll(data_dir))?;
        let ours = held.luid()?;
        let stopper = Stopper::of(held.adapter());
        let (sieve, chosen) = sieving(&local.programs, ports)?;
        let pump = Tun::new(held, sieve, upstream::bound_datagrams()).answering(echo(ours));
        let source = Pinned {
            inner: Box::new(pump),
            ours,
        };
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
        let why = match start(&on, vec![9090], dir) {
            Err(why) => why,
            Ok(_) => panic!("on, where nothing captures by route, is worth saying"),
        };
        match cfg!(windows) {
            // Elevated or not, it is one of the other cases.
            true => assert!(!matches!(why, NoSource::NotHere), "{why:?}"),
            false => assert!(matches!(why, NoSource::NotHere), "{why:?}"),
        }
    }
}
