//! The inspector: one flow, in full. Bodies are unpacked to a file on a background
//! thread and mapped from there, so a body of any size opens without a pause.

use crate::{
    body::Body,
    bodyview::{BodyView, summary},
    colors::colors,
    curl,
    detail::{Detail, Side, body_file},
    flows::{size_text, time_text},
    messages::MessagesState,
    traffic::Traffic,
    widgets::{chip, eyebrow, icon, status_colors},
};
use gpui::{
    ClipboardItem, Context, Div, Entity, FontWeight, MouseButton, MouseDownEvent, SharedString,
    div, prelude::*, px, relative, rgb,
};
use std::time::Duration;
use tracewing_store::session::BodyReader;

/// As much of a request's body as can go on a command line, and one byte more to
/// see that it did not fit.
const CURL_BODY_MAX: usize = 256 * 1024 + 1;
const COPIED_FOR: Duration = Duration::from_millis(1500);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Part {
    #[default]
    Body,
    Headers,
    /// A WebSocket's or server-sent events' messages.
    Messages,
}

#[derive(Default)]
pub enum BodyState {
    #[default]
    Loading,
    Shown(Entity<BodyView>),
    /// Nothing to show, and why.
    Note(SharedString),
}

#[derive(Default)]
pub struct InspectorState {
    pub detail: Option<Detail>,
    pub side: Side,
    pub part: Part,
    pub body: BodyState,
    /// Of the flow chosen, if it is a stream.
    pub messages: Option<MessagesState>,
    /// Which load the body is waiting for, so that a late one is ignored.
    load: u64,
    copied: bool,
}

impl Traffic {
    pub(crate) fn load_detail(&mut self, id: i64, cx: &mut Context<Self>) {
        let Some(flows) = self.flows() else {
            return;
        };
        match Detail::load(flows.conn(), self.session_dir(), id) {
            Ok(detail) => {
                let stream = matches!(detail.kind.as_str(), "websocket" | "sse");
                self.inspector.messages = stream.then(|| MessagesState::new(id));
                self.inspector.part = match (stream, self.inspector.part) {
                    (true, _) => Part::Messages,
                    (false, Part::Messages) => Part::Body,
                    (false, part) => part,
                };
                self.inspector.detail = Some(detail);
                self.inspector.copied = false;
                self.take_new_messages();
                self.load_body(cx);
            }
            Err(_) => {
                // It went with the oldest flows between the click and now.
                self.inspector.detail = None;
                self.inspector.body = BodyState::Note("This flow is gone.".into());
            }
        }
    }

    fn load_body(&mut self, cx: &mut Context<Self>) {
        let Some(detail) = &self.inspector.detail else {
            return;
        };
        let side = self.inspector.side;
        let facts = detail.body(side).clone();
        let id = detail.id;
        self.inspector.load += 1;
        let load = self.inspector.load;
        if !facts.stored.exists() {
            self.inspector.body = BodyState::Note(
                if detail.kind == "tunnel" {
                    "This host is not decrypted, so what it sends cannot be seen."
                } else if facts.bytes == 0 {
                    "No body."
                } else if facts.shed {
                    "This body was not kept: the disk was nearly full, or too much was waiting for it."
                } else {
                    "This body was not kept."
                }
                .into(),
            );
            cx.notify();
            return;
        }
        self.inspector.body = BodyState::Loading;
        let session = self.session_dir().to_owned();
        let max = self.engine.settings.view_max();
        cx.spawn(async move |this, cx| {
            let file = cx
                .background_spawn(async move { body_file(&session, &facts, id, side, max) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.inspector.load != load {
                    return;
                }
                this.inspector.body = match file {
                    None => BodyState::Note(
                        "This body is gone: it was dropped with the oldest flows, or cannot be read."
                            .into(),
                    ),
                    Some((_, 0)) => BodyState::Note("No body.".into()),
                    Some((path, _)) => match Body::open(&path) {
                        Ok(body) => BodyState::Shown(cx.new(|cx| BodyView::new(body, cx))),
                        Err(e) => BodyState::Note(format!("Cannot open this body: {e:#}").into()),
                    },
                };
                cx.notify();
            });
        })
        .detach();
    }

