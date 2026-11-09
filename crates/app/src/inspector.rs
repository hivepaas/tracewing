//! The inspector: one flow, in full. Bodies are unpacked to a file on a background
//! thread and mapped from there, so a body of any size opens without a pause.

use crate::{
    body::Body,
    bodyview::{BodyView, summary},
    colors::colors,
    connection::Certificate,
    cookies,
    detail::{Detail, Phase, Side, body_file},
    devices::device_of,
    fields::{self, Field, Value as FieldValue},
    flows::{size_text, time_text},
    input::TextInput,
    jwt::{self, Jwt},
    messages::MessagesState,
    snippets::{self, Language},
    traffic::Traffic,
    widgets::{chip, eyebrow, icon, status_colors},
};
use gpui::{
    Anchor, AnyElement, ClipboardItem, Context, Div, ElementId, Entity, FontWeight, MouseButton,
    MouseDownEvent, ObjectFit, SharedString, StyledImage, anchored, deferred, div, img, prelude::*,
    px, relative, rgb,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracewing_store::session::BodyReader;

/// As much of a request's body as can go on a command line, and one byte more to
/// see that it did not fit.
/// One byte past what a snippet takes, so that it can tell it was cut.
const SNIPPET_BODY_MAX: usize = 256 * 1024 + 1;
const COPIED_FOR: Duration = Duration::from_millis(1500);
/// Pictures larger than this show as bytes.
const PICTURE_MAX: u64 = 32 << 20;

fn is_picture(content_type: Option<&str>) -> bool {
    content_type.is_some_and(|t| t.trim().to_ascii_lowercase().starts_with("image/"))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Part {
    #[default]
    Body,
    Headers,
    Cookies,
    /// The server's address, TLS and certificates.
    Connection,
    /// The request's URL fields.
    Query,
    /// The fields of a form the request sent.
    Form,
    /// A WebSocket's or server-sent events' messages.
    Messages,
    /// A call to an LLM's API, put back together.
    Llm,
}

#[derive(Default)]
pub enum BodyState {
    #[default]
    Loading,
    Shown(Entity<BodyView>),
    /// A picture, drawn from its file, and its size.
    Image(std::path::PathBuf, u64),
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
    /// The menu of languages to copy the request as is open.
    pub copy_menu: bool,
    /// The chosen flow as a call to a model, once its bodies are read.
    pub llm: Option<(crate::llm::Provider, crate::llm::Exchange)>,
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
                let no_cookies = cookie_counts(&detail) == (0, 0);
                let a_call = crate::llm::provider_of(&detail.method, &detail.url).is_some();
                let (part_now, side_now) = (self.inspector.part, self.inspector.side);
                self.inspector.part = match (stream, self.inspector.part) {
                    (true, _) => Part::Messages,
                    (_, Part::Llm) if a_call => Part::Llm,
                    (false, Part::Messages | Part::Llm) => Part::Body,
                    (false, Part::Cookies) if no_cookies => Part::Body,
                    (false, Part::Query | Part::Form) if !has_part(&detail, part_now, side_now) => {
                        Part::Body
                    }
                    (false, part) => part,
                };
                self.inspector.detail = Some(detail);
                self.inspector.copied = false;
                self.inspector.llm = None;
                self.load_llm(cx);
                self.show_device_name(cx);
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
        let content_type = match side {
            Side::Request => detail.req_content_type.as_deref(),
            Side::Response => detail.content_type.as_deref(),
        };
        let picture = is_picture(content_type);
        let content_type = content_type.map(str::to_owned);
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
                    Some((path, size)) if picture && size <= PICTURE_MAX => {
                        BodyState::Image(path, size)
                    }
                    Some((path, _)) => match Body::open(&path) {
                        Ok(body) => BodyState::Shown(
                            cx.new(|cx| BodyView::new(body, content_type.as_deref(), cx)),
                        ),
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
        self.load_llm(cx);
        true
    }

    pub fn set_side(&mut self, side: Side, cx: &mut Context<Self>) {
        if self.inspector.side != side {
            self.inspector.side = side;
            let part = self.inspector.part;
            let gone = matches!(part, Part::Query | Part::Form)
                && self
                    .inspector
                    .detail
                    .as_ref()
                    .is_none_or(|detail| !has_part(detail, part, side));
            if gone {
                self.inspector.part = Part::Body;
            }
            self.load_body(cx);
            cx.notify();
        }
    }

    pub fn set_part(&mut self, part: Part, cx: &mut Context<Self>) {
        self.inspector.part = part;
        cx.notify();
    }

    pub(crate) fn copy_as_curl(&mut self, cx: &mut Context<Self>) {
        self.copy_as(Language::Curl, cx);
    }

    pub fn set_copy_menu(&mut self, open: bool, cx: &mut Context<Self>) {
        self.inspector.copy_menu = open;
        cx.notify();
    }

    /// Copies the chosen flow's request as code in `language`.
    pub fn copy_as(&mut self, language: Language, cx: &mut Context<Self>) {
        self.inspector.copy_menu = false;
        let Some(detail) = &self.inspector.detail else {
            return;
        };
        let body = detail
            .request
            .stored
            .exists()
            .then(|| {
                BodyReader::new(self.session_dir())
                    .unpacked(&detail.request.stored, SNIPPET_BODY_MAX)
            })
            .flatten();
        let command = snippets::snippet(
            language,
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
        let url = detail.url.clone();
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
                    .id("make-rule")
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
                    .child("Make a rule")
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.start_rule_for(&url, cx);
                    })),
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
        let side = self.inspector.side;
        let headers = detail.headers(side).map_or(0, <[_]>::len);
        let (sent, set) = cookie_counts(detail);
        let cookies = match side {
            Side::Request => sent,
            Side::Response => set,
        };
        let summary_text: Option<String> = match (&self.inspector.body, part) {
            (BodyState::Shown(view), Part::Body) => Some(summary(view.read(cx).body())),
            (BodyState::Image(_, size), Part::Body) => {
                Some(format!("Picture · {}", size_text(*size)))
            }
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
        // A narrow inspector puts the buttons on a line of their own.
        div()
            .flex()
            .flex_row()
            .flex_wrap()
            .flex_shrink_0()
            .items_center()
            .gap_x(px(14.))
            .gap_y(px(8.))
            .text_size(px(12.5))
            .when(
                self.inspector.llm.is_some()
                    || self
                        .inspector
                        .detail
                        .as_ref()
                        .is_some_and(|d| crate::llm::provider_of(&d.method, &d.url).is_some()),
                |d| d.child(tab("part-llm", "LLM".to_owned(), Part::Llm)),
            )
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
            .when(sent + set > 0, |d| {
                d.child(tab(
                    "part-cookies",
                    format!("Cookies {cookies}"),
                    Part::Cookies,
                ))
            })
            .when(side == Side::Request, |d| {
                let query = fields::query(&detail.url).len();
                let form = detail.form.as_ref().map_or(0, Vec::len);
                d.when(query > 0, |d| {
                    d.child(tab("part-query", format!("Query {query}"), Part::Query))
                })
                .when(form > 0, |d| {
                    d.child(tab("part-form", format!("Form {form}"), Part::Form))
                })
            })
            .child(tab(
                "part-connection",
                "Connection".to_owned(),
                Part::Connection,
            ))
            .child(div().flex_1())
            .children(summary_text.map(|text| {
                div()
                    .text_size(px(12.))
                    .text_color(rgb(c.muted))
                    .child(text)
            }))
            .when(
                part == Part::Body && detail.body(side).stored.exists(),
                |d| {
                    d.child(
                        div()
                            .id("save-body")
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
                            .child("Save body")
                            .on_click(cx.listener(|view, _, _, cx| view.ask_save_body(cx))),
                    )
                },
            )
            .when(
                !matches!(detail.kind.as_str(), "tunnel" | "websocket"),
                |d| {
                    let small = |id: &'static str, label: &'static str| {
                        div()
                            .id(id)
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
                            .child(label)
                    };
                    d.child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(px(6.))
                            .child(
                                small("send-again", "Send again").on_click(
                                    cx.listener(|view, _, _, cx| view.send_flow_again(cx)),
                                ),
                            )
                            .child(
                                small("edit-send", "Edit and send")
                                    .on_click(cx.listener(|view, _, _, cx| view.edit_flow(cx))),
                            )
                            .child(
                                small(
                                    "copy-curl",
                                    if self.inspector.copied {
                                        "Copied"
                                    } else {
                                        "Copy as cURL"
                                    },
                                )
                                .on_click(cx.listener(|view, _, _, cx| view.copy_as_curl(cx))),
                            )
                            .child(self.render_copy_as(cx)),
                    )
                },
            )
    }

    /// "Copy as…" and, open, the languages it offers.
    fn render_copy_as(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let open = self.inspector.copy_menu;
        let item = |language: Language| {
            div()
                .id(ElementId::Name(format!("copy-as-{language:?}").into()))
                .h(px(30.))
                .px(px(10.))
                .rounded(px(8.))
                .flex()
                .items_center()
                .cursor_pointer()
                .hover(|d| d.bg(rgb(c.tonal)))
                .child(language.name())
                .on_click(cx.listener(move |view, _, _, cx| view.copy_as(language, cx)))
        };
        div()
            .relative()
            .child(
                div()
                    .id("copy-as")
                    .h(px(28.))
                    .px(px(10.))
                    .rounded(px(14.))
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .bg(rgb(if open { c.tonal } else { c.chip }))
                    .text_color(rgb(if open { c.on_tonal } else { c.on_chip }))
                    .text_size(px(12.))
                    .font_weight(FontWeight::BOLD)
                    .child("Copy as…")
                    .on_click(cx.listener(move |view, _, _, cx| view.set_copy_menu(!open, cx))),
            )
            .when(open, |d| {
                d.child(
                    div().absolute().top(px(32.)).left_0().child(deferred(
                        anchored()
                            .anchor(Anchor::TopLeft)
                            .snap_to_window_with_margin(px(8.))
                            .child(
                                div()
                                    .id("copy-as-list")
                                    .occlude()
                                    .w(px(190.))
                                    .p(px(6.))
                                    .rounded(px(14.))
                                    .border_1()
                                    .border_color(rgb(c.line))
                                    .bg(rgb(c.surface))
                                    .shadow_lg()
                                    .flex()
                                    .flex_col()
                                    .text_size(px(13.))
                                    .text_color(rgb(c.text))
                                    .children(Language::ALL.into_iter().map(item))
                                    .on_mouse_down_out(
                                        cx.listener(|view, _, _, cx| view.set_copy_menu(false, cx)),
                                    ),
                            ),
                    )),
                )
            })
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
            Part::Llm => well.child(self.render_llm(cx)),
            Part::Headers => well.child(render_headers(detail.headers(self.inspector.side))),
            Part::Connection => {
                let naming = (detail.app.is_none() && device_of(&detail.client).is_some())
                    .then(|| self.device_name_input().clone());
                well.child(render_connection(detail, naming))
            }
            Part::Query => well.child(render_fields(fields::query(&detail.url))),
            Part::Form => well.child(render_fields(detail.form.clone().unwrap_or_default())),
            Part::Cookies => well.child(render_cookies(
                detail.headers(self.inspector.side),
                self.inspector.side,
            )),
            Part::Body => match &self.inspector.body {
                BodyState::Shown(view) => well.py(px(8.)).child(view.clone()),
                BodyState::Image(path, _) => well.child(
                    div()
                        .size_full()
                        .p(px(12.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            img(path.clone())
                                .max_w_full()
                                .max_h_full()
                                .object_fit(ObjectFit::Contain)
                                .with_fallback(|| {
                                    note("This picture cannot be drawn. Save the body to look at it.")
                                        .into_any_element()
                                }),
                        ),
                ),
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
                .font_features(gpui::FontFeatures::disable_ligatures())
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
        .children(
            detail
                .rule
                .clone()
                .map(|rule| chip(format!("Rule: {rule}"), c.tonal, c.on_tonal)),
        )
        .children(
            detail
                .graphql
                .clone()
                .map(|operation| chip(format!("GraphQL: {operation}"), c.tonal, c.on_tonal)),
        )
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
                // A long error wraps in a box of its own, where a chip would
                // run out of the panel.
                .map(|error| {
                    chip(error, c.error_ground, c.error)
                        .max_w_full()
                        .rounded(px(12.))
                }),
        )
}

/// Each step of the time, on one scale. Over a connection made before there is
/// nothing to connect, and it says so.
fn render_timing(detail: &Detail) -> Option<impl IntoElement> {
    let c = colors();
    let phases = detail.phases();
    let total = phases.iter().map(|p| p.start + p.len).max()?.max(1);
    let color = |label: &str| match label {
        "Queued" => c.faint,
        "DNS" => c.dns,
        "Connect" => c.connect,
        "TLS" => c.tls,
        "Waiting" => c.wait,
        _ => c.download,
    };
    let bar = |phase: Phase| {
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(10.))
            .text_size(px(12.))
            .child(div().w(px(64.)).text_color(rgb(c.muted)).child(phase.label))
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
                            .left(relative(phase.start as f32 / total as f32))
                            .w(relative(phase.len as f32 / total as f32))
                            .min_w(px(3.))
                            .rounded(px(4.))
                            .bg(rgb(color(phase.label))),
                    ),
            )
            .child(
                div()
                    .w(px(70.))
                    .flex()
                    .justify_end()
                    .font_weight(FontWeight::BOLD)
                    .child(time_text(phase.len)),
            )
    };
    let reused = match (detail.conn_id, detail.connect_us) {
        (Some(id), None) => Some(format!(
            "Over connection {id}, made before: no time went on connecting."
        )),
        _ => None,
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
            .children(phases.into_iter().map(bar))
            .children(reused.map(|text| {
                div()
                    .pt(px(2.))
                    .text_size(px(11.5))
                    .text_color(rgb(c.muted))
                    .child(text)
            })),
    )
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// The server's address, TLS, and the certificates it showed.
/// Who sent it, then where it went.
/// `naming` is the box for the name of the device it came from, if another sent it.
fn render_connection(detail: &Detail, naming: Option<Entity<TextInput>>) -> AnyElement {
    let c = colors();
    let row = |label: &'static str, value: String| {
        div()
            .flex()
            .flex_row()
            .gap(px(10.))
            .child(
                div()
                    .w(relative(0.26))
                    .flex_shrink_0()
                    .text_color(rgb(c.json_key))
                    .child(label),
            )
            .child(div().flex_1().min_w_0().child(value))
    };
    let heading = |text: String| {
        div()
            .pt(px(6.))
            .font_family(crate::FONT)
            .text_size(px(11.))
            .font_weight(FontWeight::BOLD)
            .text_color(rgb(c.muted))
            .child(text)
    };
    let app = match (&detail.app, detail.app_pid) {
        (Some(app), Some(pid)) => format!("{app}, pid {pid}"),
        (Some(app), None) => app.clone(),
        _ if device_of(&detail.client).is_none() => {
            "Not found: it may have ended, or belong to another user".to_owned()
        }
        _ => "On another device".to_owned(),
    };
    let list = div()
        .id("connection")
        .size_full()
        .overflow_y_scroll()
        .p(px(12.))
        .flex()
        .flex_col()
        .gap(px(6.))
        .font_family(crate::MONO_FONT)
        .font_features(gpui::FontFeatures::disable_ligatures())
        .text_size(px(12.))
        .child(heading("CLIENT".to_owned()))
        .child(row("App", app))
        .children(naming.map(|input| {
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(10.))
                .child(
                    div()
                        .w(relative(0.26))
                        .flex_shrink_0()
                        .text_color(rgb(c.json_key))
                        .child("Device"),
                )
                .child(
                    div()
                        .id("device-name")
                        .flex_1()
                        .min_w_0()
                        .max_w(px(320.))
                        .h(px(28.))
                        .px(px(10.))
                        .rounded(px(10.))
                        .flex()
                        .items_center()
                        .bg(rgb(c.input))
                        .font_family(crate::FONT)
                        .child(input),
                )
        }))
        .children(detail.app_path.clone().map(|path| row("Program", path)))
        .child(row("Address", detail.client.clone()))
        .child(heading("SERVER".to_owned()));
    let Some(connection) = detail.connection.clone() else {
        let why = match (detail.conn_id, detail.kind.as_str()) {
            (Some(_), _) => "What this connection was is no longer kept: it went with older flows.",
            (None, "tunnel") => "Not decrypted: tracewing passed it on without looking.",
            (None, _) if detail.error.is_some() => "No connection was made.",
            (None, _) => "None: it was answered without one.",
        };
        return list
            .child(
                div()
                    .font_family(crate::FONT)
                    .text_color(rgb(c.muted))
                    .child(why),
            )
            .into_any_element();
    };
    let refused = connection
        .tls
        .as_ref()
        .is_some_and(|tls| tls.error.is_some());
    let made = match (detail.conn_id, detail.connect_us) {
        (Some(id), _) if refused => format!("{id}, given up during TLS"),
        (Some(id), Some(_)) => format!("{id}, made for this flow"),
        (Some(id), None) => format!("{id}, made before and used again"),
        _ => "–".to_owned(),
    };
    let list = list
        .child(row("Server", connection.remote.clone()))
        .child(row("Connection", made));
    let Some(tls) = connection.tls else {
        return list
            .child(row("TLS", "None: plain HTTP".to_owned()))
            .into_any_element();
    };
    let now = now_secs();
    let refused = tls.error.is_some();
    let chain_heading = match (refused, tls.chain.len()) {
        (true, 0) => "THE SERVER SHOWED NO CERTIFICATES".to_owned(),
        (true, _) => "WHAT THE SERVER SHOWED, ITS OWN FIRST".to_owned(),
        (false, 0) => "NO CERTIFICATES".to_owned(),
        (false, 1) => "CERTIFICATE".to_owned(),
        (false, n) => format!("{n} CERTIFICATES, THE SERVER'S FIRST"),
    };
    let known = |text: String| (!text.is_empty()).then_some(text);
    list.children(tls.error.map(|error| {
        div()
            .p(px(10.))
            .rounded(px(10.))
            .bg(rgb(c.error_ground))
            .text_color(rgb(c.error))
            .font_family(crate::FONT)
            .child(format!(
                "tracewing would not go on with this server: {error}. If the app trusts \
                 it some other way, such as through a company CA, choose \u{201c}Do not \
                 decrypt this host\u{201d} and the app will check the server itself."
            ))
    }))
    .children(known(tls.version).map(|v| row("TLS", v)))
    .children(known(tls.cipher).map(|v| row("Cipher", v)))
    .when(!refused, |d| {
        d.child(row("ALPN", tls.alpn.unwrap_or_else(|| "none".to_owned())))
    })
    .child(heading(chain_heading))
    .children(
        tls.chain
            .into_iter()
            .map(|cert| render_certificate(cert, now)),
    )
    .into_any_element()
}

