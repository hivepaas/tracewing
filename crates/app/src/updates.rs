//! Telling that a newer tracewing is out, and installing it (see
//! `tracewing_update` and docs/updates.md): a look once a day while
//! `check_updates` is on, or when asked in Settings › About. What is offered
//! shows as a row over the traffic and in About; it never takes the keyboard.
//!
//! A copy the app can replace (one its installer put on Windows, an `.app` on
//! macOS) downloads the release when told to, checks it against the signed
//! list, and installs it as tracewing quits: at *Restart now*, or whenever the
//! user quits. A copy from the release's `.deb` or `.rpm` is given the
//! commands that install the next one. Any other copy opens the release's
//! page, or says what updates it.
//!
//! Ported from shellwing's `workspace/updates.rs`, which works the same way.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gpui::{ClipboardItem, Context, Div, FontWeight, div, prelude::*, rgb};
use serde::{Deserialize, Serialize};
use tracewing_i18n::tr;
use tracewing_update::{CheckError, Install, Version};

use crate::about_page::button;
use crate::colors::colors;
use crate::text_size::ui;
use crate::traffic::Traffic;
use crate::widgets::{Reachable, icon};

/// How often a look is due, and how often whether one is due is asked.
const CHECK_EVERY: Duration = Duration::from_secs(24 * 60 * 60);
const WAKE_EVERY: Duration = Duration::from_secs(60 * 60);
/// The first look waits for the window to settle.
const FIRST_AFTER: Duration = Duration::from_secs(20);
/// How often a download's progress is shown.
const PROGRESS_EVERY: Duration = Duration::from_millis(250);

/// What the updater remembers between runs, in `updates.json` in the data
/// folder.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateState {
    /// When it looked, in seconds since 1970.
    #[serde(default)]
    pub checked_at: Option<u64>,
    /// The release it was offered, if any (`v0.2.0`).
    #[serde(default)]
    pub offered: Option<String>,
    /// The offered release's system package for this computer, when this copy
    /// came from one and the list names it: its file, and its SHA-256.
    #[serde(default)]
    pub package: Option<(String, String)>,
    /// A release the user said to skip.
    #[serde(default)]
    pub skipped: Option<String>,
    /// A release downloaded and checked, waiting to be installed as the app
    /// quits: its version, and its file.
    #[serde(default)]
    pub staged: Option<(String, String)>,
}

impl UpdateState {
    /// What `path` holds; nothing, when it is not there or does not read.
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|data| serde_json::from_slice(&data).ok())
            .unwrap_or_default()
    }

    /// Written beside `path`, then renamed over it: whole or not at all.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let data = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        let part = path.with_extension("json.part");
        std::fs::write(&part, data)?;
        std::fs::rename(&part, path)
    }
}

/// Getting a release in.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Stage {
    #[default]
    Idle,
    /// Bytes so far, and of how many when known.
    Downloading(u64, Option<u64>),
    /// Downloaded and checked: installed as the app quits.
    Ready(Version),
    Failed(String),
}

/// How looking for updates stands.
pub struct Updates {
    pub state: UpdateState,
    state_path: PathBuf,
    /// Where a release's file is downloaded: `updates` in the data folder.
    staging: PathBuf,
    pub checking: bool,
    /// What went wrong the last time, if it did.
    pub problem: Option<String>,
    pub stage: Stage,
    /// Quitting to install the update starts the app again.
    relaunch: bool,
    /// The release whose row over the traffic was closed: back at the next
    /// start.
    row_closed: Option<Version>,
}

impl Updates {
    /// What was found before, and a release downloaded by an earlier run:
    /// ready still, unless what runs is it (or newer), whose file then goes.
    pub fn new(data_dir: &Path) -> Self {
        let state_path = data_dir.join("updates.json");
        let mut updates = Self {
            state: UpdateState::load(&state_path),
            state_path,
            staging: data_dir.join("updates"),
            checking: false,
            problem: None,
            stage: Stage::Idle,
            relaunch: false,
            row_closed: None,
        };
        if let Some((version, file)) = updates.state.staged.clone() {
            let path = PathBuf::from(&file);
            match Version::parse(&version) {
                Some(v) if v > tracewing_update::current() && path.exists() => {
                    updates.stage = Stage::Ready(v);
                }
                _ => {
                    let _ = std::fs::remove_file(&path);
                    updates.state.staged = None;
                    updates.save();
                }
            }
        }
        updates
    }

