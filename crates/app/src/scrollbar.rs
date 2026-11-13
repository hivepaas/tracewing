//! Scroll bars for an area that scrolls by itself (an `overflow_scroll` div,
//! or a list), which gpui draws none for: a thumb over the area's right edge
//! for up and down, and over its bottom edge for sideways, each shown only
//! when there is more than fits that way, dragged, or pressed beside the thumb
//! to jump there.

use std::{cell::Cell, rc::Rc};

use gpui::{
    AnyElement, Bounds, DispatchPhase, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Point, ScrollHandle, Window, canvas, div, fill, point, prelude::*, px,
    rgb, size,
};

use crate::colors::colors;

/// The strip a bar takes over the area's edge, and the thumb in it: as wide
/// as the body's.
pub const BAR_W: f32 = 13.;
pub const THUMB_W: f32 = 9.;
/// The shortest a thumb gets, however long what scrolls.
pub const THUMB_MIN: f32 = 28.;

/// A thumb's length, and how far along its track it is, for a bar `view`
/// long over something that scrolls `max` further and has scrolled `done`.
/// `None` when everything fits.
pub fn thumb(view: f32, max: f32, done: f32) -> Option<(f32, f32)> {
    if max <= 0. || view <= 0. {
        return None;
    }
    let long = (view * view / (view + max)).max(THUMB_MIN).min(view);
    Some((long, (view - long) * (done.clamp(0., max) / max)))
}

/// Up and down, or sideways.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Axis {
    Down,
    Across,
}

impl Axis {
    fn of(self, p: Point<Pixels>) -> Pixels {
        match self {
            Axis::Down => p.y,
            Axis::Across => p.x,
        }
    }
}

/// Where an area is scrolled to, and a thumb's drag while it lasts: kept
/// from frame to frame by whoever draws the area.
#[derive(Clone, Default)]
pub struct Scrolled {
    pub handle: ScrollHandle,
    /// Which thumb was pressed, and where the pointer and the area were then.
    drag: Rc<Cell<Option<(Axis, Pixels, Pixels)>>>,
}

impl Scrolled {
    /// The bars for an area that already has a handle, such as a list's.
    pub fn of(handle: ScrollHandle) -> Self {
        Scrolled {
            handle,
            drag: Rc::default(),
        }
    }

    /// `area`, which scrolls with [`Scrolled::handle`], with its bars over it.
    pub fn around(&self, area: impl IntoElement) -> AnyElement {
        let (down, across) = (self.clone(), self.clone());
        div()
            .relative()
            .size_full()
            .child(area)
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| down.paint(Axis::Down, bounds, window),
                )
                .absolute()
                .top_0()
                .bottom_0()
                .right_0()
                .w(px(BAR_W)),
            )
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| across.paint(Axis::Across, bounds, window),
                )
                .absolute()
                .left_0()
                .right(px(BAR_W))
                .bottom_0()
                .h(px(BAR_W)),
            )
            .into_any_element()
    }

    /// How much further the area scrolls that way, and how far it has.
    fn reach(&self, axis: Axis) -> (Pixels, Pixels) {
        let max = axis.of(self.handle.max_offset());
        (
            max,
            (-axis.of(self.handle.offset())).clamp(px(0.), max.max(px(0.))),
        )
    }

    /// The thumb's length and how far along it is, for a bar `view` long.
    fn thumb(&self, axis: Axis, view: Pixels) -> Option<(Pixels, Pixels)> {
        let (max, done) = self.reach(axis);
        thumb(f32::from(view), f32::from(max), f32::from(done)).map(|(l, a)| (px(l), px(a)))
    }

    /// Scrolls the area to `to`, from its start, that way.
    fn scroll_to(&self, axis: Axis, to: Pixels) {
        let (max, _) = self.reach(axis);
        let to = -to.clamp(px(0.), max.max(px(0.)));
        let now = self.handle.offset();
        self.handle.set_offset(match axis {
            Axis::Down => point(now.x, to),
            Axis::Across => point(to, now.y),
        });
    }

    fn paint(&self, axis: Axis, bar: Bounds<Pixels>, window: &mut Window) {
        let along = |b: Bounds<Pixels>| match axis {
            Axis::Down => (b.top(), b.size.height),
            Axis::Across => (b.left(), b.size.width),
        };
        let (start, view) = along(bar);
        let Some((long, at)) = self.thumb(axis, view) else {
            return;
        };
        let c = colors();
        let thumb = match axis {
            Axis::Down => Bounds::new(
                point(bar.left() + (bar.size.width - px(THUMB_W)) / 2., start + at),
                size(px(THUMB_W), long),
            ),
            // On the area's bottom edge.
            Axis::Across => Bounds::new(
                point(start + at, bar.bottom() - px(THUMB_W)),
                size(long, px(THUMB_W)),
            ),
        };
        let color = if self.drag.get().is_some_and(|(a, _, _)| a == axis) {
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
            let Some((long, at)) = me.thumb(axis, view) else {
                return;
            };
            let (max, _) = me.reach(axis);
            let p = axis.of(e.position) - start;
            // A press off the thumb brings the thumb's middle there first.
            if p < at || p > at + long {
                let track = (view - long).max(px(1.));
                me.scroll_to(axis, max * ((p - long / 2.).max(px(0.)) / track));
            }
            let (_, done) = me.reach(axis);
            me.drag.set(Some((axis, axis.of(e.position), done)));
            window.refresh();
        });
        let me = self.clone();
        window.on_mouse_event(move |e: &MouseMoveEvent, phase, window, _| {
            let Some((dragged, p0, from)) = me.drag.get() else {
                return;
            };
            if phase != DispatchPhase::Capture || dragged != axis {
                return;
            }
            let Some((long, _)) = me.thumb(axis, view) else {
                return;
            };
            let (max, _) = me.reach(axis);
            let per = max / (view - long).max(px(1.));
            me.scroll_to(axis, from + (axis.of(e.position) - p0) * per);
            window.refresh();
        });
        let me = self.clone();
        window.on_mouse_event(move |_: &MouseUpEvent, phase, window, _| {
            if phase == DispatchPhase::Capture && me.drag.get().is_some_and(|(a, _, _)| a == axis) {
                me.drag.set(None);
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
        assert_eq!(s.thumb(Axis::Down, px(300.)), None);
        assert_eq!(s.thumb(Axis::Across, px(300.)), None);
        assert_eq!(thumb(300., 0., 0.), None);
    }

    #[test]
    fn the_thumb_is_as_long_as_the_share_in_view_and_goes_along_it() {
        // Half of it in view: a thumb half the bar, at the start, then at the end.
        assert_eq!(thumb(200., 200., 0.), Some((100., 0.)));
        assert_eq!(thumb(200., 200., 200.), Some((100., 100.)));
        // A long way to go: never shorter than the least a hand can hold.
        assert_eq!(thumb(200., 100_000., 0.).map(|(l, _)| l), Some(THUMB_MIN));
    }
}
