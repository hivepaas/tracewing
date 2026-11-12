//! Where the panels are, and how large. Kept in a file, so the window comes back
//! the way it was left.

use crate::{columns::Columns, hosts::HostMarks};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracewing_i18n::tr;

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

/// How the inspector shows a flow's request and its response.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sides {
    /// One at a time, as tabs.
    #[default]
    Tabs,
    Beside,
    Stacked,
}

/// The part of Settings open, as its menu names them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SettingsSection {
    #[default]
    General,
    Network,
    Certificates,
    Decoding,
    Storage,
    Ai,
    About,
}

impl SettingsSection {
    pub const ALL: [Self; 7] = [
        Self::General,
        Self::Network,
        Self::Certificates,
        Self::Decoding,
        Self::Storage,
        Self::Ai,
        Self::About,
    ];

    pub fn name(self) -> String {
        match self {
            Self::General => tr!("General"),
            Self::Network => tr!("Network"),
            Self::Certificates => tr!("Certificates"),
            Self::Decoding => tr!("Decoding"),
            Self::Storage => tr!("Storage"),
            Self::Ai => tr!("AI tools"),
            Self::About => tr!("About"),
        }
    }
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
    pub fn name(self) -> String {
        match self {
            Self::Light => tr!("Light"),
            Self::Dark => tr!("Dark"),
            Self::System => tr!("System"),
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
    /// The list's columns: which are hidden, and how wide.
    pub columns: Columns,
    /// Hosts pinned to the top of the sidebar, and hosts hidden from the list.
    pub hosts: HostMarks,
    /// The language chosen, as a tag; `None` for the system's.
    pub language: Option<String>,
    pub sides: Sides,
    /// The inspector in a window of its own, for a second display.
    pub detached: bool,
    /// Where that window was when last seen.
    pub inspector_window: Option<WindowState>,
    /// Arrangements kept under a name, as "Phone" and "API", in the order kept.
    pub named: Vec<Named>,
    /// The part of Settings last open.
    pub settings_section: SettingsSection,
    /// The body text size chosen, in px; `None` for the design's.
    pub text_size: Option<u8>,
}

/// Where the panels are and how large, which columns show: what a named
/// layout keeps. Not the look, the language, nor where the window is.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Arrangement {
    pub place: Place,
    pub sidebar_shown: bool,
    pub inspector_shown: bool,
    pub sidebar_w: f32,
    pub inspector_w: f32,
    pub inspector_h: f32,
    pub columns: Columns,
    pub sides: Sides,
    pub detached: bool,
}

impl Default for Arrangement {
    fn default() -> Self {
        Layout::default().arrangement()
    }
}

/// An arrangement and its name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Named {
    pub name: String,
    #[serde(default)]
    pub arrangement: Arrangement,
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
            columns: Columns::default(),
            hosts: HostMarks::default(),
            language: None,
            sides: Sides::default(),
            detached: false,
            inspector_window: None,
            named: Vec::new(),
            settings_section: SettingsSection::default(),
            text_size: None,
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

    /// The panels as they are now.
    pub fn arrangement(&self) -> Arrangement {
        Arrangement {
            place: self.place,
            sidebar_shown: self.sidebar_shown,
            inspector_shown: self.inspector_shown,
            sidebar_w: self.sidebar_w,
            inspector_w: self.inspector_w,
            inspector_h: self.inspector_h,
            columns: self.columns.clone(),
            sides: self.sides,
            detached: self.detached,
        }
    }

    /// Puts the panels as `arrangement` has them.
    pub fn arrange(&mut self, arrangement: &Arrangement) {
        let Arrangement {
            place,
            sidebar_shown,
            inspector_shown,
            sidebar_w,
            inspector_w,
            inspector_h,
            columns,
            sides,
            detached,
        } = arrangement.clone();
        self.place = place;
        self.sidebar_shown = sidebar_shown;
        self.inspector_shown = inspector_shown;
        self.sidebar_w = sidebar_w;
        self.inspector_w = inspector_w;
        self.inspector_h = inspector_h;
        self.columns = columns;
        self.sides = sides;
        self.detached = detached;
    }

    /// Keeps the panels as they are now under `name`, in place of one of that
    /// name, whatever its case. False for a name that is only spaces.
    pub fn keep_as(&mut self, name: &str) -> bool {
        let name = name.trim();
        if name.is_empty() {
            return false;
        }
        let arrangement = self.arrangement();
        match self.named.iter_mut().find(|n| same_name(&n.name, name)) {
            Some(named) => {
                named.name = name.to_owned();
                named.arrangement = arrangement;
            }
            None => self.named.push(Named {
                name: name.to_owned(),
                arrangement,
            }),
        }
        true
    }

    pub fn forget(&mut self, name: &str) {
        self.named.retain(|n| !same_name(&n.name, name));
    }

    pub fn named_arrangement(&self, name: &str) -> Option<Arrangement> {
        self.named
            .iter()
            .find(|n| same_name(&n.name, name))
            .map(|n| n.arrangement.clone())
    }

    /// The name of a layout the panels are in now, if they are in one.
    pub fn current_name(&self) -> Option<&str> {
        let now = self.arrangement();
        self.named
            .iter()
            .find(|n| n.arrangement == now)
            .map(|n| n.name.as_str())
    }
}

fn same_name(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
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
    fn a_named_layout_comes_back_as_it_was_kept() {
        let mut layout = Layout {
            place: Place::Below,
            sidebar_shown: false,
            ..Layout::default()
        };
        assert!(layout.keep_as("  Phone "));
        assert!(!layout.keep_as("   "));
        assert_eq!(layout.current_name(), Some("Phone"));

        layout.place = Place::Beside;
        layout.sidebar_shown = true;
        layout.inspector_w = 600.;
        assert_eq!(layout.current_name(), None);
        layout.keep_as("API");

        // The same name, whatever its case, is kept over.
        layout.sides = Sides::Beside;
        layout.keep_as("api");
        assert_eq!(layout.named.len(), 2);
        assert_eq!(layout.named[1].name, "api");

        let phone = layout.named_arrangement("PHONE").unwrap();
        layout.arrange(&phone);
        assert_eq!(layout.place, Place::Below);
        assert!(!layout.sidebar_shown);
        assert_eq!(layout.sides, Sides::Tabs);
        assert_eq!(layout.current_name(), Some("Phone"));

        // Kept in the file, and read back.
        let text = toml::to_string(&layout).unwrap();
        assert_eq!(toml::from_str::<Layout>(&text).unwrap(), layout);

        layout.forget("phone");
        assert_eq!(layout.named.len(), 1);
        assert_eq!(layout.current_name(), None);
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
