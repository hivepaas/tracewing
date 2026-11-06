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
use tracewing_proxy::{ca::LocalCa, capture::CHECK_HOST, crypto_provider};
use tracewing_system::{self as system, Place, Scope, TakenOver, launch::Browser};

#[derive(Default)]
pub enum ProxySwitch {
    #[default]
    Off,
    Working,
    On(TakenOver),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum CaState {
    #[default]
    Checking,
    Trusted,
    NotTrusted,
    Installing,
}

#[derive(Default)]
pub struct SystemState {
    pub proxy: ProxySwitch,
    pub ca: CaState,
    /// Where the certificate is installed.
    pub ca_places: Vec<Place>,
    /// The system's own verifier accepts it, which is not what browsers on Linux use.
    pub ca_verified: bool,
    /// What went wrong last, in words.
    pub problem: Option<String>,
    /// Off in tests, whose program cannot be a watchdog.
    pub no_watchdog: bool,
    /// The browser "Open" starts, if there is one.
    pub browser: Option<Browser>,
}

/// The proxy makes the CA as it starts; making one here too would give two.
fn load_ca(data_dir: &Path) -> anyhow::Result<LocalCa> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !data_dir.join("ca.pem").exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    LocalCa::load_or_create(data_dir, crypto_provider())
}

struct CaFound {
    verified: bool,
    places: Vec<Place>,
    /// What went wrong installing, when some of it may still have gone in.
    problem: Option<String>,
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

fn look_for_ca(ca: &LocalCa) -> anyhow::Result<CaFound> {
    Ok(CaFound {
        verified: system::verify_ca(ca, crypto_provider()).is_ok(),
        places: system::ca_locations(ca.cert_path())?,
        problem: None,
    })
}

fn check_ca(data_dir: &Path) -> anyhow::Result<CaFound> {
    look_for_ca(&load_ca(data_dir)?)
}

fn install_ca(data_dir: &Path, scope: Scope) -> anyhow::Result<CaFound> {
    let ca = load_ca(data_dir)?;
    let installed = system::ca_install(ca.cert_path(), scope);
    // Some stores may have taken it although another failed: look either way.
    let mut found = look_for_ca(&ca)?;
    found.problem = installed.err().map(|e| format!("The certificate: {e:#}"));
    Ok(found)
}

impl Traffic {
    pub(crate) fn start_ca_check(&mut self, cx: &mut Context<Self>) {
        self.system.browser = system::launch::find_browser(&self.data_dir);
        let data_dir = self.data_dir.clone();
        self.run_ca_job(move || check_ca(&data_dir), cx);
    }

    pub fn install_certificate(&mut self, scope: Scope, cx: &mut Context<Self>) {
        if self.system.ca == CaState::Installing {
            return;
        }
        self.system.ca = CaState::Installing;
        self.system.problem = None;
        let data_dir = self.data_dir.clone();
        self.run_ca_job(move || install_ca(&data_dir, scope), cx);
        cx.notify();
    }

    fn run_ca_job(
        &mut self,
        job: impl FnOnce() -> anyhow::Result<CaFound> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async move { job() }).await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(found) => {
                        this.system.ca = if found.browsers_trust() {
                            CaState::Trusted
                        } else {
                            CaState::NotTrusted
                        };
                        this.system.ca_verified = found.verified;
                        this.system.ca_places = found.places;
                        if found.problem.is_some() {
                            this.system.problem = found.problem;
                        }
                    }
                    Err(e) => {
                        this.system.ca = CaState::NotTrusted;
                        this.system.problem = Some(format!("The certificate: {e:#}"));
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
        self.run_launch(
            "The terminal",
            move || {
                let ca = load_ca(&data_dir)?;
                system::launch::open_terminal(addr, ca.cert_path(), &data_dir)
            },
            cx,
        );
    }

    /// A browser with a profile of its own that goes through the proxy, at the
    /// test page.
    pub fn open_browser(&mut self, cx: &mut Context<Self>) {
        let Some(browser) = self.system.browser.clone() else {
            return;
        };
        let addr = self.engine.addr;
        // Trusted, the CA is in the browser's store, and the flag would only
        // add a warning bar.
        let spki = (self.system.ca != CaState::Trusted)
            .then(|| self.engine.leaf_spki.get().cloned())
            .flatten();
        self.run_launch(
            browser.name,
            move || {
                let url = format!("https://{CHECK_HOST}/");
                system::launch::open_browser(&browser, addr, spki.as_deref(), &url)
            },
            cx,
        );
    }

    fn run_launch(
        &mut self,
        what: &'static str,
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
                            this.system.problem = Some(format!("The system proxy: {e:#}"));
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
                        this.system.problem = Some(format!("The system proxy: {e:#}"));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Puts the system proxy back as the app closes. Waits, since nothing else will.
    pub(crate) fn release_system_proxy(&mut self) {
        if let ProxySwitch::On(taken) = std::mem::take(&mut self.system.proxy) {
            let _ = taken.release();
        }
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
                })
                .collect(),
            problem: None,
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
