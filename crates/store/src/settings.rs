//! What the user can change about how a session is stored. The app picks every
//! default, from what the benchmarks found (see docs/measurements.md), and every one can be
//! overridden: a laptop with little disk and a build server want different things.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

const MIB: u64 = 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StorageSettings {
    /// A body up to this size goes in the shared log, held in memory until it has
    /// all arrived. A larger one is written to a file of its own as it comes.
    pub log_max_kib: u64,
    /// The most kept of one body. It still reaches the client whole.
    pub body_max_mib: u64,
    /// Compress text bodies that came uncompressed. They are put back when shown.
    pub compress_stored: bool,
    /// zstd level for that, 1 to 19.
    pub compress_level: i32,
    /// Make URLs searchable, pieces of words included.
    pub index_urls: bool,
    /// Make the text of bodies searchable by word.
    pub index_bodies: bool,
    /// How much of each body's text goes in the index. The rest is found by
    /// scanning, which is slower. Also the most a body is unpacked for the index,
    /// which is what stops a zip bomb.
    pub index_kib_per_body: u64,
    /// The most a body is unpacked for viewing.
    pub view_max_mib: u64,
    /// A session past this size drops its oldest flows.
    pub session_max_mib: u64,
    /// With less than this left on the disk, stop keeping bodies and say so.
    pub disk_reserve_mib: u64,
    /// How many bytes of bodies may wait for the database, then for the disk.
    /// Past these, bodies stop being kept; traffic is never held back.
    pub queue_budget_mib: u64,
    pub spool_budget_mib: u64,
    /// How many earlier sessions to leave on the disk when a new one starts.
    pub sessions_kept: u64,
    /// Listen on every network, not only on this machine, so that phones and
    /// other computers can send their traffic here. Applies at once.
    pub other_devices: bool,
    /// Take servers' offers of HTTP/3 out of `Alt-Svc` on the way to the client.
    /// HTTP/3 goes over UDP, which a proxy does not carry, so a client that takes
    /// the offer goes around tracewing. The flow keeps the header as it came.
    /// Applies at once, through [`Live`].
    pub hide_http3: bool,
    /// Read a chosen program's HTTP/3 instead of refusing it, in a mode that
    /// takes this machine's packets by route. The program's UDP 443 goes to a
    /// QUIC door of tracewing's own rather than to the server, and the flows
    /// in it are recorded like any others. Off, HTTP/3 is refused and the
    /// program falls back to HTTP/2, which is how tracewing has always seen
    /// it. Applies at once, through [`Live`].
    pub capture_http3: bool,
    /// Trust servers whose certificates come from an authority this computer
    /// trusts, not only from the public ones. Read when the app starts.
    pub os_roots: bool,
    /// Let AI tools, through `tracewing mcp`, see the values of credential
    /// headers and cookies. Read at each call.
    pub ai_secrets: bool,
    /// Let AI tools add and remove rules, and send requests through the proxy.
    /// Read at each call.
    pub ai_actions: bool,
    /// Underline the values that have a reading behind them — a time, a
    /// token, something packed — so that a reader knows there is something
    /// there to read. Applies at once.
    pub mark_readings: bool,
    /// Open a SOCKS5 port as well as the proxy port, for the programs that
    /// speak SOCKS and not HTTP proxying: anything reading `ALL_PROXY`,
    /// Firefox's own SOCKS setting, proxychains. On this machine
    /// only, whatever `other_devices` says, because a SOCKS5 port asks for no
    /// password and reaches any address a client names. Read when tracewing
    /// starts.
    pub socks: bool,
    /// Which port that is. 1080 is the one every SOCKS client suggests, and
    /// the one a program told only "SOCKS" will try.
    pub socks_port: u64,
    /// Serve a WireGuard tunnel, so that a phone can be captured by joining
    /// it: no proxy to set by hand, and apps that ignore the Wi-Fi proxy are
    /// caught too.
    ///
    /// **Unlike `socks`, this port is open to the network, and it has to
    /// be** — a phone reaches this machine across the Wi-Fi, so loopback
    /// would serve nobody. What makes that sound is the protocol rather than
    /// the setting: an unauthenticated datagram is answered with silence, and
    /// only the one peer whose public key tracewing holds can open a session.
    /// Read when tracewing starts.
    pub wireguard: bool,
    /// Which port the tunnel listens on. 51820 is WireGuard's own, which is
    /// what a phone's configuration suggests and what a router is most
    /// likely to already allow.
    pub wireguard_port: u64,
    /// Start the browser tracewing opens **as an administrator**, rather than
    /// at the level the person using the computer has. Windows only, where
    /// the two differ; read at each launch.
    ///
    /// Off by default, and the weaker of the two cases: a Chromium refuses or
    /// complains when it is run as an administrator, so this is for the rare
    /// extension or kiosk being debugged under elevation.
    pub browser_as_administrator: bool,
    /// Start the terminal tracewing opens **as an administrator**. Windows
    /// only; read at each launch.
    ///
    /// Off by default, but a first-class case rather than an edge: a program
    /// that needs an administrator will not start in a terminal that is not
    /// one, and the terminal way in exists to put the proxy's variables in
    /// front of a program.
    pub terminal_as_administrator: bool,
    /// Look once a day whether a newer tracewing is out: the window reads the
    /// signed list of releases from GitHub (docs/updates.md), which sees this
    /// computer's address and nothing else. Off, it looks only when *Check
    /// now* is pressed in Settings › About. Applies at once.
    pub check_updates: bool,
}

