//! The messages of a stream, in the inspector, growing as they come: a list, and the
//! one chosen in the body viewer. Everything about a message but its text is held in
//! memory, a few dozen bytes each; the text is read as a row comes into view.

use crate::text_size::ui;
use crate::{
    body::Body,
    bodyview::{BodyView, summary},
    colors::colors,
    flows::{size_text, time_text},
    traffic::Traffic,
    widgets::{Reachable, tip},
};
use gpui::{
    Context, Div, Entity, FontWeight, ScrollStrategy, SharedString, Stateful,
    UniformListScrollHandle, div, prelude::*, rgb, uniform_list,
};
use std::{collections::HashMap, ops::Range};
use tracewing_i18n::tr;
use tracewing_proxy::sockets::{NotSent, Said, Toward};
use tracewing_store::session::{self, BodyReader, StoredMessage};

const ROW_HEIGHT: f32 = 26.;
/// How tall the chosen message is shown, before anyone drags it.
pub const VIEW_H: f32 = 240.;
/// How short it may be dragged, and how little of the list it may leave: a
/// message must never take the list away, however short the panel is.
pub const VIEW_MIN: f32 = 64.;
pub const LIST_MIN: f32 = 72.;
/// The strip to drag it by, with its mark in the middle.
const GRIP_H: f32 = 10.;
/// How much of each message a row shows.
const PREVIEW_BYTES: usize = 240;
/// More previews than this and the cache starts again. A screen is a few dozen.
const PREVIEWS_MAX: usize = 5000;
/// How many messages one look takes in: a flood must not hold up a frame.
const TAKE_AT_ONCE: i64 = 5000;

pub struct MessagesState {
    pub flow_id: i64,
    pub list: Vec<StoredMessage>,
    pub chosen: Option<usize>,
    pub view: Option<Entity<BodyView>>,
    previews: HashMap<i64, SharedString>,
    scroll: UniformListScrollHandle,
    /// Where a message the window sends goes.
    pub toward: Toward,
    /// How tall the chosen message is shown. Dragged while it is open, and
    /// gone with it: the next message starts at the usual height.
    pub view_h: Option<f32>,
    /// Why the last one could not be sent.
    pub send_problem: Option<String>,
    /// The calls the last message sent had written out in it, by the
    /// signatures they were taken to mean.
    pub send_calls: Vec<String>,
    /// What Find looks for in the messages, and those it found.
    pub find: Option<MessagesFound>,
    /// Which search is the latest: an older one still running stops.
    find_gen: u64,
}

/// The messages Find found, by their place in the list, and the one it is at.
pub struct MessagesFound {
    pub re: regex::Regex,
    pub places: Vec<usize>,
    /// Every message there was has been searched.
    pub done: bool,
    pub now: Option<usize>,
}

/// Messages searched between two looks at the window.
const FIND_CHUNK: usize = 500;

impl MessagesState {
    pub fn new(flow_id: i64) -> Self {
        Self {
            flow_id,
            list: Vec::new(),
            chosen: None,
            view: None,
            previews: HashMap::new(),
            scroll: UniformListScrollHandle::new(),
            toward: Toward::Server,
            view_h: None,
            send_problem: None,
            send_calls: Vec::new(),
            find: None,
            find_gen: 0,
        }
    }
}

