//! The inspector: one flow, in full. Bodies are unpacked to a file on a background
//! thread and mapped from there, so a body of any size opens without a pause.

use crate::{
    body::Body,
    bodyview::BodyView,
    colors::colors,
    connection::Certificate,
    cookies,
    detail::{Detail, Phase, Side, body_file},
    devices::device_of,
    fields::{self, Field, Value as FieldValue},
    flows::{size_text, time_text},
    input::TextInput,
    jwt::{self, Jwt},
    layout::Sides,
    messages::MessagesState,
    select::{self, Copied, Selection},
    snippets::{self, Language},
    traffic::Traffic,
    widgets::{chip, eyebrow, icon, status_colors, tip},
};
use gpui::{
    Anchor, AnyElement, ClipboardItem, Context, Div, ElementId, Entity, FontWeight, MouseButton,
    MouseDownEvent, ObjectFit, SharedString, StyledImage, anchored, deferred, div, img, prelude::*,
    px, relative, rgb,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracewing_i18n::{tr, tr_n};
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
    /// What a GraphQL request asks for, laid out.
    GraphQl,
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
    /// The other side's body, when both sides are shown at once.
    pub other: BodyState,
    /// Of the flow chosen, if it is a stream.
    pub messages: Option<MessagesState>,
    /// Which load the body is waiting for, so that a late one is ignored.
    load: u64,
    /// The same for the other side's.
    other_load: u64,
    copied: bool,
    /// The menu of languages to copy the request as is open.
    pub copy_menu: bool,
    /// The chosen flow as a call to a model, once its bodies are read.
    pub llm: Option<(crate::llm::Provider, crate::llm::Exchange)>,
    /// What the chosen flow's GraphQL request sends, once its body is read.
    pub graphql: Option<Vec<tracewing_store::graphql::Sent>>,
    /// What is selected of the part shown, when it is not a body.
    pub select: Selection,
}

impl InspectorState {
    fn slot(&mut self, other: bool) -> &mut BodyState {
        if other {
            &mut self.other
        } else {
            &mut self.body
        }
    }

