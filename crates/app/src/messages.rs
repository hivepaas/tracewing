//! The messages of a stream, in the inspector, growing as they come: a list, and the
//! one chosen in the body viewer. Everything about a message but its text is held in
//! memory, a few dozen bytes each; the text is read as a row comes into view.

use crate::{
    body::Body,
    bodyview::{BodyView, summary},
    colors::colors,
    flows::{size_text, time_text},
    traffic::Traffic,
};
use gpui::{
    Context, Div, Entity, FontWeight, ScrollStrategy, SharedString, Stateful,
    UniformListScrollHandle, div, prelude::*, px, rgb, uniform_list,
};
use std::{collections::HashMap, ops::Range};
use tracewing_store::session::{self, BodyReader, StoredMessage};

const ROW_HEIGHT: f32 = 26.;
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
}

impl MessagesState {
    pub fn new(flow_id: i64) -> Self {
        Self {
            flow_id,
            list: Vec::new(),
            chosen: None,
            view: None,
            previews: HashMap::new(),
            scroll: UniformListScrollHandle::new(),
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
                    state.view = Some(cx.new(|cx| BodyView::new(body, cx)));
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
                .map(|place| div().id(("message", place as u64)).h(px(ROW_HEIGHT)))
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
                    .h(px(ROW_HEIGHT))
                    .px(px(6.));
                let Some(message) = state.list.get(place) else {
                    return row;
                };
                let preview = state
                    .previews
                    .entry(message.id)
                    .or_insert_with(|| {
                        if message.shed {
                            return "Not kept".into();
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
                let kind = matches!(message.kind.as_str(), "text" | "event")
                    .then_some(())
                    .map_or_else(|| Some(message.kind.clone()), |()| None);
                row.flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.))
                    .rounded(px(8.))
                    .cursor_pointer()
                    .when(chosen, |d| d.bg(rgb(c.selected_row)))
                    .font_family(crate::MONO_FONT)
                    .text_size(px(12.))
                    .child(
                        div()
                            .w(px(12.))
                            .flex_shrink_0()
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(color))
                            .child(arrow),
                    )
                    .child(
                        div()
                            .w(px(64.))
                            .flex_shrink_0()
                            .text_color(rgb(c.muted))
                            .child(time_text(message.at_us - first_at)),
                    )
                    .child(
                        div()
                            .w(px(58.))
                            .flex_shrink_0()
                            .text_color(rgb(c.muted))
                            .child(size_text(message.bytes)),
                    )
                    .children(
                        kind.map(|kind| div().flex_shrink_0().text_color(rgb(c.muted)).child(kind)),
                    )
                    .child(div().flex_1().min_w_0().truncate().child(preview))
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
                    "This message was not kept: too much was waiting, or the disk was nearly full."
                        .into(),
                )
            } else if m.place.is_none() {
                Some("This message is empty.".into())
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
                .child("No messages yet")
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
            .track_scroll(&scroll)
            .into_any_element()
        };
        let has_detail = note.is_some() || view.is_some();
        let viewer = has_detail.then(|| {
            let status = view.as_ref().map(|view| {
                let said = summary(view.read(cx).body());
                match cut {
                    Some(bytes) => format!("{said} kept, of {}", size_text(bytes)),
                    None => said,
                }
            });
            div()
                .h(px(240.))
                .flex_shrink_0()
                .flex()
                .flex_col()
                .border_t_1()
                .border_color(rgb(c.line))
                .children(status.map(|status| {
                    div()
                        .px(px(10.))
                        .py(px(4.))
                        .text_size(px(12.))
                        .text_color(rgb(c.muted))
                        .child(status)
                }))
                .child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .children(view)
                        .children(note.map(|note| {
                            div()
                                .size_full()
                                .p(px(14.))
                                .text_color(rgb(c.muted))
                                .child(note)
                        })),
                )
        });
        div()
            .size_full()
            .flex()
            .flex_col()
            .pt(px(6.))
            .child(list)
            .children(viewer)
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
