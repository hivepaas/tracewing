//! The window's title bar, and on Linux its whole frame. Taken from packwing and
//! shellwing, with tracewing's colors and the window's name in the bar.
//!
//! GNOME's Wayland compositor draws no frame for clients, so the window provides
//! its own title bar (drag to move, double-click to maximise, right-click for the
//! window menu, buttons), a thin border and a shadow.
//!
//! macOS and Windows draw the frame (border, shadow, corners, resizing) but
//! hide their title bar, and the app draws one in its place, in the window's
//! own ground so the page runs up to the top. On macOS the system's traffic
//! lights stay, over the left end. On Windows the app draws the caption
//! buttons and marks them, and the empty parts of the bar, as window control
//! areas: Windows then treats them as its own, so dragging, Snap, double-click
//! to maximise, the window menu and the Snap Layouts flyout over the maximise
//! button all work as in native apps.
//!
//! Beside the window's buttons, on their inner side, a switch goes round the
//! light look, the dark one and the system's. On Linux the buttons sit where the
//! desktop lays them out (GNOME's `button-layout`).
//!
//! Resizing works as in GTK apps: an invisible margin (the client inset) runs
//! around the visible window, and pressing there resizes. Near a corner a longer
//! stretch of each edge resizes diagonally, so corners are easy to grab. Tiled
//! or maximised sides have neither margin nor border.

use gpui::{
    App, BoxShadow, Corners, CursorStyle, Decorations, Div, Edges, FontWeight, HitboxBehavior,
    Hsla, MouseButton, Pixels, Point, ResizeEdge, Size, Stateful, Tiling, Window, WindowButton,
    WindowButtonLayout, WindowControlArea, canvas, div, point, prelude::*, px, rgb, rgba, svg,
    transparent_black,
};

use crate::colors::colors;
use crate::layout::Appearance;
use crate::widgets::tip;

/// The title bar: as tall as GNOME's own on Linux; on macOS and Windows a
/// little more than the system's own.
pub const TITLEBAR_H: f32 = if cfg!(any(target_os = "linux", target_os = "freebsd")) {
    44.0
} else {
    36.0
};
/// Where macOS puts its traffic lights (the close button's top left): centred
/// in the bar, as the buttons are 14 px tall.
pub const TRAFFIC_LIGHTS: Point<Pixels> = Point {
    x: px(14.),
    y: px((TITLEBAR_H - 14.) / 2.),
};
/// How far the traffic lights reach from their left end: three 14 px
/// buttons, 20 px apart.
const TRAFFIC_LIGHTS_W: f32 = 54.0;
/// A Windows caption button's width, as Windows 11 draws them.
const CAPTION_W: f32 = 46.0;
/// The invisible margin outside the visible window that resizes it; the shadow
/// is drawn into it.
const INSET: f32 = 12.0;
const BORDER: f32 = 1.0;
/// Along each edge, this far from the window's corner resizes diagonally.
const CORNER: f32 = 28.0;
/// How round the window's corners are, as GNOME rounds its own.
pub const RADIUS: f32 = 10.0;

/// The radius of each corner of the window: [`RADIUS`] where the app draws
/// the frame and both sides of the corner are free (not tiled, not
/// maximised); none where the system draws the frame, which rounds its own.
pub fn corners(window: &Window) -> Corners<Pixels> {
    let Decorations::Client { tiling: t } = window.window_decorations() else {
        return Corners::default();
    };
    let round = |a: bool, b: bool| if a || b { px(0.) } else { px(RADIUS) };
    Corners {
        top_left: round(t.top, t.left),
        top_right: round(t.top, t.right),
        bottom_left: round(t.bottom, t.left),
        bottom_right: round(t.bottom, t.right),
    }
}

/// The invisible margin outside the visible window, on each side: none on a
/// tiled side, and none where the system draws the frame.
pub fn margins(window: &Window) -> Edges<Pixels> {
    let Decorations::Client { tiling: t } = window.window_decorations() else {
        return Edges::default();
    };
    let side = |tiled: bool| if tiled { px(0.) } else { px(INSET) };
    Edges {
        top: side(t.top),
        right: side(t.right),
        bottom: side(t.bottom),
        left: side(t.left),
    }
}

fn tiling(window: &Window) -> Tiling {
    match window.window_decorations() {
        Decorations::Client { tiling } => tiling,
        Decorations::Server => Tiling::tiled(),
    }
}

