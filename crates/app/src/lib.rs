//! The window: traffic as it happens, on top of the proxy core.

pub mod body;
pub mod bodyview;
pub mod chrome;
pub mod colors;
pub mod curl;
pub mod detail;
pub mod engine;
pub mod filter;
pub mod flows;
pub mod input;
pub mod inspector;
pub mod layout;
pub mod placement;
pub mod sessions_page;
pub mod settings_page;
pub mod setup_page;
pub mod system;
pub mod traffic;
pub mod widgets;

use gpui::{App, AssetSource, SharedString};
use std::borrow::Cow;

/// The same typeface as packwing, drawn into the binary.
pub const FONT: &str = "Nunito";

/// What bodies, paths and numbers are shown in.
pub const MONO_FONT: &str = "JetBrains Mono";

const FONTS: [&[u8]; 5] = [
    include_bytes!("../assets/fonts/Nunito-Regular.ttf"),
    include_bytes!("../assets/fonts/Nunito-Medium.ttf"),
    include_bytes!("../assets/fonts/Nunito-SemiBold.ttf"),
    include_bytes!("../assets/fonts/Nunito-Bold.ttf"),
    include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf"),
];

pub fn load_fonts(cx: &mut App) {
    let fonts = FONTS.iter().map(|f| Cow::Borrowed(*f)).collect();
    if let Err(e) = cx.text_system().add_fonts(fonts) {
        eprintln!("tracewing: cannot load the fonts: {e}");
    }
}

macro_rules! icons {
    ($($name:literal),* $(,)?) => {
        &[$((concat!("icons/", $name, ".svg"), include_bytes!(concat!("../assets/icons/", $name, ".svg")))),*]
    };
}

const ICONS: &[(&str, &[u8])] = icons![
    "logo",
    "traffic",
    "trash",
    "search",
    "beside",
    "below",
    "sidebar",
    "down",
    "check",
    "grip",
    "record",
    "pause",
    "shield",
    "shield-off",
    "settings",
    "minimize",
    "maximize",
    "close",
    "win-minimize",
    "win-maximize",
    "win-restore",
    "win-close",
    "appearance-dark",
    "appearance-light",
    "appearance-system",
    "setup",
    "sessions",
];

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        Ok(ICONS
            .iter()
            .find(|(p, _)| *p == path)
            .map(|(_, bytes)| Cow::Borrowed(*bytes)))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(ICONS
            .iter()
            .filter(|(p, _)| p.starts_with(path))
            .map(|(p, _)| (*p).into())
            .collect())
    }
}