/// One line of text from the start of a message.
pub fn preview_text(data: &[u8]) -> String {
    let data = &data[..data.len().min(PREVIEW_BYTES)];
    if data.iter().filter(|b| **b == 0).count() > 0 {
        return String::new();
    }
    String::from_utf8_lossy(data)
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

impl Traffic {
    /// Takes in what has arrived since the last look. Returns whether anything did.
    pub(crate) fn take_new_messages(&mut self) -> bool {
        let Some(state) = &self.inspector.messages else {
            return false;
        };
        let (flow_id, after) = (state.flow_id, state.list.last().map_or(0, |m| m.id));
        let Some(flows) = self.flows() else {
            return false;
        };
        let new = session::stored_messages(flows.conn(), flow_id, after, TAKE_AT_ONCE)
            .unwrap_or_default();
        if new.is_empty() {
            return false;
        }
        let state = self.inspector.messages.as_mut().expect("just looked");
        state.list.extend(new);
        // The newest stays in view until one has been chosen to read.
        if state.chosen.is_none() {
            state
                .scroll
                .scroll_to_item(state.list.len() - 1, ScrollStrategy::Nearest);
        }
        true
    }

    /// Looks for `re` in the messages, from the first, in the background a
    /// chunk at a time; `None` stops looking.
    pub fn find_in_messages(&mut self, re: Option<regex::Regex>, cx: &mut Context<Self>) {
        let session_dir = self.session_dir().to_owned();
        let Some(state) = self.inspector.messages.as_mut() else {
            return;
        };
        state.find_gen += 1;
        let generation = state.find_gen;
        // The message open marks what it holds of it too.
        if let Some(view) = &state.view {
            let re = re.clone();
            view.update(cx, |view, cx| view.set_find(re, cx));
        }
        let Some(re) = re else {
            state.find = None;
            cx.notify();
            return;
        };
        state.find = Some(MessagesFound {
            re: re.clone(),
            places: Vec::new(),
            done: false,
            now: None,
        });
        cx.spawn(async move |this, cx| {
            let mut from = 0;
            loop {
                // The messages there are now: more may have come.
                let Ok(Some(chunk)) = this.update(cx, |this, _| {
                    let state = this.inspector.messages.as_ref()?;
                    (state.find_gen == generation).then(|| {
                        state
                            .list
                            .iter()
                            .enumerate()
                            .skip(from)
                            .take(FIND_CHUNK)
                            .map(|(place, m)| (place, m.place))
                            .collect::<Vec<_>>()
                    })
                }) else {
                    break;
                };
                let end = chunk.len() < FIND_CHUNK;
                let next = chunk.last().map_or(from, |(place, _)| place + 1);
                let (dir, re) = (session_dir.clone(), re.clone());
                let found = cx
                    .background_spawn(async move {
                        let mut reader = BodyReader::new(&dir);
                        chunk
                            .into_iter()
                            .filter(|(_, at)| {
                                at.and_then(|at| reader.read_place(at).ok())
                                    .is_some_and(|data| {
                                        crate::find::found(&re, &String::from_utf8_lossy(&data))
                                            .next()
                                            .is_some()
                                    })
                            })
                            .map(|(place, _)| place)
                            .collect::<Vec<_>>()
                    })
                    .await;
                let go_on = this
                    .update(cx, |this, cx| {
                        let Some(state) = this.inspector.messages.as_mut() else {
                            return false;
                        };
                        if state.find_gen != generation {
                            return false;
                        }
                        let Some(find) = state.find.as_mut() else {
                            return false;
                        };
                        find.places.extend(found);
                        find.done = end;
                        cx.notify();
                        !end
                    })
                    .unwrap_or(false);
                if !go_on {
                    break;
                }
                from = next;
            }
        })
        .detach();
        cx.notify();
    }

    /// Opens the next message Find found, or the one before, round again at
    /// either end, with the list scrolled to it.
    pub fn messages_find_step(&mut self, forward: bool, cx: &mut Context<Self>) {
        let Some(find) = self
            .inspector
            .messages
            .as_mut()
            .and_then(|s| s.find.as_mut())
        else {
            return;
        };
        let count = find.places.len();
        if count == 0 {
            return;
        }
        let now = match (find.now, forward) {
            (None, true) => 0,
            (None, false) => count - 1,
            (Some(i), true) => (i + 1) % count,
            (Some(i), false) => (i + count - 1) % count,
        };
        find.now = Some(now);
        let place = find.places[now];
        self.choose_message(place, cx);
        if let Some(state) = &self.inspector.messages {
            state.scroll.scroll_to_item(place, ScrollStrategy::Center);
        }
    }

    /// Puts the chosen message away and gives the list its room back.
    pub fn close_message(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(state) = self.inspector.messages.as_mut() else {
            return false;
        };
        if state.chosen.is_none() && state.view.is_none() {
            return false;
        }
        state.chosen = None;
        state.view = None;
        state.view_h = None;
        cx.notify();
        true
    }

    pub fn choose_message(&mut self, place: usize, cx: &mut Context<Self>) {
        let Some(state) = &self.inspector.messages else {
            return;
        };
        let Some(message) = state.list.get(place).cloned() else {
            return;
        };
        let session_dir = self.session_dir().to_owned();
        let state = self.inspector.messages.as_mut().expect("just looked");
        state.chosen = Some(place);
        state.view = None;
        state.scroll.scroll_to_item(place, ScrollStrategy::Nearest);
        cx.notify();
        let Some(where_kept) = message.place else {
            return;
        };
        let id = message.id;
        cx.spawn(async move |this, cx| {
            let file = cx
                .background_spawn(async move {
                    let data = BodyReader::new(&session_dir).read_place(where_kept).ok()?;
                    let dir = session_dir.join("view");
                    std::fs::create_dir_all(&dir).ok()?;
                    let path = dir.join(format!("message-{id}"));
                    std::fs::write(&path, data).ok()?;
                    Some(path)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                let Some(state) = this.inspector.messages.as_mut() else {
                    return;
                };
                let still_chosen = state
                    .chosen
                    .and_then(|place| state.list.get(place))
                    .is_some_and(|m| m.id == id);
                if !still_chosen {
                    return;
                }
                if let Some(body) = file.and_then(|path| Body::open(&path).ok()) {
                    let focus = this.focus.clone();
                    // Taken before the messages are borrowed, since both are
                    // the inspector's.
                    let rules = this.inspector.remembered.clone();
                    let host = this.inspector.host();
                    let marks_on = this.inspector.marks_on.clone();
                    let Some(state) = this.inspector.messages.as_mut() else {
                        return;
                    };
                    let re = state.find.as_ref().map(|find| find.re.clone());
                    // What a message holds is read as any other body is: the
                    // rules of this host, the underline Settings asks for,
                    // and a way to ask the window for the readings pane.
                    let opens = {
                        let me = cx.entity().downgrade();
                        std::rc::Rc::new(
                            move |what: crate::bodyview::ToOpen, cx: &mut gpui::App| {
                                let _ = me.update(cx, |this: &mut Traffic, cx| match what {
                                    crate::bodyview::ToOpen::Reading(reading, from) => {
                                        this.open_reading(&reading, &from, cx);
                                    }
                                    crate::bodyview::ToOpen::WholeLine { line, text } => {
                                        this.open_line(line, &text, cx);
                                    }
                                    crate::bodyview::ToOpen::Value { text, field } => {
                                        this.read_value(text, field, cx);
                                    }
                                });
                            },
                        ) as crate::bodyview::Opens
                    };
                    let view = cx.new(|cx| BodyView::new(body, None, focus, cx));
                    view.update(cx, |view, _| {
                        view.set_rules(rules, host, marks_on);
                        view.on_open_reading(opens);
                    });
                    if re.is_some() {
                        view.update(cx, |view, cx| view.set_find(re, cx));
                    }
                    state.view = Some(view);
                    // The viewer takes room from the list: the chosen row stays
                    // in what is left of it.
                    if let Some(place) = state.chosen {
                        state.scroll.scroll_to_item(place, ScrollStrategy::Nearest);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Exactly one element per place asked for, always: see `render_rows`.
    fn render_message_rows(
        &mut self,
        range: Range<usize>,
        cx: &mut Context<Self>,
    ) -> Vec<Stateful<Div>> {
        let session_dir = self.session_dir().to_owned();
        let Some(state) = self.inspector.messages.as_mut() else {
            return range
                .map(|place| div().id(("message", place as u64)).h(ui(ROW_HEIGHT)))
                .collect();
        };
        if state.previews.len() > PREVIEWS_MAX {
            state.previews.clear();
        }
        let mut reader = BodyReader::new(&session_dir);
        let c = colors();
        let first_at = state.list.first().map_or(0, |m| m.at_us);
        range
            .map(|place| {
                let row = div()
                    .id(("message", place as u64))
                    .h(ui(ROW_HEIGHT))
                    .px(ui(6.));
                let Some(message) = state.list.get(place) else {
                    return row;
                };
                let preview = state
                    .previews
                    .entry(message.id)
                    .or_insert_with(|| {
                        if message.shed {
                            return tr!("Not kept").into();
                        }
                        message
                            .place
                            .and_then(|at| reader.read_place(at).ok())
                            .map_or(SharedString::default(), |data| preview_text(&data).into())
                    })
                    .clone();
                let chosen = state.chosen == Some(place);
                let (arrow, color) = if message.from_client {
                    ("↑", c.json_key)
                } else {
                    ("↓", c.download)
                };
                let kind = matches!(message.kind.as_str(), "text" | "event" | "line")
                    .then_some(())
                    .map_or_else(|| Some(message.kind.clone()), |()| None);
                row.flex()
                    .flex_row()
                    .items_center()
                    .gap(ui(8.))
                    .rounded(ui(8.))
                    .cursor_pointer()
                    .when(chosen, |d| d.bg(rgb(c.selected_row)))
                    .font_family(crate::mono_font())
                    .font_features(gpui::FontFeatures::disable_ligatures())
                    .text_size(ui(12.))
                    .child(
                        div()
                            .w(ui(12.))
                            .flex_shrink_0()
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(color))
                            .child(arrow),
                    )
                    .child(
                        div()
                            .w(ui(64.))
                            .flex_shrink_0()
                            .text_color(rgb(c.muted))
                            .child(time_text(message.at_us - first_at)),
                    )
                    .child(
                        div()
                            .w(ui(58.))
                            .flex_shrink_0()
                            .text_color(rgb(c.muted))
                            .child(size_text(message.bytes)),
                    )
                    .children(
                        kind.map(|kind| div().flex_shrink_0().text_color(rgb(c.muted)).child(kind)),
                    )
                    .child(div().flex_1().min_w_0().truncate().child({
                        let marks: Vec<_> = state
                            .find
                            .as_ref()
                            .map(|find| {
                                let now = find.now.map(|i| find.places[i]) == Some(place);
                                crate::find::found(&find.re, &preview)
                                    .map(|r| (r, crate::find::found_style(now)))
                                    .collect()
                            })
                            .unwrap_or_default();
                        gpui::StyledText::new(preview).with_highlights(marks)
                    }))
                    .on_click(cx.listener(move |this, _, _, cx| this.choose_message(place, cx)))
            })
            .collect()
    }

    pub(crate) fn render_messages(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let Some(state) = &self.inspector.messages else {
            return div();
        };
        let count = state.list.len();
        let scroll = state.scroll.clone();
        let chosen = state.chosen.and_then(|place| state.list.get(place));
        let cut = chosen
            .filter(|m| m.place.is_some_and(|p| u64::from(p.len) < m.bytes))
            .map(|m| m.bytes);
        let note: Option<SharedString> = chosen.and_then(|m| {
            if m.shed {
                Some(
                    tr!("This message was not kept: too much was waiting, or the disk was nearly full.")
                        .into(),
                )
            } else if m.place.is_none() {
                Some(tr!("This message is empty.").into())
            } else {
                None
            }
        });
        let view = state.view.clone();
        let list = if count == 0 {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(rgb(c.muted))
                .child(tr!("No messages yet"))
                .into_any_element()
        } else {
            uniform_list(
                "messages",
                count,
                cx.processor(|this, range: Range<usize>, _, cx| {
                    this.render_message_rows(range, cx)
                }),
            )
            .flex_1()
            .min_h(ui(LIST_MIN))
            .track_scroll(&scroll)
            .into_any_element()
        };
        let has_detail = note.is_some() || view.is_some();
        let viewer = has_detail.then(|| {
            let status = view.as_ref().map(|view| {
                let said = summary(view.read(cx).body());
                match cut {
                    Some(bytes) => tr!(
                        "{said} kept, of {size}",
                        said = said,
                        size = size_text(bytes)
                    ),
                    None => said,
                }
            });
            let tall = state.view_h.unwrap_or(VIEW_H).max(VIEW_MIN);
            // No floor here: VIEW_MIN is how short a drag may make it, but a
            // panel too short for both must still show some of each rather
            // than push the message off the bottom.
            div()
                .h(ui(tall))
                .flex()
                .flex_col()
                .border_t_1()
                .border_color(rgb(c.line))
                // The edge is a hair thick to aim at, so the strip is what
                // takes the drag and a mark in the middle says so.
                .child(
                    div()
                        .id("message-view-grip")
                        .h(ui(GRIP_H))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor(gpui::CursorStyle::ResizeUpDown)
                        .child(
                            div()
                                .w(ui(28.))
                                .h(ui(3.))
                                .rounded_full()
                                .bg(rgb(c.splitter)),
                        )
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(|this, e: &gpui::MouseDownEvent, _, cx| {
                                this.begin_message_view_drag(e.position.y, cx);
                            }),
                        ),
                )
                .children(status.map(|status| {
                    div()
                        .flex_shrink_0()
                        .px(ui(10.))
                        .pb(ui(4.))
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(ui(8.))
                        .text_size(ui(12.))
                        .text_color(rgb(c.muted))
                        .child(div().flex_1().min_w_0().child(status))
                        .child(
                            div()
                                .id("close-message")
                                .px(ui(6.))
                                .rounded(ui(8.))
                                .cursor_pointer()
                                .reachable()
                                .hover(|s| s.bg(rgb(c.chip)).text_color(rgb(c.text)))
                                .tooltip(tip(tr!("Close this message")))
                                .child("✕")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.close_message(cx);
                                })),
                        )
                }))
                .child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .children(view)
                        .children(note.map(|note| {
                            div()
                                .size_full()
                                .p(ui(14.))
                                .text_color(rgb(c.muted))
                                .child(note)
                        })),
                )
        });
        let sending = self.render_socket_send(cx);
        div()
            .size_full()
            .flex()
            .flex_col()
            .pt(ui(6.))
            .child(list)
            .children(sending)
            .children(viewer)
    }

    /// Whether the socket the inspector shows is open, to send into.
    pub fn socket_open(&self) -> bool {
        self.inspector
            .messages
            .as_ref()
            .is_some_and(|state| self.engine.sockets.is_open(state.flow_id))
    }

    pub fn set_socket_toward(&mut self, toward: Toward, cx: &mut Context<Self>) {
        if let Some(state) = &mut self.inspector.messages {
            state.toward = toward;
            cx.notify();
        }
    }

    /// Sends what is typed into the socket, the way chosen. It joins the
    /// list as it is written down, as any message.
    ///
    /// A call written out in it — `balanceOf(0xd8dA…)` where a node wants
    /// calldata — is sent as the bytes it stands for, as in Compose: a node
    /// is as often asked over a socket as over a request.
    pub fn send_socket_message(&mut self, cx: &mut Context<Self>) {
        let text = self.socket_message.read(cx).text().to_owned();
        if text.is_empty() {
            return;
        }
        let abi = self.abis.abi();
        let written = crate::calldata::as_calldata(&text, &crate::calldata::signatures_of(&abi));
        let Some(state) = &mut self.inspector.messages else {
            return;
        };
        let (text, calls) = match written {
            Ok(written) => written,
            Err(problem) => {
                state.send_problem = Some(problem);
                state.send_calls = Vec::new();
                cx.notify();
                return;
            }
        };
        let sent = self
            .engine
            .sockets
            .send(state.flow_id, state.toward, Said::Text(text));
        state.send_problem = match sent {
            Ok(()) => None,
            Err(NotSent::NotOpen) => Some(tr!("The socket has closed.")),
            Err(NotSent::Busy) => Some(tr!("Too many are still waiting to go. Try again.")),
        };
        state.send_calls = match sent.is_ok() {
            true => calls,
            false => Vec::new(),
        };
        if sent.is_ok() {
            self.socket_message
                .update(cx, |input, cx| input.set_text("", 0..0, cx));
        }
        cx.notify();
    }

    /// Where a message is typed and sent, while the socket is open.
    fn render_socket_send(&self, cx: &mut Context<Self>) -> Option<Div> {
        let state = self.inspector.messages.as_ref()?;
        let open = self.engine.sockets.is_open(state.flow_id);
        if !open && state.send_problem.is_none() {
            return None;
        }
        let c = colors();
        let toward = state.toward;
        let chip = |id: &'static str, label: String, on: bool| {
            div()
                .id(id)
                .h(ui(26.))
                .px(ui(10.))
                .rounded(ui(13.))
                .flex()
                .items_center()
                .cursor_pointer()
                .reachable()
                .text_size(ui(12.))
                .font_weight(FontWeight::BOLD)
                .when(on, |d| d.bg(rgb(c.tonal)).text_color(rgb(c.on_tonal)))
                .when(!on, |d| d.text_color(rgb(c.muted)))
                .child(label)
        };
        let problem = state.send_problem.clone();
        let calls = state.send_calls.clone();
        Some(
            div()
                .flex_shrink_0()
                .border_t_1()
                .border_color(rgb(c.line))
                .px(ui(10.))
                .py(ui(8.))
                .flex()
                .flex_col()
                .gap(ui(6.))
                .when(open, |d| {
                    d.child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(ui(4.))
                            .child(
                                chip(
                                    "send-to-server",
                                    tr!("To the server"),
                                    toward == Toward::Server,
                                )
                                .on_click(cx.listener(
                                    |this, _, _, cx| this.set_socket_toward(Toward::Server, cx),
                                )),
                            )
                            .child(
                                chip("send-to-app", tr!("To the app"), toward == Toward::App)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.set_socket_toward(Toward::App, cx)
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_end()
                            .gap(ui(8.))
                            .child(
                                div()
                                    .id("socket-message")
                                    .flex_1()
                                    .min_w_0()
                                    .px(ui(10.))
                                    .py(ui(6.))
                                    .rounded(ui(10.))
                                    .bg(rgb(c.input))
                                    .font_family(crate::mono_font())
                                    .text_size(ui(12.5))
                                    .child(self.socket_message.clone())
                                    .on_key_down(cx.listener(
                                        |this, event: &gpui::KeyDownEvent, _, cx| {
                                            let keys = &event.keystroke;
                                            if keys.key == "enter" && keys.modifiers.secondary() {
                                                cx.stop_propagation();
                                                this.send_socket_message(cx);
                                            }
                                        },
                                    )),
                            )
                            .child(
                                div()
                                    .id("send-socket-message")
                                    .h(ui(30.))
                                    .px(ui(14.))
                                    .rounded(ui(15.))
                                    .flex()
                                    .items_center()
                                    .cursor_pointer()
                                    .reachable()
                                    .bg(rgb(c.accent))
                                    .text_color(rgb(c.on_accent))
                                    .text_size(ui(12.5))
                                    .font_weight(FontWeight::BOLD)
                                    .child(tr!("Send"))
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.send_socket_message(cx)),
                                    ),
                            ),
                    )
                })
                .children(
                    problem.map(|why| div().text_size(ui(12.)).text_color(rgb(c.error)).child(why)),
                )
                .children((!calls.is_empty()).then(|| {
                    div().text_size(ui(12.)).text_color(rgb(c.muted)).child(tr!(
                        "Written out in the body and sent as calldata: {calls}",
                        calls = calls.join(", ")
                    ))
                })),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_preview_is_one_line() {
        assert_eq!(
            preview_text(b"data: {\"a\":1}\nmore\r\n"),
            "data: {\"a\":1} more  "
        );
    }

    #[test]
    fn a_preview_is_short_and_cuts_anywhere() {
        let long = "đặt".repeat(1000);
        let shown = preview_text(long.as_bytes());
        assert!(shown.len() < 300 + 3);
        assert!(shown.starts_with("đặt"));
    }

    #[test]
    fn binary_shows_no_text() {
        assert_eq!(preview_text(&[1, 0, 2]), "");
    }
}
