//! The system proxy and the certificate, from the window. All of it can take a
//! while or wait on a dialog, so it runs off the window's thread, and a failure
//! says why in the footer instead of stopping anything.

use crate::traffic::Traffic;
use gpui::{AppContext as _, Context};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tracewing_i18n::tr;
use tracewing_proxy::{
    ca::LocalCa,
    ca_key::{KeyPlace, KeyStore},
    capture::CHECK_HOST,
    crypto_provider,
};
use tracewing_system::{
    self as system, BrowserTrust, Environment, Place, QuicBlock, Scope, TakenOver, launch::Browser,
};

#[derive(Default)]
pub enum ProxySwitch {
    #[default]
    Off,
    Working,
    On(TakenOver),
}

/// Outgoing UDP to port 443, blocked so that apps fall back from HTTP/3.
#[derive(Default)]
pub enum QuicSwitch {
    #[default]
    Off,
    /// Waiting for the administrator's password, or for the block to go.
    Working,
    On(QuicBlock),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum CaState {
    #[default]
    Checking,
    Trusted,
    NotTrusted,
    Installing,
    Removing,
}

#[derive(Default)]
pub struct SystemState {
    pub proxy: ProxySwitch,
    pub quic: QuicSwitch,
    pub ca: CaState,
    /// Where the certificate is installed.
    pub ca_places: Vec<Place>,
    /// The system's own verifier accepts it, which is not what browsers on Linux use.
    pub ca_verified: bool,
    /// What went wrong last, in words.
    pub problem: Option<String>,
    /// Off in tests, whose program cannot be a watchdog.
    pub no_watchdog: bool,
    /// The browsers on this machine, and what each trusts.
    pub browsers: Vec<(Browser, BrowserTrust)>,
    /// The certificate check last started: an older one's answer is stale.
    pub ca_job: u64,
    /// What the machine lacks for the steps on Set up.
    pub environment: Environment,
    /// Set up asks before the certificate is taken out of every store.
    pub confirm_remove: bool,
    /// Where the CA's key is, once looked at.
    pub ca_key: Option<KeyPlace>,
    /// Which device's guide the Set up page shows.
    pub guide: crate::setup_page::Guide,
    pub android: crate::android::AndroidState,
    pub ios: crate::ios_sim_view::IosState,
}

/// The proxy makes the CA as it starts; making one here too would give two.
fn load_ca(data_dir: &Path, store: KeyStore) -> anyhow::Result<LocalCa> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !data_dir.join("ca.pem").exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    LocalCa::load_or_create(data_dir, crypto_provider(), store)
}

struct CaFound {
    verified: bool,
    places: Vec<Place>,
    /// What went wrong installing, when some of it may still have gone in.
    problem: Option<String>,
    key: KeyPlace,
    browsers: Vec<(Browser, BrowserTrust)>,
    environment: Environment,
}

impl CaFound {
    /// Whether a browser going through the proxy would accept its certificates.
    /// On Linux the system's verifier reads the store command-line tools use,
    /// and browsers read their own, so only a place they read counts there.
    fn browsers_trust(&self) -> bool {
        self.places.iter().any(|place| place.browsers)
            || (self.verified && !cfg!(target_os = "linux"))
    }
}

fn look_for_ca(ca: &LocalCa, data_dir: &Path) -> anyhow::Result<CaFound> {
    Ok(CaFound {
        verified: system::verify_ca(ca, crypto_provider()).is_ok(),
        places: system::ca_locations(ca.cert_path())?,
        problem: None,
        key: ca.key_place().clone(),
        browsers: system::launch::find_browsers(data_dir)
            .into_iter()
            .map(|browser| {
                let trust = system::browser_trust(&browser);
                (browser, trust)
            })
            .collect(),
        environment: system::environment(),
    })
}

fn check_ca(data_dir: &Path, store: KeyStore) -> anyhow::Result<CaFound> {
    look_for_ca(&load_ca(data_dir, store)?, data_dir)
}

fn install_ca(data_dir: &Path, scope: Scope, store: KeyStore) -> anyhow::Result<CaFound> {
    let ca = load_ca(data_dir, store)?;
    let installed = system::ca_install(ca.cert_path(), scope);
    // Some stores may have taken it although another failed: look either way.
    let mut found = look_for_ca(&ca, data_dir)?;
    found.problem = installed
        .err()
        .map(|e| tr!("The certificate: {e}", e = format!("{e:#}")));
    Ok(found)
}

fn remove_ca(data_dir: &Path, store: KeyStore) -> anyhow::Result<CaFound> {
    let ca = load_ca(data_dir, store)?;
    let removed = system::ca_remove(ca.cert_path());
    // Some stores may have let it go although another refused: look either way.
    let mut found = look_for_ca(&ca, data_dir)?;
    found.problem = removed
        .err()
        .map(|e| tr!("The certificate: {e}", e = format!("{e:#}")));
    Ok(found)
}

