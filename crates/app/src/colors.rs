//! The app's colors, in one place: what each one is for, and its value in the
//! design's light palette, Daylight, and in Graphite, the dark one picked for
//! tracewing. Dark opens in Neutral, Graphite on lighter, plainer grounds. Views ask [`colors`] for a role, never for a value, so a palette
//! is changed here alone. The window picks light or dark with [`set_dark`], and
//! a style for each with [`set_styles`], and redraws.
//!
//! A style is one of the two palettes on other grounds: the page, the panels,
//! the fields and the lines between them. Text, the accent, the states and the
//! colors of code stay the mode's, so a role added to the palette is in every
//! style at once, and what is chosen is tinted from the accent over the style's
//! own panels.
//!
//! Values are `0xRRGGBB` for `gpui::rgb`.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use tracewing_i18n::tr;

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
    /// Behind the same bytes on the other side of a hex line. A tone of its
    /// own rather than a weaker selection: weaker only, it sank into the
    /// page, and in the dark it could hardly be seen at all.
    pub echoed: u32,
    /// Behind text Find found, and behind the one it is at.
    pub found: u32,
    pub found_now: u32,
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
    pub code_comment: u32,
    pub code_keyword: u32,
    pub wait: u32,
    pub download: u32,
    /// The steps of making a connection, in the timing bars.
    pub dns: u32,
    pub connect: u32,
    pub tls: u32,
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
    echoed: 0xbfd6fb,
    found: 0xfff0b3,
    found_now: 0xffc95c,
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
    code_comment: 0x6b7280,
    code_keyword: 0xa1286a,
    wait: 0x2f55b4,
    download: 0x1a7f37,
    dns: 0x0e7c86,
    connect: 0xb35900,
    tls: 0x7b3fb5,
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
    echoed: 0x31486e,
    found: 0x4d4220,
    found_now: 0x8a6b1c,
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
    code_comment: 0x8b8f98,
    code_keyword: 0xf09ac8,
    wait: 0x7aa2ff,
    download: 0x62c985,
    dns: 0x4fc3cf,
    connect: 0xf0a050,
    tls: 0xb48cf0,
    button: 0x26282d,
    button_hover: 0x3b3e45,
    hover: 0x2e3036,
    window_edge: 0x2c2e33,
    shadow: 0x00000066,
};

/// The grounds a style puts under the mode's colors.
struct Grounds {
    page: u32,
    surface: u32,
    input: u32,
    code: u32,
    line: u32,
    /// The tab that is chosen, and the ground of plain text in a well.
    tab: u32,
    /// What is under the mouse.
    hover: u32,
    /// The selection in a field, the splitters and a button under the mouse.
    soft: u32,
}

/// `percent` of `a` over `b`, channel by channel.
const fn mix(a: u32, b: u32, percent: u32) -> u32 {
    let mut out = 0;
    let mut shift = 0;
    while shift < 24 {
        let x = (a >> shift) & 0xff;
        let y = (b >> shift) & 0xff;
        out |= ((x * percent + y * (100 - percent) + 50) / 100) << shift;
        shift += 8;
    }
    out
}

/// What is chosen, quietly and in a row: the accent over the panels, as
/// much of it as Graphite and Daylight have.
const SELECTED: u32 = 15;
const TONAL: u32 = 18;

/// Graphite on other grounds.
const fn dark(g: Grounds) -> Palette {
    Palette {
        page: g.page,
        surface: g.surface,
        input: g.input,
        code: g.code,
        tab_on: g.tab,
        line: g.line,
        track: g.tab,
        tonal: mix(GRAPHITE.accent, g.surface, TONAL),
        soft: g.soft,
        splitter: g.soft,
        chip: g.input,
        selected_row: mix(GRAPHITE.accent, g.surface, SELECTED),
        plain_ground: g.tab,
        button: g.input,
        button_hover: g.soft,
        hover: g.hover,
        window_edge: g.line,
        ..GRAPHITE
    }
}

