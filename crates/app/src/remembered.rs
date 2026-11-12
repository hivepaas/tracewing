//! How a field is read, once a reader has said to read it that way always.
//!
//! Without this the readings are a toy: the same `created_at` would be read by
//! hand on every request of the same endpoint. A rule names a host, a field
//! and the reading wanted — not the reading itself, which is worked out again
//! for each new value (see [`tracewing_convert::read_as`]).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Default, Serialize, Deserialize)]
struct File {
    #[serde(default, rename = "rule")]
    rules: Vec<Rule>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Rule {
    host: String,
    field: String,
    read_as: String,
}

/// The rules, by host and field. A host of `""` would be every host, which
/// nothing writes yet.
#[derive(Default)]
pub struct Remembered {
    path: PathBuf,
    rules: BTreeMap<(String, String), String>,
    /// Counted up as rules change, so that a view holding readings worked out
    /// from them knows when to work them out again.
    version: u64,
}

impl Remembered {
    /// What was remembered before. A file that cannot be read gives nothing,
    /// and is left alone until something is remembered.
    pub fn load(path: &Path) -> Self {
        let rules = std::fs::read_to_string(path)
            .ok()
            .and_then(|text| toml::from_str::<File>(&text).ok())
            .map(|file| {
                file.rules
                    .into_iter()
                    .map(|rule| ((rule.host, lower(&rule.field)), rule.read_as))
                    .collect()
            })
            .unwrap_or_default();
        Self {
            path: path.to_owned(),
            rules,
            version: 0,
        }
    }

    /// Changes with every rule remembered or forgotten.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// The reading a field is always read as here, if one was remembered: for
    /// this host, else for every host.
    pub fn reading_for(&self, host: &str, field: &str) -> Option<&str> {
        let field = lower(field);
        self.rules
            .get(&(host.to_owned(), field.clone()))
            .or_else(|| self.rules.get(&(String::new(), field)))
            .map(String::as_str)
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Every rule, for a list to read and to forget from.
    pub fn all(&self) -> impl Iterator<Item = (&str, &str, &str)> {
        self.rules
            .iter()
            .map(|((host, field), read_as)| (host.as_str(), field.as_str(), read_as.as_str()))
    }

    /// Reads `field` on `host` as `read_as` from now on.
    pub fn remember(&mut self, host: &str, field: &str, read_as: &str) -> Result<()> {
        self.rules
            .insert((host.to_owned(), lower(field)), read_as.to_owned());
        self.version += 1;
        self.save()
    }

    /// Forgets the rule that applies to `field` on `host`, whichever it is.
    pub fn forget(&mut self, host: &str, field: &str) -> Result<()> {
        let field = lower(field);
        if self
            .rules
            .remove(&(host.to_owned(), field.clone()))
            .is_none()
        {
            self.rules.remove(&(String::new(), field));
        }
        self.version += 1;
        self.save()
    }

    fn save(&self) -> Result<()> {
        let file = File {
            rules: self
                .rules
                .iter()
                .map(|((host, field), read_as)| Rule {
                    host: host.clone(),
                    field: field.clone(),
                    read_as: read_as.clone(),
                })
                .collect(),
        };
        let text = toml::to_string(&file)?;
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let new = self.path.with_extension("toml.new");
        std::fs::write(&new, text).with_context(|| format!("write {}", new.display()))?;
        std::fs::rename(&new, &self.path).with_context(|| format!("write {}", self.path.display()))
    }
}

/// A field's name is matched however it is written, as the converters match
/// it: `Created-At`, `created_at` and `createdAt` are one field.
fn lower(field: &str) -> String {
    field
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn in_a_new_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tracewing-remembered-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("readings.toml")
    }

    #[test]
    fn what_is_remembered_is_found_again_after_a_restart() {
        let path = in_a_new_dir("restart");
        let mut rules = Remembered::load(&path);
        assert!(rules.is_empty());
        rules
            .remember("api.example.com", "created_at", "unix-ms")
            .unwrap();
        let again = Remembered::load(&path);
        assert_eq!(
            again.reading_for("api.example.com", "created_at"),
            Some("unix-ms")
        );
        // Another host is another field, until a rule says every host.
        assert_eq!(again.reading_for("other.example.com", "created_at"), None);
    }

    #[test]
    fn a_field_is_matched_however_it_is_written() {
        let path = in_a_new_dir("spelling");
        let mut rules = Remembered::load(&path);
        rules.remember("h", "X-Created-At", "unix-s").unwrap();
        assert_eq!(rules.reading_for("h", "x-created-at"), Some("unix-s"));
        assert_eq!(rules.reading_for("h", "XCreatedAt"), Some("unix-s"));
        // A different name, not the same one written differently.
        assert_eq!(rules.reading_for("h", "created_at"), None);
        rules.remember("h", "created_at", "unix-ms").unwrap();
        assert_eq!(rules.reading_for("h", "createdAt"), Some("unix-ms"));
    }

    #[test]
    fn a_rule_for_every_host_is_the_one_to_fall_back_on() {
        let path = in_a_new_dir("every-host");
        let mut rules = Remembered::load(&path);
        rules.remember("", "exp", "unix-s").unwrap();
        rules.remember("one.example.com", "exp", "unix-ms").unwrap();
        assert_eq!(rules.reading_for("two.example.com", "exp"), Some("unix-s"));
        assert_eq!(rules.reading_for("one.example.com", "exp"), Some("unix-ms"));
    }

    #[test]
    fn what_is_forgotten_stays_forgotten() {
        let path = in_a_new_dir("forget");
        let mut rules = Remembered::load(&path);
        rules.remember("h", "ttl", "duration").unwrap();
        rules.forget("h", "ttl").unwrap();
        assert!(Remembered::load(&path).is_empty());
        // Forgetting what is not there is not an error.
        rules.forget("h", "ttl").unwrap();
    }

    #[test]
    fn a_change_shows_in_the_version() {
        let path = in_a_new_dir("version");
        let mut rules = Remembered::load(&path);
        let first = rules.version();
        rules.remember("h", "exp", "unix-s").unwrap();
        assert_ne!(rules.version(), first);
        let after = rules.version();
        rules.forget("h", "exp").unwrap();
        assert_ne!(rules.version(), after);
    }

    #[test]
    fn a_file_that_cannot_be_read_is_left_alone() {
        let path = in_a_new_dir("broken");
        std::fs::write(&path, "this is not toml {{{").unwrap();
        let rules = Remembered::load(&path);
        assert!(rules.is_empty());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "this is not toml {{{"
        );
    }
}
