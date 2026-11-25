//! Scroll bars, which gpui draws none of: one for every area in the window
//! that scrolls, whatever it scrolls by. A thumb over the area's right edge
//! for up and down, and over its bottom edge for sideways, each shown only
//! when there is more than fits that way, dragged, or pressed beside the
//! thumb to jump there.
//!
//! **Every bar in the window is drawn here**, and that is the point of this
//! file. They used to be drawn in five places, and had drifted apart: a 9 px
//! thumb in four of them and 6 px in a field, a shortest thumb of 28 px or
//! 24, a thumb that took a colour while it was held in some and not in
//! others, and a press on the thumb that dragged it from where it was in
//! some and snapped its middle to the pointer in others. Each was a choice
//! nobody had made, and they looked it. What an area scrolls by is still its
//! own -- a [`ScrollHandle`], a list that stops following the newest flow
//! when it is moved, a body that goes by lines -- and [`Scrolls`] is all a
//! bar asks of it.
//!
//! The look is shellwing's: a thin thumb that stays out of the way of what
//! it is beside, and grows under the pointer, where a hand needs something
//! to take hold of. At rest it is exactly as wide as a divider's grip
//! between two panels (`ui(6.)`, `Traffic::splitter`), and drawn in the same
//! colour, so that a bar reads as part of the same frame rather than as
//! something laid over it.

use std::{cell::Cell, rc::Rc};

use gpui::{
    AnyElement, App, Bounds, Canvas, CursorStyle, DispatchPhase, Hitbox, HitboxBehavior,
    IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point,
    ScrollHandle, Window, canvas, div, fill, point, prelude::*, px, rgb, rgba, size,
};

use crate::{colors::colors, text_size::ui};

/// The strip a bar takes over the area's edge: where a press lands, and what
/// an area leaves clear beside its text for it. It never changes width as
/// the thumb does, or a pointer aimed at the thumb would fall out of it.
pub const BAR_W: f32 = 13.;
/// The thumb at rest, at the design's size: a divider's grip, the same
/// constant rather than the same number, so that the two cannot drift.
const THIN: f32 = crate::traffic::GRIP_W;
/// The thumb under the pointer or held, at the design's size.
const WIDE: f32 = 8.;
/// Between the thumb and the area's edge, so that it does not touch it.
const INSET: f32 = 2.;
/// The shortest a thumb gets, however long what scrolls.
pub const THUMB_MIN: f32 = 28.;

// Held where the compiler holds them rather than in a test: a thumb that
// did not grow under the pointer, or grew out of its strip, would build.
const _: () = assert!(WIDE > THIN, "a thumb under the pointer has to grow");
const _: () = assert!(INSET + WIDE < BAR_W, "the grown thumb fits its strip");

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
pub enum Axis {
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

/// What a bar asks of what it scrolls, one way at a time: how much further
/// it goes at most, how far it has gone, and to go somewhere. In pixels, or
/// in any unit that stands for them, as long as the three agree: the thumb
/// is as long as the share of `max` plus the view that is in view.
pub trait Scrolls: 'static {
    /// How much further it scrolls that way at most, and how far it has.
    fn reach(&self, axis: Axis, cx: &App) -> (f32, f32);
    /// Scrolled to `to`, from its start, that way. Whatever it does besides
    /// is its own: the flow list stops following the newest flow.
    fn scroll_to(&self, axis: Axis, to: f32, cx: &mut App);
}

/// A thumb's drag while it lasts: which way, where the pointer was along it,
/// and how far the area had scrolled. Kept by whoever draws the area, from
/// frame to frame; one for both bars of an area, as only one is held.
pub type Drag = Rc<Cell<Option<(Axis, Pixels, f32)>>>;

