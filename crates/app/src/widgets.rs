//! Small pieces drawn the same way everywhere.

use crate::colors::colors;
use crate::text_size::ui;
use gpui::{
    AnyView, App, Context, Div, FontWeight, Render, SharedString, Window, div, prelude::*, px, rgb,
    svg,
};

pub fn icon(name: &'static str, size: f32, color: u32) -> impl IntoElement {
    svg()
        .path(format!("icons/{name}.svg"))
        .size(crate::text_size::ui(size))
        .flex_shrink_0()
        .text_color(rgb(color))
}

/// A section's name in small capitals.
pub fn eyebrow(text: impl AsRef<str>) -> Div {
    div()
        .text_size(ui(11.))
        .font_weight(FontWeight::BOLD)
        .text_color(rgb(colors().muted))
        .child(text.as_ref().to_uppercase())
}

/// The name of a part of the sidebar, in a pill of the accent's colour, so
/// that it stands out from the long list under it.
pub fn section_badge(text: impl AsRef<str>) -> Div {
    let c = colors();
    div()
        .flex_shrink_0()
        .px(ui(8.))
        .py(ui(2.))
        .rounded(ui(8.))
        .bg(rgb(c.accent))
        .text_color(rgb(c.on_accent))
        .text_size(ui(11.))
        .font_weight(FontWeight::BOLD)
        .child(text.as_ref().to_uppercase())
}

/// The status of a flow in a pill: its ground and its letters.
pub fn status_colors(status: Option<u16>) -> (u32, u32) {
    let c = colors();
    match status {
        None => (c.error_ground, c.error),
        Some(200..=299) => (c.ok_ground, c.ok),
        Some(400..=499) => (c.warn_ground, c.warn),
        Some(500..) => (c.error_ground, c.error),
        Some(_) => (c.plain_ground, c.plain),
    }
}

pub fn chip(text: impl Into<SharedString>, ground: u32, ink: u32) -> Div {
    div()
        .px(ui(10.))
        .py(ui(4.))
        .rounded_full()
        .bg(rgb(ground))
        .text_color(rgb(ink))
        .text_size(ui(12.))
        .font_weight(FontWeight::BOLD)
        .child(text.into())
}

/// A tooltip: a short line on a small card by the pointer.
pub struct Tip(SharedString);

impl Render for Tip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        div()
            .px(ui(10.))
            .py(ui(6.))
            .rounded(ui(10.))
            .bg(rgb(c.surface))
            .border_1()
            .border_color(rgb(c.window_edge))
            .font_family(crate::ui_font())
            .text_size(ui(13.))
            .text_color(rgb(c.text))
            .child(self.0.clone())
    }
}

/// A tooltip saying `text`, for an element's `tooltip`.
pub fn tip(text: impl Into<SharedString>) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text = text.into();
    move |_, cx| cx.new(|_| Tip(text.clone())).into()
}

/// A control the keyboard reaches: Tab stops on it, Enter and Space press it
/// as a click does (gpui's own keyboard click), and a ring shows where Tab
/// is. The ring is a shadow, so it moves nothing. A mouse click presses it
/// without taking the keyboard from where it was, so the window's keys go
/// on working after a click, as they did.
pub trait Reachable: gpui::InteractiveElement + gpui::Styled + Sized {
    fn reachable(self) -> Self {
        self.tab_index(0)
            // Runs before gpui's own focus on press, which this tells to stay.
            .on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
                window.prevent_default()
            })
            .focus_visible(|style| {
                let c = colors();
                let ring = |color: u32, spread: f32| gpui::BoxShadow {
                    color: gpui::Hsla::from(rgb(color)),
                    offset: gpui::point(px(0.), px(0.)),
                    blur_radius: px(0.),
                    spread_radius: px(spread),
                    inset: false,
                };
                // The text's color, which stands out on any button, past a
                // gap of the page's, so a blue button's ring is not lost on it.
                style.shadow(vec![ring(c.text, 4.), ring(c.page, 2.)])
            })
    }
}

impl<T: gpui::InteractiveElement + gpui::Styled> Reachable for T {}