    /// The body of `side`, whichever is chosen, when both are shown.
    fn body_of(&self, side: Side) -> &BodyState {
        if side == self.side {
            &self.body
        } else {
            &self.other
        }
    }
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
                if self.inspector.detail.as_ref().map(|d| d.id) != Some(id) {
                    self.inspector.select.clear();
                }
                self.inspector.part = match (stream, self.inspector.part) {
                    (true, _) => Part::Messages,
                    (_, Part::Llm) if a_call => Part::Llm,
                    (false, Part::Messages | Part::Llm) => Part::Body,
                    (false, Part::Cookies) if no_cookies => Part::Body,
                    (false, Part::GraphQl) if detail.graphql.is_none() => Part::Body,
                    (false, Part::Query | Part::Form) if !has_part(&detail, part_now, side_now) => {
                        Part::Body
                    }
                    (false, part) => part,
                };
                self.inspector.detail = Some(detail);
                self.inspector.copied = false;
                self.inspector.llm = None;
                self.load_llm(cx);
                self.load_graphql(cx);
                self.show_device_name(cx);
                self.take_new_messages();
                self.load_body(cx);
            }
            Err(_) => {
                // It went with the oldest flows between the click and now.
                self.inspector.detail = None;
                self.inspector.body = BodyState::Note(tr!("This flow is gone.").into());
            }
        }
    }

    fn load_body(&mut self, cx: &mut Context<Self>) {
        self.load_side(false, cx);
        if self.layout.sides != Sides::Tabs {
            self.load_side(true, cx);
        } else {
            self.inspector.other = BodyState::default();
        }
    }

    /// One side's body: the side chosen's, or with `other` the other side's,
    /// which is shown when both sides are.
    fn load_side(&mut self, other: bool, cx: &mut Context<Self>) {
        let Some(detail) = &self.inspector.detail else {
            return;
        };
        let side = if other {
            self.inspector.side.other()
        } else {
            self.inspector.side
        };
        let facts = detail.body(side).clone();
        let id = detail.id;
        let counter = if other {
            &mut self.inspector.other_load
        } else {
            &mut self.inspector.load
        };
        *counter += 1;
        let load = *counter;
        if !facts.stored.exists() {
            *self.inspector.slot(other) = BodyState::Note(
                if detail.kind == "tunnel" {
                    tr!("This host is not decrypted, so what it sends cannot be seen.")
                } else if facts.bytes == 0 {
                    tr!("No body.")
                } else if facts.shed {
                    tr!("This body was not kept: the disk was nearly full, or too much was waiting for it.")
                } else {
                    tr!("This body was not kept.")
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
        *self.inspector.slot(other) = BodyState::Loading;
        let session = self.session_dir().to_owned();
        let max = self.engine.settings.view_max();
        cx.spawn(async move |this, cx| {
            let file = cx
                .background_spawn(async move { body_file(&session, &facts, id, side, max) })
                .await;
            let _ = this.update(cx, |this, cx| {
                let now = if other {
                    this.inspector.other_load
                } else {
                    this.inspector.load
                };
                if now != load {
                    return;
                }
                *this.inspector.slot(other) = match file {
                    None => BodyState::Note(
                        tr!("This body is gone: it was dropped with the oldest flows, or cannot be read.")
                            .into(),
                    ),
                    Some((_, 0)) => BodyState::Note(tr!("No body.").into()),
                    Some((path, size)) if picture && size <= PICTURE_MAX => {
                        BodyState::Image(path, size)
                    }
                    Some((path, _)) => match Body::open(&path) {
                        Ok(body) => BodyState::Shown(
                            {
                                let focus = this.focus.clone();
                                cx.new(|cx| {
                                    BodyView::new(body, content_type.as_deref(), focus, cx)
                                })
                            },
                        ),
                        Err(e) => BodyState::Note(
                            tr!("Cannot open this body: {e}", e = format!("{e:#}")).into(),
                        ),
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
            self.inspector.select.clear();
            self.choose_side(side, cx);
        }
    }

    /// How both sides are shown, or one at a time. Kept for the next start.
    pub fn set_sides(&mut self, sides: Sides, cx: &mut Context<Self>) {
        if self.layout.sides == sides {
            return;
        }
        self.layout.sides = sides;
        self.save_layout();
        self.inspector.select.clear();
        if sides == Sides::Tabs {
            self.inspector.other = BodyState::default();
        } else if matches!(self.inspector.other, BodyState::Loading) {
            self.load_side(true, cx);
        }
        cx.notify();
    }

    /// Makes `side` the one chosen, which the body's buttons and keys act on.
    /// With both sides shown, the bodies trade places and nothing is read
    /// again; what is selected stays, as it may span both.
    fn choose_side(&mut self, side: Side, cx: &mut Context<Self>) {
        if self.inspector.side != side {
            self.inspector.side = side;
            let both = self.layout.sides != Sides::Tabs;
            let loading = matches!(self.inspector.body, BodyState::Loading)
                || matches!(self.inspector.other, BodyState::Loading);
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
            if both && !loading {
                let inspector = &mut self.inspector;
                std::mem::swap(&mut inspector.body, &mut inspector.other);
                std::mem::swap(&mut inspector.load, &mut inspector.other_load);
            } else {
                self.load_body(cx);
            }
            cx.notify();
        }
    }

    pub fn set_part(&mut self, part: Part, cx: &mut Context<Self>) {
        if self.inspector.part != part {
            self.inspector.select.clear();
        }
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
            .child(eyebrow(tr!("Inspector")));
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
                    .child(tr!("Choose a flow to see it here")),
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
            tr!("Not decrypted. New connections to this host go through unread.")
        } else {
            tr!("If an app fails with this host, it may pin its certificates.")
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
                    .child(tr!("Make a rule"))
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
                        tr!("Decrypt this host")
                    } else {
                        tr!("Do not decrypt this host")
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
        let tab = |id: &'static str, label: String, this: Side| {
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
        let sides = self.layout.sides;
        let switch = |id: &'static str, name: &'static str, said: String, this: Sides| {
            let on = sides == this;
            div()
                .id(id)
                .size(px(30.))
                .flex_shrink_0()
                .rounded(px(14.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .when(on, |d| d.bg(rgb(c.tab_on)))
                .tooltip(tip(said))
                .child(icon(name, 16., if on { c.text } else { c.muted }))
                .on_click(cx.listener(move |view, _, _, cx| view.set_sides(this, cx)))
        };
        div()
            .flex()
            .flex_row()
            .flex_shrink_0()
            .gap(px(4.))
            .p(px(4.))
            .rounded(px(18.))
            .bg(rgb(c.code))
            // Both shown, each pane says which it is.
            .when(sides == Sides::Tabs, |d| {
                d.child(tab("side-request", tr!("Request"), Side::Request))
                    .child(tab("side-response", tr!("Response"), Side::Response))
            })
            .when(sides != Sides::Tabs, |d| {
                d.child(
                    div()
                        .flex_1()
                        .h(px(30.))
                        .px(px(12.))
                        .flex()
                        .items_center()
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(c.muted))
                        .child(tr!("Request and response")),
                )
            })
            .child(div().w(px(1.)).my(px(6.)).bg(rgb(c.line)))
            .child(switch(
                "sides-tabs",
                "tabs",
                tr!("Request and response one at a time"),
                Sides::Tabs,
            ))
            .child(switch(
                "sides-beside",
                "beside",
                tr!("Request and response side by side"),
                Sides::Beside,
            ))
            .child(switch(
                "sides-stacked",
                "below",
                tr!("Request above the response"),
                Sides::Stacked,
            ))
    }

    /// Shows the chosen side's body as a dump of its bytes, or as it was told
    /// to be.
    pub fn set_body_hex(&mut self, hex: bool, cx: &mut Context<Self>) {
        if let BodyState::Shown(view) = &self.inspector.body {
            view.update(cx, |view, cx| view.set_hex(hex, cx));
            cx.notify();
        }
    }

    /// The body as it reads, or its bytes: two halves of one switch. A body
    /// that is bytes already has only the one.
    fn render_hex_switch(&self, part: Part, cx: &mut Context<Self>) -> Option<Div> {
        let BodyState::Shown(view) = &self.inspector.body else {
            return None;
        };
        let view = view.read(cx);
        let kind = view.body().kind();
        if part != Part::Body || kind == crate::body::Kind::Binary {
            return None;
        }
        let c = colors();
        let hex = view.is_hex();
        let half = |id: &'static str, label: String, on: bool, to_hex: bool| {
            div()
                .id(id)
                .h(px(24.))
                .px(px(10.))
                .rounded(px(12.))
                .flex()
                .items_center()
                .cursor_pointer()
                .text_size(px(12.))
                .font_weight(FontWeight::BOLD)
                .when(on, |d| d.bg(rgb(c.tab_on)).text_color(rgb(c.text)))
                .when(!on, |d| d.text_color(rgb(c.muted)))
                .child(label)
                .on_click(cx.listener(move |view, _, _, cx| view.set_body_hex(to_hex, cx)))
        };
        let read = match kind {
            crate::body::Kind::Json => "JSON".to_owned(),
            _ => tr!("Text"),
        };
        Some(
            div()
                .flex()
                .flex_row()
                .p(px(2.))
                .rounded(px(14.))
                .bg(rgb(c.chip))
                .child(half("body-as-read", read, !hex, false))
                .child(half("body-as-hex", "Hex".to_owned(), hex, true)),
        )
    }

    /// The parts the chosen flow has tabs for, in the tabs' order.
    pub fn parts_offered(&self) -> Vec<Part> {
        let Some(detail) = &self.inspector.detail else {
            return Vec::new();
        };
        let both = self.layout.sides != Sides::Tabs;
        let side = self.inspector.side;
        let (sent, set) = cookie_counts(detail);
        let mut parts = Vec::new();
        if self.inspector.llm.is_some()
            || crate::llm::provider_of(&detail.method, &detail.url).is_some()
        {
            parts.push(Part::Llm);
        }
        if self.inspector.messages.is_some() {
            parts.push(Part::Messages);
        }
        if detail.graphql.is_some() {
            parts.push(Part::GraphQl);
        }
        parts.extend([Part::Body, Part::Headers]);
        if sent + set > 0 {
            parts.push(Part::Cookies);
        }
        if both || side == Side::Request {
            if !fields::query(&detail.url).is_empty() {
                parts.push(Part::Query);
            }
            if detail.form.as_ref().is_some_and(|form| !form.is_empty()) {
                parts.push(Part::Form);
            }
        }
        parts.push(Part::Connection);
        parts
    }

    /// The part `by` tabs from the one shown, round the ends.
    pub fn step_part(&mut self, by: i32, cx: &mut Context<Self>) {
        let parts = self.parts_offered();
        if parts.is_empty() {
            return;
        }
        let at = parts
            .iter()
            .position(|part| *part == self.inspector.part)
            .unwrap_or(0) as i32;
        let next = (at + by).rem_euclid(parts.len() as i32) as usize;
        self.set_part(parts[next], cx);
    }

    fn render_parts(&self, detail: &Detail, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let part = self.inspector.part;
        let side = self.inspector.side;
        let both = self.layout.sides != Sides::Tabs;
        let headers = if both {
            [Side::Request, Side::Response]
                .iter()
                .map(|side| detail.headers(*side).map_or(0, <[_]>::len))
                .sum()
        } else {
            detail.headers(side).map_or(0, <[_]>::len)
        };
        let (sent, set) = cookie_counts(detail);
        let cookies = match (both, side) {
            (true, _) => sent + set,
            (false, Side::Request) => sent,
            (false, Side::Response) => set,
        };
        // Both shown, each pane says its own.
        let summary_text: Option<String> = match (&self.inspector.body, part) {
            _ if both && part == Part::Body => None,
            (BodyState::Shown(view), Part::Body) => Some(view.read(cx).summary()),
            (BodyState::Image(_, size), Part::Body) => {
                Some(tr!("Picture · {size}", size = size_text(*size)))
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
                    tr!(
                        "Messages {count}",
                        count = crate::flows::group_digits(state.list.len() as u64)
                    ),
                    Part::Messages,
                )
            }))
            .when(detail.graphql.is_some(), |d| {
                d.child(tab("part-graphql", "GraphQL".to_owned(), Part::GraphQl))
            })
            .child(tab("part-body", tr!("Body"), Part::Body))
            .child(tab(
                "part-headers",
                tr!("Headers {headers}", headers = headers),
                Part::Headers,
            ))
            .when(sent + set > 0, |d| {
                d.child(tab(
                    "part-cookies",
                    tr!("Cookies {cookies}", cookies = cookies),
                    Part::Cookies,
                ))
            })
            .when(both || side == Side::Request, |d| {
                let query = fields::query(&detail.url).len();
                let form = detail.form.as_ref().map_or(0, Vec::len);
                d.when(query > 0, |d| {
                    d.child(tab(
                        "part-query",
                        tr!("Query {query}", query = query),
                        Part::Query,
                    ))
                })
                .when(form > 0, |d| {
                    d.child(tab(
                        "part-form",
                        tr!("Form {form}", form = form),
                        Part::Form,
                    ))
                })
            })
            .child(tab("part-connection", tr!("Connection"), Part::Connection))
            .child(div().flex_1())
            .children(summary_text.map(|text| {
                div()
                    .text_size(px(12.))
                    .text_color(rgb(c.muted))
                    .child(text)
            }))
            .children(self.render_hex_switch(part, cx))
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
                            .child(tr!("Save body"))
                            .on_click(cx.listener(|view, _, _, cx| view.ask_save_body(cx))),
                    )
                },
            )
            .when(
                !matches!(detail.kind.as_str(), "tunnel" | "websocket"),
                |d| {
                    let small = |id: &'static str, label: String| {
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
                                small("send-again", tr!("Send again")).on_click(
                                    cx.listener(|view, _, _, cx| view.send_flow_again(cx)),
                                ),
                            )
                            .child(
                                small("edit-send", tr!("Edit and send"))
                                    .on_click(cx.listener(|view, _, _, cx| view.edit_flow(cx))),
                            )
                            .child(
                                small(
                                    "copy-curl",
                                    if self.inspector.copied {
                                        tr!("Copied")
                                    } else {
                                        tr!("Copy as cURL")
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
                    .child(tr!("Copy as…"))
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

    fn render_part(&mut self, detail: &Detail, cx: &mut Context<Self>) -> Div {
        let c = colors();
        let well = div()
            .flex_1()
            .min_h_0()
            .rounded(px(16.))
            .overflow_hidden()
            .bg(rgb(c.code));
        let sel = self.inspector.select.clone();
        sel.begin();
        let part = self.inspector.part;
        if self.layout.sides != Sides::Tabs
            && matches!(part, Part::Body | Part::Headers | Part::Cookies)
        {
            return self.render_both(detail, &sel, cx);
        }
        match part {
            Part::Messages => well.child(self.render_messages(cx)),
            Part::Llm => self.selectable(well, &sel, cx).child(self.render_llm(&sel)),
            Part::GraphQl => self
                .selectable(well, &sel, cx)
                .child(self.render_graphql(&sel)),
            Part::Headers => self
                .selectable(well, &sel, cx)
                .child(render_headers(detail.headers(self.inspector.side), &sel)),
            Part::Connection => {
                let naming = (detail.app.is_none() && device_of(&detail.client).is_some())
                    .then(|| self.device_name_input().clone());
                self.selectable(well, &sel, cx)
                    .child(render_connection(detail, naming, &sel))
            }
            Part::Query => self
                .selectable(well, &sel, cx)
                .child(render_fields(fields::query(&detail.url), &sel)),
            Part::Form => self
                .selectable(well, &sel, cx)
                .child(render_fields(detail.form.clone().unwrap_or_default(), &sel)),
            Part::Cookies => self.selectable(well, &sel, cx).child(render_cookies(
                detail.headers(self.inspector.side),
                self.inspector.side,
                &sel,
            )),
            Part::Body => body_well(well, &self.inspector.body),
        }
    }

    /// The part chosen, for the request and the response at once: beside
    /// each other or one above the other. A press in a pane makes its side
    /// the one chosen.
    fn render_both(&mut self, detail: &Detail, sel: &Selection, cx: &mut Context<Self>) -> Div {
        let c = colors();
        let part = self.inspector.part;
        let chosen = self.inspector.side;
        let mut panes = Vec::new();
        for side in [Side::Request, Side::Response] {
            let well = div()
                .flex_1()
                .min_h_0()
                .rounded(px(16.))
                .overflow_hidden()
                .bg(rgb(c.code))
                .border_1()
                .border_color(rgb(if side == chosen { c.link } else { c.code }));
            let content = match part {
                Part::Headers => self
                    .selectable(well, sel, cx)
                    .child(render_headers(detail.headers(side), sel)),
                Part::Cookies => self.selectable(well, sel, cx).child(render_cookies(
                    detail.headers(side),
                    side,
                    sel,
                )),
                _ => body_well(well, self.inspector.body_of(side)),
            };
            let size = match (part, self.inspector.body_of(side)) {
                (Part::Body, BodyState::Shown(view)) => Some(view.read(cx).summary()),
                (Part::Body, BodyState::Image(_, size)) => {
                    Some(tr!("Picture · {size}", size = size_text(*size)))
                }
                _ => None,
            };
            let name = match side {
                Side::Request => tr!("Request"),
                Side::Response => tr!("Response"),
            };
            panes.push(
                div()
                    .id(match side {
                        Side::Request => "pane-request",
                        Side::Response => "pane-response",
                    })
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |view, _, _, cx| view.choose_side(side, cx)),
                    )
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |view, _, _, cx| view.choose_side(side, cx)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(px(8.))
                            .text_size(px(12.))
                            .child(
                                div()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(if side == chosen { c.text } else { c.muted }))
                                    .child(name),
                            )
                            .children(size.map(|size| div().text_color(rgb(c.muted)).child(size))),
                    )
                    .child(content),
            );
        }
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .gap(px(8.))
            .map(|d| {
                if self.layout.sides == Sides::Stacked {
                    d.flex_col()
                } else {
                    d.flex_row()
                }
            })
            .children(panes)
    }
}

/// A body in its well, or why it is not there.
fn body_well(well: Div, state: &BodyState) -> Div {
    match state {
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
                            note(tr!(
                                "This picture cannot be drawn. Save the body to look at it."
                            ))
                            .into_any_element()
                        }),
                ),
        ),
        BodyState::Loading => well.child(note(tr!("Opening…"))),
        BodyState::Note(text) => well.child(note(text.clone())),
    }
}