/// A bar `axis` way over the edge of an area it is laid over, positioned for
/// an area that is `relative`. A caller with a track of its own moves it
/// with the usual styles: the body's sideways bar starts past its corner.
pub fn bar(axis: Axis, scrolls: impl Scrolls, drag: Drag) -> Canvas<Hitbox> {
    let scrolls: Rc<dyn Scrolls> = Rc::new(scrolls);
    let strip = canvas(
        |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
        move |bounds, hitbox, window, cx| {
            paint(axis, bounds, Some(&hitbox), &scrolls, &drag, window, cx)
        },
    )
    .absolute();
    match axis {
        Axis::Down => strip.top_0().bottom_0().right_0().w(px(BAR_W)),
        Axis::Across => strip.left_0().right(px(BAR_W)).bottom_0().h(px(BAR_W)),
    }
}

/// One bar, in `strip`, over something that scrolls as `scrolls` says: the
/// thumb, and the pointer's way with it. For an element that paints itself,
/// such as a field; everything else lays a [`bar`] over its area.
pub fn paint(
    axis: Axis,
    strip: Bounds<Pixels>,
    hitbox: Option<&Hitbox>,
    scrolls: &Rc<dyn Scrolls>,
    drag: &Drag,
    window: &mut Window,
    cx: &mut App,
) {
    let along = |b: Bounds<Pixels>| match axis {
        Axis::Down => (b.top(), b.size.height),
        Axis::Across => (b.left(), b.size.width),
    };
    let (start, view) = along(strip);
    let view_f = f32::from(view);
    let (max, done) = scrolls.reach(axis, cx);
    // Nothing to scroll, nothing drawn and nothing taken: a press there is
    // the area's own.
    let Some((long, at)) = thumb(view_f, max, done) else {
        return;
    };
    let held = drag.get().is_some_and(|(a, _, _)| a == axis);
    let hovered = strip.contains(&window.mouse_position());
    let wide = held || hovered;
    // At the design's size, so it grows with the text as the divider's grip
    // does -- but never out of the strip, which does not.
    let w = ui(if wide { WIDE } else { THIN })
        .to_pixels(window.rem_size())
        .min(px(BAR_W - INSET));
    let (long, at) = (px(long), px(at));
    let shape = match axis {
        Axis::Down => Bounds::new(
            point(strip.right() - px(INSET) - w, start + at),
            size(w, long),
        ),
        Axis::Across => Bounds::new(
            point(start + at, strip.bottom() - px(INSET) - w),
            size(long, w),
        ),
    };
    let c = colors();
    // At rest, the grip's own colour. Under the pointer, darker -- but seen
    // through, as shellwing's is: solid, it was a slab over the last
    // column's text, which runs under the bar where a column is cut off.
    let color = if wide {
        rgba(crate::colors::with_alpha(c.muted, 0x99))
    } else {
        rgb(c.splitter)
    };
    window.paint_quad(fill(shape, color).corner_radii(w / 2.));
    // A thumb is taken hold of, not typed into: the text's I-beam over it
    // said otherwise.
    if let Some(hitbox) = hitbox {
        window.set_cursor_style(CursorStyle::Arrow, hitbox);
    }

    let (me, d) = (Rc::clone(scrolls), Rc::clone(drag));
    window.on_mouse_event(move |e: &MouseDownEvent, phase, window, cx| {
        if phase != DispatchPhase::Capture
            || e.button != MouseButton::Left
            || !strip.contains(&e.position)
        {
            return;
        }
        // The bar's, not the start of a selection under it.
        cx.stop_propagation();
        let (max, done) = me.reach(axis, cx);
        let Some((long, at)) = thumb(view_f, max, done) else {
            return;
        };
        let p = f32::from(axis.of(e.position) - start);
        // Off the thumb: its middle there first, then held from there. On
        // it: held where it was taken, so that taking it moves nothing.
        let done = if p < at || p > at + long {
            let to = max * ((p - long / 2.).max(0.) / (view_f - long).max(1.));
            me.scroll_to(axis, to.min(max), cx);
            me.reach(axis, cx).1
        } else {
            done
        };
        d.set(Some((axis, axis.of(e.position), done)));
        window.refresh();
    });
    let (me, d) = (Rc::clone(scrolls), Rc::clone(drag));
    window.on_mouse_event(move |e: &MouseMoveEvent, phase, window, cx| {
        // Anywhere in the window while the button is held: a drag that
        // leaves the area goes on.
        let Some((dragged, p0, from)) = d.get() else {
            return;
        };
        if phase != DispatchPhase::Capture || dragged != axis {
            return;
        }
        let (max, done) = me.reach(axis, cx);
        let Some((long, _)) = thumb(view_f, max, done) else {
            return;
        };
        let per = max / (view_f - long).max(1.);
        me.scroll_to(axis, from + f32::from(axis.of(e.position) - p0) * per, cx);
        window.refresh();
    });
    let d = Rc::clone(drag);
    window.on_mouse_event(move |_: &MouseUpEvent, phase, window, cx| {
        if phase == DispatchPhase::Capture && d.get().is_some_and(|(a, _, _)| a == axis) {
            d.set(None);
            // The end of a drag, not a click on whatever it ended over: let
            // go over a token, and the token was being read.
            cx.stop_propagation();
            window.refresh();
        }
    });
    // Into the strip and out of it again: drawn afresh, so that the thumb's
    // width follows the pointer. Each frame remembers what it drew, and asks
    // for another only when that stops being true.
    window.on_mouse_event(move |e: &MouseMoveEvent, phase, window, _| {
        if phase == DispatchPhase::Capture && strip.contains(&e.position) != hovered {
            window.refresh();
        }
    });
}

