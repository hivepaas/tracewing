//! A body in a panel. It keeps its own position, a line and a pixel offset into it,
//! and draws only the lines in view, because a body can have more lines than a
//! pixel offset in an `f32` can tell apart: `uniform_list`'s step grows to 8 px at
//! 6 million lines of 20 px.

use crate::{
    body::{Body, Kind, Line, Token, tokens},
    colors::colors,
    flows::size_text,
};
use gpui::{
    Bounds, Context, HighlightStyle, Hsla, Pixels, Render, ScrollWheelEvent, SharedString,
    StyledText, Window, canvas, div, prelude::*, px, rgb,
};
use std::{cell::Cell, rc::Rc, sync::Arc, time::Duration};

pub const LINE_HEIGHT: f32 = 20.;
const GUTTER_W: f32 = 52.;
const INDENT_W: f32 = 14.;
/// Past this the indent says what it is instead of pushing the text away.
const INDENT_MAX: u32 = 40;
const THUMB_MIN: f32 = 28.;
const BAR_W: f32 = 10.;
/// How often a window with a scan running looks at its progress.
const SCAN_POLL: Duration = Duration::from_millis(50);

pub struct BodyView {
    body: Arc<Body>,
    /// The line at the top of the view, and how far it has scrolled out of it.
    top: u64,
    sub: f32,
    dragging: bool,
    /// Where the view was last drawn: the height sets the scroll range, and a drag
    /// on the bar is measured from its top.
    bounds: Rc<Cell<Bounds<Pixels>>>,
}

impl BodyView {
    pub fn new(body: Arc<Body>, cx: &mut Context<Self>) -> Self {
        // A frame only when there is something new to show, and none after.
        let poll = Arc::clone(&body);
        cx.spawn(async move |this, cx| {
            while !poll.is_scanned() {
                cx.background_executor().timer(SCAN_POLL).await;
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    poll.stop();
                    return;
                }
            }
            let _ = this.update(cx, |_, cx| cx.notify());
        })
        .detach();
        Self {
            body,
            top: 0,
            sub: 0.,
            dragging: false,
            bounds: Rc::new(Cell::new(Bounds::default())),
        }
    }

    pub fn body(&self) -> &Body {
        &self.body
    }

    pub fn top(&self) -> u64 {
        self.top
    }

    fn height(&self) -> f32 {
        f32::from(self.bounds.get().size.height)
    }

    /// The largest position with a full view below it, in pixels.
    pub fn max_position(&self) -> f64 {
        (self.body.lines() as f64 * f64::from(LINE_HEIGHT) - f64::from(self.height())).max(0.)
    }

    /// Moves the top of the view to `y` pixels into the content, clamped.
    pub fn set_position(&mut self, y: f64) {
        let y = y.clamp(0., self.max_position());
        let line_height = f64::from(LINE_HEIGHT);
        self.top = (y / line_height) as u64;
        self.sub = (y - self.top as f64 * line_height) as f32;
    }

    fn position(&self) -> f64 {
        self.top as f64 * f64::from(LINE_HEIGHT) + f64::from(self.sub)
    }

    fn on_wheel(&mut self, event: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        let delta = f64::from(f32::from(event.delta.pixel_delta(px(LINE_HEIGHT)).y));
        self.set_position(self.position() - delta);
        cx.notify();
    }

    fn drag_to(&mut self, y: Pixels, cx: &mut Context<Self>) {
        let track = f64::from(self.height() - THUMB_MIN).max(1.);
        let inside = f32::from(y - self.bounds.get().origin.y);
        let at = (f64::from(inside) - f64::from(THUMB_MIN) / 2.) / track;
        self.set_position(at.clamp(0., 1.) * self.max_position());
        cx.notify();
    }

    fn render_line(&self, line: u64, shown: Line) -> impl IntoElement {
        let json = self.body.kind() == Kind::Json;
        let indent = if shown.depth > INDENT_MAX {
            INDENT_MAX as f32 * INDENT_W
        } else {
            shown.depth as f32 * INDENT_W
        };
        let number = (line + 1).to_string();
        let highlights: Vec<_> = if json {
            tokens(&shown.text)
                .into_iter()
                .map(|(range, token)| (range, highlight(token)))
                .collect()
        } else {
            Vec::new()
        };
        let text: SharedString = shown.text.into();
        let mut row = div()
            .h(px(LINE_HEIGHT))
            .flex()
            .flex_row()
            .items_center()
            .whitespace_nowrap()
            .overflow_hidden()
            .child(
                div()
                    .w(px(GUTTER_W))
                    .flex_shrink_0()
                    .pr_3()
                    .flex()
                    .justify_end()
                    .text_color(rgb(colors().faint))
                    .child(number),
            )
            .child(div().w(px(indent)).flex_shrink_0())
            .child(
                // The note below stays in view however long the text is.
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .child(StyledText::new(text).with_highlights(highlights)),
            );
        if shown.cut > 0 {
            row = row.child(
                div()
                    .flex_shrink_0()
                    .pl_2()
                    .text_color(rgb(colors().faint))
                    .child(format!("… {} more", size_text(shown.cut))),
            );
        }
        row
    }
}

