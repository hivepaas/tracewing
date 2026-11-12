//! The inspector: one flow, in full. Bodies are unpacked to a file on a background
//! thread and mapped from there, so a body of any size opens without a pause.

use crate::{
    body::Body,
    bodyview::{BodyMode, BodyView},
    colors::colors,
    connection::Certificate,
    cookies,
    detail::{BodyFacts, Detail, Phase, Side, body_file, host_and_path},
    devices::device_of,
    fields::{self, Field, Value as FieldValue},
    flows::{size_text, time_text},
    input::TextInput,
    jwt::{self, Jwt},
    layout::Sides,
    messages::MessagesState,
    reading,
    remembered::Remembered,
    select::{self, Copied, Selection},
    snippets::{self, Language},
    traffic::Traffic,
    widgets::{Reachable, chip, eyebrow, icon, status_colors, tip},
};
use gpui::{
    Anchor, AnyElement, ClipboardItem, Context, Div, ElementId, Entity, FontWeight, MouseButton,
    MouseDownEvent, ObjectFit, SharedString, StyledImage, anchored, deferred, div, img, prelude::*,
    px, relative, rgb,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tracewing_convert::{Kind, Reading, Shown};
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
    /// What the JSON-RPC calls of a request hold, with the hex read.
    Rpc,
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
    /// The list of messages to read a Protobuf body as is open, with what
    /// narrows it.
    pub read_as: Option<Entity<crate::input::TextInput>>,
    /// The chosen flow as a call to a model, once its bodies are read.
    pub llm: Option<(crate::llm::Provider, crate::llm::Exchange)>,
    /// What the chosen flow's GraphQL request sends, once its body is read.
    pub graphql: Option<Vec<tracewing_store::graphql::Sent>>,
    /// What the chosen flow's JSON-RPC calls hold, once its bodies are read.
    pub rpc: Option<crate::rpc_view::Shown>,
    /// What is selected of the part shown, when it is not a body.
    pub select: Selection,
    /// How bodies are shown, kept from one flow to the next.
    pub body_mode: BodyMode,
    /// CSV and TSV shown as text rather than as a table, as last chosen.
    pub tables_as_text: bool,
    /// The last call before the chosen flow to the same method, host and
    /// path, which it can be compared with in a click.
    pub previous_call: Option<i64>,
    /// What the selection was read as, worked out when the menu opened: a
    /// frame must not read the selection again on every draw.
    pub readings: Vec<Reading>,
    /// The field and the value those readings came from, which is where a
    /// chosen one is drawn. None where the selection sits under no field.
    read_from: Option<(SharedString, SharedString)>,
    /// The readings chosen, by [`drawn_key`].
    drawn: HashMap<String, Reading>,
    /// How fields are always read, once a reader has said so. Shared with the
    /// body views, so that remembering in one is remembered in the other.
    pub remembered: Rc<RefCell<Remembered>>,
    /// What those rules read, by [`drawn_key`], and the rules' version they
    /// were read at: a frame must not read a value again on every draw.
    by_rule: RefCell<(u64, HashMap<String, Option<Reading>>)>,
    /// Which values are worth underlining, by [`drawn_key`]. The same reason,
    /// and the same answer kept.
    marks: RefCell<HashMap<String, bool>>,
    /// A reading being looked at as a body of its own, which the body pane
    /// shows until it is closed.
    pub opened: Option<Opened>,
    /// How many have been opened, so that each gets a file of its own.
    opened_count: u64,
}

/// A reading opened as a body of its own: JSON out of base64, a body out of
/// gzip. The bytes are written beside the data folder and read as any body
/// is, so Pretty, Raw, Hex, folds and search all work on them.
pub struct Opened {
    /// What read it: `base64 → gzip → JSON`.
    pub label: SharedString,
    /// The field or line it was read from, to say where it came from.
    pub from: SharedString,
    pub view: Entity<BodyView>,
    /// The file the bytes went into, to take away when it closes.
    path: std::path::PathBuf,
}