/// Daylight on other grounds.
const fn light(g: Grounds) -> Palette {
    Palette {
        page: g.page,
        surface: g.surface,
        input: g.input,
        code: g.code,
        tab_on: g.tab,
        line: g.line,
        track: g.input,
        tonal: mix(LIGHT.accent, g.surface, TONAL),
        soft: g.soft,
        splitter: g.soft,
        chip: g.hover,
        selected_row: mix(LIGHT.accent, g.surface, SELECTED),
        plain_ground: g.input,
        button: g.input,
        button_hover: g.soft,
        hover: g.hover,
        window_edge: g.soft,
        ..LIGHT
    }
}

const MIDNIGHT: Palette = dark(Grounds {
    page: 0x08090a,
    surface: 0x121315,
    input: 0x1c1d20,
    code: 0x0c0d0e,
    line: 0x222326,
    tab: 0x25262a,
    hover: 0x1f2023,
    soft: 0x323338,
});
const NEUTRAL: Palette = dark(Grounds {
    page: 0x1e1f22,
    surface: 0x26272b,
    input: 0x2f3035,
    code: 0x212226,
    line: 0x35363b,
    tab: 0x393a40,
    hover: 0x393a40,
    soft: 0x45474e,
});
const SLATE: Palette = dark(Grounds {
    page: 0x1b2027,
    surface: 0x232932,
    input: 0x2d343f,
    code: 0x1e242c,
    line: 0x333b47,
    tab: 0x37404d,
    hover: 0x37404d,
    soft: 0x45505f,
});
const DIM: Palette = Palette {
    // Lighter than Graphite's, as the fields are.
    muted: 0xb0b3bb,
    ..dark(Grounds {
        page: 0x2a2e35,
        surface: 0x32373f,
        input: 0x3c424b,
        code: 0x2e3239,
        line: 0x434952,
        tab: 0x464d57,
        hover: 0x464d57,
        soft: 0x535b66,
    })
};
const DEEP_SEA: Palette = dark(Grounds {
    page: 0x0e1626,
    surface: 0x152036,
    input: 0x1e2c47,
    code: 0x111a2d,
    line: 0x24324d,
    tab: 0x283857,
    hover: 0x223050,
    soft: 0x34466a,
});
const ESPRESSO: Palette = dark(Grounds {
    page: 0x1b1815,
    surface: 0x24201c,
    input: 0x2f2a25,
    code: 0x1f1c18,
    line: 0x36302a,
    tab: 0x3a332d,
    hover: 0x3a332d,
    soft: 0x4a4139,
});
const FOREST: Palette = dark(Grounds {
    page: 0x121b18,
    surface: 0x192420,
    input: 0x22302b,
    code: 0x151f1b,
    line: 0x283731,
    tab: 0x2c3c36,
    hover: 0x2c3c36,
    soft: 0x3a4c45,
});
const PLUM: Palette = dark(Grounds {
    page: 0x1a1521,
    surface: 0x221c2b,
    input: 0x2c2537,
    code: 0x1d1825,
    line: 0x322a3e,
    tab: 0x362e44,
    hover: 0x362e44,
    soft: 0x453a56,
});

