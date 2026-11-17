//! The local way in, from the window: the programs somebody picked for
//! tracewing to keep when this machine's traffic comes through its own
//! adapter.
//!
//! **There is no capture-everything mode, by decision.** Windows and Linux
//! capture by *route*, so the operating system hands tracewing every
//! connection whatever its destination -- but what is kept is always a
//! filter, and the filter is these programs. The rest is spliced straight
//! through and never recorded, which is both what keeps the volume sane and
//! what keeps tracewing's certificate away from programs that pin their own.
//! It is also what makes this way in the same promise as the macOS one:
//! choose the programs, see those programs.
//!
//! The programs are kept in `local-programs.toml`. **Whether the switch is
//! on is kept too**, in the settings, and that is the one place this parts
//! company with [`crate::mac_apps`], which deliberately starts off every run
//! so that opening tracewing installs nothing. Here it has to persist: a way
//! in whose source is assembled as the engine starts cannot be added to a
//! running one, and on Windows switching on means coming back as an
//! administrator -- a new process, which has nothing but the settings to
//! learn from.
//!
//! What the sieve matches on is the **path**, canonicalised by
//! [`tracewing_proxy::sieve`] rather than here: it has the rules for a
//! `\\?\` prefix, for case on Windows, and for a folder stding for every
//! program under it, and they are tested there against both platforms'
//! spellings.

use crate::{colors::colors, text_size::ui, widgets::eyebrow};
use anyhow::{Context as _, Result};
use gpui::{Context, Div, FontWeight, div, prelude::*, rgb};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracewing_i18n::tr;

/// Where the picked programs are kept, in the data directory.
pub const FILE: &str = "local-programs.toml";

/// What this machine can do about the local way in, which the tab shows
/// instead of a switch that could not work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Support {
    /// The switch does what it says.
    Ready,
    /// This system captures by route, and tracewing is not an administrator.
    /// Switching on means coming back as one, which is a **new session**, and
    /// the window has to say so before it asks rather than afterwards.
    NeedsAdministrator,
    /// Nothing here captures by route. On macOS that is not a gap: the apps
    /// way in is the network extension, which is a different tab.
    NotOnThisSystem,
}

impl Support {
    /// Which of the three, for a machine that captures by route or does not,
    /// and a tracewing that is elevated or is not.
    ///
    /// A plain function over two bits, so that every answer is reachable from
    /// a test on any machine -- the same reason
    /// [`tracewing_system::child::start_at`] is one. The single line no test
    /// can settle is which bits *this* machine has, and that is [`Self::here`].
    pub fn of(by_route: bool, elevated: bool) -> Self {
        match (by_route, elevated) {
            (false, _) => Self::NotOnThisSystem,
            (true, true) => Self::Ready,
            (true, false) => Self::NeedsAdministrator,
        }
    }

    /// This machine's answer.
    ///
    /// Windows only for now. Linux captures by route as well and its half is
    /// being built beside this one; when it lands, this is the line that
    /// changes, and `of` above is where the rest of the behaviour already is.
    pub fn here() -> Self {
        Self::of(cfg!(windows), tracewing_system::child::elevated())
    }
}

/// A program as it was picked: what to call it, and where it is.
///
/// No signing ids, unlike the macOS way in: nothing here asks the operating
/// system who made a program, because the question this way in answers is
/// "which socket belongs to which program", and the answer comes from the
/// path of the running program itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Picked {
    pub name: String,
    pub path: PathBuf,
}

impl Picked {
    /// A program picked at `path`, named by its file name where it has one.
    ///
    /// A folder keeps its own name, since it stands for every program under
    /// it and a folder named for one of them would read as that program.
    pub fn at(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        Self { name, path }
    }
}

/// This way in's own state: whether it is on, and what it keeps.
///
/// **The switch is kept here rather than in the settings**, with the programs
/// it belongs to, for the same reason the macOS way in keeps its apps in a
/// file of its own: it is the state of one way in rather than a preference
/// about how traffic is stored. It also leaves `every_setting_has_a_row`
/// saying what it says -- that nothing in the settings is unreachable from
/// Settings -- without a third named exception.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Local {
    /// Whether this machine's traffic is to come through tracewing's adapter.
    ///
    /// Unlike the macOS switch, which starts off every run on purpose, this
    /// one persists: the source is assembled as the engine starts and cannot
    /// be added to a running one, and on Windows switching on means coming
    /// back as an administrator -- a new process, with nothing but this file
    /// to learn from.
    pub on: bool,
    #[serde(rename = "program")]
    pub programs: Vec<Picked>,
}