/// Whether the app draws the title bar: always on macOS and Windows (inside
/// the system's frame), on Linux where it draws the whole frame (Wayland).
/// Elsewhere (X11 without client-side decorations) the system's title bar
/// stays and the app draws none.
pub fn draws_title_bar(window: &Window) -> bool {
    cfg!(any(target_os = "macos", target_os = "windows"))
        || matches!(window.window_decorations(), Decorations::Client { .. })
}

/// GNOME's own layout of the window's buttons, where the desktop says none.
const GNOME_LAYOUT: WindowButtonLayout = WindowButtonLayout {
    left: [None; 3],
    right: [
        Some(WindowButton::Minimize),
        Some(WindowButton::Maximize),
        Some(WindowButton::Close),
    ],
};

/// An end of the title bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum End {
    Left,
    Right,
}

/// The end the appearance switch takes on Linux: the one the window's buttons are
/// at, beside them on their inner side; the right where they are at both, or at
/// neither.
fn switch_end(layout: &WindowButtonLayout) -> End {
    let has = |side: &[Option<WindowButton>; 3]| side.iter().any(Option::is_some);
    if has(&layout.left) && !has(&layout.right) {
        End::Left
    } else {
        End::Right
    }
}

/// A round button with a symbolic icon, as GNOME draws the window's.
fn gnome_button(b: WindowButton) -> Stateful<Div> {
    let (icon, act): (&str, fn(&mut Window)) = match b {
        WindowButton::Minimize => ("icons/minimize.svg", |w| w.minimize_window()),
        WindowButton::Maximize => ("icons/maximize.svg", |w| w.zoom_window()),
        WindowButton::Close => ("icons/close.svg", |w| w.remove_window()),
    };
    div()
        .id(b.id())
        .size(px(24.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(rgb(colors().button))
        .hover(|s| s.bg(rgb(colors().button_hover)))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(move |_, window, _| act(window))
        .child(
            svg()
                .path(icon)
                .size(px(14.))
                .text_color(rgb(colors().text)),
        )
}

/// The title bar's switch between the light look, the dark one and the system's:
/// it shows the one in use, and a click takes the next. Drawn as the window's
/// buttons beside it are; it takes the press itself, so the bar under it neither
/// moves the window nor, on Windows, counts it its own.
pub fn appearance_switch(now: Appearance) -> Stateful<Div> {
    let c = colors();
    let icon = match now {
        Appearance::Light => "icons/appearance-light.svg",
        Appearance::Dark => "icons/appearance-dark.svg",
        Appearance::System => "icons/appearance-system.svg",
    };
    let said = format!(
        "Appearance: {}. Click for {}.",
        now.name(),
        now.next().name()
    );
    div()
        .id("appearance")
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .occlude()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .tooltip(tip(said))
        .map(|d| {
            if cfg!(target_os = "windows") {
                d.w(px(CAPTION_W)).h_full().hover(|s| s.bg(rgb(c.hover)))
            } else if cfg!(target_os = "macos") {
                d.size(px(26.)).rounded_full().hover(|s| s.bg(rgb(c.hover)))
            } else {
                d.size(px(24.))
                    .rounded_full()
                    .bg(rgb(c.button))
                    .hover(|s| s.bg(rgb(c.button_hover)))
            }
        })
        .child(svg().path(icon).size(px(14.)).text_color(rgb(c.text)))
}

/// Flat full-height buttons, as Windows draws them. They do nothing on a
/// click themselves: as window control areas, Windows sees its own caption
/// buttons there and GPUI carries out the press.
fn windows_buttons(window: &Window) -> Div {
    let button = |id: &'static str, icon: &'static str, area: WindowControlArea| {
        let close = area == WindowControlArea::Close;
        div()
            .id(id)
            .group(id)
            .w(px(CAPTION_W))
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .window_control_area(area)
            .when(close, |d| d.hover(|s| s.bg(rgb(0xc42b1c))))
            .when(!close, |d| d.hover(|s| s.bg(rgb(colors().hover))))
            .child(
                svg()
                    .path(icon)
                    .size(px(10.))
                    .text_color(rgb(colors().text))
                    .when(close, |d| {
                        d.group_hover(id, |s| s.text_color(gpui::white()))
                    }),
            )
    };
    let max = if window.is_maximized() {
        "icons/win-restore.svg"
    } else {
        "icons/win-maximize.svg"
    };
    div()
        .flex()
        .h_full()
        .child(button(
            "win-min",
            "icons/win-minimize.svg",
            WindowControlArea::Min,
        ))
        .child(button("win-max", max, WindowControlArea::Max))
        .child(button(
            "win-close",
            "icons/win-close.svg",
            WindowControlArea::Close,
        ))
}

/// The window frame around `content`, with a title bar in `bg` above it where
/// the app draws one: the window's name and buttons, and beside them the
/// appearance `switch` ([`appearance_switch`]).
pub fn window_chrome(
    content: Div,
    bg: Hsla,
    switch: Stateful<Div>,
    window: &mut Window,
    cx: &App,
) -> Div {
    let t = tiling(window);
    let client_side = matches!(window.window_decorations(), Decorations::Client { .. });
    if client_side {
        window.set_client_inset(px(INSET));
    }
    let round = corners(window);
    let inner = |r: Pixels| if r > px(0.) { r - px(BORDER) } else { r };
    let fullscreen = window.is_fullscreen();
    let (mac, windows) = (cfg!(target_os = "macos"), cfg!(target_os = "windows"));
    let row = || div().flex().items_center().h_full();
    let mut switch = Some(switch);
    let (left, right) = if windows {
        // Full screen has no caption: Windows ignores the control areas then.
        let buttons = (!fullscreen).then(|| windows_buttons(window));
        (
            row(),
            row().pl(px(6.)).children(switch.take()).children(buttons),
        )
    } else if mac {
        // The system draws the traffic lights over the left end.
        let after = TRAFFIC_LIGHTS.x + px(TRAFFIC_LIGHTS_W + 10.);
        (row().pl(after).children(switch.take()), row())
    } else {
        let layout = cx.button_layout().unwrap_or(GNOME_LAYOUT);
        let end = switch_end(&layout);
        let buttons = |side: &[Option<WindowButton>; 3]| {
            side.iter()
                .flatten()
                .map(|&b| gnome_button(b))
                .collect::<Vec<_>>()
        };
        let left = row()
            .gap(px(10.))
            .pl(px(10.))
            .pr(px(6.))
            .children(buttons(&layout.left))
            .children(switch.take_if(|_| end == End::Left));
        let right = row()
            .gap(px(10.))
            .pl(px(6.))
            .pr(px(10.))
            .children(switch.take_if(|_| end == End::Right))
            .children(buttons(&layout.right));
        (left, right)
    };
    let titlebar = draws_title_bar(window).then(|| {
        div()
            .h(px(TITLEBAR_H))
            .w_full()
            .flex_shrink_0()
            .flex()
            .items_center()
            .rounded_tl(inner(round.top_left))
            .rounded_tr(inner(round.top_right))
            // On Windows the system moves the window from the drag areas
            // itself (see `windows_buttons`); handling the press here would
            // stop it.
            .when(windows, |d| d.window_control_area(WindowControlArea::Drag))
            .when(!windows, |d| {
                d.on_mouse_down(MouseButton::Left, move |e, window, _| {
                    if e.click_count != 2 {
                        window.start_window_move();
                    } else if mac {
                        // Zoom or minimise, as set in System Settings.
                        window.titlebar_double_click();
                    } else {
                        window.zoom_window();
                    }
                })
            })
            .when(client_side, |d| {
                d.on_mouse_down(MouseButton::Right, |e, window, _| {
                    window.show_window_menu(e.position)
                })
            })
            .child(left)
            .child(
                div()
                    .flex_1()
                    .flex()
                    .justify_center()
                    .text_size(px(13.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(colors().muted))
                    .child("tracewing"),
            )
            .child(right)
    });
    let (inset, border) = (px(INSET), px(BORDER));
    let floating = !(t.top && t.bottom && t.left && t.right);
    div()
        .size_full()
        .bg(transparent_black())
        .when(!t.top, |d| d.pt(inset))
        .when(!t.bottom, |d| d.pb(inset))
        .when(!t.left, |d| d.pl(inset))
        .when(!t.right, |d| d.pr(inset))
        .child(
            canvas(
                |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
                move |bounds, hitbox, window, _| {
                    if let Some(e) = resize_edge(window.mouse_position(), bounds.size, t) {
                        use ResizeEdge::*;
                        let style = match e {
                            Top | Bottom => CursorStyle::ResizeUpDown,
                            Left | Right => CursorStyle::ResizeLeftRight,
                            TopLeft | BottomRight => CursorStyle::ResizeUpLeftDownRight,
                            TopRight | BottomLeft => CursorStyle::ResizeUpRightDownLeft,
                        };
                        window.set_cursor_style(style, &hitbox);
                    }
                },
            )
            .size_full()
            .absolute()
            .top_0()
            .left_0(),
        )
        .on_mouse_move(|_, window, _| window.refresh())
        .on_mouse_down(MouseButton::Left, move |e, window, _| {
            if let Some(edge) = resize_edge(e.position, window.viewport_size(), t) {
                window.start_window_resize(edge);
            }
        })
        .child(
            div()
                .size_full()
                .flex()
                .flex_col()
                .bg(bg)
                .rounded_tl(round.top_left)
                .rounded_tr(round.top_right)
                .rounded_bl(round.bottom_left)
                .rounded_br(round.bottom_right)
                .border_color(rgb(colors().window_edge))
                .when(!t.top, |d| d.border_t(border))
                .when(!t.bottom, |d| d.border_b(border))
                .when(!t.left, |d| d.border_l(border))
                .when(!t.right, |d| d.border_r(border))
                .when(floating, |d| {
                    d.shadow(vec![BoxShadow {
                        color: rgba(colors().shadow).into(),
                        offset: point(px(0.), px(2.)),
                        blur_radius: px(INSET - 2.),
                        spread_radius: px(0.),
                        inset: false,
                    }])
                })
                .children(titlebar)
                .child(content),
        )
}

/// Which way a press at `pos` resizes a window of `size`, if at all: anywhere in
/// the margin and border of an untiled side, diagonally near a corner.
fn resize_edge(pos: Point<Pixels>, size: Size<Pixels>, t: Tiling) -> Option<ResizeEdge> {
    use ResizeEdge::*;
    let grab = px(INSET + BORDER);
    let corner = px(INSET + CORNER);
    let (l, r) = (
        !t.left && pos.x < grab,
        !t.right && pos.x > size.width - grab,
    );
    let (top, b) = (
        !t.top && pos.y < grab,
        !t.bottom && pos.y > size.height - grab,
    );
    let (near_l, near_r) = (
        !t.left && pos.x < corner,
        !t.right && pos.x > size.width - corner,
    );
    let (near_t, near_b) = (
        !t.top && pos.y < corner,
        !t.bottom && pos.y > size.height - corner,
    );
    Some(if (top && near_l) || (l && near_t) {
        TopLeft
    } else if (top && near_r) || (r && near_t) {
        TopRight
    } else if (b && near_l) || (l && near_b) {
        BottomLeft
    } else if (b && near_r) || (r && near_b) {
        BottomRight
    } else if top {
        Top
    } else if b {
        Bottom
    } else if l {
        Left
    } else if r {
        Right
    } else {
        return None;
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::size;

    const W: f32 = 800.;
    const H: f32 = 600.;

    fn edge(x: f32, y: f32, t: Tiling) -> Option<ResizeEdge> {
        resize_edge(point(px(x), px(y)), size(px(W), px(H)), t)
    }

    #[test]
    fn corners_take_a_long_stretch_of_each_edge() {
        let t = Tiling::default();
        assert_eq!(edge(W - 30., H - 4., t), Some(ResizeEdge::BottomRight));
        assert_eq!(edge(W - 4., H - 30., t), Some(ResizeEdge::BottomRight));
        assert_eq!(edge(5., 5., t), Some(ResizeEdge::TopLeft));
        assert_eq!(edge(W / 2., H - 4., t), Some(ResizeEdge::Bottom));
        assert_eq!(edge(4., H / 2., t), Some(ResizeEdge::Left));
    }

    #[test]
    fn the_switch_sits_beside_the_window_s_buttons() {
        use WindowButton::*;
        let layout = |left: [Option<WindowButton>; 3], right| WindowButtonLayout { left, right };
        assert_eq!(switch_end(&GNOME_LAYOUT), End::Right);
        let all_left = layout([Some(Close), Some(Minimize), Some(Maximize)], [None; 3]);
        assert_eq!(switch_end(&all_left), End::Left);
        let split = layout([Some(Close), None, None], [Some(Minimize), None, None]);
        assert_eq!(switch_end(&split), End::Right);
        assert_eq!(switch_end(&layout([None; 3], [None; 3])), End::Right);
    }

    #[test]
    fn content_and_tiled_sides_never_resize() {
        assert_eq!(edge(W / 2., H / 2., Tiling::default()), None);
        assert_eq!(edge(W - 2., H / 2., Tiling::tiled()), None);
    }
}