const PAPER: Palette = Palette {
    // Darker than Daylight's, as the ground is: as easy to read.
    faint: 0x8790a3,
    ..light(Grounds {
        page: 0xf4f0e8,
        surface: 0xfffcf6,
        input: 0xeae4d8,
        code: 0xfaf7f0,
        line: 0xe5ded0,
        tab: 0xfffcf6,
        hover: 0xf1ece2,
        soft: 0xd6cdbb,
    })
};
const SOFT_GRAY: Palette = Palette {
    // Darker than Daylight's, as the ground is: as easy to read.
    faint: 0x838c9f,
    ..light(Grounds {
        page: 0xe9ebee,
        surface: 0xf5f6f8,
        input: 0xdcdfe4,
        code: 0xeff1f3,
        line: 0xd8dbe1,
        tab: 0xfbfbfc,
        hover: 0xe6e8ec,
        soft: 0xc3c8d0,
    })
};
/// White, with darker text and lines: for a pale screen or a bright room.
const CONTRAST: Palette = Palette {
    text: 0x05070b,
    muted: 0x2c3240,
    faint: 0x5d6576,
    window_edge: 0x9aa3b5,
    ..light(Grounds {
        page: 0xffffff,
        surface: 0xffffff,
        input: 0xeceff4,
        code: 0xf6f7f9,
        line: 0xbfc6d3,
        tab: 0xe8ecf4,
        hover: 0xf0f2f6,
        soft: 0xa9b2c3,
    })
};
const MIST: Palette = Palette {
    // Darker than Daylight's, as the ground is: as easy to read.
    faint: 0x828b9e,
    ..light(Grounds {
        page: 0xe3e9f2,
        surface: 0xf2f5fa,
        input: 0xd3dce9,
        code: 0xebf0f7,
        line: 0xcfd8e6,
        tab: 0xf9fbfe,
        hover: 0xdee5f0,
        soft: 0xb6c3d7,
    })
};
const SAGE: Palette = Palette {
    // Darker than Daylight's, as the ground is: as easy to read.
    faint: 0x868fa2,
    ..light(Grounds {
        page: 0xebf0eb,
        surface: 0xf8fbf8,
        input: 0xdce5dc,
        code: 0xf2f6f2,
        line: 0xd7e0d7,
        tab: 0xffffff,
        hover: 0xe5ece5,
        soft: 0xc0cdc0,
    })
};
const LAVENDER: Palette = Palette {
    // Darker than Daylight's, as the ground is: as easy to read.
    faint: 0x868fa2,
    ..light(Grounds {
        page: 0xefedf7,
        surface: 0xfbfaff,
        input: 0xe2def1,
        code: 0xf6f4fc,
        line: 0xdfdaee,
        tab: 0xffffff,
        hover: 0xebe8f5,
        soft: 0xc9c1e0,
    })
};
const DUSK: Palette = Palette {
    // Darker than Daylight's, as the ground is: as easy to read.
    faint: 0x798194,
    ..light(Grounds {
        page: 0xdedbd5,
        surface: 0xebe9e4,
        input: 0xd6d2ca,
        code: 0xe5e2dc,
        line: 0xcdc9c0,
        tab: 0xf4f2ee,
        hover: 0xdad6ce,
        soft: 0xb7b1a6,
    })
};

/// Kept in layout.toml by a name of its own: a name this build does not know,
/// from a later one, is the default rather than a layout that cannot be read.
macro_rules! style {
    ($(#[$doc:meta])* $style:ident { $($variant:ident = $key:literal => $palette:ident,)+ }) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
        #[repr(u8)]
        pub enum $style {
            #[default]
            $($variant,)+
        }

        impl $style {
            pub const ALL: [Self; [$(Self::$variant),+].len()] = [$(Self::$variant),+];

            /// Its name in layout.toml.
            pub fn key(self) -> &'static str {
                match self {
                    $(Self::$variant => $key,)+
                }
            }

            pub fn palette(self) -> &'static Palette {
                match self {
                    $(Self::$variant => &$palette,)+
                }
            }

            fn from_index(index: u8) -> Self {
                Self::ALL.get(index as usize).copied().unwrap_or_default()
            }
        }

        impl Serialize for $style {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.key())
            }
        }

        impl<'de> Deserialize<'de> for $style {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                #[derive(Deserialize)]
                #[serde(untagged)]
                enum Any {
                    Name(String),
                    Other(serde::de::IgnoredAny),
                }
                Ok(match Any::deserialize(deserializer)? {
                    Any::Name(name) => Self::ALL
                        .into_iter()
                        .find(|style| style.key() == name)
                        .unwrap_or_default(),
                    Any::Other(_) => Self::default(),
                })
            }
        }
    };
}