impl Default for StorageSettings {
    fn default() -> Self {
        Self {
            log_max_kib: 64,
            body_max_mib: 1024,
            compress_stored: true,
            compress_level: 1,
            index_urls: true,
            index_bodies: true,
            index_kib_per_body: 64,
            view_max_mib: 4096,
            session_max_mib: 2048,
            disk_reserve_mib: 2048,
            queue_budget_mib: 256,
            spool_budget_mib: 256,
            sessions_kept: 5,
            other_devices: false,
            hide_http3: true,
            // Off: it terminates QUIC for the programs somebody chose, which
            // is a thing to ask for rather than to find already happening.
            capture_http3: false,
            os_roots: true,
            ai_secrets: false,
            ai_actions: false,
            mark_readings: true,
            socks: false,
            socks_port: 1080,
            wireguard: false,
            wireguard_port: 51820,
            // Both off: what tracewing starts for somebody to use gets the
            // level they have, until they say otherwise.
            browser_as_administrator: false,
            terminal_as_administrator: false,
            // On, as shellwing's is: an update that nobody hears of is one
            // nobody installs.
            check_updates: true,
        }
    }
}

/// The settings a running session follows as they change, from any thread.
/// The rest are read when it starts.
#[derive(Debug, Default)]
pub struct Live {
    hide_http3: AtomicBool,
    capture_http3: AtomicBool,
    body_max: AtomicU64,
}

impl Live {
    pub fn new(settings: &StorageSettings) -> Self {
        let live = Self::default();
        live.apply(settings);
        live
    }

    pub fn apply(&self, settings: &StorageSettings) {
        self.hide_http3
            .store(settings.hide_http3, Ordering::Relaxed);
        self.capture_http3
            .store(settings.capture_http3, Ordering::Relaxed);
        self.body_max.store(settings.body_max(), Ordering::Relaxed);
    }

    /// The setting as it stands, which the Settings page shows and the page
    /// alone should ask for: what the proxy acts on is
    /// [`Self::hiding_http3`].
    pub fn hide_http3(&self) -> bool {
        self.hide_http3.load(Ordering::Relaxed)
    }

    pub fn capture_http3(&self) -> bool {
        self.capture_http3.load(Ordering::Relaxed)
    }

    /// Whether servers' offers of HTTP/3 are taken out of what a client
    /// sees.
    ///
    /// Not the setting on its own. **Capturing HTTP/3 and hiding the offers
    /// of it are opposite things**, and one of them has to give: a program
    /// whose offers are hidden learns of HTTP/3 from nobody, never speaks
    /// it, and leaves the door with nothing to read. So while capture is on
    /// the offers go through, for every program and not only for the chosen
    /// ones -- a program nobody chose then tries QUIC and is refused, which
    /// costs it one attempt and is what a client that learns HTTP/3 from a
    /// DNS record has always done anyway, since hiding the offers never
    /// reached those.
    pub fn hiding_http3(&self) -> bool {
        self.hide_http3() && !self.capture_http3()
    }

    /// See [`StorageSettings::body_max`]. A body on its way is held to the
    /// limit as it is at each piece.
    pub fn body_max(&self) -> u64 {
        self.body_max.load(Ordering::Relaxed)
    }
}

impl StorageSettings {
    /// What applies at once from `other`, taken into these: the rest wait for
    /// the next start.
    pub fn take_live(&mut self, other: &StorageSettings) {
        self.other_devices = other.other_devices;
        self.hide_http3 = other.hide_http3;
        self.capture_http3 = other.capture_http3;
        self.ai_secrets = other.ai_secrets;
        self.ai_actions = other.ai_actions;
        self.mark_readings = other.mark_readings;
        self.body_max_mib = other.body_max_mib;
        self.view_max_mib = other.view_max_mib;
        self.check_updates = other.check_updates;
    }