impl Drop for Opened {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// How many values' worth of "is this worth reading" to keep. A flow has
/// tens of them; this is for the reader who walks through hundreds.
const MARKS_MAX: usize = 512;

/// What a drawn reading is kept under: the field and the value it was read
/// from, rather than where they sat. Rows move as a flow is chosen or a side
/// is switched, and the same header in the next flow says itself again.
fn drawn_key(field: &str, value: &str) -> String {
    format!("{field}\u{1}{value}")
}

impl InspectorState {
    /// Reads what is selected, and keeps the field and value it was read
    /// from. Called as the menu opens, not as it draws.
    pub fn read_here(&mut self, sel: &Selection) {
        self.readings = Vec::new();
        self.read_from = None;
        let Copied::Text(text) = sel.selected_text() else {
            return;
        };
        if text.len() > tracewing_convert::MAX_INPUT {
            return;
        }
        let Some((start, _)) = sel.bounds() else {
            return;
        };
        let field = sel.field_of(start.piece);
        self.readings = reading::of(&text, field.as_deref());
        if let (Some(field), Some(value)) = (field, sel.text_of(start.piece)) {
            self.read_from = Some((field, value));
        }
    }

    /// Draws `reading` under the field the last read came from, which is
    /// what choosing it from the menu does.
    pub fn draw_reading(&mut self, reading: Reading) {
        if let Some((field, value)) = self.read_from.clone() {
            self.drawn
                .insert(drawn_key(&field, &value), reading::small(reading));
        }
    }

    /// How many readings are drawn under a field.
    pub fn readings_drawn(&self) -> usize {
        self.drawn.len()
    }

    /// Takes away everything drawn under a field.
    pub fn clear_readings(&mut self) {
        self.drawn.clear();
    }

    /// The reading drawn under `field`'s value, if one was chosen.
    fn drawn_under(&self, field: &str, value: &str) -> Option<&Reading> {
        if self.drawn.is_empty() {
            return None;
        }
        self.drawn.get(&drawn_key(field, value))
    }

    /// The host of the flow shown. Rules are remembered against it, since the
    /// `created_at` of one API is not the `created_at` of another.
    pub fn host(&self) -> String {
        self.detail
            .as_ref()
            .map(|detail| host_and_path(&detail.url).0.to_owned())
            .unwrap_or_default()
    }

    /// What a remembered rule reads under this field, worked out once for
    /// each value and kept until the rules change.
    fn ruled(&self, field: &str, value: &str) -> Option<Reading> {
        let (id, version) = {
            let rules = self.remembered.borrow();
            if rules.is_empty() {
                return None;
            }
            (
                rules.reading_for(&self.host(), field)?.to_owned(),
                rules.version(),
            )
        };
        let key = drawn_key(field, value);
        {
            let mut kept = self.by_rule.borrow_mut();
            if kept.0 != version {
                kept.0 = version;
                kept.1.clear();
            } else if let Some(found) = kept.1.get(&key) {
                return found.clone();
            }
        }
        let found = reading::read_as(&id, value, Some(field));
        self.by_rule.borrow_mut().1.insert(key, found.clone());
        found
    }

    /// The runs that underline a value worth reading, so that a reader knows
    /// there is something here. Nothing, where the reading is already shown.
    pub fn marks_of(
        &self,
        field: &str,
        value: &str,
        shown: bool,
    ) -> Vec<(std::ops::Range<usize>, gpui::HighlightStyle)> {
        if shown || value.is_empty() {
            return Vec::new();
        }
        let key = drawn_key(field, value);
        let known = self.marks.borrow().get(&key).copied();
        let worth = match known {
            Some(worth) => worth,
            None => {
                let worth = reading::worth_marking(value, Some(field));
                let mut marks = self.marks.borrow_mut();
                if marks.len() > MARKS_MAX {
                    marks.clear();
                }
                marks.insert(key, worth);
                worth
            }
        };
        if worth {
            vec![(0..value.len(), reading::underlined())]
        } else {
            Vec::new()
        }
    }