    /// A stream that has ended: its body and its time are known now.
    pub(crate) fn look_at_stream(&mut self, cx: &mut Context<Self>) -> bool {
        let got_messages = self.take_new_messages();
        let Some(detail) = &self.inspector.detail else {
            return got_messages;
        };
        if !detail.open {
            return got_messages;
        }
        let id = detail.id;
        let Some(flows) = self.flows() else {
            return got_messages;
        };
        let Ok(now) = Detail::load(flows.conn(), self.session_dir(), id) else {
            return got_messages;
        };
        if now.open {
            return got_messages;
        }
        self.inspector.detail = Some(now);
        self.forget_row(id);
        self.load_body(cx);
        true
    }

    pub fn set_side(&mut self, side: Side, cx: &mut Context<Self>) {
        if self.inspector.side != side {
            self.inspector.side = side;
            self.load_body(cx);
            cx.notify();
        }
    }

    pub fn set_part(&mut self, part: Part, cx: &mut Context<Self>) {
        self.inspector.part = part;
        cx.notify();
    }

    fn copy_as_curl(&mut self, cx: &mut Context<Self>) {
        let Some(detail) = &self.inspector.detail else {
            return;
        };
        let body = detail
            .request
            .stored
            .exists()
            .then(|| {
                BodyReader::new(self.session_dir()).unpacked(&detail.request.stored, CURL_BODY_MAX)
            })
            .flatten();
        let command = curl::command(
            &detail.method,
            &detail.url,
            detail.headers(Side::Request).unwrap_or_default(),
            body.as_deref(),
        );
        cx.write_to_clipboard(ClipboardItem::new_string(command));
        self.inspector.copied = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(COPIED_FOR).await;
            let _ = this.update(cx, |this, cx| {
                this.inspector.copied = false;
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn render_inspector(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let handle = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.))
            .child(
                div()
                    .id("inspector-handle")
                    .size(px(28.))
                    .rounded(px(10.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_grab()
                    .bg(rgb(c.input))
                    .child(icon("grip", 14., c.muted))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _: &MouseDownEvent, _, cx| {
                            this.begin_inspector_drag(cx)
                        }),
                    ),
            )
            .child(eyebrow("Inspector"));
        let panel = div()
            .size_full()
            .p(px(14.))
            .rounded(px(22.))
            .flex()
            .flex_col()
            .gap(px(12.))
            .overflow_hidden()
            .bg(rgb(c.surface))
            .child(handle);
        let Some(detail) = self.inspector.detail.clone() else {
            return panel.child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgb(c.muted))
                    .child("Choose a flow to see it here"),
            );
        };
        panel
            .child(render_title(&detail))
            .child(self.render_decrypt_choice(&detail, cx))
            .child(render_chips(&detail))
            .children(render_timing(&detail))
            .child(self.render_sides(cx))
            .child(self.render_parts(&detail, cx))
            .child(self.render_part(&detail, cx))
    }

    /// Whether this host is decrypted, and the way to change that. An app that fails
    /// through the proxy usually pins its certificates, and this is the way out.
    fn render_decrypt_choice(&self, detail: &Detail, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let left_alone = self
            .engine
            .rules
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .leaves_alone(&detail.host);
        let host = detail.host.clone();
        let text = if left_alone {
            "Not decrypted. New connections to this host go through unread."
        } else {
            "If an app fails with this host, it may pin its certificates."
        };
        div()
            .flex()
            .flex_row()
            .flex_shrink_0()
            .items_center()
            .gap(px(10.))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(px(12.))
                    .text_color(rgb(c.muted))
                    .child(text),
            )
            .child(
                div()
                    .id("decrypt-choice")
                    .flex_shrink_0()
                    .h(px(28.))
                    .px(px(10.))
                    .rounded(px(14.))
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .bg(rgb(c.chip))
                    .text_color(rgb(c.on_chip))
                    .text_size(px(12.))
                    .font_weight(FontWeight::BOLD)
                    .child(if left_alone {
                        "Decrypt this host"
                    } else {
                        "Do not decrypt this host"
                    })
                    .on_click(cx.listener(move |view, _, _, cx| {
                        let host = host.clone();
                        view.change_rules(
                            |rules| {
                                if left_alone {
                                    rules.decrypt(&host);
                                } else {
                                    rules.leave_alone(&host);
                                }
                            },
                            cx,
                        );
                    })),
            )
    }

    fn render_sides(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let side = self.inspector.side;
        let tab = |id: &'static str, label: &'static str, this: Side| {
            let on = side == this;
            div()
                .id(id)
                .flex_1()
                .h(px(30.))
                .rounded(px(14.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .font_weight(FontWeight::BOLD)
                .when(on, |d| d.bg(rgb(c.tab_on)).text_color(rgb(c.text)))
                .when(!on, |d| d.text_color(rgb(c.muted)))
                .child(label)
                .on_click(cx.listener(move |view, _, _, cx| view.set_side(this, cx)))
        };
        div()
            .flex()
            .flex_row()
            .flex_shrink_0()
            .gap(px(4.))
            .p(px(4.))
            .rounded(px(18.))
            .bg(rgb(c.code))
            .child(tab("side-request", "Request", Side::Request))
            .child(tab("side-response", "Response", Side::Response))
    }

    fn render_parts(&self, detail: &Detail, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let part = self.inspector.part;
        let headers = detail.headers(self.inspector.side).map_or(0, <[_]>::len);
        let summary_text: Option<String> = match (&self.inspector.body, part) {
            (BodyState::Shown(view), Part::Body) => Some(summary(view.read(cx).body())),
            _ => None,
        };
        let tab = |id: &'static str, label: String, this: Part| {
            let on = part == this;
            div()
                .id(id)
                .cursor_pointer()
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(if on { c.text } else { c.muted }))
                .pb(px(3.))
                .border_b_2()
                .border_color(rgb(if on { c.link } else { c.surface }))
                .child(label)
                .on_click(cx.listener(move |view, _, _, cx| view.set_part(this, cx)))
        };
        div()
            .flex()
            .flex_row()
            .flex_shrink_0()
            .items_center()
            .gap(px(14.))
            .text_size(px(12.5))
            .children(self.inspector.messages.as_ref().map(|state| {
                tab(
                    "part-messages",
                    format!(
                        "Messages {}",
                        crate::flows::group_digits(state.list.len() as u64)
                    ),
                    Part::Messages,
                )
            }))
            .child(tab("part-body", "Body".to_owned(), Part::Body))
            .child(tab(
                "part-headers",
                format!("Headers {headers}"),
                Part::Headers,
            ))
            .child(div().flex_1())
            .children(summary_text.map(|text| {
                div()
                    .text_size(px(12.))
                    .text_color(rgb(c.muted))
                    .child(text)
            }))
            .when(
                !matches!(detail.kind.as_str(), "tunnel" | "websocket"),
                |d| {
                    d.child(
                        div()
                            .id("copy-curl")
                            .h(px(28.))
                            .px(px(10.))
                            .rounded(px(14.))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .bg(rgb(c.chip))
                            .text_color(rgb(c.on_chip))
                            .text_size(px(12.))
                            .font_weight(FontWeight::BOLD)
                            .child(if self.inspector.copied {
                                "Copied"
                            } else {
                                "Copy as cURL"
                            })
                            .on_click(cx.listener(|view, _, _, cx| view.copy_as_curl(cx))),
                    )
                },
            )
    }

    fn render_part(&mut self, detail: &Detail, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let well = div()
            .flex_1()
            .min_h_0()
            .rounded(px(16.))
            .overflow_hidden()
            .bg(rgb(c.code));
        match self.inspector.part {
            Part::Messages => well.child(self.render_messages(cx)),
            Part::Headers => well.child(render_headers(detail.headers(self.inspector.side))),
            Part::Body => match &self.inspector.body {
                BodyState::Shown(view) => well.py(px(8.)).child(view.clone()),
                BodyState::Loading => well.child(note("Opening…")),
                BodyState::Note(text) => well.child(note(text.clone())),
            },
        }
    }
}

