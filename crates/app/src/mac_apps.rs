//! The macOS apps way in, from the window: the apps somebody picked, and the
//! switch that has the network extension take their connections.
//!
//! The engine opens the extension's port as the window starts
//! ([`Engine::start_for_window`](crate::engine::Engine::start_for_window)),
//! so nothing here needs a restart: switching on installs the extension where
//! it is not yet, and saves the proxy configuration with the port, this run's
//! key and the apps' signing ids; choosing another app while it is on saves
//! it again. Every call to macOS waits for an answer that comes on the main
//! queue, so each runs on the background executor, never on the window's
//! thread.
//!
//! The apps are kept in `mac-apps.toml`. Whether the switch is on is not: it
//! starts off, as the system proxy does, so that opening tracewing never
//! installs anything or asks anything of macOS by itself.

use crate::{
    colors::colors,
    layout::Way,
    text_size::ui,
    traffic::Traffic,
    widgets::{Reachable, eyebrow},
};
use anyhow::{Context as _, Result};
use gpui::{Context, Div, FontWeight, PathPromptOptions, div, prelude::*, rgb};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tracewing_i18n::tr;
use tracewing_system::extension::{self, Config, Installed};

/// Where the apps are kept, in the data directory.
pub const FILE: &str = "mac-apps.toml";

/// An app as it was picked: where it was, and the signing ids read from it
/// then — its own first. Read once, so that a list of apps does not run
/// `codesign` over each of them every time the window starts.
///
/// Or a program, or a folder of them, taken **by where it is**: one with no
/// signing id of its own, such as what `go build` makes. Its path is what the
/// extension is told, made real as it is told, so that a link chosen — a
/// Homebrew program's — follows the version it points at.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chosen {
    pub name: String,
    pub path: PathBuf,
    pub ids: Vec<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub by_path: bool,
}

impl Chosen {
    /// A folder, for every program under it.
    pub fn is_folder(&self) -> bool {
        self.by_path && self.path.is_dir() && self.path.extension().is_none_or(|e| e != "app")
    }
}

#[derive(Default, Serialize, Deserialize)]
struct File {
    #[serde(default, rename = "app")]
    apps: Vec<Chosen>,
}

/// The apps kept in `dir`, none where the file is missing or unreadable.
pub fn load(dir: &Path) -> Vec<Chosen> {
    std::fs::read_to_string(dir.join(FILE))
        .ok()
        .and_then(|text| toml::from_str::<File>(&text).ok())
        .map(|file| file.apps)
        .unwrap_or_default()
}

pub fn save(dir: &Path, apps: &[Chosen]) -> Result<()> {
    let text = toml::to_string(&File {
        apps: apps.to_vec(),
    })?;
    std::fs::create_dir_all(dir)?;
    let path = dir.join(FILE);
    let new = path.with_extension("toml.new");
    std::fs::write(&new, text).with_context(|| format!("write {}", new.display()))?;
    std::fs::rename(&new, &path).with_context(|| format!("write {}", path.display()))
}

/// Every signing id of `apps`, each once, in the order they were picked.
pub fn ids_of(apps: &[Chosen]) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for id in apps
        .iter()
        .filter(|app| !app.by_path)
        .flat_map(|app| &app.ids)
    {
        if !ids.contains(id) {
            ids.push(id.clone());
        }
    }
    ids
}

/// What the extension is told of those taken by where they are; and those
/// that are not there any more, by name.
pub fn programs_of(apps: &[Chosen]) -> (Vec<String>, Vec<String>) {
    let mut rules = Vec::new();
    let mut gone = Vec::new();
    for app in apps.iter().filter(|app| app.by_path) {
        match extension::program_rule(&app.path) {
            Ok(rule) if !rules.contains(&rule) => rules.push(rule),
            Ok(_) => {}
            Err(_) => gone.push(app.path.display().to_string()),
        }
    }
    (rules, gone)
}