/// What is kept in `dir`: off, with no programs, where the file is missing or
/// unreadable.
///
/// Unreadable gives the off state rather than an error, and that is the
/// direction that matters: a file somebody cannot parse would otherwise leave
/// a machine capturing whatever it held last, or capturing with a list nobody
/// can see.
pub fn load(dir: &Path) -> Local {
    std::fs::read_to_string(dir.join(FILE))
        .ok()
        .and_then(|text| toml::from_str::<Local>(&text).ok())
        .unwrap_or_default()
}

/// Writes it, through a temporary file so that a half-written list is never
/// the list: the same reason [`crate::mac_apps::save`] does.
pub fn save(dir: &Path, local: &Local) -> Result<()> {
    let text = toml::to_string(local)?;
    std::fs::create_dir_all(dir)?;
    let path = dir.join(FILE);
    let new = path.with_extension("toml.new");
    std::fs::write(&new, text).with_context(|| format!("write {}", new.display()))?;
    std::fs::rename(&new, &path).with_context(|| format!("write {}", path.display()))
}

/// What the sieve is given: each path as it was picked, once.
///
/// As written rather than canonicalised, because the canonicalising is the
/// sieve's and it does it on both sides of the comparison. Doing it here as
/// well would mean two spellings of the same rule, and the one that is
/// tested against Windows' own spellings is not this one.
pub fn programs_of(picked: &[Picked]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for path in picked.iter().map(|program| program.path.to_string_lossy()) {
        let path = path.into_owned();
        if !out.contains(&path) {
            out.push(path);
        }
    }
    out
}

/// The local way in as the window holds it.
pub struct Shown {
    pub local: Local,
    /// What this machine can do about it.
    ///
    /// **Held rather than asked for per frame**, and that is what lets a test
    /// draw the states this machine is not in. A tab assembled per answer
    /// inherits text written while another answer was on screen, and the only
    /// way to catch that is to draw every answer and look -- which is
    /// impossible if the answer is a `cfg!` inside the drawing.
    pub support: Support,
    /// What went wrong turning it on or writing the list, for the tab.
    pub problem: Option<String>,
}

impl Shown {
    pub fn load(dir: &std::path::Path) -> Self {
        Self {
            local: load(dir),
            support: Support::here(),
            problem: None,
        }
    }
}