fn note(text: impl Into<SharedString>) -> Div {
    div()
        .size_full()
        .p(px(16.))
        .flex()
        .items_center()
        .justify_center()
        .text_color(rgb(colors().muted))
        .child(text.into())
}

fn render_title(detail: &Detail) -> impl IntoElement {
    let c = colors();
    div()
        .flex()
        .flex_row()
        .flex_shrink_0()
        .items_start()
        .gap(px(10.))
        .child(
            div()
                .flex_shrink_0()
                .px(px(9.))
                .py(px(3.))
                .rounded(px(8.))
                .bg(rgb(c.tonal))
                .text_color(rgb(c.on_tonal))
                .text_size(px(12.))
                .font_weight(FontWeight::BOLD)
                .child(detail.method.clone()),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .max_h(px(58.))
                .overflow_hidden()
                .font_family(crate::MONO_FONT)
                .text_size(px(12.5))
                .line_height(relative(1.5))
                .child(detail.url.clone()),
        )
}

fn status_text(status: Option<u16>) -> String {
    match status {
        None => "No response".to_owned(),
        Some(status) => {
            let reason = http::StatusCode::from_u16(status)
                .ok()
                .and_then(|code| code.canonical_reason());
            match reason {
                Some(reason) => format!("{status} {reason}"),
                None => status.to_string(),
            }
        }
    }
}