style! {
    /// How the light palette looks; Daylight is the design's.
    LightStyle {
        Daylight = "daylight" => LIGHT,
        Paper = "paper" => PAPER,
        SoftGray = "soft-gray" => SOFT_GRAY,
        Contrast = "contrast" => CONTRAST,
        Mist = "mist" => MIST,
        Sage = "sage" => SAGE,
        Lavender = "lavender" => LAVENDER,
        Dusk = "dusk" => DUSK,
    }
}

style! {
    /// How the dark palette looks; Neutral unless another is chosen.
    DarkStyle {
        Neutral = "neutral" => NEUTRAL,
        Graphite = "graphite" => GRAPHITE,
        Midnight = "midnight" => MIDNIGHT,
        Slate = "slate" => SLATE,
        Dim = "dim" => DIM,
        DeepSea = "deep-sea" => DEEP_SEA,
        Espresso = "espresso" => ESPRESSO,
        Forest = "forest" => FOREST,
        Plum = "plum" => PLUM,
    }
}

impl LightStyle {
    pub fn name(self) -> String {
        match self {
            Self::Daylight => tr!("Daylight"),
            Self::Paper => tr!("Paper"),
            Self::SoftGray => tr!("Soft gray"),
            Self::Contrast => tr!("High contrast"),
            Self::Mist => tr!("Mist"),
            Self::Sage => tr!("Sage"),
            Self::Lavender => tr!("Lavender"),
            Self::Dusk => tr!("Dusk"),
        }
    }
}

impl DarkStyle {
    pub fn name(self) -> String {
        match self {
            Self::Neutral => tr!("Neutral"),
            Self::Graphite => tr!("Graphite"),
            Self::Midnight => tr!("Midnight"),
            Self::Slate => tr!("Slate"),
            Self::Dim => tr!("Dim"),
            Self::DeepSea => tr!("Deep sea"),
            Self::Espresso => tr!("Espresso"),
            Self::Forest => tr!("Forest"),
            Self::Plum => tr!("Plum"),
        }
    }
}

/// The dark palette is in use.
static DARK_IN_USE: AtomicBool = AtomicBool::new(true);
/// Which style each is in, as its place in `ALL`.
static LIGHT_STYLE: AtomicU8 = AtomicU8::new(0);
static DARK_STYLE: AtomicU8 = AtomicU8::new(0);

/// The palette in use.
pub fn colors() -> &'static Palette {
    chosen(
        DARK_IN_USE.load(Ordering::Relaxed),
        LightStyle::from_index(LIGHT_STYLE.load(Ordering::Relaxed)),
        DarkStyle::from_index(DARK_STYLE.load(Ordering::Relaxed)),
    )
}

fn chosen(dark: bool, light_style: LightStyle, dark_style: DarkStyle) -> &'static Palette {
    if dark {
        dark_style.palette()
    } else {
        light_style.palette()
    }
}

/// Chooses light or dark; the caller redraws.
pub fn set_dark(dark: bool) {
    DARK_IN_USE.store(dark, Ordering::Relaxed);
}

/// Chooses how each looks; the caller redraws.
pub fn set_styles(light: LightStyle, dark: DarkStyle) {
    LIGHT_STYLE.store(light as u8, Ordering::Relaxed);
    DARK_STYLE.store(dark as u8, Ordering::Relaxed);
}

