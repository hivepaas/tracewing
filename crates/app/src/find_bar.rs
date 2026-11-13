//! Find in the inspector: a bar over the part shown, what is typed searched
//! for in it, with case, whole words or a regular expression; Enter for the
//! next, Shift+Enter for the one before, Esc to close.

use gpui::{
    AppContext as _, Context, Div, Focusable, FontWeight, KeyDownEvent, Window, div, point,
    prelude::*, px, rgb,
};
use tracewing_i18n::tr;

use crate::{
    colors::colors,
    detail::Side,
    find::{self, FindOptions},
    input::{Changed, TextInput},
    inspector::{BodyState, FindBar, FindSwitch, Part},
    text_size::ui,
    traffic::Traffic,
    widgets::{Reachable, icon, tip},
};

impl Traffic {
    /// Opens Find, or goes back to it: what was typed is kept, all of it
    /// chosen to be typed over. Opened for the first time, it starts from what
    /// the list is filtered by, when that is text rather than a field.
    pub fn open_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.inspector.find.is_none() {
            let input = cx.new(|cx| TextInput::said(|| tr!("Find"), cx));
            let changed = cx.subscribe(&input, |this, _, _: &Changed, cx| this.apply_find(cx));
            self.inspector.find = Some(FindBar::new(input.clone(), changed));
            let filtered = self.filter_text(cx);
            if !filtered.is_empty() && !filtered.contains(':') {
                input.update(cx, |input, cx| input.set_text(&filtered, 0..0, cx));
            }
        }
        if let Some(bar) = &self.inspector.find {
            let input = bar.input.clone();
            input.update(cx, |input, cx| input.select_all(cx));
            let handle = input.read(cx).focus_handle(cx);
            window.focus(&handle, cx);
        }
        self.apply_find(cx);
    }

    /// Opens Find, or closes it if it is open: what the magnifier and Ctrl+F
    /// do.
    pub fn toggle_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.inspector.find.is_some() {
            self.close_find(window, cx);
        } else {
            self.open_find(window, cx);
        }
    }

    /// Closes Find: what it found is no longer marked.
    pub fn close_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.inspector.find.take().is_none() {
            return;
        }
        if let BodyState::Shown(view) = &self.inspector.body {
            view.update(cx, |view, cx| view.set_find(None, cx));
        }
        self.inspector.select.set_find(None);
        self.find_in_messages(None, cx);
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// Searches what is shown for what is typed, as the switches say: the
    /// body, and the parts drawn whole (headers, query, cookies…).
    pub fn apply_find(&mut self, cx: &mut Context<Self>) {
        let options = self.inspector.find_options;
        let Some(bar) = &mut self.inspector.find else {
            return;
        };
        let typed = bar.input.read(cx).text().to_owned();
        let re = match find::matcher(&typed, options) {
            Ok(re) => {
                bar.problem = None;
                re
            }
            Err(why) => {
                bar.problem = Some(why);
                None
            }
        };
        self.inspector.select.set_find(re.clone());
        if let BodyState::Shown(view) = &self.inspector.body {
            bar.mark_applied(view.entity_id());
            let re = re.clone();
            view.update(cx, |view, cx| view.set_find(re, cx));
        }
        self.find_in_messages(re, cx);
        cx.notify();
    }

    /// Gives the search to a body that has not had it: another flow or side
    /// chosen while Find is open.
    pub(crate) fn find_follows(&mut self, cx: &mut Context<Self>) {
        let stale = match (&self.inspector.find, &self.inspector.body) {
            (Some(bar), BodyState::Shown(view)) => !bar.applied(view.entity_id()),
            _ => false,
        };
        // Another flow's messages, not yet searched.
        let messages_stale = self
            .inspector
            .find
            .as_ref()
            .is_some_and(|bar| bar.problem.is_none() && !bar.input.read(cx).text().is_empty())
            && self
                .inspector
                .messages
                .as_ref()
                .is_some_and(|m| m.find.is_none());
        if stale || messages_stale {
            self.apply_find(cx);
        }
        // In a part drawn whole, the one Find is at, once it has moved.
        if self.inspector.find.is_some()
            && self.inspector.part != Part::Body
            && let Some(key) = self.inspector.select.take_find_scroll()
        {
            self.scroll_to_piece(key);
        }
    }

    /// Scrolls the part shown so that piece `key` is in view: in the pane it
    /// was drawn in, a third of the way down.
    fn scroll_to_piece(&mut self, key: u64) {
        let Some(piece) = self.inspector.select.drawn_at(key) else {
            return;
        };
        let part = self.inspector.part;
        let center = piece.center();
        let pane = [Side::Request, Side::Response]
            .into_iter()
            .map(|side| self.inspector.scrolled(part, side).handle)
            .find(|handle| {
                let view = handle.bounds();
                view.size.width > px(0.) && view.left() <= center.x && center.x <= view.right()
            });
        let Some(handle) = pane else {
            return;
        };
        let view = handle.bounds();
        if piece.top() >= view.top() && piece.bottom() <= view.bottom() {
            return;
        }
        let now = handle.offset();
        let max = handle.max_offset().y.max(px(0.));
        let y = now.y - (piece.top() - (view.top() + view.size.height / 3.));
        handle.set_offset(point(now.x, y.clamp(-max, px(0.))));
    }

    /// The next found, or the one before.
    pub fn find_step(&mut self, forward: bool, cx: &mut Context<Self>) {
        if self.inspector.part == Part::Messages {
            self.messages_find_step(forward, cx);
        } else if self.inspector.part != Part::Body {
            self.inspector.select.find_step(forward);
            cx.notify();
        } else if let BodyState::Shown(view) = &self.inspector.body {
            view.update(cx, |view, cx| view.find_step(forward, cx));
        }
    }

    /// One of the switches turned.
    pub fn flip_find(&mut self, switch: FindSwitch, cx: &mut Context<Self>) {
        let o = &mut self.inspector.find_options;
        match switch {
            FindSwitch::Case => o.case = !o.case,
            FindSwitch::Word => o.word = !o.word,
            FindSwitch::Regex => o.regex = !o.regex,
        }
        self.apply_find(cx);
    }

    /// How far Find has got, in words.
    fn find_said(&self, cx: &Context<Self>) -> (String, bool) {
        let Some(bar) = &self.inspector.find else {
            return (String::new(), false);
        };
        if let Some(problem) = &bar.problem {
            return (problem.clone(), true);
        }
        if bar.input.read(cx).text().is_empty() {
            return (String::new(), false);
        }
        let count = if self.inspector.part == Part::Messages {
            // The messages that hold it.
            self.inspector
                .messages
                .as_ref()
                .and_then(|m| m.find.as_ref())
                .map(|find| crate::bodyview::FindCount {
                    now: find.now,
                    found: find.places.len(),
                    done: find.done,
                    capped: false,
                })
        } else if self.inspector.part != Part::Body {
            // A part drawn whole: all of it is searched as it is drawn.
            self.inspector
                .select
                .find_count()
                .map(|(now, found)| crate::bodyview::FindCount {
                    now,
                    found,
                    done: true,
                    capped: false,
                })
        } else {
            match &self.inspector.body {
                BodyState::Shown(view) => view.read(cx).find_count(),
                _ => None,
            }
        };
        let Some(count) = count else {
            return (String::new(), false);
        };
        if count.found == 0 {
            return if count.done {
                (tr!("Not found"), true)
            } else {
                (tr!("Looking…"), false)
            };
        }
        let mut total = crate::flows::group_digits(count.found as u64);
        if count.capped {
            total.push('+');
        } else if !count.done {
            total.push('…');
        }
        let now = count.now.map_or(0, |i| i + 1);
        (
            tr!("{now} of {count}", now = now.to_string(), count = total),
            false,
        )
    }

    /// The bar, over the part shown.
    pub(crate) fn render_find_bar(&self, cx: &mut Context<Self>) -> Option<gpui::Stateful<Div>> {
        let bar = self.inspector.find.as_ref()?;
        let c = colors();
        let options: FindOptions = self.inspector.find_options;
        let (said, bad) = self.find_said(cx);
        let button = |id: &'static str, tip_text: String| {
            div()
                .id(id)
                .size(ui(28.))
                .rounded(ui(10.))
                .flex()
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .reachable()
                .hover(|d| d.bg(rgb(c.chip)))
                .tooltip(tip(tip_text))
        };
        let switch = |id: &'static str, label: &'static str, on: bool, tip_text: String| {
            button(id, tip_text)
                .font_family(crate::mono_font())
                .text_size(ui(12.))
                .font_weight(FontWeight::BOLD)
                .when(on, |d| d.bg(rgb(c.accent)).text_color(rgb(c.on_accent)))
                .when(!on, |d| d.text_color(rgb(c.muted)))
                .child(label)
        };
        Some(
            div()
                .id("find-bar")
                .flex()
                .flex_row()
                .items_center()
                .gap(ui(4.))
                // Enter and Shift+Enter step, Esc closes: before the field
                // takes them.
                .capture_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                    let k = &e.keystroke;
                    match k.key.as_str() {
                        "enter" | "f3" => {
                            this.find_step(!k.modifiers.shift, cx);
                            cx.stop_propagation();
                        }
                        "escape" => {
                            this.close_find(window, cx);
                            cx.stop_propagation();
                        }
                        _ => {}
                    }
                }))
                .child(
                    div()
                        .flex_1()
                        .min_w(ui(140.))
                        .h(ui(30.))
                        .px(ui(10.))
                        .rounded(ui(12.))
                        .flex()
                        .items_center()
                        .bg(rgb(c.input))
                        .border_1()
                        .border_color(rgb(if bad && bar.problem.is_some() {
                            c.error
                        } else {
                            c.input
                        }))
                        .font_family(crate::mono_font())
                        .font_features(gpui::FontFeatures::disable_ligatures())
                        .child(bar.input.clone()),
                )
                .child(
                    div()
                        .px(ui(6.))
                        .flex_shrink_0()
                        .text_size(ui(12.))
                        .text_color(rgb(if bad { c.error } else { c.muted }))
                        .child(said),
                )
                .child(
                    switch("find-case", "Aa", options.case, tr!("Match case")).on_click(
                        cx.listener(|this, _, _, cx| this.flip_find(FindSwitch::Case, cx)),
                    ),
                )
                .child(
                    switch("find-word", "ab", options.word, tr!("Whole word")).on_click(
                        cx.listener(|this, _, _, cx| this.flip_find(FindSwitch::Word, cx)),
                    ),
                )
                .child(
                    switch("find-regex", ".*", options.regex, tr!("Regular expression")).on_click(
                        cx.listener(|this, _, _, cx| this.flip_find(FindSwitch::Regex, cx)),
                    ),
                )
                .child(
                    button("find-prev", tr!("The one before (Shift+Enter)"))
                        .child(icon("up", 14., c.muted))
                        .on_click(cx.listener(|this, _, _, cx| this.find_step(false, cx))),
                )
                .child(
                    button("find-next", tr!("The next (Enter)"))
                        .child(icon("down", 14., c.muted))
                        .on_click(cx.listener(|this, _, _, cx| this.find_step(true, cx))),
                )
                .child(
                    button("find-close", tr!("Close (Esc)"))
                        .child(icon("close", 14., c.muted))
                        .on_click(cx.listener(|this, _, window, cx| this.close_find(window, cx))),
                ),
        )
    }
}