impl Traffic {
    /// A view whose text can be selected with `sel`: a press starts a
    /// selection, a drag moves its end, and a right click opens the menu to
    /// copy it.
    pub(crate) fn selectable(&mut self, view: Div, sel: &Selection, cx: &mut Context<Self>) -> Div {
        let me = cx.entity().downgrade();
        let redraw = {
            let me = me.clone();
            move |cx: &mut gpui::App| {
                let _ = me.update(cx, |_, cx| cx.notify());
            }
        };
        let follow = select::follow_drag(sel, {
            let (sel, redraw) = (sel.clone(), redraw.clone());
            move |at, _, cx| {
                if sel.drag_to(at) {
                    redraw(cx);
                }
            }
        });
        let menu = sel.menu().map(|at| {
            let (copy, all, close) = (sel.clone(), sel.clone(), sel.clone());
            let (r1, r2, r3) = (redraw.clone(), redraw.clone(), redraw.clone());
            select::copy_menu(
                at,
                !sel.is_empty(),
                true,
                move |_, cx| {
                    copy.set_menu(None);
                    write_copied(copy.selected_text(), cx);
                    r1(cx);
                },
                move |_, cx| {
                    all.set_menu(None);
                    write_copied(all.all_text(), cx);
                    r2(cx);
                },
                move |_, cx| {
                    close.set_menu(None);
                    r3(cx);
                },
            )
        });
        let (press, menu_at) = (sel.clone(), sel.clone());
        view.relative()
            .cursor(gpui::CursorStyle::IBeam)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                    window.focus(&this.focus, cx);
                    press.press(e.position, e.click_count, e.modifiers.shift);
                    cx.notify();
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                    window.focus(&this.focus, cx);
                    menu_at.set_menu(Some(e.position));
                    cx.notify();
                }),
            )
            .children(follow)
            .children(menu)
    }

    /// Copies what is selected in the part shown, a body's or another's.
    /// False when nothing is.
    pub fn copy_selection(&mut self, cx: &mut Context<Self>) -> bool {
        self.inspector.select.set_menu(None);
        let copied = match (&self.inspector.part, &self.inspector.body) {
            (Part::Body, BodyState::Shown(view)) => view.read(cx).selected_text(),
            (Part::Messages, _) => match self
                .inspector
                .messages
                .as_ref()
                .and_then(|m| m.view.as_ref())
            {
                Some(view) => view.read(cx).selected_text(),
                None => Copied::Nothing,
            },
            _ => self.inspector.select.selected_text(),
        };
        cx.notify();
        write_copied(copied, cx)
    }

    /// Closes the menu to copy with, wherever it is open. False if it is not.
    pub(crate) fn close_copy_menu(&mut self, cx: &mut Context<Self>) -> bool {
        let mut views: Vec<Entity<BodyView>> = Vec::new();
        for state in [&self.inspector.body, &self.inspector.other] {
            if let BodyState::Shown(view) = state {
                views.push(view.clone());
            }
        }
        if let Some(view) = self
            .inspector
            .messages
            .as_ref()
            .and_then(|m| m.view.clone())
        {
            views.push(view);
        }
        let mut closed = false;
        for view in views {
            let sel = view.read(cx).selection().clone();
            if sel.menu().is_some() {
                sel.set_menu(None);
                view.update(cx, |_, cx| cx.notify());
                closed = true;
            }
        }
        let compose = self.compose_page.as_ref().map(|page| page.select.clone());
        for sel in [Some(self.inspector.select.clone()), compose]
            .into_iter()
            .flatten()
        {
            if sel.menu().is_some() {
                sel.set_menu(None);
                cx.notify();
                closed = true;
            }
        }
        closed
    }

    /// Selects all of the part shown.
    pub fn select_all(&mut self, cx: &mut Context<Self>) -> bool {
        match (&self.inspector.part, &self.inspector.body) {
            (Part::Body, BodyState::Shown(view)) => view.update(cx, |v, cx| v.select_all(cx)),
            (Part::Messages, _) => match self
                .inspector
                .messages
                .as_ref()
                .and_then(|m| m.view.clone())
            {
                Some(view) => view.update(cx, |v, cx| v.select_all(cx)),
                None => return false,
            },
            (
                Part::Headers
                | Part::Cookies
                | Part::Connection
                | Part::Query
                | Part::Form
                | Part::Llm,
                _,
            ) => {
                self.inspector.select.select_all();
                cx.notify();
            }
            _ => return false,
        }
        true
    }
}

