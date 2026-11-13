//! The sessions on disk: open one again, give it a name to keep it, delete it. An
//! old session opens in the Traffic screen, read-only, with recording going on
//! behind it, and what its search index had not reached is indexed as it opens.

use crate::text_size::ui;
use crate::{
    colors::colors,
    files::FileState,
    flows::{group_digits, size_text},
    input::{Changed, TextInput},
    traffic::{Screen, Traffic},
    widgets::Reachable,
};
use chrono::{Local, TimeZone};
use gpui::{AppContext as _, Context, Entity, FontWeight, SharedString, div, prelude::*, rgb};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tracewing_i18n::{tr, tr_n};
use tracewing_store::session::{self, SessionInfo, Trouble};

/// `1 Nov 2026, 14:05`, in the computer's time zone.
pub fn when_text(micros: Option<i64>) -> String {
    micros
        .and_then(|us| Local.timestamp_micros(us).single())
        .map_or_else(
            || tr!("No flows"),
            |at| at.format("%-d %b %Y, %H:%M").to_string(),
        )
}

#[derive(Default)]
pub struct SessionsPage {
    pub sessions: Vec<SessionInfo>,
    names: HashMap<PathBuf, Entity<TextInput>>,
    /// Asked once to delete this one; a second press does it.
    pub deleting: Option<PathBuf>,
    /// The session whose pages are being read through, while they are.
    pub checking: Option<PathBuf>,
    /// What the last check said, by the session it was asked about.
    pub checked: HashMap<PathBuf, SharedString>,
    pub problem: Option<String>,
    loading: bool,
    pub files: FileState,
}

impl Traffic {
    pub(crate) fn sessions_dir(&self) -> PathBuf {
        self.data_dir.join("sessions")
    }

    /// Reads them off the disk, which adds up sizes, so not on the window's thread.
    pub fn refresh_sessions(&mut self, cx: &mut Context<Self>) {
        if self.sessions_page.loading {
            return;
        }
        self.sessions_page.loading = true;
        let dir = self.sessions_dir();
        cx.spawn(async move |this, cx| {
            let sessions = cx
                .background_spawn(async move { session::list(&dir) })
                .await;
            let _ = this.update(cx, |this, cx| this.show_sessions(sessions, cx));
        })
        .detach();
    }

    fn show_sessions(&mut self, sessions: Vec<SessionInfo>, cx: &mut Context<Self>) {
        for about in &sessions {
            if self.sessions_page.names.contains_key(&about.dir) {
                continue;
            }
            let text = about.name.clone().unwrap_or_default();
            let input = cx.new(|cx| {
                let mut input = TextInput::said(|| tr!("Name it to keep it"), cx);
                input.set_text(&text, text.len()..text.len(), cx);
                input
            });
            let dir = about.dir.clone();
            cx.subscribe(&input, move |this, input, _: &Changed, cx| {
                if let Err(e) = session::set_name(&dir, input.read(cx).text()) {
                    this.sessions_page.problem = Some(format!("{e:#}"));
                    cx.notify();
                }
            })
            .detach();
            self.sessions_page.names.insert(about.dir.clone(), input);
        }
        self.sessions_page.sessions = sessions;
        self.sessions_page.loading = false;
        cx.notify();
    }

