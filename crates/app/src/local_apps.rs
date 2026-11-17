//! The local way in, from the window: the programs somebody picked for
//! tracewing to keep when this machine's traffic comes through its own
//! adapter.
//!
//! **There is no capture-everything mode, by decision.** Windows and Linux
//! capture by *route*, so the operating system hands tracewing every
//! connection whatever its destination -- but what is kept is always a
//! filter, and the filter is these programs. The rest is spliced straight
//! through and never recorded, which is both what keeps the volume sane and
//! what keeps tracewing's certificate away from programs that pin their own.
//! It is also what makes this way in the same promise as the macOS one:
//! choose the programs, see those programs.
//!
//! The programs are kept in `local-programs.toml`. **Whether the switch is
//! on is kept too**, in the settings, and that is the one place this parts
//! company with [`crate::mac_apps`], which deliberately starts off every run
//! so that opening tracewing installs nothing. Here it has to persist: a way
//! in whose source is assembled as the engine starts cannot be added to a
//! running one, and on Windows switching on means coming back as an
//! administrator -- a new process, which has nothing but the settings to
//! learn from.
//!
//! What the sieve matches on is the **path**, canonicalised by
//! [`tracewing_proxy::sieve`] rather than here: it has the rules for a
//! `\\?\` prefix, for case on Windows, and for a folder stding for every
//! program under it, and they are tested there against both platforms'
//! spellings.

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Where the picked programs are kept, in the data directory.
pub const FILE: &str = "local-programs.toml";

/// A program as it was picked: what to call it, and where it is.
///
/// No signing ids, unlike the macOS way in: nothing here asks the operating
/// system who made a program, because the question this way in answers is
/// "which socket belongs to which program", and the answer comes from the
/// path of the running program itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Picked {
    pub name: String,
    pub path: PathBuf,
}

impl Picked {
    /// A program picked at `path`, named by its file name where it has one.
    ///
    /// A folder keeps its own name, since it stands for every program under
    /// it and a folder named for one of them would read as that program.
    pub fn at(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        Self { name, path }
    }
}

/// This way in's own state: whether it is on, and what it keeps.
///
/// **The switch is kept here rather than in the settings**, with the programs
/// it belongs to, for the same reason the macOS way in keeps its apps in a
/// file of its own: it is the state of one way in rather than a preference
/// about how traffic is stored. It also leaves `every_setting_has_a_row`
/// saying what it says -- that nothing in the settings is unreachable from
/// Settings -- without a third named exception.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Local {
    /// Whether this machine's traffic is to come through tracewing's adapter.
    ///
    /// Unlike the macOS switch, which starts off every run on purpose, this
    /// one persists: the source is assembled as the engine starts and cannot
    /// be added to a running one, and on Windows switching on means coming
    /// back as an administrator -- a new process, with nothing but this file
    /// to learn from.
    pub on: bool,
    #[serde(rename = "program")]
    pub programs: Vec<Picked>,
}

/// What is kept in `dir`: off, with no programs, where the file is missing or
/// unreadable.
///
/// Unreadable gives the off state rather than an error, and that is the
/// direction that matters: a file somebody cannot parse would otherwise leave
/// a machine capturing whatever it held last, or capturing with a list nobody
/// can see.
pub fn load(dir: &Path) -> Local {
    std::fs::read_to_string(dir.join(FILE))
        .ok()
        .and_then(|text| toml::from_str::<Local>(&text).ok())
        .unwrap_or_default()
}

/// Writes it, through a temporary file so that a half-written list is never
/// the list: the same reason [`crate::mac_apps::save`] does.
pub fn save(dir: &Path, local: &Local) -> Result<()> {
    let text = toml::to_string(local)?;
    std::fs::create_dir_all(dir)?;
    let path = dir.join(FILE);
    let new = path.with_extension("toml.new");
    std::fs::write(&new, text).with_context(|| format!("write {}", new.display()))?;
    std::fs::rename(&new, &path).with_context(|| format!("write {}", path.display()))
}

/// What the sieve is given: each path as it was picked, once.
///
/// As written rather than canonicalised, because the canonicalising is the
/// sieve's and it does it on both sides of the comparison. Doing it here as
/// well would mean two spellings of the same rule, and the one that is
/// tested against Windows' own spellings is not this one.
pub fn programs_of(picked: &[Picked]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for path in picked.iter().map(|program| program.path.to_string_lossy()) {
        let path = path.into_owned();
        if !out.contains(&path) {
            out.push(path);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One directory per test, named for it: these run in parallel threads of
    /// one process, so a directory named only for the process is one they take
    /// from each other -- which is how this was found, a test removing the
    /// file another had just written. Three runs said FAILED, ok, FAILED.
    fn dir(which: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("tracewing-local-{}-{which}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn the_switch_and_the_programs_survive_being_written_and_read() {
        let dir = dir("round-trip");
        let local = Local {
            on: true,
            programs: vec![
                Picked::at(PathBuf::from(r"C:\Program Files\nodejs\node.exe")),
                Picked::at(PathBuf::from("/usr/bin/curl")),
            ],
        };
        save(&dir, &local).unwrap();
        assert_eq!(
            load(&dir),
            local,
            "the switch, a path with a space in it, and one of each platform's spelling"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The name is read by **this** platform's path rules, which is right
    /// because a program is picked on the machine it runs on: `std::path`
    /// treats `\` as a separator on Windows and as an ordinary character
    /// elsewhere, so the Windows path below has no file name on Linux.
    ///
    /// Deliberately not the sieve's behaviour, and the difference is worth
    /// knowing: `sieve::plain` understands **both** spellings on purpose,
    /// because its rules are tested from Linux against what Windows does.
    /// A display name has no such second reader.
    #[test]
    fn a_program_is_named_by_this_platforms_rules() {
        assert_eq!(Picked::at(PathBuf::from("/usr/bin/curl")).name, "curl");
        let windows = Picked::at(PathBuf::from(r"C:\Program Files\nodejs\node.exe"));
        match cfg!(windows) {
            true => assert_eq!(windows.name, "node.exe"),
            false => assert_eq!(
                windows.name, r"C:\Program Files\nodejs\node.exe",
                "no separator here, so the whole path is the name"
            ),
        }
        // A folder keeps its own name: it stands for every program under it,
        // and a folder named for one of them would read as that program.
        assert_eq!(Picked::at(PathBuf::from("/opt/mine")).name, "mine");
    }

    /// Nothing keeps nothing. A list that cannot be read must not leave the
    /// machine capturing what it held last.
    #[test]
    fn a_list_that_cannot_be_read_is_no_programs() {
        let dir = dir("unreadable");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(FILE), "on = true\nthis is not toml {{{").unwrap();
        assert_eq!(
            load(&dir),
            Local::default(),
            "off, although the unreadable file says on"
        );
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(load(&dir), Local::default(), "and so is a missing file");
    }

    #[test]
    fn the_sieve_is_given_each_path_once_as_it_was_picked() {
        let picked = vec![
            Picked::at(PathBuf::from("/usr/bin/curl")),
            Picked::at(PathBuf::from("/opt/my build/program")),
            Picked::at(PathBuf::from("/usr/bin/curl")),
        ];
        assert_eq!(
            programs_of(&picked),
            ["/usr/bin/curl", "/opt/my build/program"],
            "a space is left in it, and the repeat is not sent twice"
        );
    }
}