    /// What is drawn under a field: the reading chosen for it, else the one a
    /// remembered rule asks for.
    pub fn reading_under(&self, field: &str, value: &str) -> Option<Reading> {
        self.drawn_under(field, value)
            .cloned()
            .or_else(|| self.ruled(field, value))
    }

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
                self.inspector.previous_call =
                    crate::compare_page::previous_call(flows.conn(), &detail);
                let stream = matches!(detail.kind.as_str(), "websocket" | "sse" | "ndjson");
                self.inspector.messages = stream.then(|| MessagesState::new(id));
                let no_cookies = cookie_counts(&detail) == (0, 0);
                let a_call = crate::llm::provider_of(&detail.method, &detail.url).is_some();
                let (part_now, side_now) = (self.inspector.part, self.inspector.side);
                if self.inspector.detail.as_ref().map(|d| d.id) != Some(id) {
                    self.inspector.select.clear();
                }
                self.inspector.part = match (stream, self.inspector.part) {
                    // A socket's calls are in its messages, so the RPC part
                    // reads them too: it is kept when it was chosen.
                    (true, Part::Rpc) => Part::Rpc,
                    (true, _) => Part::Messages,
                    (_, Part::Llm) if a_call => Part::Llm,
                    (false, Part::Messages | Part::Llm) => Part::Body,
                    (false, Part::Cookies) if no_cookies => Part::Body,
                    (false, Part::GraphQl) if detail.graphql.is_none() => Part::Body,
                    (false, Part::Rpc) if detail.rpc.is_none() => Part::Body,
                    (false, Part::Query | Part::Form) if !has_part(&detail, part_now, side_now) => {
                        Part::Body
                    }
                    (false, part) => part,
                };
                self.inspector.detail = Some(detail);
                self.inspector.opened = None;
                self.inspector.copied = false;
                self.inspector.llm = None;
                self.load_llm(cx);
                self.load_graphql(cx);
                self.load_rpc(cx);
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

