//! The app's colors, in one place: what each one is for, and its value in
//! Graphite, the dark palette picked for tracewing. Views ask [`colors`] for a
//! role, never for a value, so a palette is changed here alone.
//!
//! Values are `0xRRGGBB` for `gpui::rgb`.

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
}

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
};

pub fn colors() -> &'static Palette {
    &GRAPHITE
}

/// `0xRRGGBBAA` for `gpui::rgba`.
pub fn with_alpha(color: u32, alpha: u8) -> u32 {
    (color << 8) | alpha as u32
}
