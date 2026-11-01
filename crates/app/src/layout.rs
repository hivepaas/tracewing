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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    pub place: Place,
    pub sidebar_shown: bool,
    pub inspector_shown: bool,
    pub sidebar_w: f32,
    /// Beside the list.
    pub inspector_w: f32,
    /// Below it.
    pub inspector_h: f32,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            place: Place::Beside,
            sidebar_shown: true,
            inspector_shown: true,
            sidebar_w: 220.,
            inspector_w: 460.,
            inspector_h: 380.,
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
    fn it_comes_back_as_it_was_left() {
        let dir = std::env::temp_dir().join(format!("layout-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("layout.toml");
        let layout = Layout {
            place: Place::Below,
            sidebar_shown: false,
            inspector_h: 300.,
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