impl Render for BodyView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        // Enough for any size of the panel; what does not fit is clipped.
        let in_view = (f32::from(window.viewport_size().height) / LINE_HEIGHT).ceil() as u64 + 1;
        let list_height = self.height();
        let lines = self.body.lines();
        let end = lines.min(self.top + in_view);
        let top = self.top;
        let rows: Vec<_> = self
            .body
            .read(top..end)
            .into_iter()
            .enumerate()
            .map(|(i, shown)| self.render_line(top + i as u64, shown))
            .collect();

        let max = self.max_position();
        let thumb = thumb_size(list_height, lines);
        let thumb_top = if max > 0. {
            (self.position() / max) as f32 * (list_height - thumb)
        } else {
            0.
        };
        let bounds = Rc::clone(&self.bounds);

        div()
            .id("body-list")
            .size_full()
            .relative()
            .overflow_hidden()
            .bg(rgb(c.code))
            .text_color(rgb(c.text))
            .font_family(crate::MONO_FONT)
            .text_size(px(12.))
            .on_scroll_wheel(cx.listener(Self::on_wheel))
            .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _, cx| {
                if this.dragging {
                    this.drag_to(event.position.y, cx);
                }
            }))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, _| this.dragging = false),
            )
            .child(
                canvas(
                    move |b, window, _| {
                        // What is drawn from it was drawn from the old size.
                        if bounds.replace(b).size != b.size {
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .child(
                div()
                    .absolute()
                    .top(px(-self.sub))
                    .left_0()
                    .right(px(BAR_W))
                    .children(rows),
            )
            .child(
                div()
                    .id("body-bar")
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .right_0()
                    .w(px(BAR_W))
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, event: &gpui::MouseDownEvent, _, cx| {
                            this.dragging = true;
                            this.drag_to(event.position.y, cx);
                        }),
                    )
                    .child(
                        div()
                            .absolute()
                            .top(px(thumb_top))
                            .h(px(thumb))
                            .left(px(2.))
                            .right(px(2.))
                            .rounded_sm()
                            .bg(rgb(c.splitter)),
                    ),
            )
    }
}

fn thumb_size(list_height: f32, lines: u64) -> f32 {
    let content = lines as f32 * LINE_HEIGHT;
    if content <= list_height {
        list_height
    } else {
        (list_height * list_height / content).max(THUMB_MIN)
    }
}

fn highlight(token: Token) -> HighlightStyle {
    let c = colors();
    let color = match token {
        Token::Key => c.json_key,
        Token::Str => c.json_string,
        Token::Number => c.json_number,
        Token::Literal => c.json_literal,
        Token::Punct => c.json_punct,
    };
    HighlightStyle {
        color: Some(Hsla::from(rgb(color))),
        ..Default::default()
    }
}

/// `2.1 MB · 40,000 lines`, or how far the reading has got.
pub fn summary(body: &Body) -> String {
    if body.is_scanned() {
        format!(
            "{} · {} lines",
            size_text(body.size()),
            crate::flows::group_digits(body.lines())
        )
    } else {
        format!(
            "{} · reading {:.0}%",
            size_text(body.size()),
            body.scanned() as f64 * 100. / body.size().max(1) as f64
        )
    }
}