impl Traffic {
    /// Looks again for the certificate in every store, and for the browsers.
    pub fn start_ca_check(&mut self, cx: &mut Context<Self>) {
        if matches!(self.system.ca, CaState::Installing | CaState::Removing) {
            return;
        }
        let (data_dir, store) = (self.data_dir.clone(), self.engine.key_store);
        self.run_ca_job(move || check_ca(&data_dir, store), cx);
    }

    /// Asks, on Set up, whether to take the certificate out of every store.
    pub fn ask_to_remove_certificate(&mut self, ask: bool, cx: &mut Context<Self>) {
        self.system.confirm_remove = ask;
        cx.notify();
    }

    /// Takes the certificate out of every store that holds it, the system
    /// asking for rights where a store needs them.
    pub fn remove_certificate(&mut self, cx: &mut Context<Self>) {
        if matches!(self.system.ca, CaState::Installing | CaState::Removing) {
            return;
        }
        self.system.confirm_remove = false;
        self.system.ca = CaState::Removing;
        self.system.problem = None;
        let (data_dir, store) = (self.data_dir.clone(), self.engine.key_store);
        self.run_ca_job(move || remove_ca(&data_dir, store), cx);
        cx.notify();
    }

    pub fn install_certificate(&mut self, scope: Scope, cx: &mut Context<Self>) {
        if matches!(self.system.ca, CaState::Installing | CaState::Removing) {
            return;
        }
        self.system.ca = CaState::Installing;
        self.system.problem = None;
        let (data_dir, store) = (self.data_dir.clone(), self.engine.key_store);
        self.run_ca_job(move || install_ca(&data_dir, scope, store), cx);
        cx.notify();
    }

