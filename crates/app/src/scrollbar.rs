//! A scroll bar for an area that scrolls up and down by itself (an
//! `overflow_y_scroll` div), which gpui draws none for: a thumb over the
//! area's right edge, shown only when there is more than fits, dragged, or
//! pressed above or below to jump there.

use std::{cell::Cell, rc::Rc};

use gpui::{
    AnyElement, Bounds, DispatchPhase, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, ScrollHandle, Window, canvas, div, fill, point, prelude::*, px, rgb,
    size,
};

use crate::colors::colors;

/// The strip the bar takes over the area's edge, and the thumb in it: as
/// wide as the body's.
pub const BAR_W: f32 = 13.;
pub const THUMB_W: f32 = 9.;
/// The shortest the thumb gets, however long what scrolls.
const THUMB_MIN: f32 = 28.;

/// Where an area is scrolled to, and the thumb's drag while it lasts: kept
/// from frame to frame by whoever draws the area.
#[derive(Clone, Default)]
pub struct Scrolled {
    pub handle: ScrollHandle,
    /// Where the pointer and the area were when the thumb was pressed.
    drag: Rc<Cell<Option<(Pixels, Pixels)>>>,
}

impl Scrolled {
    /// The bar for an area that already has a handle, such as a list's.
    pub fn of(handle: ScrollHandle) -> Self {
        Scrolled {
            handle,
            drag: Rc::default(),
        }
    }

    /// `area`, which scrolls with [`Scrolled::handle`], with the bar over it.
    pub fn around(&self, area: impl IntoElement) -> AnyElement {
        let me = self.clone();
        div()
            .relative()
            .size_full()
            .child(area)
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| me.paint(bounds, window),
                )
                .absolute()
                .top_0()
                .bottom_0()
                .right_0()
                .w(px(BAR_W)),
            )
            .into_any_element()
    }

    /// The thumb's length and how far down it is, for a bar `view` high.
    fn thumb(&self, view: Pixels) -> Option<(Pixels, Pixels)> {
        let max = self.handle.max_offset().y;
        if max <= px(0.) || view <= px(0.) {
            return None;
        }
        let long = (view * (view / (view + max))).max(px(THUMB_MIN)).min(view);
        let scrolled = (-self.handle.offset().y).clamp(px(0.), max);
        Some((long, (view - long) * (scrolled / max)))
    }

    /// Scrolls the area to `to`, from its top.
    fn scroll_to(&self, to: Pixels) {
        let max = self.handle.max_offset().y;
        let x = self.handle.offset().x;
        self.handle
            .set_offset(point(x, -to.clamp(px(0.), max.max(px(0.)))));
    }

    fn paint(&self, bar: Bounds<Pixels>, window: &mut Window) {
        let Some((long, down)) = self.thumb(bar.size.height) else {
            return;
        };
        let c = colors();
        let inset = (bar.size.width - px(THUMB_W)) / 2.;
        let thumb = Bounds::new(
            point(bar.left() + inset, bar.top() + down),
            size(px(THUMB_W), long),
        );
        let color = if self.drag.get().is_some() {
            c.muted
        } else {
            c.splitter
        };
        window.paint_quad(fill(thumb, rgb(color)).corner_radii(px(THUMB_W / 2.)));

        let me = self.clone();
        window.on_mouse_event(move |e: &MouseDownEvent, phase, window, cx| {
            if phase != DispatchPhase::Capture
                || e.button != MouseButton::Left
                || !bar.contains(&e.position)
            {
                return;
            }
            // The bar's, not the start of a selection under it.
            cx.stop_propagation();
            let Some((long, down)) = me.thumb(bar.size.height) else {
                return;
            };
            let max = me.handle.max_offset().y;
            let y = e.position.y - bar.top();
            // A press off the thumb brings the thumb's middle there first.
            if y < down || y > down + long {
                let track = (bar.size.height - long).max(px(1.));
                me.scroll_to(max * ((y - long / 2.).max(px(0.)) / track));
            }
            me.drag.set(Some((e.position.y, -me.handle.offset().y)));
            window.refresh();
        });
        let me = self.clone();
        window.on_mouse_event(move |e: &MouseMoveEvent, phase, window, _| {
            let Some((y0, from)) = me.drag.get() else {
                return;
            };
            if phase != DispatchPhase::Capture {
                return;
            }
            let Some((long, _)) = me.thumb(bar.size.height) else {
                return;
            };
            let track = (bar.size.height - long).max(px(1.));
            let per = me.handle.max_offset().y / track;
            me.scroll_to(from + (e.position.y - y0) * per);
            window.refresh();
        });
        let me = self.clone();
        window.on_mouse_event(move |_: &MouseUpEvent, phase, window, _| {
            if phase == DispatchPhase::Capture && me.drag.take().is_some() {
                window.refresh();
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_to_scroll_has_no_thumb() {
        let s = Scrolled::default();
        assert_eq!(s.thumb(px(300.)), None);
    }
}
