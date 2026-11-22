//! Where the window opens: where it was last left, if that is still on a
//! screen, and never larger than the screen it opens on.
//! Taken from shellwing.
//!
//! Wayland does not let a client know or choose its position, so there only the
//! size and the maximised or fullscreen state come back; the compositor places
//! the window.

use crate::layout::{WindowMode, WindowState};
use gpui::{App, Bounds, Pixels, WindowBounds, point, px, size};

const DEFAULT_SIZE: (f32, f32) = (1440., 900.);
pub const MIN_SIZE: (f32, f32) = (900., 560.);
/// How much of the window must be on a display to keep its old position: enough
/// to grab the title bar.
const MIN_VISIBLE: (f32, f32) = (120., 40.);

pub fn initial(saved: Option<WindowState>, cx: &App) -> WindowBounds {
    // Where windows may go on each display: its work area, without the
    // taskbar or the dock.
    let areas: Vec<Bounds<Pixels>> = cx.displays().iter().map(|d| d.visible_bounds()).collect();
    let primary = cx.primary_display().map(|d| d.visible_bounds());
    let (bounds, mode) = placed(saved, &areas, primary);
    match mode {
        WindowMode::Windowed => WindowBounds::Windowed(bounds),
        WindowMode::Maximized => WindowBounds::Maximized(bounds),
        WindowMode::Fullscreen => WindowBounds::Fullscreen(bounds),
    }
}

/// The bounds to open with, given each display's work area and the primary
/// one's.
///
/// **Never larger than the display it opens on.** A size kept from a larger
/// screen, or from a smaller scale, came back as it was, and so did the
/// default: on a 1920x1080 laptop at 150%, a work area of 1280x672, the
/// window opened 1440 wide and its caption buttons were past the screen's
/// edge, where no pointer reaches. Found in the Windows VM at 150%, where
/// the title bar's check (S11) could not get the pointer onto the maximize
/// and close buttons of a window 2065 pixels wide on a screen of 2039.
fn placed(
    saved: Option<WindowState>,
    areas: &[Bounds<Pixels>],
    primary: Option<Bounds<Pixels>>,
) -> (Bounds<Pixels>, WindowMode) {
    let primary = primary.or_else(|| areas.first().copied());
    let Some(s) = saved else {
        let wanted = size(px(DEFAULT_SIZE.0), px(DEFAULT_SIZE.1));
        return match primary {
            Some(area) => (
                centered_in(area, fitted(wanted, area)),
                WindowMode::Windowed,
            ),
            None => (
                Bounds::new(point(px(0.), px(0.)), wanted),
                WindowMode::Windowed,
            ),
        };
    };
    let wanted = Bounds::new(
        point(px(s.x), px(s.y)),
        size(px(s.width.max(MIN_SIZE.0)), px(s.height.max(MIN_SIZE.1))),
    );
    let bounds = match (on_a_display(wanted, areas), primary) {
        // Where it was, made to fit, and moved in as far as that takes.
        (Some(area), _) => inside(area, wanted.origin, fitted(wanted.size, area)),
        // The display it was on is gone: centered on the primary one.
        (None, Some(area)) => centered_in(area, fitted(wanted.size, area)),
        (None, None) => wanted,
    };
    (bounds, s.mode)
}

/// `wanted`, no larger than `area` in either direction.
fn fitted(wanted: gpui::Size<Pixels>, area: Bounds<Pixels>) -> gpui::Size<Pixels> {
    size(
        wanted.width.min(area.size.width),
        wanted.height.min(area.size.height),
    )
}

fn centered_in(area: Bounds<Pixels>, of: gpui::Size<Pixels>) -> Bounds<Pixels> {
    Bounds::new(
        point(
            area.origin.x + (area.size.width - of.width) / 2.,
            area.origin.y + (area.size.height - of.height) / 2.,
        ),
        of,
    )
}

/// At `at`, moved only as far as it takes for all of `of` to be in `area`.
fn inside(area: Bounds<Pixels>, at: gpui::Point<Pixels>, of: gpui::Size<Pixels>) -> Bounds<Pixels> {
    let right = area.origin.x + area.size.width - of.width;
    let bottom = area.origin.y + area.size.height - of.height;
    Bounds::new(
        point(
            at.x.min(right).max(area.origin.x),
            at.y.min(bottom).max(area.origin.y),
        ),
        of,
    )
}

