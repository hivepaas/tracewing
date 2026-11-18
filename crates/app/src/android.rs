//! Android emulators and phones on USB, from the Set up page: every device
//! found set up in one go, or one at a time. Everything goes through `adb` off
//! the window's thread; nothing touches a device until the user asks, and
//! `adb` is not even started until they do.

use crate::traffic::Traffic;
use gpui::{AppContext as _, Context};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use tracewing_system::{
    self as system,
    android::{self, Device, SetUp},
};

#[derive(Default)]
pub struct AndroidState {
    /// `None` until looked for; then where `adb` is, if anywhere.
    pub adb: Option<Option<PathBuf>>,
    pub devices: Vec<Device>,
    pub looking: bool,
    /// The device being set up or put back.
    pub working: Option<String>,
    /// The devices pointed at the proxy, by serial.
    pub set: BTreeMap<String, SetUp>,
    /// What went wrong last, in words.
    pub problem: Option<String>,
}

/// Where `adb` is, if anywhere, and what it lists.
type Looked = (Option<PathBuf>, Option<anyhow::Result<Vec<Device>>>);

/// Finds `adb`, unless one was given, as a test does, and asks it for the
/// devices.
async fn look(given: Option<PathBuf>) -> Looked {
    let adb = given.or_else(android::find_adb);
    let found = adb.as_deref().map(android::devices);
    (adb, found)
}

/// Points one device at the proxy, with the certificate.
fn set_up_one(adb: &Path, serial: &str, port: u16, data_dir: &Path) -> anyhow::Result<SetUp> {
    let pem = data_dir.join("ca.pem");
    let der = system::ca_der(&pem)?;
    android::set_up(adb, serial, port, &pem, &der, data_dir)
}

impl Traffic {
    pub fn find_android_devices(&mut self, cx: &mut Context<Self>) {
        if self.system.android.looking {
            return;
        }
        self.system.android.looking = true;
        self.system.android.problem = None;
        let given = self.system.android.adb.clone().flatten();
        cx.spawn(async move |this, cx| {
            let found = cx.background_spawn(look(given)).await;
            let _ = this.update(cx, |this, cx| {
                this.system.android.looking = false;
                this.found_android(found, cx);
            });
        })
        .detach();
        cx.notify();
    }

    fn found_android(&mut self, (adb, found): Looked, cx: &mut Context<Self>) {
        let state = &mut self.system.android;
        match found {
            Some(Ok(devices)) => state.devices = devices,
            Some(Err(e)) => state.problem = Some(format!("{e:#}")),
            None => state.devices.clear(),
        }
        state.adb = Some(adb);
        cx.notify();
    }

    /// The devices ready and not yet set up, by serial.
    pub fn android_to_set_up(&self) -> Vec<String> {
        let state = &self.system.android;
        state
            .devices
            .iter()
            .filter(|device| device.ready() && !state.set.contains_key(&device.serial))
            .map(|device| device.serial.clone())
            .collect()
    }

    /// Looks for the devices again, which also tells what changed on them
    /// since, and sets up every one ready, one after another.
    pub fn set_up_android_all(&mut self, cx: &mut Context<Self>) {
        let state = &mut self.system.android;
        if state.looking || state.working.is_some() {
            return;
        }
        state.looking = true;
        state.problem = None;
        let given = state.adb.clone().flatten();
        let data_dir = self.data_dir.clone();
        let port = self.engine.addr.port();
        cx.spawn(async move |this, cx| {
            let found = cx.background_spawn(look(given)).await;
            let _ = this.update(cx, |this, cx| this.found_android(found, cx));
            let Ok((Some(Some(adb)), serials)) = this.update(cx, |this, _| {
                (this.system.android.adb.clone(), this.android_to_set_up())
            }) else {
                let _ = this.update(cx, |this, cx| {
                    this.system.android.looking = false;
                    cx.notify();
                });
                return;
            };
            let mut failed = Vec::new();
            for serial in serials {
                let _ = this.update(cx, |this, cx| {
                    this.system.android.working = Some(serial.clone());
                    cx.notify();
                });
                let (adb, target, data_dir) = (adb.clone(), serial.clone(), data_dir.clone());
                let result = cx
                    .background_spawn(async move { set_up_one(&adb, &target, port, &data_dir) })
                    .await;
                let _ = this.update(cx, |this, _| {
                    let state = &mut this.system.android;
                    state.working = None;
                    match result {
                        Ok(done) => {
                            state.set.insert(serial, done);
                        }
                        Err(e) => failed.push(format!("{serial}: {e:#}")),
                    }
                });
            }
            let _ = this.update(cx, |this, cx| {
                let state = &mut this.system.android;
                state.looking = false;
                if !failed.is_empty() {
                    state.problem = Some(failed.join("\n"));
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Points the device at the proxy and leaves the certificate trusted, or
    /// ready to be.
    pub fn set_up_android(&mut self, serial: String, cx: &mut Context<Self>) {
        let Some(Some(adb)) = self.system.android.adb.clone() else {
            return;
        };
        if self.system.android.working.is_some() || self.system.android.looking {
            return;
        }
        self.system.android.working = Some(serial.clone());
        self.system.android.problem = None;
        let data_dir = self.data_dir.clone();
        let port = self.engine.addr.port();
        cx.spawn(async move |this, cx| {
            let target = serial.clone();
            let result = cx
                .background_spawn(async move { set_up_one(&adb, &target, port, &data_dir) })
                .await;
            let _ = this.update(cx, |this, cx| {
                let state = &mut this.system.android;
                state.working = None;
                match result {
                    Ok(done) => {
                        state.set.insert(serial, done);
                    }
                    Err(e) => state.problem = Some(format!("{e:#}")),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Takes the proxy off the device again.
    pub fn undo_android(&mut self, serial: String, cx: &mut Context<Self>) {
        let Some(Some(adb)) = self.system.android.adb.clone() else {
            return;
        };
        if self.system.android.working.is_some() || self.system.android.looking {
            return;
        }
        self.system.android.working = Some(serial.clone());
        self.system.android.problem = None;
        let data_dir = self.data_dir.clone();
        let port = self.engine.addr.port();
        cx.spawn(async move |this, cx| {
            let target = serial.clone();
            let result = cx
                .background_spawn(async move { android::undo(&adb, &target, port, &data_dir) })
                .await;
            let _ = this.update(cx, |this, cx| {
                let state = &mut this.system.android;
                state.working = None;
                match result {
                    Ok(()) => {
                        state.set.remove(&serial);
                    }
                    Err(e) => state.problem = Some(format!("{e:#}")),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Puts back devices a crash left pointed at the proxy, if any were.
    pub(crate) fn recover_android(&mut self, cx: &mut Context<Self>) {
        if android::read_journal(&self.data_dir).is_empty() {
            return;
        }
        let data_dir = self.data_dir.clone();
        cx.background_spawn(async move {
            if let Some(adb) = android::find_adb()
                && let Err(e) = android::recover(&adb, &data_dir)
            {
                eprintln!("tracewing: cannot put Android devices back: {e:#}");
            }
        })
        .detach();
    }

    /// Takes the proxy off every device set up, as the app closes. Waits, since
    /// nothing else will.
    pub(crate) fn release_android(&mut self) {
        let state = &mut self.system.android;
        let (Some(Some(adb)), false) = (&state.adb, state.set.is_empty()) else {
            return;
        };
        let port = self.engine.addr.port();
        for serial in std::mem::take(&mut state.set).into_keys() {
            let _ = android::undo(adb, &serial, port, &self.data_dir);
        }
    }
}
