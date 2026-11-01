//! What the user can change about how a session is stored. The app picks every
//! default, from what the benchmarks found (see the README), and every one can be
//! overridden: a laptop with little disk and a build server want different things.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::Path;

const MIB: u64 = 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StorageSettings {
    /// A body up to this size goes in the shared log, held in memory until it has
    /// all arrived. A larger one is written to a file of its own as it comes.
    pub log_max_kib: u64,
    /// The most kept of one body. It still reaches the client whole.
    pub body_max_mib: u64,
    /// Compress text bodies that came uncompressed. They are put back when shown.
    pub compress_stored: bool,
    /// zstd level for that, 1 to 19.
    pub compress_level: i32,
    /// Make URLs searchable, pieces of words included.
    pub index_urls: bool,
    /// Make the text of bodies searchable by word.
    pub index_bodies: bool,
    /// How much of each body's text goes in the index. The rest is found by
    /// scanning, which is slower. Also the most a body is unpacked for the index,
    /// which is what stops a zip bomb.
    pub index_kib_per_body: u64,
    /// The most a body is unpacked for viewing.
    pub view_max_mib: u64,
    /// A session past this size drops its oldest flows.
    pub session_max_mib: u64,
    /// With less than this left on the disk, stop keeping bodies and say so.
    pub disk_reserve_mib: u64,
    /// How many bytes of bodies may wait for the database, then for the disk.
    /// Past these, bodies stop being kept; traffic is never held back.
    pub queue_budget_mib: u64,
    pub spool_budget_mib: u64,
    /// How many earlier sessions to leave on the disk when a new one starts.
    pub sessions_kept: u64,
}

impl Default for StorageSettings {
    fn default() -> Self {
        Self {
            log_max_kib: 64,
            body_max_mib: 1024,
            compress_stored: true,
            compress_level: 1,
            index_urls: true,
            index_bodies: true,
            index_kib_per_body: 64,
            view_max_mib: 4096,
            session_max_mib: 2048,
            disk_reserve_mib: 2048,
            queue_budget_mib: 256,
            spool_budget_mib: 256,
            sessions_kept: 5,
        }
    }
}

impl StorageSettings {
    pub fn load(path: &Path) -> Result<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let settings: Self =
            toml::from_str(&text).with_context(|| format!("read {}", path.display()))?;
        settings.check()?;
        Ok(settings)
    }

    pub fn to_toml(&self) -> Result<String> {
        Ok(toml::to_string(self)?)
    }

    /// Refuses what cannot work, and only that: a strange choice is the user's.
    pub fn check(&self) -> Result<()> {
        if !(1..=19).contains(&self.compress_level) {
            bail!(
                "compress_level must be 1 to 19, not {}",
                self.compress_level
            );
        }
        if self.log_max_kib > 1024 {
            bail!(
                "log_max_kib must be at most 1024, not {}: every body in flight holds this much in memory",
                self.log_max_kib
            );
        }
        if self.session_max_mib < 64 {
            bail!(
                "session_max_mib must be at least 64, not {}: one body can be larger than that",
                self.session_max_mib
            );
        }
        Ok(())
    }

    pub fn log_max(&self) -> usize {
        (self.log_max_kib * 1024) as usize
    }

    pub fn body_max(&self) -> u64 {
        self.body_max_mib * MIB
    }

    pub fn view_max(&self) -> u64 {
        self.view_max_mib * MIB
    }

    pub fn index_per_body(&self) -> usize {
        (self.index_kib_per_body * 1024) as usize
    }

    pub fn session_max(&self) -> u64 {
        self.session_max_mib * MIB
    }

    pub fn disk_reserve(&self) -> u64 {
        self.disk_reserve_mib * MIB
    }

    pub fn queue_budget(&self) -> u64 {
        self.queue_budget_mib * MIB
    }

    pub fn spool_budget(&self) -> u64 {
        self.spool_budget_mib * MIB
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Result<StorageSettings> {
        let settings: StorageSettings = toml::from_str(text)?;
        settings.check()?;
        Ok(settings)
    }

    #[test]
    fn nothing_set_means_the_defaults() {
        assert_eq!(parse("").unwrap(), StorageSettings::default());
    }

    #[test]
    fn one_setting_leaves_the_others_alone() {
        let settings = parse("session_max_mib = 500").unwrap();
        assert_eq!(settings.session_max(), 500 * MIB);
        assert_eq!(settings.log_max_kib, StorageSettings::default().log_max_kib);
    }

    #[test]
    fn the_defaults_survive_being_written_out() {
        let written = StorageSettings::default().to_toml().unwrap();
        assert_eq!(parse(&written).unwrap(), StorageSettings::default());
    }

    #[test]
    fn a_misspelt_setting_is_an_error() {
        // Silently ignoring it would leave the user believing it took effect.
        assert!(parse("session_max = 500").is_err());
    }

    #[test]
    fn what_cannot_work_is_refused() {
        assert!(parse("compress_level = 0").is_err());
        assert!(parse("compress_level = 20").is_err());
        assert!(parse("session_max_mib = 1").is_err());
        assert!(parse("log_max_kib = 4096").is_err());
    }

    #[test]
    fn turning_things_off_is_allowed() {
        let settings =
            parse("index_bodies = false\nindex_urls = false\ncompress_stored = false").unwrap();
        assert!(!settings.index_bodies);
    }
}