/// How many of a certificate's names are listed before the rest are counted.
const NAMES_SHOWN: usize = 12;

fn render_certificate(cert: Certificate, now: i64) -> impl IntoElement {
    let c = colors();
    let late = cert.out_of_date(now);
    let when = if now < cert.not_before {
        format!("not valid until {}", jwt::relative(cert.not_before, now))
    } else if late {
        format!("expired {}", jwt::relative(cert.not_after, now))
    } else {
        format!("expires {}", jwt::relative(cert.not_after, now))
    };
    let names = match cert.names.len() {
        0 => None,
        n if n > NAMES_SHOWN => Some(format!(
            "{}, and {} more",
            cert.names[..NAMES_SHOWN].join(", "),
            n - NAMES_SHOWN
        )),
        _ => Some(cert.names.join(", ")),
    };
    let line = |text: String| div().child(text);
    div()
        .p(px(10.))
        .rounded(px(10.))
        .bg(rgb(c.surface))
        .flex()
        .flex_col()
        .gap(px(3.))
        .child(
            div()
                .font_family(crate::FONT)
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(c.text))
                .child(if cert.is_ca {
                    format!("{} · CA", cert.subject)
                } else {
                    cert.subject.clone()
                }),
        )
        .child(line(format!("Issued by {}", cert.issuer)))
        .child(
            div()
                .text_color(rgb(if late { c.error } else { c.text }))
                .child(format!(
                    "Valid {} to {} · {when}",
                    jwt::local_time(cert.not_before),
                    jwt::local_time(cert.not_after)
                )),
        )
        .children(names.map(|names| line(format!("Names {names}"))))
        .child(
            div()
                .text_color(rgb(c.muted))
                .child(format!("SHA-256 {}", cert.sha256)),
        )
        .child(
            div()
                .text_color(rgb(c.muted))
                .child(format!("Serial {}", cert.serial)),
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
        .font_features(gpui::FontFeatures::disable_ligatures())
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
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(value.clone())
                        .children(Jwt::find(value).map(render_jwt)),
                )
        }))
        .into_any_element()
}