/// `chosen` with `picked` added, an app picked again taking the place of
/// what was read of it before.
pub fn with(mut chosen: Vec<Chosen>, picked: Vec<Chosen>) -> Vec<Chosen> {
    for app in picked {
        match chosen.iter_mut().find(|c| c.path == app.path) {
            Some(there) => *there = app,
            None => chosen.push(app),
        }
    }
    chosen
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AppsSwitch {
    #[default]
    Off,
    /// Being switched on, or saved again with other apps.
    Starting,
    Stopping,
    On,
}

#[derive(Default)]
pub struct MacApps {
    pub chosen: Vec<Chosen>,
    pub switch: AppsSwitch,
    /// Set from the background while macOS waits for the person to allow
    /// the extension in System Settings.
    pub waiting: Arc<AtomicBool>,
    /// Whether the extension is installed, once asked.
    pub installed: Option<Installed>,
    /// Apps being read after they were picked.
    pub reading: bool,
    pub problem: Option<String>,
    /// The switch's last job: an older one's answer is stale.
    job: u64,
}

impl MacApps {
    pub fn load(dir: &Path) -> Self {
        Self {
            chosen: load(dir),
            ..Self::default()
        }
    }
}

/// How often the window looks at whether macOS has started waiting.
const LOOK_EVERY: Duration = Duration::from_millis(300);

impl Traffic {
    /// What the extension is to be saved with: this engine's port and key,
    /// and the chosen apps.
    fn apps_config(&self) -> Result<Config, String> {
        let mut config = self.engine.extension.clone().ok_or_else(|| {
            self.engine
                .extension_problem
                .clone()
                .unwrap_or_else(|| tr!("tracewing has no port open for the extension."))
        })?;
        config.apps = ids_of(&self.mac_apps.chosen);
        let (programs, gone) = programs_of(&self.mac_apps.chosen);
        config.programs = programs;
        if !gone.is_empty() {
            return Err(tr!(
                "Not there any more: {paths}. Remove it, or put it back.",
                paths = gone.join(", ")
            ));
        }
        Ok(config)
    }

    /// Asks macOS whether the extension is installed, for the tab to say.
    pub fn check_extension(&mut self, cx: &mut Context<Self>) {
        if !cfg!(target_os = "macos") {
            return;
        }
        cx.spawn(async move |this, cx| {
            let status = cx.background_spawn(async { extension::status() }).await;
            let _ = this.update(cx, |this, cx| {
                if let Ok(status) = status {
                    this.mac_apps.installed = Some(status.installed);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Asks for apps, or programs, and adds them.
    pub fn ask_add_mac_apps(&mut self, cx: &mut Context<Self>) {
        let chosen = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: true,
            multiple: true,
            prompt: Some(tr!("Add").into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = chosen.await {
                let _ = this.update(cx, |this, cx| this.add_mac_apps(paths, cx));
            }
        })
        .detach();
    }

    /// Reads the signing ids of what is at `paths`, then adds each.
    pub fn add_mac_apps(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        self.mac_apps.reading = true;
        self.mac_apps.problem = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let read: Vec<Result<Chosen>> = cx
                .background_spawn(async move {
                    paths
                        .into_iter()
                        .map(|path| {
                            extension::program_at(&path).map(|program| Chosen {
                                name: program.name,
                                path,
                                ids: program.ids,
                                by_path: program.by_path,
                            })
                        })
                        .collect()
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.mac_apps.reading = false;
                let mut picked = Vec::new();
                for app in read {
                    match app {
                        Ok(app) => picked.push(app),
                        Err(e) => this.mac_apps.problem = Some(format!("{e:#}")),
                    }
                }
                if !picked.is_empty() {
                    let chosen = with(std::mem::take(&mut this.mac_apps.chosen), picked);
                    this.set_mac_apps(chosen, cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub fn remove_mac_app(&mut self, index: usize, cx: &mut Context<Self>) {
        let mut chosen = self.mac_apps.chosen.clone();
        if index < chosen.len() {
            chosen.remove(index);
            self.set_mac_apps(chosen, cx);
        }
    }

    /// Keeps `chosen`, and has the running proxy take them.
    fn set_mac_apps(&mut self, chosen: Vec<Chosen>, cx: &mut Context<Self>) {
        if let Err(e) = save(&self.data_dir, &chosen) {
            self.mac_apps.problem = Some(format!("{e:#}"));
        }
        self.mac_apps.chosen = chosen;
        if matches!(self.mac_apps.switch, AppsSwitch::On | AppsSwitch::Starting) {
            if self.mac_apps.chosen.is_empty() {
                self.stop_mac_apps(cx);
            } else {
                self.start_mac_apps(cx);
            }
        }
        cx.notify();
    }

    pub fn toggle_mac_apps(&mut self, cx: &mut Context<Self>) {
        match self.mac_apps.switch {
            AppsSwitch::Off => self.start_mac_apps(cx),
            AppsSwitch::On | AppsSwitch::Starting => self.stop_mac_apps(cx),
            AppsSwitch::Stopping => {}
        }
    }

    /// Installs the extension where it is not, and starts the proxy with
    /// this engine's port and the chosen apps; or saves it again with them.
    pub fn start_mac_apps(&mut self, cx: &mut Context<Self>) {
        if self.mac_apps.chosen.is_empty() {
            self.mac_apps.problem = Some(tr!(
                "Add an app first: it is the apps you add that are taken."
            ));
            cx.notify();
            return;
        }
        let config = match self.apps_config() {
            Ok(config) => config,
            Err(problem) => {
                self.mac_apps.problem = Some(problem);
                cx.notify();
                return;
            }
        };
        let was_on = self.mac_apps.switch == AppsSwitch::On;
        self.mac_apps.switch = AppsSwitch::Starting;
        self.mac_apps.problem = None;
        self.mac_apps.waiting.store(false, Ordering::Relaxed);
        self.mac_apps.job += 1;
        let job = self.mac_apps.job;
        let waiting = self.mac_apps.waiting.clone();
        self.look_while_starting(job, cx);
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    extension::install(&|| waiting.store(true, Ordering::Relaxed))?;
                    extension::start(&config)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.mac_apps.job != job {
                    return;
                }
                this.mac_apps.waiting.store(false, Ordering::Relaxed);
                match result {
                    Ok(()) => {
                        this.mac_apps.switch = AppsSwitch::On;
                        this.mac_apps.installed = Some(Installed::Yes);
                        // Asked from when it could carry anything, and not
                        // again for a list saved anew while it was on.
                        if !was_on {
                            this.on_since.insert(Way::Apps, std::time::Instant::now());
                        }
                    }
                    Err(e) => {
                        this.mac_apps.switch = AppsSwitch::Off;
                        this.on_since.remove(&Way::Apps);
                        this.mac_apps.problem = Some(format!("{e:#}"));
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Redraws while a start is in hand, so that macOS starting to wait for
    /// the person is said as it happens: that is told on another thread.
    fn look_while_starting(&mut self, job: u64, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(LOOK_EVERY).await;
                let going = this
                    .update(cx, |this, cx| {
                        let going = this.mac_apps.job == job
                            && this.mac_apps.switch == AppsSwitch::Starting;
                        if going {
                            cx.notify();
                        }
                        going
                    })
                    .unwrap_or(false);
                if !going {
                    break;
                }
            }
        })
        .detach();
    }

    pub fn stop_mac_apps(&mut self, cx: &mut Context<Self>) {
        self.mac_apps.switch = AppsSwitch::Stopping;
        self.mac_apps.problem = None;
        self.mac_apps.job += 1;
        let job = self.mac_apps.job;
        self.on_since.remove(&Way::Apps);
        cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async { extension::stop() }).await;
            let _ = this.update(cx, |this, cx| {
                if this.mac_apps.job != job {
                    return;
                }
                this.mac_apps.switch = AppsSwitch::Off;
                if let Err(e) = result {
                    this.mac_apps.problem = Some(format!("{e:#}"));
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// After the engine started again, on another port with another key:
    /// the proxy, if on, is saved again with those.
    pub(crate) fn engine_restarted_for_mac_apps(&mut self, cx: &mut Context<Self>) {
        if self.mac_apps.switch == AppsSwitch::On {
            self.start_mac_apps(cx);
        }
    }

    /// Stops the proxy as the app closes. Sent, not waited for: waiting
    /// would mean serving macOS's main queue from the window's own thread in
    /// the middle of quitting, which runs the window's tasks inside its quit.
    /// Left running, it would be harmless — with nothing proving it holds the
    /// key on the port, every flow goes out as it would have — but it would
    /// still be in System Settings as connected.
    pub(crate) fn release_mac_apps(&mut self) {
        if matches!(self.mac_apps.switch, AppsSwitch::On | AppsSwitch::Starting) {
            extension::stop_on_exit();
        }
    }

    /// The switch, the apps and what to do about them: the same on the
    /// Capture tab and on Set up.
    pub(crate) fn render_mac_apps(&mut self, cx: &mut Context<Self>) -> Div {
        let c = colors();
        let apps = &self.mac_apps;
        let waiting = apps.waiting.load(Ordering::Relaxed);
        let says = match apps.switch {
            AppsSwitch::Starting if waiting => tr!(
                "macOS is waiting for you to allow tracewing's network extension, in {place}.",
                place = extension::WHERE_TO_ALLOW
            ),
            AppsSwitch::Starting => tr!(
                "Starting. The first time, macOS asks you to allow tracewing to add a proxy configuration."
            ),
            AppsSwitch::Stopping => tr!("Stopping."),
            AppsSwitch::On => tr!(
                "On: the connections of the apps below come to tracewing, whatever their proxy settings say."
            ),
            AppsSwitch::Off => match apps.installed {
                Some(Installed::No) | None => tr!(
                    "Off. Switching it on installs tracewing's network extension the first time, and macOS asks you to allow it, once."
                ),
                Some(Installed::WaitingForApproval) => tr!(
                    "Off. The extension is installed and waits to be allowed, in {place}.",
                    place = extension::WHERE_TO_ALLOW
                ),
                Some(Installed::Yes) => tr!("Off."),
                Some(Installed::Unknown) => tr!(
                    "Off. This tracewing does not run from tracewing.app, and macOS lets only the app install its extension: switch it on from tracewing in Applications."
                ),
            },
        };
        let (label, loud) = match apps.switch {
            AppsSwitch::Off => (tr!("Turn on"), true),
            AppsSwitch::Starting | AppsSwitch::On => (tr!("Turn off"), false),
            AppsSwitch::Stopping => (tr!("Turn off"), false),
        };
        let switch = (apps.switch != AppsSwitch::Stopping).then(|| {
            crate::setup_page::button("mac-apps-switch", label, loud)
                .on_click(cx.listener(|this, _, _, cx| this.toggle_mac_apps(cx)))
        });
        let open_settings = (apps.switch == AppsSwitch::Starting && waiting
            || apps.installed == Some(Installed::WaitingForApproval))
        .then(|| {
            small_button("mac-apps-allow", tr!("Open System Settings")).on_click(|_, _, _| {
                let _ = std::process::Command::new("/usr/bin/open")
                    .arg("x-apple.systempreferences:com.apple.LoginItems-Settings.extension")
                    .spawn();
            })
        });
        let rows: Vec<_> = apps
            .chosen
            .iter()
            .enumerate()
            .map(|(index, app)| {
                let inside = app.ids.len().saturating_sub(1);
                let path = app.path.display().to_string();
                let ids = if app.is_folder() {
                    tr!("every program in {path}", path = path)
                } else if app.by_path {
                    tr!("{path}, by where it is", path = path)
                } else if inside == 0 {
                    app.ids.first().cloned().unwrap_or_default()
                } else {
                    tr_inside(&app.ids[0], inside)
                };
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(ui(10.))
                    .py(ui(6.))
                    .border_t_1()
                    .border_color(rgb(c.line))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(div().font_weight(FontWeight::BOLD).child(app.name.clone()))
                            .child(
                                div()
                                    .text_size(ui(12.))
                                    .text_color(rgb(c.muted))
                                    .font_family(crate::mono_font())
                                    .child(ids),
                            ),
                    )
                    .child(
                        small_button(("mac-app-remove", index), tr!("Remove")).on_click(
                            cx.listener(move |this, _, _, cx| this.remove_mac_app(index, cx)),
                        ),
                    )
            })
            .collect();
        let empty = apps.chosen.is_empty();
        div()
            .flex()
            .flex_col()
            .gap(ui(8.))
            .w_full()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap(ui(12.))
                    .children(switch)
                    .child(div().flex_1().min_w(ui(240.)).child(says))
                    .children(open_settings),
            )
            .child(eyebrow(tr!("Apps it takes")))
            .when(empty, |d| {
                d.child(div().text_color(rgb(c.muted)).child(tr!(
                    "None yet. Add an app from Applications, a program such as /usr/bin/curl, or a folder your builds go to; what is inside an app — the helpers a browser or an Electron app connects from — comes with it, and a program with no signing id of its own, such as what go build makes, is taken by where it is."
                )))
            })
            .children(rows)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(ui(10.))
                    .child(
                        small_button("mac-apps-add", tr!("Add an app…"))
                            .on_click(cx.listener(|this, _, _, cx| this.ask_add_mac_apps(cx))),
                    )
                    .when(apps.reading, |d| {
                        d.child(div().text_color(rgb(c.muted)).child(tr!("Reading its signing ids…")))
                    }),
            )
            .children(apps.problem.clone().map(|problem| {
                div().text_color(rgb(c.error)).child(problem)
            }))
    }
}

/// An app's own id, and how many more come from inside it.
fn tr_inside(own: &str, inside: usize) -> String {
    tracewing_i18n::tr_n!(
        inside,
        "{own} and {n} inside it",
        "{own} and {n} inside it",
        own = own.to_owned()
    )
}

fn small_button(id: impl Into<gpui::ElementId>, label: String) -> gpui::Stateful<Div> {
    let c = colors();
    div()
        .id(id)
        .flex_shrink_0()
        .h(ui(30.))
        .px(ui(12.))
        .rounded(ui(15.))
        .flex()
        .items_center()
        .cursor_pointer()
        .reachable()
        .text_size(ui(12.5))
        .font_weight(FontWeight::BOLD)
        .bg(rgb(c.chip))
        .text_color(rgb(c.on_chip))
        .child(label)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(name: &str, ids: &[&str]) -> Chosen {
        Chosen {
            name: name.into(),
            path: PathBuf::from(format!("/Applications/{name}.app")),
            ids: ids.iter().map(|s| s.to_string()).collect(),
            by_path: false,
        }
    }

    fn by_path(path: &Path) -> Chosen {
        Chosen {
            name: "built".into(),
            path: path.to_owned(),
            ids: Vec::new(),
            by_path: true,
        }
    }

    #[test]
    fn a_program_by_path_is_told_as_its_real_path_and_not_as_an_id() {
        let dir = std::env::temp_dir().join(format!("tracewing-by-path-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("bin")).unwrap();
        let real = std::fs::canonicalize(&dir).unwrap();
        let apps = [
            app("curl", &["com.apple.curl"]),
            by_path(&dir.join("bin")),
            by_path(&dir.join("gone")),
        ];
        assert_eq!(ids_of(&apps), ["com.apple.curl"]);
        let (rules, gone) = programs_of(&apps);
        // The folder's real path, with the trailing slash the rule adds —
        // joined by `Path`, not by a `/` written here, which on Windows put
        // the separator in a place no path has it.
        assert_eq!(rules, [format!("{}/", real.join("bin").display())]);
        assert_eq!(gone, [dir.join("gone").display().to_string()]);
        assert!(apps[1].is_folder());
        // Kept, and read back as chosen; an app by id keeps the file as it was.
        save(&dir, &apps).unwrap();
        assert_eq!(load(&dir), apps);
        assert!(
            !std::fs::read_to_string(dir.join(FILE))
                .unwrap()
                .split("[[app]]")
                .nth(1)
                .unwrap()
                .contains("by_path"),
            "nothing new in what an app by id is kept as"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_apps_are_kept_and_read_back() {
        let dir = std::env::temp_dir().join(format!("tracewing-mac-apps-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(load(&dir), [], "none before any is picked");
        let apps = vec![
            app(
                "Slack",
                &[
                    "com.tinyspeck.slackmacgap",
                    "com.tinyspeck.slackmacgap.helper",
                ],
            ),
            app("curl", &["com.apple.curl"]),
        ];
        save(&dir, &apps).unwrap();
        assert_eq!(load(&dir), apps);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_id_two_apps_share_is_given_once() {
        let apps = [
            app("Chrome", &["com.google.Chrome", "com.google.Chrome.helper"]),
            app(
                "Chrome again",
                &["com.google.Chrome.helper", "com.google.GoogleUpdater"],
            ),
        ];
        assert_eq!(
            ids_of(&apps),
            [
                "com.google.Chrome",
                "com.google.Chrome.helper",
                "com.google.GoogleUpdater"
            ]
        );
    }

    #[test]
    fn an_app_picked_again_is_read_again_rather_than_listed_twice() {
        let before = vec![app("Slack", &["old"]), app("curl", &["com.apple.curl"])];
        let after = with(before, vec![app("Slack", &["new"])]);
        assert_eq!(after.len(), 2);
        assert_eq!(after[0].ids, ["new"], "in its place");
    }
}