    pub(crate) fn load_body(&mut self, cx: &mut Context<Self>) {
        self.inspector.read_as = None;
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
        let url = detail.url.clone();
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
                } else if detail.open {
                    tr!("Still coming: it shows once it ends.")
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
        let protos = Arc::clone(&self.protos);
        cx.spawn(async move |this, cx| {
            let file = cx
                .background_spawn({
                    let content_type = content_type.clone();
                    async move {
                        let file = body_file(&session, &facts, id, side, max);
                        let json = file.as_ref().and_then(|(path, size)| {
                            let content_type = content_type.as_deref();
                            let message =
                                crate::detail::message_of(&protos, &url, content_type, side);
                            crate::detail::json_file(path, *size, content_type, message.as_ref())
                        });
                        let table = file.as_ref().and_then(|(path, _)| {
                            let separator =
                                crate::table::separator_of(content_type.as_deref())?;
                            crate::table::Table::open(path, separator)
                        });
                        (file, json, table)
                    }
                })
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
                let (file, json, table) = file;
                let tables_as_text = this.inspector.tables_as_text;
                *this.inspector.slot(other) = match file {
                    None => BodyState::Note(
                        tr!("This body is gone: it was dropped with the oldest flows, or cannot be read.")
                            .into(),
                    ),
                    Some((_, 0)) => BodyState::Note(tr!("No body.").into()),
                    Some((path, size)) if picture && size <= PICTURE_MAX => {
                        BodyState::Image(path, size)
                    }
                    Some((path, _)) => match Body::open_typed(&path, content_type.as_deref()) {
                        Ok(body) => BodyState::Shown({
                            let focus = this.focus.clone();
                            let mode = this.inspector.body_mode;
                            // The rules this body's fields are read by, and
                            // the host they are remembered against.
                            let rules = this.inspector.remembered.clone();
                            let host = this.inspector.host();
                            // A body view cannot open a pane of its own, so
                            // it asks the window to.
                            let opens = {
                                let me = cx.entity().downgrade();
                                Rc::new(move |reading: Reading, from: String, cx: &mut gpui::App| {
                                    let _ = me.update(cx, |this: &mut Traffic, cx| {
                                        this.open_reading(&reading, &from, cx);
                                    });
                                }) as crate::bodyview::OpensReading
                            };
                            // MessagePack, CBOR or Protobuf, shown as the JSON made of it.
                            let json = json.and_then(|(json, name)| {
                                Some((Body::open(&json).ok()?, name))
                            });
                            cx.new(|cx| {
                                let mut view = match json {
                                    Some((json, name)) => {
                                        let mut view =
                                            BodyView::new(json, Some("application/json"), focus, cx);
                                        view.set_original(body, name, cx);
                                        view
                                    }
                                    None => BodyView::new(body, content_type.as_deref(), focus, cx),
                                };
                                view.set_rules(rules, host);
                                view.on_open_reading(opens);
                                view.set_mode(mode, cx);
                                if let Some(table) = table {
                                    view.set_table(table, cx);
                                    if !tables_as_text {
                                        view.set_mode(BodyMode::Table, cx);
                                    }
                                }
                                view
                            })
                        }),
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
        // Dragged to the side of or below the list; not from a window of its own.
        let detached = self.inspector_detached();
        let handle = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.))
            .when(!detached, |row| {
                row.child(
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
            })
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
                    .reachable()
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
                    .reachable()
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
                .reachable()
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
                .reachable()
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

    /// Shows bodies laid out, as they were sent, or as dumps of their bytes:
    /// both sides' when both are shown, and the next flows' too.
    /// The body the toolbar above it acts on: what a reading was opened as,
    /// while that is what the pane shows, else the body itself.
    pub(crate) fn shown_body(&self) -> Option<&Entity<BodyView>> {
        if let Some(opened) = &self.inspector.opened {
            return Some(&opened.view);
        }
        match &self.inspector.body {
            BodyState::Shown(view) => Some(view),
            _ => None,
        }
    }

    pub fn set_body_mode(&mut self, mode: BodyMode, cx: &mut Context<Self>) {
        // Chosen for a table, a way says how tables show, and no more; chosen
        // for another body, how the rest show.
        let on_a_table = self
            .shown_body()
            .is_some_and(|view| view.read(cx).has_table());
        if mode == BodyMode::Table {
            self.inspector.tables_as_text = false;
        } else if on_a_table {
            self.inspector.tables_as_text = true;
        } else {
            self.inspector.body_mode = mode;
        }
        // While a reading is open, the switch is about that body alone.
        let opened = self
            .inspector
            .opened
            .as_ref()
            .map(|opened| opened.view.clone());
        let slots: Vec<Entity<BodyView>> = match opened {
            Some(view) => vec![view],
            None => [&self.inspector.body, &self.inspector.other]
                .into_iter()
                .filter_map(|slot| match slot {
                    BodyState::Shown(view) => Some(view.clone()),
                    _ => None,
                })
                .collect(),
        };
        for view in slots {
            {
                view.update(cx, |view, cx| {
                    if mode != BodyMode::Table || view.has_table() {
                        view.set_mode(mode, cx);
                    }
                });
            }
        }
        cx.notify();
    }

    /// The body laid out, as it was sent, or its bytes: the halves of one
    /// switch. Only JSON is laid out, and a body that is bytes already has no
    /// switch.
    fn render_hex_switch(&self, part: Part, cx: &mut Context<Self>) -> Option<Div> {
        let view = self.shown_body()?.read(cx);
        let kind = view.body().kind();
        if part != Part::Body || kind == crate::body::Kind::Binary {
            return None;
        }
        let c = colors();
        let (mode, pretty, raw) = (view.mode(), view.has_pretty(), view.has_raw());
        let table = view.has_table();
        let half = |id: &'static str, label: String, to: BodyMode| {
            let on = mode == to;
            div()
                .id(id)
                .h(px(24.))
                .px(px(10.))
                .rounded(px(12.))
                .flex()
                .items_center()
                .cursor_pointer()
                .reachable()
                .text_size(px(12.))
                .font_weight(FontWeight::BOLD)
                .when(on, |d| d.bg(rgb(c.tab_on)).text_color(rgb(c.text)))
                .when(!on, |d| d.text_color(rgb(c.muted)))
                .child(label)
                .on_click(cx.listener(move |view, _, _, cx| view.set_body_mode(to, cx)))
        };
        Some(
            div()
                .flex()
                .flex_row()
                .p(px(2.))
                .rounded(px(14.))
                .bg(rgb(c.chip))
                .when(table, |d| {
                    d.child(half("body-as-table", tr!("Table"), BodyMode::Table))
                })
                .when(pretty || !raw, |d| {
                    d.child(half("body-as-pretty", tr!("Pretty"), BodyMode::Pretty))
                })
                .when(raw, |d| {
                    d.child(half("body-as-raw", tr!("Raw"), BodyMode::Raw))
                })
                .child(half("body-as-hex", "Hex".to_owned(), BodyMode::Hex)),
        )
    }

    /// The parts the chosen flow has tabs for, in the tabs' order.
    /// Whether the flow has JSON-RPC to show: a request that named a call,
    /// or a socket whose messages turned out to hold some.
    pub fn offers_rpc(&self, detail: &Detail) -> bool {
        detail.rpc.is_some()
            || self
                .inspector
                .rpc
                .as_ref()
                .is_some_and(|shown| !shown.sections.is_empty())
    }

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
        if self.offers_rpc(detail) {
            parts.push(Part::Rpc);
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
            (_, Part::Body) if self.inspector.opened.is_some() => {
                self.shown_body().map(|view| view.read(cx).summary())
            }
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
                .reachable()
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
            .when(self.offers_rpc(detail), |d| {
                d.child(tab("part-rpc", "RPC".to_owned(), Part::Rpc))
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
            .children(self.render_read_as_button(cx))
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
                            .reachable()
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
                            .reachable()
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
                            .flex_wrap()
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
                            .child(self.render_copy_as(cx))
                            .child(
                                small(
                                    "compare-with",
                                    if self.compare_pick == Some(detail.id) {
                                        tr!("Click another flow…")
                                    } else {
                                        tr!("Compare with…")
                                    },
                                )
                                .on_click(cx.listener(|view, _, _, cx| view.start_compare(cx))),
                            )
                            .children(self.inspector.previous_call.map(|previous| {
                                let id = detail.id;
                                small("compare-previous", tr!("Compare with the call before"))
                                    .on_click(cx.listener(move |view, _, _, cx| {
                                        view.compare(previous, id, cx)
                                    }))
                            })),
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
                .reachable()
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
                    .reachable()
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
            Part::Rpc => self.selectable(well, &sel, cx).child(self.render_rpc(&sel)),
            Part::Headers => {
                let side = self.inspector.side;
                self.selectable(well, &sel, cx).child(render_headers(
                    detail.headers(side),
                    detail.trailers(side),
                    &sel,
                    &self.inspector,
                ))
            }
            Part::Connection => {
                let naming = (detail.app.is_none() && device_of(&detail.client).is_some())
                    .then(|| self.device_name_input().clone());
                self.selectable(well, &sel, cx)
                    .child(render_connection(detail, naming, &sel))
            }
            Part::Query => self.selectable(well, &sel, cx).child(render_fields(
                fields::query(&detail.url),
                &sel,
                &self.inspector,
            )),
            Part::Form => self.selectable(well, &sel, cx).child(render_fields(
                detail.form.clone().unwrap_or_default(),
                &sel,
                &self.inspector,
            )),
            Part::Cookies => {
                let side = self.inspector.side;
                self.selectable(well, &sel, cx).child(render_cookies(
                    detail.headers(side),
                    side,
                    &sel,
                    &self.inspector,
                ))
            }
            Part::Body => match (self.render_read_as(cx), &self.inspector.opened) {
                (Some(picker), _) => well.child(picker),
                // What a reading held, until it is closed again.
                (None, Some(opened)) => {
                    let (label, from, view) = (
                        opened.label.clone(),
                        opened.from.clone(),
                        opened.view.clone(),
                    );
                    well.child(self.render_opened(label, from, view, cx))
                }
                (None, None) => body_well(well, &self.inspector.body),
            },
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
                Part::Headers => self.selectable(well, sel, cx).child(render_headers(
                    detail.headers(side),
                    detail.trailers(side),
                    sel,
                    &self.inspector,
                )),
                Part::Cookies => self.selectable(well, sel, cx).child(render_cookies(
                    detail.headers(side),
                    side,
                    sel,
                    &self.inspector,
                )),
                _ => match self.render_read_as(cx).filter(|_| side == chosen) {
                    Some(picker) => well.child(picker),
                    None => body_well(well, self.inspector.body_of(side)),
                },
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
    /// Opens a reading that holds bytes as a body of its own: the JSON inside
    /// base64, what gzip held. The bytes go into a file beside the others,
    /// because a body is read from a file however it was made.
    pub fn open_reading(&mut self, reading: &Reading, from: &str, cx: &mut Context<Self>) -> bool {
        let Shown::Bytes { kind, bytes } = &reading.shown else {
            return false;
        };
        let (extension, content_type) = match kind {
            Kind::Json => ("json", Some("application/json")),
            Kind::Xml => ("xml", Some("application/xml")),
            Kind::Text => ("txt", Some("text/plain")),
            Kind::Binary => ("bin", None),
        };
        let folder = self.data_dir.join("opened");
        if std::fs::create_dir_all(&folder).is_err() {
            return false;
        }
        // One at a time, and a name of its own each time: the last one may
        // still be mapped into memory while this one is written.
        self.inspector.opened_count = self.inspector.opened_count.wrapping_add(1);
        let path = folder.join(format!("{}.{extension}", self.inspector.opened_count));
        if std::fs::write(&path, bytes).is_err() {
            return false;
        }
        let Ok(body) = Body::open_typed(&path, content_type) else {
            let _ = std::fs::remove_file(&path);
            return false;
        };
        let focus = self.focus.clone();
        let view = cx.new(|cx| BodyView::new(body, content_type, focus, cx));
        let label = reading.chain.join(" → ");
        self.inspector.opened = Some(Opened {
            label: label.into(),
            from: from.to_owned().into(),
            view,
            path,
        });
        self.inspector.part = Part::Body;
        cx.notify();
        true
    }

    /// What was opened, over the body it came out of, with the way back.
    fn render_opened(
        &self,
        label: SharedString,
        from: SharedString,
        view: Entity<BodyView>,
        cx: &mut Context<Self>,
    ) -> Div {
        let c = colors();
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.))
                    .px(px(12.))
                    .pt(px(10.))
                    .pb(px(6.))
                    .child(eyebrow(tr!("Read from {from}", from = from.as_ref())))
                    .child(chip(label, c.chip, c.on_chip))
                    .child(div().flex_1())
                    .child(
                        div()
                            .id("close-opened")
                            .px(px(10.))
                            .py(px(4.))
                            .rounded_full()
                            .cursor_pointer()
                            .reachable()
                            .text_size(px(12.))
                            .font_weight(FontWeight::BOLD)
                            .bg(rgb(c.chip))
                            .text_color(rgb(c.on_chip))
                            .child(tr!("Back to the body"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.close_opened(cx);
                            })),
                    ),
            )
            .child(div().flex_1().min_h_0().py(px(4.)).child(view))
    }

    /// Closes what was opened, and shows the body itself again.
    pub fn close_opened(&mut self, cx: &mut Context<Self>) -> bool {
        let was = self.inspector.opened.take().is_some();
        if was {
            cx.notify();
        }
        was
    }

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
            let mut items = select::copy_items(
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
            );
            // What the selection turned out to be. Choosing one draws it
            // under the field it was read from, as a token's claims are.
            let (picked, pick_me) = (sel.clone(), me.clone());
            items.extend(reading::items(
                &self.inspector.readings,
                move |reading, _, cx| {
                    let _ = pick_me.update(cx, |this, cx| {
                        let from = this
                            .inspector
                            .read_from
                            .clone()
                            .map(|(field, _)| field.to_string())
                            .unwrap_or_default();
                        // What holds a body opens as one; the rest is drawn
                        // where the value stands.
                        if !this.open_reading(&reading, &from, cx) {
                            this.inspector.draw_reading(reading);
                        }
                        this.inspector.readings = Vec::new();
                        cx.notify();
                    });
                    picked.set_menu(None);
                },
            ));
            let (copied_one, r4) = (sel.clone(), redraw.clone());
            items.extend(reading::copy_item(
                &self.inspector.readings,
                move |_, cx| {
                    copied_one.set_menu(None);
                    r4(cx);
                },
            ));
            // Always, and never again: a rule for this field on this host.
            if let Some((field, value)) = self.inspector.read_from.clone() {
                let host = self.inspector.host();
                let known = self
                    .inspector
                    .remembered
                    .borrow()
                    .reading_for(&host, &field)
                    .is_some();
                let chosen = self
                    .inspector
                    .drawn_under(&field, &value)
                    .or_else(|| self.inspector.readings.first())
                    .map(|reading| reading.id);
                let (ruled, rule_me) = (sel.clone(), me.clone());
                let item = if known {
                    select::Item::new(
                        "forget-reading",
                        tr!("Forget how {field} is read", field = field.as_ref()),
                        Some(Rc::new(move |_: &mut gpui::Window, cx: &mut gpui::App| {
                            let _ = rule_me.update(cx, |this, cx| {
                                let host = this.inspector.host();
                                let _ =
                                    this.inspector.remembered.borrow_mut().forget(&host, &field);
                                cx.notify();
                            });
                            ruled.set_menu(None);
                        }) as select::Act),
                    )
                } else {
                    select::Item::new(
                        "remember-reading",
                        tr!("Always read {field} this way", field = field.as_ref()),
                        chosen.map(|id| {
                            Rc::new(move |_: &mut gpui::Window, cx: &mut gpui::App| {
                                let _ = rule_me.update(cx, |this, cx| {
                                    let host = this.inspector.host();
                                    let _ = this
                                        .inspector
                                        .remembered
                                        .borrow_mut()
                                        .remember(&host, &field, id);
                                    cx.notify();
                                });
                                ruled.set_menu(None);
                            }) as select::Act
                        }),
                    )
                };
                items.push(item);
            }
            if !self.inspector.drawn.is_empty() {
                let (cleared, clear_me) = (sel.clone(), me.clone());
                items.push(select::Item::new(
                    "clear-readings",
                    tr!("Clear what was read"),
                    Some(Rc::new(move |_: &mut gpui::Window, cx: &mut gpui::App| {
                        let _ = clear_me.update(cx, |this, cx| {
                            this.inspector.clear_readings();
                            cx.notify();
                        });
                        cleared.set_menu(None);
                    }) as select::Act),
                ));
            }
            select::render_menu(at, items, move |_, cx| {
                close.set_menu(None);
                r3(cx);
            })
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
                    this.inspector.read_here(&menu_at);
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
                | Part::Llm
                | Part::Rpc
                | Part::GraphQl,
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
    let size = size_chip_text(&detail.request, &detail.response);
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .flex_shrink_0()
        .gap(px(6.))
        .child(chip(status_text(detail.status), ground, ink))
        .children(detail.grpc_status.map(|code| {
            let (ground, ink) = status_colors(Some(crate::grpc::as_http(code)));
            let mut text = format!("gRPC {}", crate::grpc::code_text(code));
            if let Some(message) = detail.grpc_message().filter(|_| code != 0) {
                text.push_str(" · ");
                text.push_str(&message);
            }
            // A message can be long: it wraps, as an error does.
            chip(text, ground, ink).max_w_full().rounded(px(12.))
        }))
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
        .children(
            detail
                .rpc
                .clone()
                .map(|method| chip(tr!("RPC: {method}", method = method), c.tonal, c.on_tonal)),
        )
        .children(detail.rpc_error.map(|code| {
            // A failed call answered 200, so the status chip says nothing is
            // wrong: this is what says it. The message is in the body.
            let (ground, ink) = status_colors(Some(500));
            let text = format!("RPC {}", tracewing_store::jsonrpc::code_text(code));
            chip(text, ground, ink).max_w_full().rounded(px(12.))
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

/// How much went each way, and how it was packed: the response alone, or with
/// what was sent before it when a body was, `↑ 2.1 KB · ↓ 1.3 KB · gzip`.
fn size_chip_text(request: &BodyFacts, response: &BodyFacts) -> String {
    let side = |facts: &BodyFacts| match &facts.stored.encoding {
        Some(encoding) => format!("{} · {encoding}", size_text(facts.bytes)),
        None => size_text(facts.bytes),
    };
    if request.bytes > 0 {
        format!("↑ {} · ↓ {}", side(request), side(response))
    } else {
        side(response)
    }
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

/// Headers, and the trailers after the body under a heading of their own.
fn render_headers(
    headers: Option<&[(String, String)]>,
    trailers: &[(String, String)],
    sel: &Selection,
    read: &InspectorState,
) -> impl IntoElement {
    let c = colors();
    let Some(headers) = headers else {
        return note(tr!("These headers were not kept.")).into_any_element();
    };
    if headers.is_empty() && trailers.is_empty() {
        return note(tr!("No headers.")).into_any_element();
    }
    let row = |(name, value): &(String, String)| {
        let reading = read.reading_under(name, value);
        let shown = reading.is_some() || Jwt::find(value).is_some();
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
                    .child(sel.piece_under(
                        name.clone(),
                        value.clone(),
                        ": ",
                        read.marks_of(name, value, shown),
                    ))
                    .children(Jwt::find(value).map(|jwt| render_jwt(jwt, sel)))
                    .children(reading.map(|reading| render_reading(&reading, sel))),
            )
    };
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
        .children(headers.iter().map(row))
        .when(!trailers.is_empty(), |d| {
            d.child(
                div()
                    .pt(px(10.))
                    .font_family(crate::FONT)
                    .text_size(px(11.5))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(c.muted))
                    .child(sel.piece(tr!("Trailers, sent after the body"), "\n", Vec::new())),
            )
            .children(trailers.iter().map(row))
        })
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

fn render_fields(found: Vec<Field>, sel: &Selection, read: &InspectorState) -> impl IntoElement {
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
            let field_name = field.name.clone();
            let name = sel.piece(field.name, "\n", Vec::new());
            let drawn = match &field.value {
                FieldValue::Text(text) => read.reading_under(&field_name, text),
                FieldValue::File { .. } => None,
            };
            let value = match field.value {
                FieldValue::Text(text) => {
                    div().child(sel.piece_under(field_name.clone(), text, ": ", Vec::new()))
                }
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
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(value)
                        .children(drawn.map(|reading| render_reading(&reading, sel))),
                )
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
    read: &InspectorState,
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
            let drawn = read.reading_under(&cookie.name, &cookie.value);
            let cookie_name = cookie.name.clone();
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
                        .child({
                            let marks = read.marks_of(
                                &cookie_name,
                                &cookie.value,
                                drawn.is_some() || jwt.is_some(),
                            );
                            sel.piece_under(cookie_name, cookie.value, ": ", marks)
                        })
                        .when(!cookie.attributes.is_empty(), |d| {
                            d.child(
                                div().text_size(px(11.5)).text_color(rgb(c.muted)).child(
                                    sel.piece(cookie.attributes.join("; "), "; ", Vec::new()),
                                ),
                            )
                        })
                        .children(jwt.map(|jwt| render_jwt(jwt, sel)))
                        .children(drawn.map(|reading| render_reading(&reading, sel))),
                )
        }))
        .into_any_element()
}

/// What a token claims, under the header or cookie that carries it.
/// A reading under the field it was read from, as a token's claims are.
fn render_reading(reading: &Reading, sel: &Selection) -> Div {
    let c = colors();
    let line = |text: String| div().child(sel.piece(text, "\n", Vec::new()));
    let pair = |name: &str, value: &str| {
        div()
            .flex()
            .flex_row()
            .gap(px(8.))
            .child(
                div()
                    .w(relative(0.3))
                    .flex_shrink_0()
                    .text_color(rgb(c.muted))
                    .child(sel.piece(name.to_owned(), "\n", Vec::new())),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(sel.piece(value.to_owned(), ": ", Vec::new())),
            )
    };
    let said: Vec<Div> = match &reading.shown {
        Shown::Line(text) => vec![line(text.clone())],
        Shown::Fields(fields) => fields
            .iter()
            .map(|(name, value)| pair(name, value))
            .collect(),
        Shown::Table { head, rows } => std::iter::once(head.join(" · "))
            .chain(rows.iter().map(|row| row.join(" · ")))
            .map(line)
            .collect(),
        Shown::Bytes { .. } => vec![line(reading.beside())],
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
                .child(sel.piece(reading.label.clone(), "\n\n", Vec::new())),
        )
        .children(said)
}

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
    fn a_value_worth_reading_is_underlined_and_the_rest_is_not() {
        let state = InspectorState::default();
        let date = "Thu, 12 Nov 2026 15:20:02 GMT";
        assert_eq!(state.marks_of("date", date, false).len(), 1);
        assert_eq!(state.marks_of("date", date, false)[0].0, 0..date.len());
        // Once the reading itself is shown, the hint has nothing to say.
        assert!(state.marks_of("date", date, true).is_empty());
        // A number that is only a number, and a word, are left alone.
        assert!(state.marks_of("total", "868617", false).is_empty());
        assert!(state.marks_of("customer", "Chi Pham", false).is_empty());
        assert!(state.marks_of("x", "", false).is_empty());
        // The answer is kept, and the same the second time.
        assert_eq!(state.marks_of("date", date, false).len(), 1);
    }

    #[test]
    fn the_size_says_both_ways_when_a_body_was_sent() {
        let mut request = BodyFacts::default();
        let response = BodyFacts {
            bytes: 1300,
            stored: tracewing_store::session::StoredBody {
                encoding: Some("gzip".to_owned()),
                ..Default::default()
            },
            ..BodyFacts::default()
        };
        assert_eq!(size_chip_text(&request, &response), "1.3 KB · gzip");
        request.bytes = 2100;
        assert_eq!(
            size_chip_text(&request, &response),
            "↑ 2.1 KB · ↓ 1.3 KB · gzip"
        );
    }

    #[test]
    fn statuses_read_as_words() {
        assert_eq!(status_text(Some(200)), "200 OK");
        assert_eq!(status_text(Some(404)), "404 Not Found");
        assert_eq!(status_text(Some(599)), "599");
        assert_eq!(status_text(None), "No response");
    }
}