/// The work area most of the window is on, where enough of it is on one to
/// grab its title bar.
fn on_a_display(window: Bounds<Pixels>, areas: &[Bounds<Pixels>]) -> Option<Bounds<Pixels>> {
    areas
        .iter()
        .filter_map(|area| {
            let v = area.intersect(&window).size;
            (v.width >= px(MIN_VISIBLE.0) && v.height >= px(MIN_VISIBLE.1))
                .then(|| (f32::from(v.width) * f32::from(v.height), *area))
        })
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, area)| area)
}

pub fn state(bounds: WindowBounds) -> WindowState {
    let (b, mode) = match bounds {
        WindowBounds::Windowed(b) => (b, WindowMode::Windowed),
        WindowBounds::Maximized(b) => (b, WindowMode::Maximized),
        WindowBounds::Fullscreen(b) => (b, WindowMode::Fullscreen),
    };
    WindowState {
        x: f32::from(b.origin.x),
        y: f32::from(b.origin.y),
        width: f32::from(b.size.width),
        height: f32::from(b.size.height),
        mode,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
        Bounds::new(point(px(x), px(y)), size(px(w), px(h)))
    }

    #[test]
    fn keeps_a_position_only_on_a_display() {
        let screens = [b(0., 0., 1920., 1080.), b(1920., 0., 2560., 1440.)];
        assert_eq!(
            on_a_display(b(2000., 100., 800., 600.), &screens),
            Some(screens[1]),
            "second screen"
        );
        assert_eq!(
            on_a_display(b(1800., 100., 800., 600.), &screens),
            Some(screens[1]),
            "across both, on the one that holds most of it"
        );
        // The second screen was unplugged.
        assert_eq!(
            on_a_display(b(2000., 100., 800., 600.), &screens[..1]),
            None
        );
        // Only a sliver left on screen.
        assert_eq!(
            on_a_display(b(1900., 100., 800., 600.), &screens[..1]),
            None
        );
    }

    fn saved(x: f32, y: f32, w: f32, h: f32) -> Option<WindowState> {
        Some(WindowState {
            x,
            y,
            width: w,
            height: h,
            mode: WindowMode::Windowed,
        })
    }

    /// A 1920x1080 screen at 150%, less its taskbar: what a common Windows
    /// laptop gives a window, in the pixels a window is measured in.
    const LAPTOP_150: (f32, f32) = (1280., 672.);

    #[test]
    fn the_default_size_fits_a_small_screen_and_is_centered_on_it() {
        let area = b(0., 0., LAPTOP_150.0, LAPTOP_150.1);
        let (bounds, mode) = placed(None, &[area], Some(area));
        assert_eq!(bounds, area, "the whole work area, and no more");
        assert_eq!(mode, WindowMode::Windowed);
        // And on a screen with room, the default as it is, in the middle.
        let big = b(0., 0., 2560., 1400.);
        let (bounds, _) = placed(None, &[big], Some(big));
        assert_eq!(bounds, b(560., 250., 1440., 900.));
    }

    #[test]
    fn a_saved_size_larger_than_its_screen_is_made_to_fit_and_moved_in() {
        let area = b(0., 0., LAPTOP_150.0, LAPTOP_150.1);
        // Left at 1440x900 from 100%, at (114, 7).
        let (bounds, _) = placed(saved(114., 7., 1440., 900.), &[area], Some(area));
        assert_eq!(
            bounds, area,
            "its right edge, and the buttons there, on screen"
        );
        // One that fits stays where it was.
        let (bounds, _) = placed(saved(100., 50., 1000., 600.), &[area], Some(area));
        assert_eq!(bounds, b(100., 50., 1000., 600.));
        // One that fits but hangs over the right edge is moved in.
        let (bounds, _) = placed(saved(600., 50., 1000., 600.), &[area], Some(area));
        assert_eq!(bounds, b(280., 50., 1000., 600.));
    }

    #[test]
    fn a_window_whose_screen_is_gone_is_centered_on_the_primary_one() {
        let primary = b(0., 0., 1920., 1040.);
        let (bounds, _) = placed(saved(2500., 100., 2400., 1300.), &[primary], Some(primary));
        assert_eq!(bounds, primary, "made to fit as well");
    }

    #[test]
    fn state_keeps_the_restore_bounds_and_mode() {
        let s = state(WindowBounds::Maximized(b(10., 20., 1200., 800.)));
        assert_eq!((s.x, s.y, s.width, s.height), (10., 20., 1200., 800.));
        assert_eq!(s.mode, WindowMode::Maximized);
    }
}