    /// Reads every page of a session's database and says what SQLite makes
    /// of it. Asked for, never run by itself: a session of a million flows
    /// is more than a gigabyte, and this screen lists every session there is.
    pub fn check_session(&mut self, dir: &Path, cx: &mut Context<Self>) {
        if self.sessions_page.checking.is_some() {
            return;
        }
        self.sessions_page.checking = Some(dir.to_owned());
        self.sessions_page.checked.remove(dir);
        let path = dir.to_owned();
        cx.spawn(async move |this, cx| {
            let dir = path.clone();
            let said = cx
                .background_spawn(async move { session::check(&dir) })
                .await;
            let text: SharedString = match said {
                Ok(lines) if lines.is_empty() => tr!("Nothing wrong with it.").into(),
                Ok(lines) => {
                    // A row has space for one of them. Cut, because SQLite's
                    // own lines run long, and the page knows the rest.
                    let first: String = lines[0].chars().take(90).collect();
                    let first = match first.len() < lines[0].len() {
                        true => format!("{first}…"),
                        false => first,
                    };
                    tr_n!(
                        lines.len(),
                        "SQLite found {count} problem: {first}",
                        "SQLite found {count} problems, the first: {first}",
                        count = lines.len(),
                        first = first
                    )
                    .into()
                }
                Err(e) => format!("{e:#}").into(),
            };
            let _ = this.update(cx, |this, cx| {
                this.sessions_page.checking = None;
                this.sessions_page.checked.insert(path, text);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub fn delete_session(&mut self, dir: &Path, cx: &mut Context<Self>) {
        if self.is_in_use(dir) {
            return;
        }
        if self.sessions_page.deleting.as_deref() != Some(dir) {
            self.sessions_page.deleting = Some(dir.to_owned());
            cx.notify();
            return;
        }
        self.sessions_page.deleting = None;
        self.sessions_page.problem = session::delete(dir).err().map(|e| format!("{e:#}"));
        self.sessions_page.names.remove(dir);
        self.sessions_page.loading = false;
        self.refresh_sessions(cx);
    }

    fn is_in_use(&self, dir: &Path) -> bool {
        dir == self.engine.session || self.viewing.as_deref() == Some(dir)
    }

    /// Names a session from a test, as typing in its field does.
    #[doc(hidden)]
    pub fn name_session(&mut self, dir: &Path, name: &str, cx: &mut Context<Self>) {
        if let Some(input) = self.sessions_page.names.get(dir) {
            input.update(cx, |input, cx| {
                input.set_text(name, name.len()..name.len(), cx);
                cx.emit(Changed);
            });
        }
    }

    pub(crate) fn render_sessions(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let kept = self.engine.settings.sessions_kept;
        let page = &self.sessions_page;
        let rows: Vec<_> = page
            .sessions
            .iter()
            .map(|about| {
                let dir = about.dir.clone();
                let live = dir == self.engine.session;
                let viewing = self.viewing.as_deref() == Some(dir.as_path())
                    || (live && self.viewing.is_none());
                let in_use = live || viewing;
                let asked = page.deleting.as_deref() == Some(dir.as_path());
                let facts: SharedString = if let Some(trouble) = &about.trouble {
                    let size = size_text(about.bytes);
                    match trouble {
                        Trouble::OtherVersion(_) => tr!(
                            "{size} · written by another version, so it cannot be opened",
                            size = size
                        ),
                        Trouble::Damaged(_) => {
                            tr!("{size} · damaged, so it cannot be opened", size = size)
                        }
                        Trouble::Unreadable(why) => tr!(
                            "{size} · cannot be read: {why}",
                            size = size,
                            why = why.clone()
                        ),
                    }
                    .into()
                } else {
                    tr_n!(
                        about.flows,
                        "{count} flow · {size}",
                        "{count} flows · {size}",
                        count = group_digits(about.flows),
                        size = size_text(about.bytes)
                    )
                    .into()
                };
                let (open_dir, delete_dir, export_dir) = (dir.clone(), dir.clone(), dir.clone());
                let check_dir = dir.clone();
                let busy = page.checking.as_deref() == Some(dir.as_path());
                let said = page.checked.get(&dir).cloned();
                let button = |id: (&'static str, u64), label: String, loud: bool| {
                    div()
                        .id(id)
                        .flex_shrink_0()
                        .h(ui(32.))
                        .px(ui(14.))
                        .rounded(ui(16.))
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .reachable()
                        .font_weight(FontWeight::BOLD)
                        .text_size(ui(12.5))
                        .when(loud, |d| d.bg(rgb(c.error_ground)).text_color(rgb(c.error)))
                        .when(!loud, |d| d.bg(rgb(c.chip)).text_color(rgb(c.on_chip)))
                        .child(label)
                };
                let key = fingerprint(&dir);
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap(ui(14.))
                    .px(ui(16.))
                    .py(ui(12.))
                    .rounded(ui(18.))
                    .bg(rgb(c.surface))
                    .child(
                        div()
                            .w(ui(240.))
                            .flex_shrink_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .font_weight(FontWeight::BOLD)
                                    .child(when_text(about.first_us)),
                            )
                            .child(
                                div()
                                    .text_size(ui(12.))
                                    .text_color(rgb(c.muted))
                                    .child(facts),
                            )
                            .children(said.map(|said| {
                                div().text_size(ui(12.)).text_color(rgb(c.text)).child(said)
                            })),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(ui(160.))
                            .h(ui(32.))
                            .px(ui(10.))
                            .rounded(ui(12.))
                            .flex()
                            .items_center()
                            .bg(rgb(c.input))
                            .children(page.names.get(&dir).cloned()),
                    )
                    .when(live, |d| {
                        d.child(
                            div()
                                .flex_shrink_0()
                                .text_size(ui(12.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(c.ok))
                                .child(tr!("Recording now")),
                        )
                    })
                    .when(about.readable && !viewing, |d| {
                        d.child(button(("session-open", key), tr!("Open"), false).on_click(
                            cx.listener(move |this, _, _, cx| {
                                this.open_session(&open_dir, cx);
                                this.set_screen(Screen::Traffic, cx);
                            }),
                        ))
                    })
                    .when(about.readable && about.flows > 0, |d| {
                        d.child(
                            button(("session-export", key), tr!("Export HAR"), false).on_click(
                                cx.listener(move |this, _, _, cx| {
                                    this.ask_export_session(export_dir.clone(), cx);
                                }),
                            ),
                        )
                    })
                    .child(
                        button(
                            ("session-check", key),
                            if busy {
                                tr!("Checking…")
                            } else {
                                tr!("Check")
                            },
                            false,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.check_session(&check_dir, cx);
                        })),
                    )
                    .when(!in_use, |d| {
                        d.child(
                            button(
                                ("session-delete", key),
                                if asked {
                                    tr!("Delete for good?")
                                } else {
                                    tr!("Delete")
                                },
                                asked,
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.delete_session(&delete_dir, cx);
                                },
                            )),
                        )
                    })
            })
            .collect();
        div()
            .id("sessions")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(ui(8.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .pb(ui(4.))
                    .child(
                        div()
                            .text_size(ui(20.))
                            .font_weight(FontWeight::BOLD)
                            .child(tr!("Sessions")),
                    )
                    .child(div().text_color(rgb(c.muted)).child(tr!(
                        "A session with a name is kept until you delete it. Of the others, the newest {kept} stay.",
                        kept = kept
                    ))),
            )
            .child(self.render_har_bar(cx))
            .children(
                page.problem
                    .clone()
                    .map(|problem| div().text_color(rgb(c.error)).child(problem)),
            )
            .children(rows)
    }
}

impl Traffic {
    /// Bringing a file from another tool in, and how a HAR goes out.
    fn render_har_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let har = &self.sessions_page.files;
        let without = har.without_secrets;
        div()
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .gap(ui(12.))
            .pb(ui(4.))
            .child(
                div()
                    .id("import")
                    .flex_shrink_0()
                    .h(ui(32.))
                    .px(ui(14.))
                    .rounded(ui(16.))
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .reachable()
                    .font_weight(FontWeight::BOLD)
                    .text_size(ui(12.5))
                    .bg(rgb(c.accent))
                    .text_color(rgb(c.on_accent))
                    .child(if har.busy {
                        tr!("Working…")
                    } else {
                        tr!("Open a session file")
                    })
                    .on_click(cx.listener(|this, _, _, cx| this.ask_import(cx))),
            )
            .child(
                div()
                    .text_size(ui(12.))
                    .text_color(rgb(c.muted))
                    .child(tr!("HAR, Fiddler .saz, or Charles .chlsj")),
            )
            .child(
                div()
                    .id("har-secrets")
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(ui(8.))
                    .cursor_pointer()
                    .reachable()
                    .child(
                        div()
                            .size(ui(16.))
                            .rounded(ui(4.))
                            .border_2()
                            .border_color(rgb(if without { c.accent } else { c.muted }))
                            .when(without, |d| d.bg(rgb(c.accent))),
                    )
                    .child(tr!(
                        "Exported HAR files leave out cookies, tokens, keys and passwords, in \
                         headers, URLs and bodies"
                    ))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_har_without_secrets(!without, cx)
                    })),
            )
            .children(har.note.clone().map(|(note, failed)| {
                div()
                    .text_size(ui(12.))
                    .text_color(rgb(if failed { c.error } else { c.muted }))
                    .child(note)
            }))
    }
}

/// A number for a session's buttons: elements want an id, and paths are not numbers.
fn fingerprint(dir: &Path) -> u64 {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    dir.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_session_with_no_flows_says_so() {
        assert_eq!(when_text(None), "No flows");
        // The year is enough: this is the computer's time zone.
        assert!(when_text(Some(1_793_000_000_000_000)).contains("2026"));
    }
}