/// Puts a copy on the clipboard. False when there was nothing to put.
fn write_copied(copied: Copied, cx: &mut gpui::App) -> bool {
    match copied {
        Copied::Text(text) => {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
            true
        }
        Copied::Nothing | Copied::TooLarge => false,
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
        None => tr!("No response"),
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
            .child(plain(tr!("Tunnel, not decrypted")));
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
                .map(|rule| chip(tr!("Rule: {rule}", rule = rule), c.tonal, c.on_tonal)),
        )
        .children(detail.graphql.clone().map(|operation| {
            chip(
                tr!("GraphQL: {operation}", operation = operation),
                c.tonal,
                c.on_tonal,
            )
        }))
        .child(plain(detail.protocols()))
        .children(if detail.open {
            Some(chip(tr!("Open"), c.ok_ground, c.ok))
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
            .child(
                div()
                    .w(px(64.))
                    .text_color(rgb(c.muted))
                    .child(phase.name()),
            )
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
        (Some(id), None) => Some(tr!(
            "Over connection {id}, made before: no time went on connecting.",
            id = id.to_string()
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
fn render_connection(
    detail: &Detail,
    naming: Option<Entity<TextInput>>,
    sel: &Selection,
) -> AnyElement {
    let c = colors();
    let row = |label: String, value: String| {
        div()
            .flex()
            .flex_row()
            .gap(px(10.))
            .child(
                div()
                    .w(relative(0.26))
                    .flex_shrink_0()
                    .text_color(rgb(c.json_key))
                    .child(sel.piece(label, "\n", Vec::new())),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(sel.piece(value, ": ", Vec::new())),
            )
    };
    let heading = |text: String| {
        div()
            .pt(px(6.))
            .font_family(crate::FONT)
            .text_size(px(11.))
            .font_weight(FontWeight::BOLD)
            .text_color(rgb(c.muted))
            .child(sel.piece(text, "\n\n", Vec::new()))
    };
    let app = match (&detail.app, detail.app_pid) {
        (Some(app), Some(pid)) => tr!("{app}, pid {pid}", app = app, pid = pid.to_string()),
        (Some(app), None) => app.clone(),
        _ if device_of(&detail.client).is_none() => {
            tr!("Not found: it may have ended, or belong to another user")
        }
        _ => tr!("On another device"),
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
        .child(heading(tr!("CLIENT")))
        .child(row(tr!("App"), app))
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
                        .child(tr!("Device")),
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
        .children(
            detail
                .app_path
                .clone()
                .map(|path| row(tr!("Program"), path)),
        )
        .child(row(tr!("Address"), detail.client.clone()))
        .child(heading(tr!("SERVER")));
    let Some(connection) = detail.connection.clone() else {
        let why = match (detail.conn_id, detail.kind.as_str()) {
            (Some(_), _) => {
                tr!("What this connection was is no longer kept: it went with older flows.")
            }
            (None, "tunnel") => tr!("Not decrypted: tracewing passed it on without looking."),
            (None, _) if detail.error.is_some() => tr!("No connection was made."),
            (None, _) => tr!("None: it was answered without one."),
        };
        return list
            .child(
                div()
                    .font_family(crate::FONT)
                    .text_color(rgb(c.muted))
                    .child(sel.piece(why, "\n", Vec::new())),
            )
            .into_any_element();
    };
    let refused = connection
        .tls
        .as_ref()
        .is_some_and(|tls| tls.error.is_some());
    let made = match (detail.conn_id, detail.connect_us) {
        (Some(id), _) if refused => tr!("{id}, given up during TLS", id = id.to_string()),
        (Some(id), Some(_)) => tr!("{id}, made for this flow", id = id.to_string()),
        (Some(id), None) => tr!("{id}, made before and used again", id = id.to_string()),
        _ => "–".to_owned(),
    };
    let list = list
        .child(row(tr!("Server"), connection.remote.clone()))
        .child(row(tr!("Connection"), made));
    let Some(tls) = connection.tls else {
        return list
            .child(row("TLS".to_owned(), tr!("None: plain HTTP")))
            .into_any_element();
    };
    let now = now_secs();
    let refused = tls.error.is_some();
    let chain_heading = match (refused, tls.chain.len()) {
        (true, 0) => tr!("THE SERVER SHOWED NO CERTIFICATES"),
        (true, _) => tr!("WHAT THE SERVER SHOWED, ITS OWN FIRST"),
        (false, 0) => tr!("NO CERTIFICATES"),
        (false, 1) => tr!("CERTIFICATE"),
        (false, n) => tr_n!(
            n,
            "{n} CERTIFICATE, THE SERVER'S FIRST",
            "{n} CERTIFICATES, THE SERVER'S FIRST"
        ),
    };
    let known = |text: String| (!text.is_empty()).then_some(text);
    list.children(tls.error.map(|error| {
        div()
            .p(px(10.))
            .rounded(px(10.))
            .bg(rgb(c.error_ground))
            .text_color(rgb(c.error))
            .font_family(crate::FONT)
            .child(sel.piece(
                tr!(
                    "tracewing would not go on with this server: {error}. If the app trusts it some other way, such as through a company CA, choose “Do not decrypt this host” and the app will check the server itself.",
                    error = error
                ),
                "\n",
                Vec::new(),
            ))
    }))
    .children(known(tls.version).map(|v| row("TLS".to_owned(), v)))
    .children(known(tls.cipher).map(|v| row(tr!("Cipher"), v)))
    .when(!refused, |d| {
        d.child(row("ALPN".to_owned(), tls.alpn.unwrap_or_else(|| tr!("none"))))
    })
    .child(heading(chain_heading))
    .children(
        tls.chain
            .into_iter()
            .map(|cert| render_certificate(cert, now, sel)),
    )
    .into_any_element()
}

/// How many of a certificate's names are listed before the rest are counted.
const NAMES_SHOWN: usize = 12;

fn render_certificate(cert: Certificate, now: i64, sel: &Selection) -> impl IntoElement {
    let c = colors();
    let late = cert.out_of_date(now);
    let when = if now < cert.not_before {
        tr!(
            "not valid until {when}",
            when = jwt::relative(cert.not_before, now)
        )
    } else if late {
        tr!("expired {when}", when = jwt::relative(cert.not_after, now))
    } else {
        tr!("expires {when}", when = jwt::relative(cert.not_after, now))
    };
    let names = match cert.names.len() {
        0 => None,
        n if n > NAMES_SHOWN => Some(tr_n!(
            n - NAMES_SHOWN,
            "{names}, and {n} more",
            "{names}, and {n} more",
            names = cert.names[..NAMES_SHOWN].join(", ")
        )),
        _ => Some(cert.names.join(", ")),
    };
    let line = |text: String| div().child(sel.piece(text, "\n", Vec::new()));
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
                .child(sel.piece(
                    if cert.is_ca {
                        format!("{} · CA", cert.subject)
                    } else {
                        cert.subject.clone()
                    },
                    "\n\n",
                    Vec::new(),
                )),
        )
        .child(line(tr!("Issued by {issuer}", issuer = &cert.issuer)))
        .child(
            div()
                .text_color(rgb(if late { c.error } else { c.text }))
                .child(line(tr!(
                    "Valid {from} to {to} · {when}",
                    from = jwt::local_time(cert.not_before),
                    to = jwt::local_time(cert.not_after),
                    when = when
                ))),
        )
        .children(names.map(|names| line(tr!("Names {names}", names = names))))
        .child(
            div()
                .text_color(rgb(c.muted))
                .child(line(format!("SHA-256 {}", cert.sha256))),
        )
        .child(
            div()
                .text_color(rgb(c.muted))
                .child(line(tr!("Serial {serial}", serial = &cert.serial))),
        )
}

fn render_headers(headers: Option<&[(String, String)]>, sel: &Selection) -> impl IntoElement {
    let c = colors();
    let Some(headers) = headers else {
        return note(tr!("These headers were not kept.")).into_any_element();
    };
    if headers.is_empty() {
        return note(tr!("No headers.")).into_any_element();
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
                        .child(sel.piece(name.clone(), "\n", Vec::new())),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(sel.piece(value.clone(), ": ", Vec::new()))
                        .children(Jwt::find(value).map(|jwt| render_jwt(jwt, sel))),
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

fn render_fields(found: Vec<Field>, sel: &Selection) -> impl IntoElement {
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
            let name = sel.piece(field.name, "\n", Vec::new());
            let value = match field.value {
                FieldValue::Text(text) => div().child(sel.piece(text, ": ", Vec::new())),
                FieldValue::File {
                    filename,
                    content_type,
                    bytes,
                } => div().text_color(rgb(c.muted)).child(sel.piece(
                    tr!(
                        "File {filename} · {kind} · {size}",
                        filename = filename,
                        kind = content_type.unwrap_or_else(|| tr!("no type")),
                        size = size_text(bytes as u64)
                    ),
                    ": ",
                    Vec::new(),
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
                        .child(name),
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

fn render_cookies(
    headers: Option<&[(String, String)]>,
    side: Side,
    sel: &Selection,
) -> impl IntoElement {
    let c = colors();
    let Some(headers) = headers else {
        return note(tr!("These headers were not kept.")).into_any_element();
    };
    let found = cookies::of(headers, side);
    if found.is_empty() {
        return note(match side {
            Side::Request => tr!("No cookies were sent."),
            Side::Response => tr!("No cookies were set."),
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
                        .child(sel.piece(cookie.name, "\n", Vec::new())),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(sel.piece(cookie.value, ": ", Vec::new()))
                        .when(!cookie.attributes.is_empty(), |d| {
                            d.child(
                                div().text_size(px(11.5)).text_color(rgb(c.muted)).child(
                                    sel.piece(cookie.attributes.join("; "), "; ", Vec::new()),
                                ),
                            )
                        })
                        .children(jwt.map(|jwt| render_jwt(jwt, sel))),
                )
        }))
        .into_any_element()
}

/// What a token claims, under the header or cookie that carries it.
fn render_jwt(jwt: Jwt, sel: &Selection) -> Div {
    let c = colors();
    let now = now_secs();
    let title = match jwt.algorithm() {
        Some(algorithm) => tr!(
            "JWT · {algorithm} · signature not checked",
            algorithm = algorithm
        ),
        None => tr!("JWT · signature not checked"),
    };
    let label = |text: String| {
        div()
            .mt(px(4.))
            .text_size(px(11.))
            .text_color(rgb(c.muted))
            .child(sel.piece(text, "\n", Vec::new()))
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
                .child(sel.piece(title, "\n\n", Vec::new())),
        )
        .children(jwt.times().into_iter().map(|(label, at)| {
            let late = label == "Expires" && at <= now;
            div()
                .text_color(rgb(if late { c.error } else { c.text }))
                .child(sel.piece(
                    format!(
                        "{} {} · {}",
                        jwt::time_label(label),
                        jwt::local_time(at),
                        jwt::relative(at, now)
                    ),
                    "\n",
                    Vec::new(),
                ))
        }))
        .child(label(tr!("Header")))
        .child(sel.piece(jwt.header_text(), "\n", Vec::new()))
        .child(label(tr!("Payload")))
        .child(sel.piece(jwt.payload_text(), "\n", Vec::new()))
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