/// `0xRRGGBBAA` for `gpui::rgba`.
pub fn with_alpha(color: u32, alpha: u8) -> u32 {
    (color << 8) | alpha as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WCAG's relative luminance.
    fn luminance(color: u32) -> f64 {
        let channel = |shift: u32| {
            let c = f64::from((color >> shift) & 0xff) / 255.;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
    }

    /// WCAG's contrast ratio, 1 to 21.
    fn contrast(a: u32, b: u32) -> f64 {
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    fn every_palette() -> Vec<(String, &'static Palette)> {
        let light = LightStyle::ALL
            .into_iter()
            .map(|s| (format!("light {}", s.key()), s.palette()));
        let dark = DarkStyle::ALL
            .into_iter()
            .map(|s| (format!("dark {}", s.key()), s.palette()));
        light.chain(dark).collect()
    }

    /// Light opens in the design's own; dark in Neutral, and Graphite is
    /// still there as it was.
    #[test]
    fn the_defaults_are_daylight_and_neutral() {
        assert_eq!(LightStyle::default().palette(), &LIGHT);
        assert_eq!(DarkStyle::default().palette(), &NEUTRAL);
        assert_eq!(LIGHT.page, 0xf4f6fb);
        assert_eq!(GRAPHITE.page, 0x141517);
    }

    #[test]
    fn every_style_can_be_read() {
        let mut short = Vec::new();
        for (name, p) in every_palette() {
            let mut need = |what: &str, fore: u32, back: u32, least: f64| {
                let got = contrast(fore, back);
                if got < least {
                    short.push(format!("{name}: {what} {got:.2} < {least}"));
                }
            };
            for (ground, back) in [
                ("page", p.page),
                ("surface", p.surface),
                ("code", p.code),
                ("input", p.input),
            ] {
                need(&format!("text on {ground}"), p.text, back, 7.);
                need(&format!("muted on {ground}"), p.muted, back, 4.5);
                need(&format!("link on {ground}"), p.link, back, 4.5);
            }
            // What is said least loudly: as plain as Daylight has it, no less.
            for (ground, back) in [("page", p.page), ("surface", p.surface)] {
                need(&format!("faint on {ground}"), p.faint, back, 2.8);
            }
            need("text on the chosen row", p.text, p.selected_row, 7.);
            need("on_tonal on tonal", p.on_tonal, p.tonal, 4.5);
            need("on_chip on chip", p.on_chip, p.chip, 4.5);
            need("on_accent on accent", p.on_accent, p.accent, 4.5);
            need("accent on surface", p.accent, p.surface, 3.);
        }
        assert!(short.is_empty(), "{}", short.join("\n"));
    }

    /// The chosen row and what the mouse is over stand out from the panel
    /// they are on, and from each other.
    #[test]
    fn what_is_chosen_can_be_seen() {
        let mut short = Vec::new();
        for (name, p) in every_palette() {
            for (what, a, b) in [
                ("chosen row on surface", p.selected_row, p.surface),
                ("hover on surface", p.hover, p.surface),
            ] {
                let got = contrast(a, b);
                if got < 1.08 {
                    short.push(format!("{name}: {what} {got:.3}"));
                }
            }
        }
        assert!(short.is_empty(), "{}", short.join("\n"));
    }

    #[test]
    fn a_style_is_kept_by_name_and_an_unknown_one_is_the_default() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct Kept {
            light: LightStyle,
            dark: DarkStyle,
        }
        let kept = Kept {
            light: LightStyle::SoftGray,
            dark: DarkStyle::DeepSea,
        };
        let text = toml::to_string(&kept).unwrap();
        assert!(text.contains("light = \"soft-gray\""), "{text}");
        assert!(text.contains("dark = \"deep-sea\""), "{text}");
        assert_eq!(toml::from_str::<Kept>(&text).unwrap(), kept);
        let later: Kept = toml::from_str("light = \"aurora\"\ndark = 7").unwrap();
        assert_eq!(
            later,
            Kept {
                light: LightStyle::Daylight,
                dark: DarkStyle::Neutral
            }
        );
    }

    /// Pure, so that no other test sees the colors change under it.
    #[test]
    fn styles_are_chosen_apart_for_light_and_dark() {
        assert_eq!(chosen(false, LightStyle::Paper, DarkStyle::Forest), &PAPER);
        assert_eq!(chosen(true, LightStyle::Paper, DarkStyle::Forest), &FOREST);
        for (i, style) in DarkStyle::ALL.into_iter().enumerate() {
            assert_eq!(DarkStyle::from_index(style as u8), style);
            assert_eq!(style as usize, i);
        }
        for (i, style) in LightStyle::ALL.into_iter().enumerate() {
            assert_eq!(LightStyle::from_index(style as u8), style);
            assert_eq!(style as usize, i);
        }
        assert_eq!(DarkStyle::from_index(200), DarkStyle::Neutral);
    }
}