/// Whether `part` has anything to show for this flow on `side`.
fn has_part(detail: &Detail, part: Part, side: Side) -> bool {
    match part {
        Part::Query => side == Side::Request && !fields::query(&detail.url).is_empty(),
        Part::Form => side == Side::Request && detail.form.as_ref().is_some_and(|f| !f.is_empty()),
        _ => true,
    }
}

fn render_fields(found: Vec<Field>) -> impl IntoElement {
    let c = colors();
    div()
        .id("fields")
        .size_full()
        .overflow_y_scroll()
        .p(px(12.))
        .flex()
        .flex_col()
        .gap(px(4.))
        .font_family(crate::MONO_FONT)
        .font_features(gpui::FontFeatures::disable_ligatures())
        .text_size(px(12.))
        .children(found.into_iter().map(|field| {
            let value = match field.value {
                FieldValue::Text(text) => div().child(text),
                FieldValue::File {
                    filename,
                    content_type,
                    bytes,
                } => div().text_color(rgb(c.muted)).child(format!(
                    "File {filename} · {} · {}",
                    content_type.unwrap_or_else(|| "no type".to_owned()),
                    size_text(bytes as u64)
                )),
            };
            div()
                .flex()
                .flex_row()
                .gap(px(10.))
                .child(
                    div()
                        .w(relative(0.34))
                        .flex_shrink_0()
                        .text_color(rgb(c.json_key))
                        .child(field.name),
                )
                .child(div().flex_1().min_w_0().child(value))
        }))
}

