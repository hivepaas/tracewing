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
//!   ([`tracewing_proxy::wintun::Held`]). On Linux it is a persistent TUN
//!   device that one privileged script made
//!   ([`tracewing_system::adapter`]) and this opens unprivileged
//!   ([`tracewing_proxy::tun_linux::TunDevice`]) -- so there, holding the
//!   device *is* the switch, and letting go of it is the whole of turning
//!   the mode off.
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
// Used by both arms: these are the parts of the assembly that are not the
// platform's. The mark stays rather than going, because macOS is still a
// platform where none of this is reached and `-D warnings` is still on
// there -- it is the `cfg` that narrowed, not the need for it. Same idiom
// as `system::launch`'s `cmd_script`.
#[cfg_attr(not(any(windows, target_os = "linux")), allow(dead_code))]
const LOOK_EVERY: std::time::Duration = std::time::Duration::from_secs(2);

/// How long an echo may take before it counts as lost. `ping` itself waits
/// about this long.
#[cfg_attr(not(any(windows, target_os = "linux")), allow(dead_code))]
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
    ///
    /// Who turns it off depends on where the refusal happened, and both are
    /// the same rule: the window does it in
    /// [`crate::local_apps::Local`]'s `toggle_local` when it asked for the
    /// relaunch itself, and [`crate::engine::Engine::start`] does it when a
    /// platform asked while assembling -- which is Linux, where `pkexec`
    /// runs inside this call.
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

/// The arm for Linux: one privileged step, and a device that was already
/// there.
#[cfg(target_os = "linux")]
fn platform(
    local: &Local,
    ports: Vec<u16>,
    // No library to find beside the program, which is the one thing the
    // Windows arm wants it for. What becomes of a refusal is
    // `Engine::start`'s, so that the file has one writer.
    _data_dir: &std::path::Path,
) -> Result<Option<Started>, NoSource> {
    linux::start(local, ports)
        .map(Some)
        .map_err(|why| linux::which(&why))
}

