//! What the extension is to be told of an app somebody picked: the signing
//! ids its connections will carry.
//!
//! macOS names a flow by the signing id of the process that opened it, and
//! for most apps that is **not one process**. A browser's connections come
//! from a helper — Chrome's from `com.google.Chrome.helper`, an Electron
//! app's from `….helper` — so an app picked by its own id alone would show
//! nothing, which is exactly the failure this way in exists to put right.
//! So an app is read whole: the bundle, and every app, XPC service, app
//! extension and helper program inside it.
//!
//! **A program whose id is not its own is taken by where it is.** What
//! `go build` makes is `a.out`, every one of them, and what a linker signs
//! ad hoc is named after its file and a hash that changes with each build
//! (`git-55554944…`). Neither picks out the program somebody chose, so such a
//! program — and a folder, for every program built into it — is given to the
//! extension as a path, which it matches against the file the process runs.

use anyhow::{Result, bail};
use std::path::{Path, PathBuf};

/// An app or a program, as the person picked it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Program {
    /// What to call it: the bundle's or the file's name.
    pub name: String,
    /// Its signing id first, then those of what is inside it. None when it
    /// is taken by its path.
    pub ids: Vec<String>,
    /// Taken by where it is: a program signed ad hoc or not at all, an app
    /// not signed, or a folder of programs.
    pub by_path: bool,
}

/// The most pieces of code read from one app. Xcode has hundreds; what is
/// past this is not where an app's connections come from.
const MOST: usize = 160;

/// Kinds of bundle that run as a process of their own.
const RUNS: &[&str] = &["app", "xpc", "appex"];

/// The app, program or folder at `path`, read for its signing ids.
pub fn program_at(path: &Path) -> Result<Program> {
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    if !path.exists() {
        bail!("{} is not there", path.display());
    }
    let bundle = is_bundle(path);
    if path.is_dir() && !bundle {
        return Ok(Program {
            name,
            ids: Vec::new(),
            by_path: true,
        });
    }
    let by_path = Program {
        name: name.clone(),
        ids: Vec::new(),
        by_path: true,
    };
    if !bundle {
        // A program: its own id if it has one, else where it is.
        return Ok(match signing(path) {
            Some(Signed { id, adhoc: false }) => Program {
                name,
                ids: vec![id],
                by_path: false,
            },
            _ => by_path,
        });
    }
    let code = code_within(path);
    let ids = signing_ids(&code);
    // An app nobody signed is taken as the folder it is.
    let Some(own) = ids.first().and_then(Option::as_ref) else {
        return Ok(by_path);
    };
    let mut all = vec![own.clone()];
    // What is inside it is kept when it is named as its maker names things.
    // A bare name such as `chrome_crashpad_handler` is the same in every app
    // that ships it, so keeping it would take another app's helper too.
    for id in ids.into_iter().skip(1).flatten() {
        if id.contains('.') && !all.contains(&id) {
            all.push(id);
        }
    }
    Ok(Program {
        name,
        ids: all,
        by_path: false,
    })
}

/// What the extension is told of a program taken by its path: the file it
/// runs, links followed, as the system reports a process's; a folder, the
/// same, ending in `/`.
pub fn program_rule(path: &Path) -> Result<String> {
    let real =
        std::fs::canonicalize(path).map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
    let mut rule = real.to_string_lossy().into_owned();
    if real.is_dir() && !rule.ends_with('/') {
        rule.push('/');
    }
    Ok(rule)
}

fn is_bundle(path: &Path) -> bool {
    path.is_dir()
        && path
            .extension()
            .is_some_and(|e| RUNS.iter().any(|r| e == *r))
}

/// What `codesign` says of a piece of code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signed {
    pub id: String,
    /// Signed with no identity: by a linker, or by hand with `-s -`. Its id
    /// is whatever was made up for it.
    pub adhoc: bool,
}

/// What `codesign -d -v` says, which it says on standard error.
pub fn signed_in(said: &str) -> Option<Signed> {
    Some(Signed {
        id: identifier_in(said)?,
        adhoc: said.lines().any(|line| line.trim() == "Signature=adhoc"),
    })
}

/// The pieces of code to read: `path` itself first, then, in a bundle,
/// every bundle that runs as a process and every program in a `Helpers` or
/// `MacOS` folder. Links are not followed — a framework's `Current` is one —
/// so nothing is read twice through them.
pub fn code_within(path: &Path) -> Vec<PathBuf> {
    let mut found = vec![path.to_owned()];
    if path.is_dir() {
        walk(&path.join("Contents"), 0, &mut found);
    }
    found.truncate(MOST);
    found
}