    fn save(&self) {
        if let Err(e) = self.state.save(&self.state_path) {
            eprintln!("tracewing: cannot keep what the updater found: {e}");
        }
    }

    /// The release offered, unless skipped or already running.
    pub fn offered(&self) -> Option<Version> {
        let s = &self.state;
        let v = Version::parse(s.offered.as_deref()?)?;
        (v > tracewing_update::current() && s.skipped.as_deref() != s.offered.as_deref())
            .then_some(v)
    }

    /// A look is due: never looked, or a day since.
    fn due(&self) -> bool {
        self.state
            .checked_at
            .is_none_or(|t| now_secs().saturating_sub(t) >= CHECK_EVERY.as_secs())
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// How this copy was installed, as told from where it runs.
fn install() -> Install {
    #[cfg(feature = "headless")]
    if let Some(how) = tracewing_update::test_install() {
        return how;
    }
    let exe = std::env::current_exe().unwrap_or_default();
    tracewing_update::install_of(&exe, |k| std::env::var(k).ok())
}

/// What updates this copy, in the language in use: names stay as they are,
/// the others are said.
fn by_said(by: &str) -> String {
    match by {
        "the system's package manager" => tr!("the system's package manager"),
        "the App Store" => tr!("the App Store"),
        "the Microsoft Store" => tr!("the Microsoft Store"),
        name => name.to_string(),
    }
}

/// The release a list offers, and its package's file and SHA-256.
type Found = (Option<Version>, Option<(String, String)>);

/// What a signed list offers this copy, and that release's package for it.
fn found_in(list: &[u8]) -> Result<Found, CheckError> {
    let offered = tracewing_update::offer(list, &tracewing_update::current())?;
    let package = match (&offered, install()) {
        (Some(v), Install::Package(tool)) => tracewing_update::package_file(list, v, tool)?,
        _ => None,
    };
    Ok((offered, package))
}

/// `job` on a runtime of its own, on the thread that calls this: the window
/// has none, and the engine's is the proxy's.
fn on_tokio<T>(
    job: impl std::future::Future<Output = Result<T, CheckError>>,
) -> Result<T, CheckError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CheckError::Io(e.to_string()))?
        .block_on(job)
}

/// "12.3 MB".
fn megabytes(n: u64) -> String {
    format!("{:.1} MB", n as f64 / 1_000_000.)
}

/// Said as a download goes: "12.3 MB of 45.6 MB".
fn download_said(got: u64, of: Option<u64>) -> String {
    match of {
        Some(of) => tr!(
            "Downloading… {got} of {total}",
            got = megabytes(got),
            total = megabytes(of)
        ),
        None => tr!("Downloading… {got}", got = megabytes(got)),
    }
}

