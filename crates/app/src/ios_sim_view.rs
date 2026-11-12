//! iOS simulators on this Mac, from the Set up page: the certificate trusted
//! by each running one, with Xcode's `simctl`. Nothing is looked for until the
//! user asks, and nothing shows where there is no `xcrun`.

use crate::traffic::Traffic;
use gpui::{AppContext as _, Context};
use std::{collections::BTreeSet, path::PathBuf};
use tracewing_i18n::tr;
use tracewing_system::ios_sim::{self, Simulator};

#[derive(Default)]
pub struct IosState {
    /// `None` until looked for; then where `xcrun` is, if anywhere.
    pub xcrun: Option<Option<PathBuf>>,
    pub simulators: Vec<Simulator>,
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
}

impl Traffic {
    pub fn find_simulators(&mut self, cx: &mut Context<Self>) {
        let state = &mut self.system.ios;
        if state.looking {
            return;
        }
        state.looking = true;
        state.problem = None;
        // One given already, as a test does, is kept.
        let given = state.xcrun.clone().flatten();
        cx.spawn(async move |this, cx| {
            let (xcrun, found) = cx
                .background_spawn(async move {
                    let xcrun = given.or_else(ios_sim::find_xcrun);
                    let found = xcrun.as_deref().map(ios_sim::booted);
                    (xcrun, found)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                let state = &mut this.system.ios;
                state.looking = false;
                match found {
                    Some(Ok(simulators)) => state.simulators = simulators,
                    Some(Err(e)) => state.problem = Some(format!("{e:#}")),
                    None => state.simulators.clear(),
                }
                state.xcrun = Some(xcrun);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Puts the certificate among the simulator's trusted roots.
    pub fn trust_simulator(&mut self, udid: String, cx: &mut Context<Self>) {
        let Some(Some(xcrun)) = self.system.ios.xcrun.clone() else {
            return;
        };
        if self.system.ios.working.is_some() {
            return;
        }
        self.system.ios.working = Some(udid.clone());
        self.system.ios.problem = None;
        let pem = self.data_dir.join("ca.pem");
        cx.spawn(async move |this, cx| {
            let target = udid.clone();
            let result = cx
                .background_spawn(async move { ios_sim::trust(&xcrun, &target, &pem) })
                .await;
            let _ = this.update(cx, |this, cx| {
                let state = &mut this.system.ios;
                state.working = None;
                match result {
                    Ok(()) => {
                        state.trusted.insert(udid);
                    }
                    Err(e) => {
                        state.problem = Some(tr!(
                            "The simulator did not take it: {why}",
                            why = format!("{e:#}")
                        ))
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}