fn cookie_counts(detail: &Detail) -> (usize, usize) {
    match &detail.headers {
        Some((request, response)) => cookies::counts(request, response),
        None => (0, 0),
    }
}

fn render_cookies(headers: Option<&[(String, String)]>, side: Side) -> impl IntoElement {
    let c = colors();
    let Some(headers) = headers else {
        return note("These headers were not kept.").into_any_element();
    };
    let found = cookies::of(headers, side);
    if found.is_empty() {
        return note(match side {
            Side::Request => "No cookies were sent.",
            Side::Response => "No cookies were set.",
        })
        .into_any_element();
    }
    div()
        .id("cookies")
        .size_full()
        .overflow_y_scroll()
        .p(px(12.))
        .flex()
        .flex_col()
        .gap(px(8.))
        .font_family(crate::MONO_FONT)
        .font_features(gpui::FontFeatures::disable_ligatures())
        .text_size(px(12.))
        .children(found.into_iter().map(|cookie| {
            let jwt = Jwt::find(&cookie.value);
            div()
                .flex()
                .flex_row()
                .gap(px(10.))
                .child(
                    div()
                        .w(relative(0.34))
                        .flex_shrink_0()
                        .text_color(rgb(c.json_key))
                        .child(cookie.name),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(cookie.value)
                        .when(!cookie.attributes.is_empty(), |d| {
                            d.child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(rgb(c.muted))
                                    .child(cookie.attributes.join(" · ")),
                            )
                        })
                        .children(jwt.map(render_jwt)),
                )
        }))
        .into_any_element()
}

/// What a token claims, under the header or cookie that carries it.
fn render_jwt(jwt: Jwt) -> Div {
    let c = colors();
    let now = now_secs();
    let title = match jwt.algorithm() {
        Some(algorithm) => format!("JWT · {algorithm} · signature not checked"),
        None => "JWT · signature not checked".to_owned(),
    };
    let label = |text: &'static str| {
        div()
            .mt(px(4.))
            .text_size(px(11.))
            .text_color(rgb(c.muted))
            .child(text)
    };
    div()
        .mt(px(6.))
        .p(px(10.))
        .rounded(px(10.))
        .bg(rgb(c.surface))
        .flex()
        .flex_col()
        .gap(px(2.))
        .child(
            div()
                .font_family(crate::FONT)
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(c.text))
                .child(title),
        )
        .children(jwt.times().into_iter().map(|(label, at)| {
            let late = label == "Expires" && at <= now;
            div()
                .text_color(rgb(if late { c.error } else { c.text }))
                .child(format!(
                    "{label} {} · {}",
                    jwt::local_time(at),
                    jwt::relative(at, now)
                ))
        }))
        .child(label("Header"))
        .child(jwt.header_text())
        .child(label("Payload"))
        .child(jwt.payload_text())
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