    fn run_ca_job(
        &mut self,
        job: impl FnOnce() -> anyhow::Result<CaFound> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        self.system.ca_job += 1;
        let number = self.system.ca_job;
        cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async move { job() }).await;
            let _ = this.update(cx, |this, cx| {
                if this.system.ca_job != number {
                    return;
                }
                match result {
                    Ok(found) => {
                        this.system.ca = if found.browsers_trust() {
                            CaState::Trusted
                        } else {
                            CaState::NotTrusted
                        };
                        this.system.ca_verified = found.verified;
                        this.system.ca_key = Some(found.key);
                        this.system.ca_places = found.places;
                        this.system.browsers = found.browsers;
                        this.system.environment = found.environment;
                        if found.problem.is_some() {
                            this.system.problem = found.problem;
                        }
                    }
                    Err(e) => {
                        this.system.ca = CaState::NotTrusted;
                        this.system.problem =
                            Some(tr!("The certificate: {e}", e = format!("{e:#}")));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// A terminal that goes through the proxy, the system proxy left alone.
    pub fn open_terminal(&mut self, cx: &mut Context<Self>) {
        let (addr, data_dir) = (self.engine.addr, self.data_dir.clone());
        let store = self.engine.key_store;
        self.run_launch(
            tr!("The terminal"),
            move || {
                let ca = load_ca(&data_dir, store)?;
                system::launch::open_terminal(addr, ca.cert_path(), &data_dir)
            },
            cx,
        );
    }

    /// The test page in `name`, in the user's own profile, to see whether that
    /// browser goes through tracewing and trusts it.
    pub fn check_in_browser(&mut self, name: &str, cx: &mut Context<Self>) {
        let Some((browser, _)) = self.system.browsers.iter().find(|(b, _)| b.name == name) else {
            return;
        };
        let browser = browser.clone();
        self.run_launch(
            browser.name.to_owned(),
            move || system::launch::open_url(&browser, &format!("https://{CHECK_HOST}/")),
            cx,
        );
    }

    /// `name` with a profile of its own that goes through the proxy, at the
    /// test page, the user's own profile and the system proxy left alone.
    pub fn open_browser(&mut self, name: &str, cx: &mut Context<Self>) {
        let Some((browser, trust)) = self.system.browsers.iter().find(|(b, _)| b.name == name)
        else {
            return;
        };
        let browser = browser.clone();
        // Trusted, the CA is in the browser's store, and the flag would only
        // add a warning bar.
        let trusted = match *trust {
            BrowserTrust::System => self.system.ca == CaState::Trusted,
            BrowserTrust::Own { holding, of } => of > 0 && holding >= of,
        };
        let spki = (!trusted)
            .then(|| self.engine.leaf_spki.get().cloned())
            .flatten();
        let (addr, data_dir, store) = (
            self.engine.addr,
            self.data_dir.clone(),
            self.engine.key_store,
        );
        self.run_launch(
            browser.name.to_owned(),
            move || {
                let ca = load_ca(&data_dir, store)?;
                let url = format!("https://{CHECK_HOST}/");
                system::launch::open_browser(&browser, addr, spki.as_deref(), ca.cert_path(), &url)
            },
            cx,
        );
    }

    fn run_launch(
        &mut self,
        what: String,
        job: impl FnOnce() -> anyhow::Result<()> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        self.system.problem = None;
        cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async move { job() }).await;
            if let Err(e) = result {
                let _ = this.update(cx, |this, cx| {
                    this.system.problem = Some(format!("{what}: {e:#}"));
                    cx.notify();
                });
            }
        })
        .detach();
        cx.notify();
    }

    pub fn toggle_system_proxy(&mut self, cx: &mut Context<Self>) {
        match std::mem::take(&mut self.system.proxy) {
            ProxySwitch::Working => self.system.proxy = ProxySwitch::Working,
            ProxySwitch::Off => {
                self.system.proxy = ProxySwitch::Working;
                self.system.problem = None;
                self.take_over_system_proxy(self.data_dir.clone(), self.engine.addr, cx);
            }
            ProxySwitch::On(taken) => {
                self.system.proxy = ProxySwitch::Working;
                self.system.problem = None;
                cx.spawn(async move |this, cx| {
                    let result = cx.background_spawn(async move { taken.release() }).await;
                    let _ = this.update(cx, |this, cx| {
                        this.system.proxy = ProxySwitch::Off;
                        if let Err(e) = result {
                            this.system.problem =
                                Some(tr!("The system proxy: {e}", e = format!("{e:#}")));
                        }
                        cx.notify();
                    });
                })
                .detach();
            }
        }
        cx.notify();
    }

    fn take_over_system_proxy(
        &mut self,
        data_dir: PathBuf,
        addr: SocketAddr,
        cx: &mut Context<Self>,
    ) {
        let watchdog = !self.system.no_watchdog;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { system::take_over(&data_dir, addr, watchdog) })
                .await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(taken) => this.system.proxy = ProxySwitch::On(taken),
                    Err(e) => {
                        this.system.proxy = ProxySwitch::Off;
                        this.system.problem =
                            Some(tr!("The system proxy: {e}", e = format!("{e:#}")));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Blocks outgoing UDP to port 443, or lets it through again.
    pub fn toggle_quic_block(&mut self, cx: &mut Context<Self>) {
        match std::mem::take(&mut self.system.quic) {
            QuicSwitch::Working => self.system.quic = QuicSwitch::Working,
            QuicSwitch::Off => {
                self.system.quic = QuicSwitch::Working;
                self.system.problem = None;
                let data_dir = self.data_dir.clone();
                cx.spawn(async move |this, cx| {
                    let result = cx
                        .background_spawn(async move { system::block_quic(&data_dir) })
                        .await;
                    let _ = this.update(cx, |this, cx| {
                        match result {
                            Ok(block) => this.system.quic = QuicSwitch::On(block),
                            Err(e) => {
                                this.system.quic = QuicSwitch::Off;
                                this.system.problem =
                                    Some(tr!("Blocking HTTP/3: {e}", e = format!("{e:#}")));
                            }
                        }
                        cx.notify();
                    });
                })
                .detach();
            }
            QuicSwitch::On(block) => {
                self.system.quic = QuicSwitch::Working;
                self.system.problem = None;
                cx.spawn(async move |this, cx| {
                    let result = cx.background_spawn(async move { block.release() }).await;
                    let _ = this.update(cx, |this, cx| {
                        this.system.quic = QuicSwitch::Off;
                        if let Err(e) = result {
                            this.system.problem =
                                Some(tr!("Blocking HTTP/3: {e}", e = format!("{e:#}")));
                        }
                        cx.notify();
                    });
                })
                .detach();
            }
        }
        cx.notify();
    }

    /// Puts the system proxy back, lets UDP through again, and takes the proxy
    /// off Android devices, as the app closes.
    /// Waits, since nothing else will.
    pub(crate) fn release_system_proxy(&mut self) {
        if let ProxySwitch::On(taken) = std::mem::take(&mut self.system.proxy) {
            let _ = taken.release();
        }
        if let QuicSwitch::On(block) = std::mem::take(&mut self.system.quic) {
            let _ = block.release();
        }
        self.release_android();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(verified: bool, places: &[bool]) -> CaFound {
        CaFound {
            verified,
            places: places
                .iter()
                .map(|&browsers| Place {
                    name: String::new(),
                    browsers,
                    scope: Scope::User,
                })
                .collect(),
            problem: None,
            key: KeyPlace::Keychain,
            browsers: Vec::new(),
            environment: Environment::default(),
        }
    }

    #[test]
    fn a_store_browsers_read_is_trusted() {
        assert!(found(false, &[true]).browsers_trust());
        assert!(found(true, &[false, true]).browsers_trust());
        assert!(!found(false, &[]).browsers_trust());
    }

    #[test]
    fn the_system_store_alone_is_not_enough_on_linux() {
        let machine_only = found(true, &[false]);
        assert_eq!(
            machine_only.browsers_trust(),
            !cfg!(target_os = "linux"),
            "browsers on Linux do not read the system's store"
        );
    }
}
