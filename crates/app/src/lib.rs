//! The window: traffic as it happens, on top of the proxy core.

pub mod about_page;
pub mod android;
pub mod body;
pub mod bodyview;
pub mod calldata;
pub mod chrome;
pub mod cli;
pub mod client_certs_view;
pub mod colors;
pub mod columns;
pub mod company_ca_view;
pub mod compare;
pub mod compare_page;
pub mod compose;
pub mod compose_page;
pub mod connection;
pub mod cookies;
pub mod detail;
pub mod devices;
pub mod engine;
pub mod fields;
pub mod files;
pub mod filter;
pub mod find;
pub mod find_bar;
pub mod flows;
pub mod graphql_view;
pub mod grpc;
pub mod history;
pub mod hosts;
pub mod input;
pub mod inspector;
pub mod inspector_window;
pub mod ios_sim_view;
pub mod jwt;
pub mod layout;
pub mod llm;
pub mod llm_view;
pub mod mcp;
pub mod messages;
pub mod named_layouts;
pub mod navigation;
pub mod network;
pub mod paused_page;
pub mod placement;
pub mod protos_view;
pub mod qr;
pub mod reading;
pub mod reflection;
pub mod relay_view;
pub mod remembered;
pub mod rpc_view;
pub mod rules_page;
pub mod scrollbar;
pub mod select;
pub mod sessions_page;
pub mod settings_page;
pub mod setup_page;
pub mod shortcuts;
pub mod signed;
pub mod snippets;
pub mod syntax;
pub mod system;
pub mod table;
pub mod text_size;
pub mod traffic;
pub mod trusted;
pub mod widgets;
pub mod wrap;

use gpui::{App, AssetSource, SharedString};
use std::borrow::Cow;

/// The same typeface as packwing, drawn into the binary: the interface's
/// unless the system's is chosen.
pub const FONT: &str = "Nunito";

/// What bodies, paths and numbers are shown in, unless another monospaced
/// font on the computer is chosen.
pub const MONO_FONT: &str = "JetBrains Mono";

/// The fonts chosen in Settings, as the window last applied them: read by
/// every view as it draws.
static FONTS_IN_USE: std::sync::RwLock<Option<(SharedString, SharedString)>> =
    std::sync::RwLock::new(None);

/// The interface's font, as chosen.
pub fn ui_font() -> SharedString {
    match &*FONTS_IN_USE.read().unwrap_or_else(|e| e.into_inner()) {
        Some((ui, _)) => ui.clone(),
        None => FONT.into(),
    }
}

/// The font for bodies, paths and numbers, as chosen.
pub fn mono_font() -> SharedString {
    match &*FONTS_IN_USE.read().unwrap_or_else(|e| e.into_inner()) {
        Some((_, mono)) => mono.clone(),
        None => MONO_FONT.into(),
    }
}

/// The fonts every view draws with from now on.
pub fn use_fonts(ui: impl Into<SharedString>, mono: impl Into<SharedString>) {
    let mut fonts = FONTS_IN_USE.write().unwrap_or_else(|e| e.into_inner());
    let new = (ui.into(), mono.into());
    if fonts.as_ref() != Some(&new) {
        *fonts = Some(new);
    }
}

/// The system's own interface font, of those this computer has: what
/// "System" means for the interface.
pub fn system_ui_font(installed: &[String]) -> Option<String> {
    let wanted: &[&str] = if cfg!(target_os = "macos") {
        &[".SystemUIFont", "SF Pro Text", "Helvetica Neue"]
    } else if cfg!(target_os = "windows") {
        &["Segoe UI"]
    } else {
        &[
            "Ubuntu",
            "Cantarell",
            "Noto Sans",
            "DejaVu Sans",
            "Liberation Sans",
        ]
    };
    wanted
        .iter()
        .find(|name| name.starts_with('.') || installed.iter().any(|i| i == *name))
        .map(|name| (*name).to_owned())
}

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
    "packwing",
    "packwing-feather",
    "shellwing",
    "send",
    "traffic",
    "trash",
    "search",
    "beside",
    "below",
    "window",
    "layouts",
    "tabs",
    "sidebar",
    "down",
    "up",
    "columns",
    "pin",
    "eye",
    "eye-off",
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
    "rules",
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