    pub fn load(path: &Path) -> Result<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let settings: Self =
            toml::from_str(&text).with_context(|| format!("read {}", path.display()))?;
        settings.check()?;
        Ok(settings)
    }

    pub fn to_toml(&self) -> Result<String> {
        Ok(toml::to_string(self)?)
    }

    /// Refuses what cannot work, and only that: a strange choice is the user's.
    pub fn check(&self) -> Result<()> {
        if !(1..=19).contains(&self.compress_level) {
            bail!(
                "compress_level must be 1 to 19, not {}",
                self.compress_level
            );
        }
        if self.log_max_kib > 1024 {
            bail!(
                "log_max_kib must be at most 1024, not {}: every body in flight holds this much in memory",
                self.log_max_kib
            );
        }
        // Checked whether or not it is on: a number that cannot work is
        // worth saying so about before the switch is turned on.
        if !(1..=65535).contains(&self.wireguard_port) {
            bail!(
                "wireguard_port must be 1 to 65535, not {}: a phone has to be told which port to reach",
                self.wireguard_port
            );
        }
        if !(1..=65535).contains(&self.socks_port) {
            bail!(
                "socks_port must be 1 to 65535, not {}: a client has to be told which port to use",
                self.socks_port
            );
        }
        if self.session_max_mib < 64 {
            bail!(
                "session_max_mib must be at least 64, not {}: one body can be larger than that",
                self.session_max_mib
            );
        }
        Ok(())
    }

    pub fn log_max(&self) -> usize {
        (self.log_max_kib * 1024) as usize
    }

    pub fn body_max(&self) -> u64 {
        self.body_max_mib * MIB
    }

    pub fn view_max(&self) -> u64 {
        self.view_max_mib * MIB
    }

    pub fn index_per_body(&self) -> usize {
        (self.index_kib_per_body * 1024) as usize
    }

    pub fn session_max(&self) -> u64 {
        self.session_max_mib * MIB
    }

    pub fn disk_reserve(&self) -> u64 {
        self.disk_reserve_mib * MIB
    }

    pub fn queue_budget(&self) -> u64 {
        self.queue_budget_mib * MIB
    }

    pub fn spool_budget(&self) -> u64 {
        self.spool_budget_mib * MIB
    }

    /// Where the SOCKS5 port listens when it is on. `check` has already
    /// refused a number no port can have.
    pub fn socks_at(&self) -> std::net::SocketAddr {
        (std::net::Ipv4Addr::LOCALHOST, self.socks_port as u16).into()
    }

    /// Where the WireGuard tunnel listens. Every address this machine has,
    /// because the phone is on another one: see [`Self::wireguard`] for why
    /// that is the protocol's business and not the setting's.
    pub fn wireguard_at(&self) -> std::net::SocketAddr {
        (std::net::Ipv4Addr::UNSPECIFIED, self.wireguard_port as u16).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Result<StorageSettings> {
        let settings: StorageSettings = toml::from_str(text)?;
        settings.check()?;
        Ok(settings)
    }

    #[test]
    fn nothing_set_means_the_defaults() {
        assert_eq!(parse("").unwrap(), StorageSettings::default());
    }

    #[test]
    fn one_setting_leaves_the_others_alone() {
        let settings = parse("session_max_mib = 500").unwrap();
        assert_eq!(settings.session_max(), 500 * MIB);
        assert_eq!(settings.log_max_kib, StorageSettings::default().log_max_kib);
    }

    #[test]
    fn the_defaults_survive_being_written_out() {
        let written = StorageSettings::default().to_toml().unwrap();
        assert_eq!(parse(&written).unwrap(), StorageSettings::default());
    }

    #[test]
    fn a_misspelt_setting_is_an_error() {
        // Silently ignoring it would leave the user believing it took effect.
        assert!(parse("session_max = 500").is_err());
    }

    #[test]
    fn what_cannot_work_is_refused() {
        assert!(parse("compress_level = 0").is_err());
        assert!(parse("compress_level = 20").is_err());
        assert!(parse("session_max_mib = 1").is_err());
        assert!(parse("log_max_kib = 4096").is_err());
        // No port is 0 or past 65535, and a SOCKS5 client has to be told one.
        assert!(parse("socks_port = 0").is_err());
        assert!(parse("socks_port = 70000").is_err());
        let settings = parse("socks = true\nsocks_port = 1080").unwrap();
        assert_eq!(settings.socks_at(), "127.0.0.1:1080".parse().unwrap());

        assert!(parse("wireguard_port = 0").is_err());
        assert!(parse("wireguard_port = 70000").is_err());
        let settings = parse("wireguard = true\nwireguard_port = 51820").unwrap();
        assert_eq!(
            settings.wireguard_at(),
            "0.0.0.0:51820".parse().unwrap(),
            "every address, because the phone is on another one"
        );
        assert_ne!(
            settings.wireguard_at().ip(),
            settings.socks_at().ip(),
            "the two differ on purpose: one is this machine's alone, the other is not"
        );
    }

    #[test]
    fn turning_things_off_is_allowed() {
        let settings =
            parse("index_bodies = false\nindex_urls = false\ncompress_stored = false").unwrap();
        assert!(!settings.index_bodies);
    }
}
