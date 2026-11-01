//! The app's colors, in one place: what each one is for, and its value in the
//! design's light palette and in Graphite, the dark one picked for tracewing. Views
//! ask [`colors`] for a role, never for a value, so a palette is changed here alone.
//! The window picks one with [`set_dark`] and redraws.
//!
//! Values are `0xRRGGBB` for `gpui::rgb`.

use std::sync::atomic::{AtomicBool, Ordering};

/// Every color the app draws with, by what it is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    /// The window's ground.
    pub page: u32,
    /// Panels on it.
    pub surface: u32,
    /// Text fields.
    pub input: u32,
    /// Where a body is shown, and other wells in a panel.
    pub code: u32,
    /// The tab that is chosen, in a well.
    pub tab_on: u32,
    pub line: u32,
    pub track: u32,
    /// What is chosen, quietly.
    pub tonal: u32,
    pub on_tonal: u32,
    /// What is chosen, loudly.
    pub accent: u32,
    pub on_accent: u32,
    pub link: u32,
    pub text: u32,
    pub muted: u32,
    pub faint: u32,
    /// The selection behind text in a field.
    pub soft: u32,
    pub splitter: u32,
    pub chip: u32,
    pub on_chip: u32,
    pub ok_ground: u32,
    pub ok: u32,
    pub record: u32,
    pub selected_row: u32,
    pub slow: u32,
    pub warn_ground: u32,
    pub warn: u32,
    pub error_ground: u32,
    pub error: u32,
    pub plain_ground: u32,
    pub plain: u32,
    pub json_key: u32,
    pub json_string: u32,
    pub json_number: u32,
    pub json_literal: u32,
    pub json_punct: u32,
    pub wait: u32,
    pub download: u32,
    /// The window's own buttons, and what is under the mouse.
    pub button: u32,
    pub button_hover: u32,
    pub hover: u32,
    pub window_edge: u32,
    /// `0xRRGGBBAA`: the window's shadow.
    pub shadow: u32,
}

const LIGHT: Palette = Palette {
    page: 0xf4f6fb,
    surface: 0xffffff,
    input: 0xe4e9f4,
    code: 0xf8f9fc,
    tab_on: 0xffffff,
    line: 0xe4e9f4,
    track: 0xe4e9f4,
    tonal: 0xd7e2ff,
    on_tonal: 0x0b2557,
    accent: 0x2f55b4,
    on_accent: 0xffffff,
    link: 0x2f55b4,
    text: 0x161b26,
    muted: 0x4a5263,
    faint: 0x8a93a6,
    soft: 0xc9d2e6,
    splitter: 0xc9d2e6,
    chip: 0xeef1f7,
    on_chip: 0x2e3545,
    ok_ground: 0xd3f0dc,
    ok: 0x0e4d24,
    record: 0xd92d20,
    selected_row: 0xdce4f7,
    slow: 0x8a3a00,
    warn_ground: 0xffe1cc,
    warn: 0x6b2c05,
    error_ground: 0xffdad6,
    error: 0x7a1e16,
    plain_ground: 0xe4e9f4,
    plain: 0x2e3545,
    json_key: 0x2f55b4,
    json_string: 0x9a3412,
    json_number: 0x6d28d9,
    json_literal: 0xb54708,
    json_punct: 0x4a5263,
    wait: 0x2f55b4,
    download: 0x1a7f37,
    button: 0xe4e9f4,
    button_hover: 0xc9d2e6,
    hover: 0xeef1f7,
    window_edge: 0xc9d2e6,
    shadow: 0x00000033,
};

const GRAPHITE: Palette = Palette {
    page: 0x141517,
    surface: 0x1c1d21,
    input: 0x26282d,
    code: 0x17181b,
    tab_on: 0x2e3036,
    line: 0x2c2e33,
    track: 0x2e3036,
    tonal: 0x2c3548,
    on_tonal: 0xdce5ff,
    accent: 0x7aa2ff,
    on_accent: 0x0d1a3a,
    link: 0x8fb0ff,
    text: 0xe4e5e8,
    muted: 0xa2a5ad,
    faint: 0x7d818b,
    soft: 0x3b3e45,
    splitter: 0x3b3e45,
    chip: 0x26282d,
    on_chip: 0xcfd1d6,
    ok_ground: 0x1b3324,
    ok: 0x8bdda4,
    record: 0xf26b5b,
    selected_row: 0x283244,
    slow: 0xf5b07c,
    warn_ground: 0x3f2a19,
    warn: 0xf5b07c,
    error_ground: 0x43211f,
    error: 0xf7a098,
    plain_ground: 0x2e3036,
    plain: 0xc7c9ce,
    json_key: 0x8fb0ff,
    json_string: 0xe6a57c,
    json_number: 0xc3a6f5,
    json_literal: 0xf2c46e,
    json_punct: 0xa2a5ad,
    wait: 0x7aa2ff,
    download: 0x62c985,
    button: 0x26282d,
    button_hover: 0x3b3e45,
    hover: 0x2e3036,
    window_edge: 0x2c2e33,
    shadow: 0x00000066,
};

/// The dark palette is in use.
static DARK_IN_USE: AtomicBool = AtomicBool::new(true);

/// The palette in use.
pub fn colors() -> &'static Palette {
    if DARK_IN_USE.load(Ordering::Relaxed) {
        &GRAPHITE
    } else {
        &LIGHT
    }
}

/// Chooses the palette; the caller redraws.
pub fn set_dark(dark: bool) {
    DARK_IN_USE.store(dark, Ordering::Relaxed);
}

/// `0xRRGGBBAA` for `gpui::rgba`.
pub fn with_alpha(color: u32, alpha: u8) -> u32 {
    (color << 8) | alpha as u32
}