/// Where an area is scrolled to, and a thumb's drag while it lasts: kept
/// from frame to frame by whoever draws the area. For an area that scrolls
/// by a [`ScrollHandle`]: an `overflow_scroll` div, or a list.
#[derive(Clone, Default)]
pub struct Scrolled {
    pub handle: ScrollHandle,
    drag: Drag,
}

impl Scrolled {
    /// The bars for an area that already has a handle, such as a list's.
    pub fn of(handle: ScrollHandle) -> Self {
        Scrolled {
            handle,
            drag: Rc::default(),
        }
    }

    /// Whether there is more than fits up and down, as last laid out: for an
    /// area that leaves room beside its text for a bar only when it has one.
    pub fn scrolls(&self) -> bool {
        self.handle.max_offset().y > px(0.)
    }

    /// `area` -- a flex child that grows into its parent (`flex_1`) and
    /// scrolls up and down with [`Scrolled::handle`] -- with its bars over
    /// it, in the room the area took. For a page, whose scrolling area is
    /// laid out among others rather than filling a box of its own as
    /// [`Scrolled::around`]'s does.
    pub fn in_place(&self, area: impl IntoElement) -> AnyElement {
        div()
            .relative()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .child(area)
            .children(self.bars())
            .into_any_element()
    }

    /// `area`, which scrolls with [`Scrolled::handle`], with its bars over it.
    pub fn around(&self, area: impl IntoElement) -> AnyElement {
        div()
            .relative()
            .size_full()
            .child(area)
            .children(self.bars())
            .into_any_element()
    }

    /// The bars alone, to lay over an area of the caller's own: up and down
    /// over its right edge, sideways over its bottom. The area must be
    /// positioned for them (relative).
    pub fn bars(&self) -> [AnyElement; 2] {
        [
            bar(Axis::Down, self.clone(), Rc::clone(&self.drag)).into_any_element(),
            bar(Axis::Across, self.clone(), Rc::clone(&self.drag)).into_any_element(),
        ]
    }
}

impl Scrolls for Scrolled {
    fn reach(&self, axis: Axis, _: &App) -> (f32, f32) {
        let max = axis.of(self.handle.max_offset()).max(px(0.));
        let done = (-axis.of(self.handle.offset())).clamp(px(0.), max);
        (f32::from(max), f32::from(done))
    }

    fn scroll_to(&self, axis: Axis, to: f32, cx: &mut App) {
        let (max, _) = self.reach(axis, cx);
        let to = -px(to.clamp(0., max));
        let now = self.handle.offset();
        self.handle.set_offset(match axis {
            Axis::Down => point(now.x, to),
            Axis::Across => point(to, now.y),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_to_scroll_has_no_thumb() {
        assert_eq!(thumb(300., 0., 0.), None);
        assert_eq!(thumb(0., 100., 0.), None);
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