impl Traffic {
    /// The looks to come: the first a moment after the window opens, then
    /// whenever one is due while `check_updates` is on.
    pub(crate) fn start_update_checks(cx: &mut Context<Self>) {
        // A build without release keys could believe no list: it does not look.
        if !tracewing_update::has_keys() {
            return;
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(FIRST_AFTER).await;
            loop {
                let go_on = this.update(cx, |this, cx| {
                    if this.engine.settings.check_updates && this.updates.due() {
                        this.check_updates(cx);
                    }
                });
                if go_on.is_err() {
                    break;
                }
                cx.background_executor().timer(WAKE_EVERY).await;
            }
        })
        .detach();
    }

    /// Reads the signed list now, and says what it offers.
    pub fn check_updates(&mut self, cx: &mut Context<Self>) {
        if self.updates.checking || !tracewing_update::has_keys() {
            return;
        }
        // A build for the window tests reaches only a list a test named.
        #[cfg(feature = "headless")]
        if !tracewing_update::has_test_source() {
            return;
        }
        self.updates.checking = true;
        cx.notify();
        let url = tracewing_update::list_url();
        let job = cx.background_spawn(async move {
            match on_tokio(tracewing_update::fetch(&url)) {
                Ok(list) => found_in(&list).map_err(|e| e.to_string()),
                // Before the first release: up to date.
                Err(CheckError::NoList) => Ok((None, None)),
                Err(e) => Err(e.to_string()),
            }
        });
        cx.spawn(async move |this, cx| {
            let found = job.await;
            let _ = this.update(cx, |this, cx| {
                let updates = &mut this.updates;
                updates.checking = false;
                match found {
                    Ok((offered, package)) => {
                        updates.problem = None;
                        updates.state.checked_at = Some(now_secs());
                        updates.state.offered = offered.map(|v| v.to_string());
                        updates.state.package = package;
                    }
                    Err(e) => updates.problem = Some(e),
                }
                updates.save();
                cx.notify();
            });
        })
        .detach();
    }

    /// The offered release: downloaded to install, when this copy can take
    /// it; else its page. A package's commands and what updates a managed
    /// copy are said in About, which is where this is asked from.
    pub fn get_update(&mut self, cx: &mut Context<Self>) {
        let Some(v) = self.updates.offered() else {
            return;
        };
        match install() {
            i if i.updates_itself() => self.download_update(v, cx),
            Install::Manual => cx.open_url(&tracewing_update::release_page(&v)),
            _ => {}
        }
    }

    /// The commands that install the offered release's package, when this
    /// copy came from one and the list names it.
    fn update_steps(&self) -> Option<Vec<String>> {
        let Install::Package(tool) = install() else {
            return None;
        };
        let v = self.updates.offered()?;
        let (file, sha256) = self.updates.state.package.as_ref()?;
        Some(tracewing_update::package_steps(&v, tool, file, sha256))
    }

    /// Downloads `v`'s file for this platform, as the signed list (read
    /// again, and checked again) names it.
    fn download_update(&mut self, v: Version, cx: &mut Context<Self>) {
        if matches!(self.updates.stage, Stage::Downloading(..) | Stage::Ready(_)) {
            return;
        }
        let dir = self.updates.staging.clone();
        let (got, total) = (Arc::new(AtomicU64::new(0)), Arc::new(AtomicU64::new(0)));
        let (job_got, job_total, wanted) = (got.clone(), total.clone(), v.clone());
        let job = cx.background_spawn(async move {
            on_tokio(async move {
                let list = tracewing_update::fetch(&tracewing_update::list_url()).await?;
                let file = tracewing_update::release_file(&list, &wanted)?;
                let dest = dir.join(&file.file);
                tracewing_update::download::download(&file.url, &file.sha256, &dest, |n, of| {
                    job_got.store(n, Ordering::Relaxed);
                    job_total.store(of.unwrap_or(0), Ordering::Relaxed);
                })
                .await?;
                Ok(dest)
            })
        });
        self.updates.stage = Stage::Downloading(0, None);
        cx.notify();
        let done = Arc::new(AtomicBool::new(false));
        // The progress, shown as it comes, until the download ends.
        let shown = done.clone();
        cx.spawn(async move |this, cx| {
            while !shown.load(Ordering::Relaxed) {
                let (n, of) = (got.load(Ordering::Relaxed), total.load(Ordering::Relaxed));
                let alive = this.update(cx, |this, cx| {
                    if matches!(this.updates.stage, Stage::Downloading(..)) {
                        this.updates.stage = Stage::Downloading(n, (of > 0).then_some(of));
                        cx.notify();
                    }
                });
                if alive.is_err() {
                    return;
                }
                cx.background_executor().timer(PROGRESS_EVERY).await;
            }
        })
        .detach();
        cx.spawn(async move |this, cx| {
            let downloaded = job.await;
            done.store(true, Ordering::Relaxed);
            let _ = this.update(cx, |this, cx| {
                let updates = &mut this.updates;
                match downloaded {
                    Ok(path) => {
                        updates.state.staged =
                            Some((v.to_string(), path.to_string_lossy().to_string()));
                        updates.save();
                        updates.stage = Stage::Ready(v);
                    }
                    Err(e) => updates.stage = Stage::Failed(e.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Quits to install the downloaded release, and starts it again.
    pub fn restart_to_update(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.updates.stage, Stage::Ready(_)) {
            return;
        }
        self.updates.relaunch = true;
        cx.quit();
    }

    /// As the app quits, after everything else it puts back: the downloaded
    /// release in place of this copy, and started again after a Restart.
    pub fn install_staged(&mut self) {
        let Stage::Ready(v) = &self.updates.stage else {
            return;
        };
        let Some((_, file)) = self.updates.state.staged.clone() else {
            return;
        };
        let how = install();
        match tracewing_update::apply::apply(&how, &PathBuf::from(&file), self.updates.relaunch) {
            Ok(()) => {
                // The installer on Windows runs once this app has gone: its
                // file stays until the next run sees the new version.
                if how != Install::WindowsInstaller {
                    let _ = std::fs::remove_file(&file);
                    self.updates.state.staged = None;
                }
            }
            Err(e) => {
                eprintln!("tracewing: cannot install tracewing {v}: {e}");
                // Not tried again on every quit: the next look offers it anew.
                let _ = std::fs::remove_file(&file);
                self.updates.state.staged = None;
            }
        }
        self.updates.save();
    }

    /// Not offered again until a newer one comes.
    pub fn skip_update(&mut self, cx: &mut Context<Self>) {
        self.updates.state.skipped = self.updates.state.offered.clone();
        self.updates.save();
        cx.notify();
    }

    /// Closes the row over the traffic until the next start.
    fn close_update_row(&mut self, v: Version, cx: &mut Context<Self>) {
        self.updates.row_closed = Some(v);
        cx.notify();
    }

    /// The row over the traffic: a release that is ready, or, while looking
    /// for updates is on, one that is out. Nothing while one downloads: About
    /// shows that, and it was asked for there.
    pub(crate) fn render_update_row(&self, cx: &mut Context<Self>) -> Option<gpui::Stateful<Div>> {
        let (version, ready) = match &self.updates.stage {
            Stage::Ready(v) => (v.clone(), true),
            Stage::Downloading(..) => return None,
            _ if self.engine.settings.check_updates => (self.updates.offered()?, false),
            _ => return None,
        };
        if self.updates.row_closed.as_ref() == Some(&version) {
            return None;
        }
        let c = colors();
        let said = version.to_string();
        let row = div()
            .id("update-row")
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .gap(ui(10.))
            .px(ui(14.))
            .py(ui(8.))
            .rounded(ui(16.))
            .bg(rgb(c.tonal))
            .text_color(rgb(c.on_tonal))
            .child(div().size(ui(8.)).rounded_full().bg(rgb(c.accent)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .font_weight(FontWeight::BOLD)
                    .child(if ready {
                        tr!("tracewing {version} is ready", version = said)
                    } else {
                        tr!("tracewing {version} is out", version = said)
                    }),
            );
        let row = if ready {
            row.child(
                button("update-row-restart", tr!("Restart now"), true)
                    .bg(rgb(c.accent))
                    .text_color(rgb(c.on_accent))
                    .on_click(cx.listener(|this, _, _, cx| this.restart_to_update(cx))),
            )
        } else {
            row.child(
                button("update-row-open", tr!("Update"), true)
                    .bg(rgb(c.accent))
                    .text_color(rgb(c.on_accent))
                    .on_click(cx.listener(|this, _, _, cx| this.show_about(cx))),
            )
            .child(
                div()
                    .id("update-row-skip")
                    .cursor_pointer()
                    .reachable()
                    .text_size(ui(12.))
                    .hover(|s| s.underline())
                    .on_click(cx.listener(|this, _, _, cx| this.skip_update(cx)))
                    .child(tr!("Skip this version")),
            )
        };
        Some(
            row.child(
                div()
                    .id("update-row-close")
                    .size(ui(24.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(ui(12.))
                    .cursor_pointer()
                    .reachable()
                    .on_click(
                        cx.listener(move |this, _, _, cx| {
                            this.close_update_row(version.clone(), cx)
                        }),
                    )
                    .child(icon("close", 12., c.on_tonal)),
            ),
        )
    }

    /// About's part of it: where this copy stands, and what to do.
    pub(crate) fn render_update_status(&self, cx: &mut Context<Self>) -> Div {
        let c = colors();
        let u = &self.updates;
        let how = install();
        let offered = u.offered();
        let line = |text: String| div().text_color(rgb(c.muted)).child(text);
        let mut says: Vec<Div> = Vec::new();
        let mut actions: Vec<gpui::Stateful<Div>> = Vec::new();
        if !tracewing_update::has_keys() {
            says.push(line(tr!(
                "This build cannot tell of updates: it was built without the release keys."
            )));
            return div().flex().flex_col().gap(ui(6.)).children(says);
        }
        match (&u.stage, &offered) {
            (Stage::Downloading(got, of), _) => says.push(line(download_said(*got, *of))),
            (Stage::Ready(v), _) => {
                says.push(line(tr!(
                    "tracewing {version} is ready: it is installed when you quit, or now with Restart now.",
                    version = v.to_string()
                )));
                actions.push(
                    button("update-restart", tr!("Restart now"), true)
                        .on_click(cx.listener(|this, _, _, cx| this.restart_to_update(cx))),
                );
            }
            (stage, Some(v)) => {
                says.push(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(tr!("tracewing {version} is out.", version = v.to_string())),
                );
                if let Stage::Failed(why) = stage {
                    says.push(div().text_color(rgb(c.error)).child(tr!(
                        "The download failed: {problem}",
                        problem = why.as_str()
                    )));
                }
                match &how {
                    i if i.updates_itself() => actions.push(
                        button("update-get", tr!("Download and install"), true)
                            .on_click(cx.listener(|this, _, _, cx| this.get_update(cx))),
                    ),
                    Install::Managed { by, command } => says.push(line(match command {
                        Some(command) => tr!(
                            "{by} updates this copy: {command}",
                            by = by_said(by),
                            command = *command
                        ),
                        None => tr!("{by} updates this copy.", by = by_said(by)),
                    })),
                    Install::Package(tool) => match self.update_steps() {
                        Some(steps) => {
                            let joined = steps.join(" &&\n");
                            says.push(line(tr!(
                                "Install it with {tool}, in a terminal:",
                                tool = tool.name()
                            )));
                            says.push(
                                div()
                                    .p(ui(10.))
                                    .rounded(ui(10.))
                                    .bg(rgb(c.page))
                                    .font_family(crate::mono_font())
                                    .text_size(ui(12.))
                                    .child(joined.clone()),
                            );
                            let one_line = steps.join(" && ");
                            actions.push(
                                button("update-copy", tr!("Copy the commands"), true).on_click(
                                    move |_, _, cx| {
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            one_line.clone(),
                                        ))
                                    },
                                ),
                            );
                        }
                        None => actions.push(open_release_page(v)),
                    },
                    _ => actions.push(open_release_page(v)),
                }
                actions.push(
                    button("update-skip", tr!("Skip this version"), false)
                        .on_click(cx.listener(|this, _, _, cx| this.skip_update(cx))),
                );
            }
            (_, None) => {
                if let Some(problem) = &u.problem {
                    says.push(div().text_color(rgb(c.error)).child(tr!(
                        "Could not look for a new version: {problem}",
                        problem = problem.as_str()
                    )));
                } else if u.checking {
                    says.push(line(tr!("Looking for a new version…")));
                } else if let Some(at) = u.state.checked_at {
                    let at = chrono::DateTime::<chrono::Local>::from(
                        UNIX_EPOCH + Duration::from_secs(at),
                    );
                    says.push(line(tr!(
                        "Up to date. Last looked {when}.",
                        when = at.format("%Y-%m-%d %H:%M").to_string()
                    )));
                } else {
                    says.push(line(tr!("Not looked for a new version yet.")));
                }
            }
        }
        if !matches!(u.stage, Stage::Downloading(..) | Stage::Ready(_)) {
            actions.push(
                button("update-check", tr!("Check now"), false)
                    .on_click(cx.listener(|this, _, _, cx| this.check_updates(cx))),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap(ui(8.))
            .children(says)
            .child(div().flex().flex_wrap().gap(ui(8.)).children(actions))
    }
}

/// The button that opens `v`'s page, for a copy the app cannot replace.
fn open_release_page(v: &Version) -> gpui::Stateful<Div> {
    let page = tracewing_update::release_page(v);
    button("update-page", tr!("Open the release page"), true)
        .on_click(move |_, _, cx| cx.open_url(&page))
}
