//! iOS simulators on this Mac, from the Set up page, in one go: the
//! certificate trusted by each running one, with Xcode's `simctl`, and the
//! Mac's proxy turned on, since that is what a simulator goes through. Nothing
//! is looked for until the user asks, and nothing shows where there is no
//! `xcrun`.

use crate::{system::ProxySwitch, traffic::Traffic};
use gpui::{AppContext as _, Context};
use std::{collections::BTreeSet, path::PathBuf};
use tracewing_i18n::tr;
use tracewing_system::ios_sim::{self, Simulator};

/// Whether setting the simulators up turns the system proxy on too: only on a
/// Mac, where they are. Anywhere else an `xcrun` is one a test gave, and the
/// proxy is this machine's own.
pub const TURNS_THE_PROXY_ON: bool = cfg!(target_os = "macos");

#[derive(Default)]
pub struct IosState {
    /// `None` until looked for; then where `xcrun` is, if anywhere.
    pub xcrun: Option<Option<PathBuf>>,
    pub simulators: Vec<Simulator>,
    /// Looking for them and trusting the certificate in each.
    pub looking: bool,
    /// The one being given the certificate.
    pub working: Option<String>,
    /// Those given it, by UDID.
    pub trusted: BTreeSet<String>,
    pub problem: Option<String>,
}

impl IosState {
    /// Whether this machine can have simulators: a Mac, or `xcrun` given.
    pub fn possible(&self) -> bool {
        cfg!(target_os = "macos") || matches!(self.xcrun, Some(Some(_)))
    }

    /// Whether a simulator running now has been given the certificate.
    pub fn any_trusted(&self) -> bool {
        self.simulators
            .iter()
            .any(|sim| self.trusted.contains(&sim.udid))
    }
}

impl Traffic {
    /// Looks for the simulators running, trusts the certificate in each that
    /// lacks it, and turns the system proxy on if it is off.
    pub fn set_up_simulators(&mut self, cx: &mut Context<Self>) {
        let state = &mut self.system.ios;
        if state.looking {
            return;
        }
        state.looking = true;
        state.problem = None;
        // One given already, as a test does, is kept.
        let given = state.xcrun.clone().flatten();
        let pem = self.data_dir.join("ca.pem");
        cx.spawn(async move |this, cx| {
            let (xcrun, found) = cx
                .background_spawn(async move {
                    let xcrun = given.or_else(ios_sim::find_xcrun);
                    let found = xcrun.as_deref().map(ios_sim::booted);
                    (xcrun, found)
                })
                .await;
            let Ok(lacking) = this.update(cx, |this, cx| {
                let state = &mut this.system.ios;
                match found {
                    Some(Ok(simulators)) => state.simulators = simulators,
                    Some(Err(e)) => state.problem = Some(format!("{e:#}")),
                    None => state.simulators.clear(),
                }
                state.xcrun = Some(xcrun.clone());
                cx.notify();
                state
                    .simulators
                    .iter()
                    .filter(|sim| !state.trusted.contains(&sim.udid))
                    .map(|sim| (sim.udid.clone(), sim.name.clone()))
                    .collect::<Vec<_>>()
            }) else {
                return;
            };
            let mut failed = Vec::new();
            if let Some(xcrun) = xcrun {
                for (udid, name) in lacking {
                    let _ = this.update(cx, |this, cx| {
                        this.system.ios.working = Some(udid.clone());
                        cx.notify();
                    });
                    let (xcrun, target, pem) = (xcrun.clone(), udid.clone(), pem.clone());
                    let result = cx
                        .background_spawn(async move { ios_sim::trust(&xcrun, &target, &pem) })
                        .await;
                    let _ = this.update(cx, |this, _| {
                        let state = &mut this.system.ios;
                        state.working = None;
                        match result {
                            Ok(()) => {
                                state.trusted.insert(udid);
                            }
                            Err(e) => failed.push(tr!(
                                "{name} did not take the certificate: {why}",
                                name = name,
                                why = format!("{e:#}")
                            )),
                        }
                    });
                }
            }
            let _ = this.update(cx, |this, cx| {
                this.system.ios.looking = false;
                if !failed.is_empty() {
                    this.system.ios.problem = Some(failed.join("\n"));
                }
                if TURNS_THE_PROXY_ON
                    && this.system.ios.any_trusted()
                    && matches!(this.system.proxy, ProxySwitch::Off)
                {
                    this.toggle_system_proxy(cx);
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}
