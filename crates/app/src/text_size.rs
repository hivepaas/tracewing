//! The size text is drawn at, as packwing and shellwing do it: every size the
//! views name is written for body text at [`DRAWN_AT`] px and given in rems
//! ([`ui`]), and the window's rem follows the body size chosen in Settings.
//! So the whole window grows and shrinks together, text, rows and gaps, and
//! nothing that fitted stops fitting. The design's size is the default: the
//! window shows a lot, and was drawn for it.
//!
//! Sizes a person chose by dragging, as a panel's width, stay in pixels.

use gpui::{Pixels, Rems, Window, px, rems};

/// The body size the views' sizes are written for.
pub const DRAWN_AT: f32 = 13.;

/// The body sizes Settings offer, in px.
pub const CHOICES: [u8; 8] = [11, 12, 13, 14, 15, 16, 18, 20];

/// A size as drawn at [`DRAWN_AT`]: it grows and shrinks with the body size.
pub fn ui(px: f32) -> Rems {
    rems(px / 16.)
}

/// The window's rem for body text of `body` px.
pub fn rem_for(body: f32) -> Pixels {
    px(16. * body / DRAWN_AT)
}

/// A size as drawn at [`DRAWN_AT`], in pixels as the window draws it now:
/// for what is worked out rather than laid out, as a list's row height.
pub fn scaled(window: &Window, px_at_design: f32) -> Pixels {
    px(px_at_design * f32::from(window.rem_size()) / 16.)
}

/// How much larger than the design the window draws now.
pub fn scale(window: &Window) -> f32 {
    f32::from(window.rem_size()) / 16.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_design_size_is_drawn_as_it_was() {
        // At the design's body size a rem is 16 px, so ui(13.) is 13 px.
        assert_eq!(rem_for(DRAWN_AT), px(16.));
        assert_eq!(ui(13.).to_pixels(rem_for(DRAWN_AT)), px(13.));
        assert_eq!(ui(13.).to_pixels(rem_for(16.)), px(16.));
        assert!(CHOICES.contains(&(DRAWN_AT as u8)));
    }
}
