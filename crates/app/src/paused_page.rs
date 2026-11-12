//! The paused screen: what breakpoints hold, each to look at, change and send
//! on, or drop. A request or a response waits here until then, and so does the
//! app that sent it.

use crate::text_size::ui;
use crate::{
    colors::colors,
    input::TextInput,
    traffic::{Screen, Traffic},
    widgets::{Reachable, eyebrow},
};
use gpui::{AppContext as _, Context, Div, Entity, FontWeight, Stateful, div, prelude::*, rgb};
use tracewing_i18n::{tr, tr_n};
use tracewing_proxy::pause::{self, Message, Paused, Stage, Verdict};

pub struct PausedPage {
    /// What is held, as last read.
    pub held: Vec<Paused>,
    version: u64,
    /// The one in the editor.
    pub chosen: Option<u64>,
    pub method: Entity<TextInput>,
    pub url: Entity<TextInput>,
    pub status: Entity<TextInput>,
    pub headers: Entity<TextInput>,
    pub body: Entity<TextInput>,
    /// A body that is not text cannot be edited, and goes on as it came.
    binary_body: bool,
    pub problem: Option<String>,
}

/// Headers as the editor shows them: one `name: value` a line.
pub fn headers_text(headers: &[(String, String)]) -> String {
    headers
        .iter()
        .map(|(name, value)| format!("{name}: {value}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// And back. Empty lines are passed over.
pub fn parse_headers(text: &str) -> Result<Vec<(String, String)>, String> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(at, line)| {
            let (name, value) = line
                .split_once(':')
                .ok_or_else(|| tr!("Header line {line} has no \":\"", line = at + 1))?;
            if name.trim().is_empty() {
                return Err(tr!("Header line {line} has no name", line = at + 1));
            }
            Ok((name.trim().to_owned(), value.trim().to_owned()))
        })
        .collect()
}

fn set(input: &Entity<TextInput>, text: &str, cx: &mut Context<Traffic>) {
    input.update(cx, |input, cx| input.set_text(text, 0..0, cx));
}

impl PausedPage {
    fn new(cx: &mut Context<Traffic>) -> Self {
        Self {
            held: Vec::new(),
            version: u64::MAX,
            chosen: None,
            method: cx.new(|cx| TextInput::said(|| tr!("Method"), cx)),
            url: cx.new(|cx| TextInput::said(|| tr!("URL"), cx)),
            status: cx.new(|cx| TextInput::said(|| tr!("Status"), cx)),
            headers: cx
                .new(|cx| TextInput::said(|| tr!("name: value, one a line"), cx).multiline(4, 14)),
            body: cx.new(|cx| TextInput::said(|| tr!("No body"), cx).multiline(6, 24)),
            binary_body: false,
            problem: None,
        }
    }

    fn current(&self) -> Option<&Paused> {
        let chosen = self.chosen?;
        self.held.iter().find(|paused| paused.id == chosen)
    }
}

impl Traffic {
    fn paused_page(&mut self, cx: &mut Context<Self>) -> &mut PausedPage {
        if self.paused_page.is_none() {
            self.paused_page = Some(PausedPage::new(cx));
        }
        self.paused_page.as_mut().expect("just made")
    }

    /// Reads what is held when it has changed. The first to arrive, when none
    /// was, brings this screen up: someone asked for it to stop, and is waiting.
    pub(crate) fn look_at_pauses(&mut self, cx: &mut Context<Self>) -> bool {
        let version = self.engine.pauses.version();
        if self
            .paused_page
            .as_ref()
            .is_some_and(|page| page.version == version)
        {
            return false;
        }
        if self.paused_page.is_none() && version == 0 {
            return false;
        }
        let held = self.engine.pauses.held();
        let page = self.paused_page(cx);
        let was_empty = page.held.is_empty();
        page.version = version;
        page.held = held;
        let arrived = was_empty && !page.held.is_empty();
        if page.current().is_none() {
            let first = page.held.first().map(|paused| paused.id);
            self.choose_paused(first, cx);
        }
        if arrived && self.screen != Screen::Paused {
            self.set_screen(Screen::Paused, cx);
        }
        true
    }

    pub fn choose_paused(&mut self, id: Option<u64>, cx: &mut Context<Self>) {
        let page = self.paused_page(cx);
        page.chosen = id;
        page.problem = None;
        let Some(message) = page.current().map(|paused| paused.message.clone()) else {
            return;
        };
        let text = String::from_utf8(message.body.clone());
        page.binary_body = text.is_err();
        let body = match text {
            Ok(text) => text,
            Err(_) => tr_n!(
                message.body.len(),
                "{n} bytes that are not text: they go on as they came.",
                "{n} bytes that are not text: they go on as they came."
            ),
        };
        let fields = [
            (page.method.clone(), message.method.clone()),
            (page.url.clone(), message.url.clone()),
            (page.status.clone(), message.status.to_string()),
            (page.headers.clone(), headers_text(&message.headers)),
            (page.body.clone(), body),
        ];
        for (input, text) in fields {
            set(&input, &text, cx);
        }
        cx.notify();
    }

    /// The editor's message, if it can be sent.
    fn edited_message(&self, cx: &Context<Self>) -> Result<(u64, Stage, Message), String> {
        let page = self
            .paused_page
            .as_ref()
            .ok_or_else(|| tr!("Nothing is paused"))?;
        let paused = page.current().ok_or_else(|| tr!("Nothing is paused"))?;
        let read = |input: &Entity<TextInput>| input.read(cx).text().to_owned();
        let mut message = paused.message.clone();
        if !paused.stage.is_message() {
            message.headers = parse_headers(&read(&page.headers))?;
        }
        if !page.binary_body {
            message.body = read(&page.body).into_bytes();
        }
        match paused.stage {
            Stage::Request => {
                message.method = read(&page.method).trim().to_owned();
                message.url = read(&page.url).trim().to_owned();
                pause::request_from(message.clone())?;
            }
            Stage::Response => {
                let status = read(&page.status);
                message.status = status
                    .trim()
                    .parse()
                    .map_err(|_| tr!("\"{status}\" is not a status", status = status.trim()))?;
                pause::response_from(message.clone())?;
            }
            // A socket's message is only its text.
            Stage::ToServer | Stage::ToApp => {}
        }
        Ok((paused.id, paused.stage, message))
    }

    pub fn send_paused(&mut self, cx: &mut Context<Self>) {
        match self.edited_message(cx) {
            Ok((id, _, message)) => {
                self.engine.pauses.decide(id, Verdict::Send(message));
            }
            Err(problem) => self.paused_page(cx).problem = Some(problem),
        }
        cx.notify();
    }

    pub fn send_paused_unchanged(&mut self, cx: &mut Context<Self>) {
        let page = self.paused_page(cx);
        if let Some(paused) = page.current() {
            let (id, message) = (paused.id, paused.message.clone());
            self.engine.pauses.decide(id, Verdict::Send(message));
        }
        cx.notify();
    }

    pub fn drop_paused(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.paused_page(cx).chosen {
            self.engine.pauses.decide(id, Verdict::Drop);
        }
        cx.notify();
    }

    pub fn release_all_paused(&mut self, cx: &mut Context<Self>) {
        self.engine.pauses.release_all();
        cx.notify();
    }

    pub(crate) fn paused_count(&self) -> usize {
        self.paused_page.as_ref().map_or(0, |page| page.held.len())
    }

    pub(crate) fn render_paused(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        self.paused_page(cx);
        let page = self.paused_page.as_ref().expect("just made");
        let chip = |id: &'static str, label: String, loud: bool| -> Stateful<Div> {
            div()
                .id(id)
                .flex_shrink_0()
                .h(ui(34.))
                .px(ui(16.))
                .rounded(ui(17.))
                .flex()
                .items_center()
                .cursor_pointer()
                .reachable()
                .font_weight(FontWeight::BOLD)
                .when(loud, |d| d.bg(rgb(c.accent)).text_color(rgb(c.on_accent)))
                .when(!loud, |d| d.bg(rgb(c.chip)).text_color(rgb(c.on_chip)))
                .child(label)
        };
        let field = |input: &Entity<TextInput>| {
            div()
                .w_full()
                .px(ui(12.))
                .py(ui(8.))
                .rounded(ui(12.))
                .bg(rgb(c.input))
                .font_family(crate::MONO_FONT)
                .font_features(gpui::FontFeatures::disable_ligatures())
                .child(input.clone())
        };

        let list = page.held.iter().map(|paused| {
            let on = page.chosen == Some(paused.id);
            let id = paused.id;
            let stage = match paused.stage {
                Stage::Request => tr!("REQUEST"),
                Stage::Response => tr!("RESPONSE"),
                Stage::ToServer => tr!("MESSAGE TO THE SERVER"),
                Stage::ToApp => tr!("MESSAGE TO THE APP"),
            };
            // A socket's message by its start, as its URL is the socket's.
            let line = if paused.stage.is_message() {
                crate::messages::preview_text(&paused.message.body)
            } else {
                format!("{} {}", paused.message.method, paused.message.url)
            };
            let at = chrono::DateTime::<chrono::Local>::from(paused.since).format("%H:%M:%S");
            div()
                .id(("paused", id))
                .flex()
                .flex_col()
                .gap(ui(2.))
                .px(ui(12.))
                .py(ui(10.))
                .rounded(ui(14.))
                .cursor_pointer()
                .reachable()
                .when(on, |d| d.bg(rgb(c.tonal)).text_color(rgb(c.on_tonal)))
                .when(!on, |d| d.bg(rgb(c.surface)))
                .child(
                    div()
                        .text_size(ui(11.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(c.muted))
                        .child(format!("{stage} · {at}")),
                )
                .child(
                    div()
                        .font_family(crate::MONO_FONT)
                        .font_features(gpui::FontFeatures::disable_ligatures())
                        .text_size(ui(12.))
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(line),
                )
                .child(
                    div()
                        .text_size(ui(12.))
                        .text_color(rgb(c.muted))
                        .child(paused.rule.clone()),
                )
                .on_click(cx.listener(move |this, _, _, cx| this.choose_paused(Some(id), cx)))
        });

        let editor = match page.current() {
            None => div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(rgb(c.muted))
                .child(tr!(
                    "Nothing is paused. Add a Pause rule on the Rules screen to stop traffic here."
                ))
                .into_any_element(),
            Some(paused) => {
                let socket_line = || {
                    div()
                        .font_family(crate::MONO_FONT)
                        .font_features(gpui::FontFeatures::disable_ligatures())
                        .text_size(ui(12.))
                        .text_color(rgb(c.muted))
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(tr!(
                            "{kind} on {url}",
                            kind = &paused.message.method,
                            url = &paused.message.url
                        ))
                };
                let first_line = match paused.stage {
                    Stage::ToServer | Stage::ToApp => socket_line(),
                    Stage::Request => div()
                        .flex()
                        .flex_row()
                        .gap(ui(8.))
                        .child(div().w(ui(110.)).flex_shrink_0().child(field(&page.method)))
                        .child(div().flex_1().min_w_0().child(field(&page.url))),
                    Stage::Response => div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(ui(8.))
                        .child(div().w(ui(110.)).flex_shrink_0().child(field(&page.status)))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .font_family(crate::MONO_FONT)
                                .font_features(gpui::FontFeatures::disable_ligatures())
                                .text_size(ui(12.))
                                .text_color(rgb(c.muted))
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .child(tr!(
                                    "for {method} {url}",
                                    method = &paused.message.method,
                                    url = &paused.message.url
                                )),
                        ),
                };
                div()
                    .id("paused-editor")
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap(ui(10.))
                    .p(ui(14.))
                    .rounded(ui(22.))
                    .bg(rgb(c.surface))
                    .child(eyebrow(match paused.stage {
                        Stage::Request => tr!("The request, before the server gets it"),
                        Stage::Response => tr!("The response, before the app gets it"),
                        Stage::ToServer => {
                            tr!("A socket's message, before the server gets it")
                        }
                        Stage::ToApp => tr!("A socket's message, before the app gets it"),
                    }))
                    .child(first_line)
                    .when(!paused.stage.is_message(), |d| {
                        d.child(eyebrow(tr!("Headers")))
                            .child(field(&page.headers))
                            .child(eyebrow(tr!("Body")))
                    })
                    .when(paused.stage.is_message(), |d| {
                        d.child(eyebrow(tr!("Message")))
                    })
                    .child(field(&page.body))
                    .children(
                        page.problem
                            .clone()
                            .map(|problem| div().text_color(rgb(c.error)).child(problem)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .flex_wrap()
                            .gap(ui(8.))
                            .child(
                                chip("paused-send", tr!("Send"), true)
                                    .on_click(cx.listener(|this, _, _, cx| this.send_paused(cx))),
                            )
                            .child(
                                chip("paused-unchanged", tr!("Send as it came"), false).on_click(
                                    cx.listener(|this, _, _, cx| this.send_paused_unchanged(cx)),
                                ),
                            )
                            .child(
                                chip("paused-drop", tr!("Drop"), false)
                                    .on_click(cx.listener(|this, _, _, cx| this.drop_paused(cx))),
                            ),
                    )
                    .into_any_element()
            }
        };

        let count = page.held.len();
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .gap(ui(10.))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap(ui(10.))
                    .child(
                        div()
                            .flex_1()
                            .min_w(ui(280.))
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(ui(20.))
                                    .font_weight(FontWeight::BOLD)
                                    .child(tr!("Paused")),
                            )
                            .child(div().text_color(rgb(c.muted)).child(match count {
                                0 => tr!("Nothing is waiting."),
                                n => {
                                    tr_n!(n, "{n} waits to be sent on.", "{n} wait to be sent on.")
                                }
                            })),
                    )
                    .when(count > 1, |d| {
                        d.child(
                            chip("paused-all", tr!("Send all as they came"), false).on_click(
                                cx.listener(|this, _, _, cx| this.release_all_paused(cx)),
                            ),
                        )
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_row()
                    .gap(ui(10.))
                    .when(count > 0, |d| {
                        d.child(
                            div()
                                .id("paused-list")
                                .w(ui(320.))
                                .flex_shrink_0()
                                .overflow_y_scroll()
                                .flex()
                                .flex_col()
                                .gap(ui(6.))
                                .children(list),
                        )
                    })
                    .child(editor),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headers_go_to_text_and_back() {
        let headers = vec![
            ("content-type".to_owned(), "application/json".to_owned()),
            ("x-two".to_owned(), "a: b".to_owned()),
        ];
        let text = headers_text(&headers);
        assert_eq!(text, "content-type: application/json\nx-two: a: b");
        assert_eq!(parse_headers(&text).unwrap(), headers);
        assert_eq!(
            parse_headers("\n  a:1  \n\n").unwrap(),
            [("a".into(), "1".into())]
        );
    }

    #[test]
    fn a_bad_header_line_says_which() {
        assert_eq!(
            parse_headers("a: 1\nno colon").unwrap_err(),
            "Header line 2 has no \":\""
        );
        assert_eq!(
            parse_headers(": x").unwrap_err(),
            "Header line 1 has no name"
        );
    }
}
