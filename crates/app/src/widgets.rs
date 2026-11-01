//! Small pieces drawn the same way everywhere.

use crate::colors::colors;
use gpui::{Div, FontWeight, SharedString, div, prelude::*, px, rgb, svg};

pub fn icon(name: &'static str, size: f32, color: u32) -> impl IntoElement {
    svg()
        .path(format!("icons/{name}.svg"))
        .size(px(size))
        .flex_shrink_0()
        .text_color(rgb(color))
}

/// A section's name in small capitals.
pub fn eyebrow(text: &'static str) -> Div {
    div()
        .text_size(px(11.))
        .font_weight(FontWeight::BOLD)
        .text_color(rgb(colors().muted))
        .child(text.to_uppercase())
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
        .px(px(10.))
        .py(px(4.))
        .rounded_full()
        .bg(rgb(ground))
        .text_color(rgb(ink))
        .text_size(px(12.))
        .font_weight(FontWeight::BOLD)
        .child(text.into())
}
