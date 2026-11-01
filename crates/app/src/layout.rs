//! Where the panels are, and how large. Kept in a file, so the window comes back
//! the way it was left.

use serde::{Deserialize, Serialize};
use std::path::Path;

pub const SIDEBAR_MIN: f32 = 140.;
pub const SIDEBAR_MAX: f32 = 480.;
pub const INSPECTOR_MIN: f32 = 280.;
pub const LIST_MIN: f32 = 240.;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Place {
    Beside,
    Below,
}

/// How the window was left.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WindowMode {
    #[default]
    Windowed,
    Maximized,
    Fullscreen,
}

/// Where the window was when last seen, in logical pixels. For a maximised or
/// fullscreen window, the bounds it returns to.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowState {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    #[serde(default)]
    pub mode: WindowMode,
}

/// The look the window wears.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    Light,
    Dark,
    #[default]
    System,
}

impl Appearance {
    pub fn name(self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::System => "System",
        }
    }

    /// The one after this, as the title bar's switch goes round them.
    pub fn next(self) -> Self {
        match self {
            Self::System => Self::Light,
            Self::Light => Self::Dark,
            Self::Dark => Self::System,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    pub appearance: Appearance,
    /// Set up has been left once: it does not open by itself again.
    pub setup_seen: bool,
    pub place: Place,
    pub sidebar_shown: bool,
    pub inspector_shown: bool,
    pub sidebar_w: f32,
    /// Beside the list.
    pub inspector_w: f32,
    /// Below it.
    pub inspector_h: f32,
    /// `None` until the window has been seen once.
    pub window: Option<WindowState>,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            appearance: Appearance::System,
            setup_seen: false,
            place: Place::Beside,
            sidebar_shown: true,
            inspector_shown: true,
            sidebar_w: 220.,
            inspector_w: 460.,
            inspector_h: 380.,
            window: None,
        }
    }
}

impl Layout {
    /// A file that cannot be read is no reason not to start.
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) {
        if let Ok(text) = toml::to_string(self) {
            let _ = std::fs::write(path, text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_switch_goes_round_all_three() {
        let start = Appearance::System;
        assert_eq!(start.next(), Appearance::Light);
        assert_eq!(start.next().next(), Appearance::Dark);
        assert_eq!(start.next().next().next(), start);
    }

    #[test]
    fn a_file_from_before_the_switch_still_loads() {
        let layout: Layout = toml::from_str("place = \"below\"").unwrap();
        assert_eq!(layout.place, Place::Below);
        assert_eq!(layout.appearance, Appearance::System);
    }

    #[test]
    fn it_comes_back_as_it_was_left() {
        let dir = std::env::temp_dir().join(format!("layout-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("layout.toml");
        let layout = Layout {
            appearance: Appearance::Dark,
            place: Place::Below,
            sidebar_shown: false,
            inspector_h: 300.,
            window: Some(WindowState {
                x: 10.,
                y: 20.,
                width: 1000.,
                height: 700.,
                mode: WindowMode::Maximized,
            }),
            ..Layout::default()
        };
        layout.save(&path);
        assert_eq!(Layout::load(&path), layout);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_broken_file_means_the_defaults() {
        assert_eq!(
            Layout::load(Path::new("/no/such/file.toml")),
            Layout::default()
        );
        let dir = std::env::temp_dir().join(format!("layout-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("layout.toml");
        std::fs::write(&path, "place = 'on the moon'").unwrap();
        assert_eq!(Layout::load(&path), Layout::default());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