fn render_chips(detail: &Detail) -> impl IntoElement {
    let c = colors();
    let (ground, ink) = status_colors(detail.status);
    let plain = |text: String| chip(text, c.chip, c.on_chip);
    // All a tunnel tells is that it was opened: nothing in it is read or counted.
    if detail.kind == "tunnel" {
        return div()
            .flex()
            .flex_row()
            .flex_shrink_0()
            .gap(px(6.))
            .child(plain("Tunnel, not decrypted".to_owned()));
    }
    let encoding = detail.response.stored.encoding.clone();
    let size = match encoding {
        Some(encoding) => format!("{} · {encoding}", size_text(detail.response.bytes)),
        None => size_text(detail.response.bytes),
    };
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .flex_shrink_0()
        .gap(px(6.))
        .child(chip(status_text(detail.status), ground, ink))
        .child(plain(detail.protocols()))
        .children(if detail.open {
            Some(chip("Open", c.ok_ground, c.ok))
        } else {
            detail.duration_us.map(|d| plain(time_text(d)))
        })
        .when(detail.kind != "websocket", |d| d.child(plain(size)))
        .children(
            detail
                .error
                .clone()
                .map(|error| chip(error, c.error_ground, c.error)),
        )
}

/// What can be told of the time: DNS, connecting and TLS are not measured yet, so
/// they are not shown.
fn render_timing(detail: &Detail) -> Option<impl IntoElement> {
    let c = colors();
    let total = detail.duration_us?.max(1);
    let waiting = detail.ttfb_us?.clamp(0, total);
    let download = total - waiting;
    let bar = |label: &'static str, left: i64, width: i64, value: i64, color: u32| {
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(10.))
            .text_size(px(12.))
            .child(div().w(px(64.)).text_color(rgb(c.muted)).child(label))
            .child(
                div()
                    .flex_1()
                    .h(px(8.))
                    .rounded(px(4.))
                    .relative()
                    .overflow_hidden()
                    .bg(rgb(c.track))
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .left(relative(left as f32 / total as f32))
                            .w(relative(width as f32 / total as f32))
                            .rounded(px(4.))
                            .bg(rgb(color)),
                    ),
            )
            .child(
                div()
                    .w(px(70.))
                    .flex()
                    .justify_end()
                    .font_weight(FontWeight::BOLD)
                    .child(time_text(value)),
            )
    };
    Some(
        div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .gap(px(5.))
            .px(px(12.))
            .py(px(10.))
            .rounded(px(16.))
            .bg(rgb(c.code))
            .child(bar("Waiting", 0, waiting, waiting, c.wait))
            .child(bar("Download", waiting, download, download, c.download)),
    )
}

fn render_headers(headers: Option<&[(String, String)]>) -> impl IntoElement {
    let c = colors();
    let Some(headers) = headers else {
        return note("These headers were not kept.").into_any_element();
    };
    if headers.is_empty() {
        return note("No headers.").into_any_element();
    }
    div()
        .id("headers")
        .size_full()
        .overflow_y_scroll()
        .p(px(12.))
        .flex()
        .flex_col()
        .gap(px(4.))
        .font_family(crate::MONO_FONT)
        .text_size(px(12.))
        .children(headers.iter().map(|(name, value)| {
            div()
                .flex()
                .flex_row()
                .gap(px(10.))
                .child(
                    div()
                        .w(relative(0.34))
                        .flex_shrink_0()
                        .text_color(rgb(c.json_key))
                        .child(name.clone()),
                )
                .child(div().flex_1().min_w_0().child(value.clone()))
        }))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_read_as_words() {
        assert_eq!(status_text(Some(200)), "200 OK");
        assert_eq!(status_text(Some(404)), "404 Not Found");
        assert_eq!(status_text(Some(599)), "599");
        assert_eq!(status_text(None), "No response");
    }
}