impl crate::traffic::Traffic {
    /// The local way in's tab: what it can do here, what it keeps, and the
    /// switch.
    ///
    /// Every state is drawn from `self.local.support` and the engine's own
    /// answer rather than from a `cfg!`, so that `tests/local_tab.rs` can
    /// take a picture of each one from any machine. A tab assembled per
    /// answer otherwise carries text written while another answer was on
    /// screen -- correct text, about the wrong page.
    pub(crate) fn render_local_apps(&mut self, cx: &mut Context<Self>) -> Div {
        let c = colors();
        let on = self.local.local.on;
        let running = self.engine.local.is_some();
        let nothing = self.local.local.programs.is_empty();
        let says = match (self.local.support, on, running) {
            (Support::NotOnThisSystem, _, _) => tr!(
                "Not on this system. tracewing takes a chosen program's traffic by route on Windows; on macOS that is the network extension, which has a tab of its own."
            ),
            (Support::NeedsAdministrator, _, _) => tr!(
                "Off. Taking this machine's traffic needs an administrator: switching it on starts tracewing again as one, and that starts a new session — the one open now stays on disk and can be reopened from the list."
            ),
            (Support::Ready, false, _) => tr!(
                "Off. Switching it on takes effect when tracewing starts again, and that starts a new session — the one open now stays on disk and can be reopened from the list."
            ),
            // On in the file and not in this run: the way in is made as the
            // engine starts, so saying "on" alone would be a switch that
            // looks like it is taking traffic when nothing is.
            (Support::Ready, true, false) => tr!(
                "On from the next start. Nothing is being taken in this session: the way in is made as tracewing starts."
            ),
            (Support::Ready, true, true) if nothing => tr!(
                "On, and keeping nothing: this machine's traffic goes through tracewing's adapter and straight out again until a program is added below."
            ),
            (Support::Ready, true, true) => tr!(
                "On: the connections of the programs below come to tracewing, whatever their proxy settings say. Everything else goes straight through, unrecorded."
            ),
        };
        let switch = (self.local.support != Support::NotOnThisSystem).then(|| {
            let label = match (on, self.local.support) {
                (true, _) => tr!("Turn off"),
                (false, Support::NeedsAdministrator) => tr!("Turn on as administrator…"),
                (false, _) => tr!("Turn on"),
            };
            crate::setup_page::button("local-switch", label, !on)
                .on_click(cx.listener(|this, _, _, cx| this.toggle_local(cx)))
        });
        let rows: Vec<_> = self
            .local
            .local
            .programs
            .iter()
            .enumerate()
            .map(|(index, program)| {
                let path = program.path.display().to_string();
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
                            .child(
                                div()
                                    .font_weight(FontWeight::BOLD)
                                    .child(program.name.clone()),
                            )
                            .child(
                                div()
                                    .text_size(ui(12.))
                                    .text_color(rgb(c.muted))
                                    .font_family(crate::mono_font())
                                    .child(path),
                            ),
                    )
                    .child(
                        crate::mac_apps::small_button(("local-remove", index), tr!("Remove"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.remove_local_program(index, cx)
                            })),
                    )
            })
            .collect();
        let chooses = self.local.support != Support::NotOnThisSystem;
        // What went wrong, said once. The engine's answer is a case, with a
        // sentence of its own per case; two of them are already the switch's
        // line above, so saying them again here would be the same fact
        // twice.
        let problem =
            self.local
                .problem
                .clone()
                .or_else(|| match self.engine.local_problem.as_ref()? {
                    crate::local_source::NoSource::NotHere
                    | crate::local_source::NoSource::NeedsAdministrator => None,
                    crate::local_source::NoSource::Refused => Some(tr!(
                        "Administrator rights were not given, so nothing was turned on."
                    )),
                    crate::local_source::NoSource::Failed(why) => Some(why.clone()),
                });
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
                    .child(div().flex_1().min_w(ui(240.)).child(says)),
            )
            .when(chooses, |d| {
                d.child(eyebrow(tr!("Programs it keeps")))
                    .when(nothing, |d| {
                        d.child(div().text_color(rgb(c.muted)).child(tr!(
                            "None yet. This way in takes every connection on the machine and keeps only what is listed here — there is no capture-everything mode, by design: the rest is spliced straight through and never recorded."
                        )))
                    })
                    .children(rows)
                    .child(
                        crate::mac_apps::small_button("local-add", tr!("Add a program…")).on_click(
                            cx.listener(|this, _, _, cx| this.ask_add_local_programs(cx)),
                        ),
                    )
            })
            .children(problem.map(|problem| div().text_color(rgb(c.error)).child(problem)))
    }

    /// Turns it on or off.
    ///
    /// **Off takes effect now**, which is what was decided: the device's read
    /// is ended and the adapter, its address and its routes go with it. **On
    /// takes a new engine**, because the proxy's sources are gathered as it
    /// starts -- so on Windows without an administrator it is the relaunch,
    /// and with one it is the next start, which the tab says.
    pub fn toggle_local(&mut self, cx: &mut Context<Self>) {
        self.local.problem = None;
        let want = !self.local.local.on;
        if !want && let Some(running) = &self.engine.local {
            running.stopper.stop();
        }
        self.local.local.on = want;
        if let Err(e) = save(&self.data_dir, &self.local.local) {
            self.local.problem = Some(format!("{e:#}"));
            self.local.local.on = !want;
            cx.notify();
            return;
        }
        if want && self.local.support == Support::NeedsAdministrator {
            match tracewing_system::child::relaunch_as_administrator() {
                // It started; this one's turn to go, so that two copies do
                // not race for the proxy's ports.
                Ok(tracewing_system::child::Raised::Started) => cx.quit(),
                // Refused is an answer, not a fault: nothing started and
                // nothing changed, so the switch goes back off rather than
                // looking on with nothing behind it.
                Ok(tracewing_system::child::Raised::Refused) => {
                    self.local.local.on = false;
                    let _ = save(&self.data_dir, &self.local.local);
                }
                Err(e) => {
                    self.local.problem = Some(format!("{e:#}"));
                    self.local.local.on = false;
                    let _ = save(&self.data_dir, &self.local.local);
                }
            }
        }
        cx.notify();
    }

    /// Asks for programs to add, and adds them.
    pub fn ask_add_local_programs(&mut self, cx: &mut Context<Self>) {
        let asked = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: true,
            multiple: true,
            prompt: Some(tr!("Add").into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = asked.await {
                let _ = this.update(cx, |this, cx| this.add_local_programs(paths, cx));
            }
        })
        .detach();
    }

    fn add_local_programs(&mut self, paths: Vec<std::path::PathBuf>, cx: &mut Context<Self>) {
        let mut programs = self.local.local.programs.clone();
        for path in paths {
            if !programs.iter().any(|program| program.path == path) {
                programs.push(Picked::at(path));
            }
        }
        self.set_local_programs(programs, cx);
    }

    pub fn remove_local_program(&mut self, index: usize, cx: &mut Context<Self>) {
        let mut programs = self.local.local.programs.clone();
        if index < programs.len() {
            programs.remove(index);
            self.set_local_programs(programs, cx);
        }
    }

    /// Keeps them, and has the running capture keep them too.
    ///
    /// The running one is told through `Chosen`, which is the same list the
    /// sieve reads: no restart, and no rebuilding the source, which is why
    /// the sieve takes the list by handle rather than by value.
    fn set_local_programs(&mut self, programs: Vec<Picked>, cx: &mut Context<Self>) {
        self.local.problem = None;
        if let Err(e) = save(
            &self.data_dir,
            &Local {
                on: self.local.local.on,
                programs: programs.clone(),
            },
        ) {
            self.local.problem = Some(format!("{e:#}"));
            cx.notify();
            return;
        }
        if let Some(running) = &self.engine.local {
            running.chosen.these(programs_of(&programs));
        }
        self.local.local.programs = programs;
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One directory per test, named for it: these run in parallel threads of
    /// one process, so a directory named only for the process is one they take
    /// from each other -- which is how this was found, a test removing the
    /// file another had just written. Three runs said FAILED, ok, FAILED.
    fn dir(which: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("tracewing-local-{}-{which}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// All four inputs, including the two no machine here can be in, so that
    /// the tab's three states are settled by a test rather than by whichever
    /// machine somebody happens to run it on.
    #[test]
    fn what_a_machine_can_do_about_the_local_way_in() {
        assert_eq!(Support::of(true, true), Support::Ready);
        assert_eq!(
            Support::of(true, false),
            Support::NeedsAdministrator,
            "it could, once tracewing comes back as one"
        );
        assert_eq!(
            Support::of(false, false),
            Support::NotOnThisSystem,
            "and where nothing captures by route, elevation changes nothing"
        );
        assert_eq!(Support::of(false, true), Support::NotOnThisSystem);
        // What this machine is, which is the one line `of` cannot settle.
        match cfg!(windows) {
            false => assert_eq!(Support::here(), Support::NotOnThisSystem),
            // Elevated or not, it is one of the two that captures by route.
            true => assert_ne!(Support::here(), Support::NotOnThisSystem),
        }
    }

    #[test]
    fn the_switch_and_the_programs_survive_being_written_and_read() {
        let dir = dir("round-trip");
        let local = Local {
            on: true,
            programs: vec![
                Picked::at(PathBuf::from(r"C:\Program Files\nodejs\node.exe")),
                Picked::at(PathBuf::from("/usr/bin/curl")),
            ],
        };
        save(&dir, &local).unwrap();
        assert_eq!(
            load(&dir),
            local,
            "the switch, a path with a space in it, and one of each platform's spelling"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The name is read by **this** platform's path rules, which is right
    /// because a program is picked on the machine it runs on: `std::path`
    /// treats `\` as a separator on Windows and as an ordinary character
    /// elsewhere, so the Windows path below has no file name on Linux.
    ///
    /// Deliberately not the sieve's behaviour, and the difference is worth
    /// knowing: `sieve::plain` understands **both** spellings on purpose,
    /// because its rules are tested from Linux against what Windows does.
    /// A display name has no such second reader.
    #[test]
    fn a_program_is_named_by_this_platforms_rules() {
        assert_eq!(Picked::at(PathBuf::from("/usr/bin/curl")).name, "curl");
        let windows = Picked::at(PathBuf::from(r"C:\Program Files\nodejs\node.exe"));
        match cfg!(windows) {
            true => assert_eq!(windows.name, "node.exe"),
            false => assert_eq!(
                windows.name, r"C:\Program Files\nodejs\node.exe",
                "no separator here, so the whole path is the name"
            ),
        }
        // A folder keeps its own name: it stands for every program under it,
        // and a folder named for one of them would read as that program.
        assert_eq!(Picked::at(PathBuf::from("/opt/mine")).name, "mine");
    }

    /// Nothing keeps nothing. A list that cannot be read must not leave the
    /// machine capturing what it held last.
    #[test]
    fn a_list_that_cannot_be_read_is_no_programs() {
        let dir = dir("unreadable");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(FILE), "on = true\nthis is not toml {{{").unwrap();
        assert_eq!(
            load(&dir),
            Local::default(),
            "off, although the unreadable file says on"
        );
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(load(&dir), Local::default(), "and so is a missing file");
    }

    #[test]
    fn the_sieve_is_given_each_path_once_as_it_was_picked() {
        let picked = vec![
            Picked::at(PathBuf::from("/usr/bin/curl")),
            Picked::at(PathBuf::from("/opt/my build/program")),
            Picked::at(PathBuf::from("/usr/bin/curl")),
        ];
        assert_eq!(
            programs_of(&picked),
            ["/usr/bin/curl", "/opt/my build/program"],
            "a space is left in it, and the repeat is not sent twice"
        );
    }
}
