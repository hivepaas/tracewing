//! The sessions on disk: open one again, give it a name to keep it, delete it. An
//! old session opens in the Traffic screen, read-only, with recording going on
//! behind it, and what its search index had not reached is indexed as it opens.

use crate::{
    colors::colors,
    flows::{group_digits, size_text},
    input::{Changed, TextInput},
    traffic::{Screen, Traffic},
};
use chrono::{Local, TimeZone};
use gpui::{AppContext as _, Context, Entity, FontWeight, SharedString, div, prelude::*, px, rgb};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tracewing_store::session::{self, SessionInfo};

/// `1 Nov 2026, 14:05`, in the computer's time zone.
pub fn when_text(micros: Option<i64>) -> String {
    micros
        .and_then(|us| Local.timestamp_micros(us).single())
        .map_or_else(
            || "No flows".to_owned(),
            |at| at.format("%-d %b %Y, %H:%M").to_string(),
        )
}

#[derive(Default)]
pub struct SessionsPage {
    pub sessions: Vec<SessionInfo>,
    names: HashMap<PathBuf, Entity<TextInput>>,
    /// Asked once to delete this one; a second press does it.
    pub deleting: Option<PathBuf>,
    pub problem: Option<String>,
    loading: bool,
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
                let mut input = TextInput::new("Name it to keep it", cx);
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
        let rows: Vec<_> =
            page.sessions
                .iter()
                .map(|about| {
                    let dir = about.dir.clone();
                    let live = dir == self.engine.session;
                    let viewing = self.viewing.as_deref() == Some(dir.as_path())
                        || (live && self.viewing.is_none());
                    let in_use = live || viewing;
                    let asked = page.deleting.as_deref() == Some(dir.as_path());
                    let facts: SharedString = if !about.readable {
                        format!(
                            "{} · written by another version, so it cannot be opened",
                            size_text(about.bytes)
                        )
                        .into()
                    } else {
                        format!(
                            "{} flows · {}",
                            group_digits(about.flows),
                            size_text(about.bytes)
                        )
                        .into()
                    };
                    let (open_dir, delete_dir) = (dir.clone(), dir.clone());
                    let button = |id: (&'static str, u64), label: &'static str, loud: bool| {
                        div()
                            .id(id)
                            .flex_shrink_0()
                            .h(px(32.))
                            .px(px(14.))
                            .rounded(px(16.))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .font_weight(FontWeight::BOLD)
                            .text_size(px(12.5))
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
                        .gap(px(14.))
                        .px(px(16.))
                        .py(px(12.))
                        .rounded(px(18.))
                        .bg(rgb(c.surface))
                        .child(
                            div()
                                .w(px(240.))
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
                                        .text_size(px(12.))
                                        .text_color(rgb(c.muted))
                                        .child(facts),
                                ),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(160.))
                                .h(px(32.))
                                .px(px(10.))
                                .rounded(px(12.))
                                .flex()
                                .items_center()
                                .bg(rgb(c.input))
                                .children(page.names.get(&dir).cloned()),
                        )
                        .when(live, |d| {
                            d.child(
                                div()
                                    .flex_shrink_0()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(c.ok))
                                    .child("Recording now"),
                            )
                        })
                        .when(about.readable && !viewing, |d| {
                            d.child(button(("session-open", key), "Open", false).on_click(
                                cx.listener(move |this, _, _, cx| {
                                    this.open_session(&open_dir, cx);
                                    this.set_screen(Screen::Traffic, cx);
                                }),
                            ))
                        })
                        .when(!in_use, |d| {
                            d.child(
                                button(
                                    ("session-delete", key),
                                    if asked { "Delete for good?" } else { "Delete" },
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
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .pb(px(4.))
                    .child(
                        div()
                            .text_size(px(20.))
                            .font_weight(FontWeight::BOLD)
                            .child("Sessions"),
                    )
                    .child(div().text_color(rgb(c.muted)).child(format!(
                        "A session with a name is kept until you delete it. Of the others, the newest {kept} stay."
                    ))),
            )
            .children(
                page.problem
                    .clone()
                    .map(|problem| div().text_color(rgb(c.error)).child(problem)),
            )
            .children(rows)
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