fn walk(dir: &Path, depth: usize, found: &mut Vec<PathBuf>) {
    if depth > 10 || found.len() >= MOST {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    let programs_here = dir
        .file_name()
        .is_some_and(|n| n == "Helpers" || n == "MacOS");
    for entry in entries {
        let Ok(meta) = std::fs::symlink_metadata(&entry) else {
            continue;
        };
        if meta.is_dir() {
            let runs = entry
                .extension()
                .is_some_and(|e| RUNS.iter().any(|r| e == *r));
            if runs {
                found.push(entry.clone());
            }
            walk(&entry, depth + 1, found);
        } else if meta.is_file() && programs_here && executable(&meta) {
            found.push(entry);
        }
    }
}

#[cfg(unix)]
fn executable(meta: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    meta.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn executable(_: &std::fs::Metadata) -> bool {
    true
}

/// The `Identifier=` line of what `codesign -d -v` says, which it says on
/// standard error.
pub fn identifier_in(said: &str) -> Option<String> {
    said.lines()
        .find_map(|line| line.strip_prefix("Identifier="))
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
}

/// Each piece's signing id, in the same order, read a few at a time.
fn signing_ids(code: &[PathBuf]) -> Vec<Option<String>> {
    let mut ids = vec![None; code.len()];
    std::thread::scope(|scope| {
        for (paths, ids) in code.chunks(8).zip(ids.chunks_mut(8)) {
            scope.spawn(move || {
                for (path, id) in paths.iter().zip(ids.iter_mut()) {
                    *id = signing(path).map(|signed| signed.id);
                }
            });
        }
    });
    ids
}

#[cfg(target_os = "macos")]
fn signing(path: &Path) -> Option<Signed> {
    let out = std::process::Command::new("/usr/bin/codesign")
        .args(["-d", "-v"])
        .arg(path)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| signed_in(&String::from_utf8_lossy(&out.stderr)))
        .flatten()
}

#[cfg(not(target_os = "macos"))]
fn signing(_: &Path) -> Option<Signed> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_identifier_is_read_from_what_codesign_says() {
        let said = "Executable=/Applications/Google Chrome.app/Contents/MacOS/Google Chrome\n\
                    Identifier=com.google.Chrome\n\
                    Format=app bundle with Mach-O universal (x86_64 arm64)\n";
        assert_eq!(identifier_in(said).as_deref(), Some("com.google.Chrome"));
        assert_eq!(identifier_in("x: code object is not signed at all\n"), None);
        let go = "Identifier=a.out\n\
                  CodeDirectory v=20400 size=162718 flags=0x20002(adhoc,linker-signed)\n\
                  Signature=adhoc\n";
        assert_eq!(
            signed_in(go),
            Some(Signed {
                id: "a.out".into(),
                adhoc: true
            })
        );
        assert!(!signed_in(said).unwrap().adhoc);
    }

    #[test]
    fn a_rule_is_the_real_path_and_a_folder_ends_in_a_slash() {
        let dir = std::env::temp_dir().join(format!("tracewing-rule-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("bin")).unwrap();
        std::fs::write(dir.join("bin/server"), b"").unwrap();
        let real = std::fs::canonicalize(&dir).unwrap();
        // Joined with `Path`, and only the trailing slash the rule itself
        // adds written by hand. `format!("{real}/bin/")` is the same string
        // as this on Linux and a different one on Windows, where the rule
        // says `…\bin/` while a hand-joined expectation says `…/bin/`.
        assert_eq!(
            program_rule(&dir.join("bin")).unwrap(),
            format!("{}/", real.join("bin").display())
        );
        assert_eq!(
            program_rule(&dir.join("bin/../bin/server")).unwrap(),
            real.join("bin").join("server").display().to_string()
        );
        assert!(program_rule(&dir.join("gone")).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_folder_is_taken_by_where_it_is() {
        let dir = std::env::temp_dir().join(format!("tracewing-folder-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let folder = program_at(&dir).unwrap();
        assert!(folder.by_path && folder.ids.is_empty());
        assert!(program_at(&dir.join("gone")).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The shape of Chrome: helpers inside a framework, a version reached
    /// through a link as well as by name, and programs beside them.
    #[cfg(unix)]
    #[test]
    fn an_app_is_read_with_its_helpers_and_not_twice_through_a_link() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let root = std::env::temp_dir().join(format!("tracewing-program-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let app = root.join("Browser.app");
        let version = app.join("Contents/Frameworks/Browser Framework.framework/Versions/1");
        let helpers = version.join("Helpers");
        std::fs::create_dir_all(helpers.join("Browser Helper.app/Contents/MacOS")).unwrap();
        std::fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
        std::fs::create_dir_all(app.join("Contents/XPCServices/Fetch.xpc")).unwrap();
        std::fs::create_dir_all(app.join("Contents/Resources")).unwrap();
        let program = |path: PathBuf, mode: u32| {
            std::fs::write(&path, b"").unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
        };
        program(app.join("Contents/MacOS/Browser"), 0o755);
        program(helpers.join("crashpad_handler"), 0o755);
        program(helpers.join("notes.txt"), 0o644);
        program(app.join("Contents/Resources/script.sh"), 0o755);
        symlink("1", version.parent().unwrap().join("Current")).unwrap();

        let found = code_within(&app);
        let names: Vec<_> = found
            .iter()
            .map(|p| p.strip_prefix(&root).unwrap().display().to_string())
            .collect();
        assert_eq!(names[0], "Browser.app", "the app itself first");
        for wanted in [
            "Browser Helper.app",
            "crashpad_handler",
            "Fetch.xpc",
            "MacOS/Browser",
        ] {
            assert_eq!(
                names.iter().filter(|n| n.ends_with(wanted)).count(),
                1,
                "{wanted} once, in {names:#?}"
            );
        }
        assert!(
            !names
                .iter()
                .any(|n| n.ends_with("notes.txt") || n.ends_with("script.sh")),
            "only programs, and only where an app keeps them: {names:#?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_signed_program_is_read_by_its_id() {
        let curl = program_at(Path::new("/usr/bin/curl")).unwrap();
        assert_eq!(curl.name, "curl");
        assert_eq!(curl.ids, ["com.apple.curl"]);
        assert!(!curl.by_path);
    }

    /// This test's own program is what a linker signed ad hoc, as `go build`
    /// and `cargo build` sign theirs: its id is made up, so it goes by path.
    #[cfg(target_os = "macos")]
    #[test]
    fn a_program_signed_ad_hoc_is_taken_by_where_it_is() {
        let me = program_at(&std::env::current_exe().unwrap()).unwrap();
        assert!(me.by_path, "{me:?}");
        assert!(me.ids.is_empty());
    }
}