/// Nothing here captures by route, and [`Support::here`] has already said so
/// above -- so this is unreachable rather than a case. macOS alone: its way
/// in is the network extension, which is a different tab.
#[cfg(not(any(windows, target_os = "linux")))]
fn platform(
    _local: &Local,
    _ports: Vec<u16>,
    _data_dir: &std::path::Path,
) -> Result<Option<Started>, NoSource> {
    unreachable!("Support::here() answers NotOnThisSystem where nothing captures by route")
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
#[cfg_attr(not(any(windows, target_os = "linux")), allow(dead_code))]
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
#[cfg_attr(not(any(windows, target_os = "linux")), allow(dead_code))]
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
#[cfg_attr(not(any(windows, target_os = "linux")), allow(dead_code))]
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

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use anyhow::Context as _;
    use std::sync::Arc;
    use tracewing_proxy::{icmp, tun::Tun, tun_linux::TunDevice};
    use tracewing_system::adapter;

    /// Brings the adapter up if it is not up already, opens it, and builds
    /// the pump on it.
    ///
    /// **The adapter is made and read back before the device is opened**, and
    /// the order is not a preference. [`adapter::make_adapter`] deletes the
    /// device and makes it again -- that is what makes switching on twice
    /// work -- so a pump that opened it first would be holding a descriptor
    /// to a device that is about to go, and reading nothing for the rest of
    /// the session. Windows has no analogue of this step at all:
    /// `WintunAdapter::open` creates the adapter and opens it in one call.
    ///
    /// No `data_dir`: there is no library to find beside the program, which
    /// is the one thing the Windows arm needs it for.
    pub(super) fn start(local: &Local, ports: Vec<u16>) -> Built<Started> {
        adapter::make_adapter()?;
        // Read back rather than trusted. The step is a script run by another
        // process, and a script that exits 0 having done something else is
        // not a thing this can rule out -- while holding a device that is
        // not routed into is a capture that silently sees nothing.
        let look = adapter::look();
        anyhow::ensure!(
            look.ready(),
            "the adapter {} is still missing {}",
            adapter::DEVICE,
            look.missing().join(", ")
        );
        // The interface to leave out of our own way out. One number for both
        // families here, unlike Windows: see `roads::index_of`.
        let ours = u64::from(
            roads::index_of(adapter::DEVICE)
                .with_context(|| format!("the kernel does not number {}", adapter::DEVICE))?,
        );
        // Held twice on purpose: the pump reads it, and the window ends the
        // capture by shutting it down. The second handle is why `Device` is
        // implemented for `Arc`.
        let device = Arc::new(TunDevice::open(adapter::DEVICE)?);
        let stopper = Stopper::of(Arc::clone(&device) as Arc<dyn tracewing_proxy::tun::Device>);
        let (sieve, chosen) = sieving(&local.programs, ports)?;
        let pump = Tun::new(Arc::clone(&device), sieve, upstream::bound_datagrams());
        // The echo is the one optional piece, and on Linux it is optional in
        // fact as well as in shape: a ping socket needs this process's group
        // to be inside `net.ipv4.ping_group_range`, and where it is not,
        // `ping` goes unanswered rather than the mode failing.
        let pump = match echo(ours) {
            Some(echo) => pump.answering(echo),
            None => {
                eprintln!(
                    "tracewing: no ping socket here (net.ipv4.ping_group_range), so ping \
                     and traceroute through the adapter go unanswered"
                );
                pump
            }
        };
        Ok(Started {
            source: Box::new(Pinned {
                inner: Box::new(pump),
                ours,
            }),
            chosen,
            stopper,
        })
    }

    /// Which case a failure is, for the window.
    ///
    /// By type rather than by its words: `pkexec` turning the step away is
    /// the one thing somebody can act on, it is also the commonest -- a
    /// dialog dismissed -- and the sentence for it is written in fourteen
    /// languages in the tab. Everything else is handed on as it came.
    pub(super) fn which(why: &anyhow::Error) -> NoSource {
        match why.downcast_ref::<tracewing_system::Denied>() {
            Some(tracewing_system::Denied::NotGiven) => NoSource::Refused,
            _ => NoSource::Failed(format!("{why:#}")),
        }
    }

    /// Where an echo leaves from: an address of the interface the machine's
    /// own traffic leaves by, never one of the adapter's.
    ///
    /// Shorter than the Windows one by a lookup, and that is the one place
    /// Linux's single numbering shows: there, the way out comes back as a
    /// per-family index and the addresses are identified by LUID, so the two
    /// have to be joined through the routing table. Here the index *is* the
    /// identity, in both families and in both tables.
    fn echo(ours: u64) -> Option<icmp::Echo> {
        icmp::by_ping_socket(
            Arc::new(move |v6| {
                let roads = roads::roads_now().ok()?;
                let out = roads::out_by(&roads, ours, v6)?;
                roads::source_on(&roads::addresses_now().ok()?, u64::from(out), v6)
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
        // What `on` means, asked only where asking costs nothing. On a
        // system that captures by route `start` really brings the adapter
        // up: Windows without an administrator stops at
        // `NeedsAdministrator` before it tries, but an elevated Windows or
        // any Linux would go on and do it -- and on Linux that means a
        // password dialog in the middle of `cargo test`. The assembly has
        // live tests of its own for that, one per platform.
        let on = Local {
            on: true,
            ..Local::default()
        };
        match Support::here() {
            Support::NotOnThisSystem => assert!(
                matches!(start(&on, vec![9090], dir), Err(NoSource::NotHere)),
                "on, where nothing captures by route, is worth saying"
            ),
            Support::NeedsAdministrator => assert!(
                matches!(
                    start(&on, vec![9090], dir),
                    Err(NoSource::NeedsAdministrator)
                ),
                "it could, once tracewing comes back as one"
            ),
            // Nothing left to ask that would not change this machine.
            Support::Ready => {}
        }
    }

    /// Which failures reach the window as a translated sentence, and which
    /// reach it as the words that happened.
    ///
    /// The seam this tests is a `downcast_ref` through an
    /// `anyhow::Error`'s context chain, which is a thing to **check** rather
    /// than assume: if it did not find the case, every dismissed polkit
    /// dialog would arrive as `Failed` and the window would show English.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_refusal_is_a_case_and_everything_else_is_the_words_that_happened() {
        use tracewing_system::Denied;
        // As `run_as_admin` builds it: the case, with what it was for in
        // front of it.
        let refused = anyhow::Error::new(Denied::NotGiven)
            .context("make tracewing's network adapter".to_owned());
        assert!(
            matches!(linux::which(&refused), NoSource::Refused),
            "found through the context, or the tab shows English"
        );
        // Nothing was asked rather than something refused, and the remedy
        // differs -- start an agent, not answer a dialog -- so it is not the
        // same case.
        let no_agent =
            anyhow::Error::new(Denied::NoAgent).context("make tracewing's adapter".to_owned());
        match linux::which(&no_agent) {
            NoSource::Failed(why) => assert!(why.contains("polkit agent"), "{why}"),
            other => panic!("no agent is not a refusal: {other:?}"),
        }
        // And a failure of our own, which never saw polkit at all.
        let ours = anyhow::anyhow!("the device tracewing is open already");
        match linux::which(&ours) {
            NoSource::Failed(why) => assert!(why.contains("open already"), "{why}"),
            other => panic!("{other:?}"),
        }
    }
}
