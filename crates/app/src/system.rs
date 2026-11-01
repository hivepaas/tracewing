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
use tracewing_proxy::{ca::LocalCa, crypto_provider};
use tracewing_system::{self as system, Scope, TakenOver};

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
    /// What went wrong last, in words.
    pub problem: Option<String>,
    /// Off in tests, whose program cannot be a watchdog.
    pub no_watchdog: bool,
}

/// The proxy makes the CA as it starts; making one here too would give two.
fn load_ca(data_dir: &Path) -> anyhow::Result<LocalCa> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !data_dir.join("ca.pem").exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    LocalCa::load_or_create(data_dir, crypto_provider())
}

fn check_ca(data_dir: &Path) -> anyhow::Result<bool> {
    let ca = load_ca(data_dir)?;
    Ok(system::verify_ca(&ca, crypto_provider()).is_ok())
}

fn install_ca(data_dir: &Path) -> anyhow::Result<bool> {
    let ca = load_ca(data_dir)?;
    system::ca_install(ca.cert_path(), Scope::User)?;
    Ok(system::verify_ca(&ca, crypto_provider()).is_ok())
}

impl Traffic {
    pub(crate) fn start_ca_check(&mut self, cx: &mut Context<Self>) {
        let data_dir = self.data_dir.clone();
        self.run_ca_job(move || check_ca(&data_dir), cx);
    }

    pub fn install_certificate(&mut self, cx: &mut Context<Self>) {
        if self.system.ca == CaState::Installing {
            return;
        }
        self.system.ca = CaState::Installing;
        self.system.problem = None;
        let data_dir = self.data_dir.clone();
        self.run_ca_job(move || install_ca(&data_dir), cx);
        cx.notify();
    }

    fn run_ca_job(
        &mut self,
        job: impl FnOnce() -> anyhow::Result<bool> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async move { job() }).await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(true) => this.system.ca = CaState::Trusted,
                    Ok(false) => this.system.ca = CaState::NotTrusted,
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
