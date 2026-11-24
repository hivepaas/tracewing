//! Installing a downloaded release in place of the app, as it was installed
//! (see [`Install`]):
//!
//! - **An AppImage** is replaced by the new one: written beside it, then
//!   renamed over it, so it is whole or not at all.
//! - **An `.app`** is replaced whole: the new one is unpacked beside it,
//!   checked (`codesign`: valid, and signed by the same team as the one
//!   running), and the two are swapped; the old one is put back if the swap
//!   fails half way.
//! - **On Windows**, the release's installer runs, quiet, once the app has
//!   quit.
//!
//! Then, when asked, the new app starts.

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::install::Install;

/// Installs the release in `staged` over the app installed as `install`,
/// then starts it when `relaunch`. Call it as the app quits.
pub fn apply(install: &Install, staged: &Path, relaunch: bool) -> io::Result<()> {
    match install {
        Install::AppImage(image) => {
            replace_file(image, staged)?;
            if relaunch {
                Command::new(image).spawn()?;
            }
            Ok(())
        }
        Install::MacBundle(bundle) => {
            replace_bundle(bundle, staged, signed_alike)?;
            if relaunch {
                // From the app's quit, on its main thread: not one of GPUI's
                // GCD workers, whose blocked signals a child would inherit.
                Command::new("open").arg("-n").arg(bundle).spawn()?;
            }
            Ok(())
        }
        Install::WindowsInstaller => run_installer(staged, relaunch),
        Install::Manual | Install::Package(_) | Install::Managed { .. } => Err(io::Error::other(
            "this copy of the app is not one it can update itself",
        )),
    }
}

/// `target` replaced by a copy of `new`, executable: written beside it, then
/// renamed over it.
pub fn replace_file(target: &Path, new: &Path) -> io::Result<()> {
    let dir = target
        .parent()
        .ok_or_else(|| io::Error::other("no folder"))?;
    let name = target
        .file_name()
        .ok_or_else(|| io::Error::other("no file name"))?
        .to_string_lossy()
        .to_string();
    let tmp = dir.join(format!(".{name}.update"));
    std::fs::copy(new, &tmp)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
    }
    std::fs::rename(&tmp, target).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// `bundle` (an `.app`) replaced by the one in the tar.gz `archive`, once
/// `check(new, old)` accepts it.
pub fn replace_bundle(
    bundle: &Path,
    archive: &Path,
    check: impl Fn(&Path, &Path) -> io::Result<()>,
) -> io::Result<()> {
    let parent = bundle
        .parent()
        .ok_or_else(|| io::Error::other("no folder"))?;
    // Beside it, so the swap is a rename on one volume.
    let work = parent.join(format!(".tracewing-update-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir(&work)?;
    let done = (|| {
        let file = std::fs::File::open(archive)?;
        // Entries that would land outside `work` (`..`, absolute) are refused.
        tar::Archive::new(flate2::read::GzDecoder::new(file)).unpack(work.join("new"))?;
        let new = only_app_in(&work.join("new"))?;
        check(&new, bundle)?;
        let old = work.join("old.app");
        std::fs::rename(bundle, &old)?;
        if let Err(e) = std::fs::rename(&new, bundle) {
            // Back as it was.
            let _ = std::fs::rename(&old, bundle);
            return Err(e);
        }
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&work);
    done
}

/// The one `.app` in `dir`.
fn only_app_in(dir: &Path) -> io::Result<PathBuf> {
    let apps: Vec<PathBuf> = std::fs::read_dir(dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "app") && p.is_dir())
        .collect();
    match apps.as_slice() {
        [one] => Ok(one.clone()),
        _ => Err(io::Error::other("the release holds no single .app")),
    }
}

/// The new `.app` is validly signed, by the team that signed the running one
/// (unless that one is not signed: a build of one's own).
fn signed_alike(new: &Path, old: &Path) -> io::Result<()> {
    let ok = Command::new("codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(new)
        .status()?
        .success();
    if !ok {
        return Err(io::Error::other("the new app's signature does not verify"));
    }
    let team = |app: &Path| -> io::Result<Option<String>> {
        let out = Command::new("codesign")
            .args(["-dv", "--verbose=2"])
            .arg(app)
            .output()?;
        // codesign says it on stderr.
        Ok(String::from_utf8_lossy(&out.stderr)
            .lines()
            .find_map(|l| l.strip_prefix("TeamIdentifier="))
            .filter(|t| *t != "not set")
            .map(str::to_string))
    };
    match team(old)? {
        None => Ok(()),
        Some(t) if team(new)?.as_ref() == Some(&t) => Ok(()),
        Some(_) => Err(io::Error::other(
            "the new app is signed by another team than this one",
        )),
    }
}

/// The installer in `setup`, run quiet once this app has gone, then the app
/// again when `relaunch`.
fn run_installer(setup: &Path, relaunch: bool) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let exe = std::env::current_exe()?;
        let system = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
        let powershell = Path::new(&system).join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
        Command::new(powershell)
            .args(["-NoProfile", "-NonInteractive", "-Command"])
            .arg(installer_script(
                std::process::id(),
                setup,
                relaunch.then_some(exe.as_path()),
            ))
            .creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW)
            .spawn()?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = (setup, relaunch);
        Err(io::Error::other("an installer runs on Windows only"))
    }
}

/// What PowerShell runs once the app has asked for the installer: it waits for
/// the app, process `pid`, to be gone, then runs the installer quietly, then
/// starts `relaunch`.
///
/// **Waited for, not a moment.** The installer copies tracewing.exe over the
/// running one, tries for five seconds, and when quiet gives up after that
/// (packaging/windows/installer.nsi). Quitting takes as long as the proxy
/// takes to stop and the system proxy to be put back, which is not a number
/// of seconds anyone can promise, so the app's own process is waited for
/// (`Wait-Process`, for a minute at most). For everyone, the installer asks
/// Windows for an administrator's rights itself, and the user is asked.
///
/// Paths are put in single quotes, where PowerShell reads nothing but a
/// doubled quote: a name with `$` or a backtick in it stays a name.
pub fn installer_script(pid: u32, setup: &Path, relaunch: Option<&Path>) -> String {
    let quoted = |p: &Path| format!("'{}'", p.display().to_string().replace('\'', "''"));
    let mut script = format!(
        "Wait-Process -Id {pid} -Timeout 60 -ErrorAction SilentlyContinue; \
         Start-Process -FilePath {} -ArgumentList '/S' -Wait",
        quoted(setup)
    );
    if let Some(exe) = relaunch {
        script.push_str(&format!("; Start-Process -FilePath {}", quoted(exe)));
    }
    script
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_installer_waits_for_the_app_and_its_paths_stay_paths() {
        let script = installer_script(
            4242,
            Path::new(r"C:\Users\o'neil\AppData\Local\tracewing\updates\setup.exe"),
            Some(Path::new(r"C:\Program Files\tracewing\tracewing.exe")),
        );
        assert_eq!(
            script,
            "Wait-Process -Id 4242 -Timeout 60 -ErrorAction SilentlyContinue; \
             Start-Process -FilePath 'C:\\Users\\o''neil\\AppData\\Local\\tracewing\\updates\\setup.exe' \
             -ArgumentList '/S' -Wait; \
             Start-Process -FilePath 'C:\\Program Files\\tracewing\\tracewing.exe'"
        );
        assert!(!installer_script(1, Path::new("s.exe"), None).contains("tracewing.exe"));
    }

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("tracewing-apply-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }

    #[test]
    fn an_appimage_is_replaced_whole_and_left_runnable() {
        let dir = temp("appimage");
        let image = dir.join("tracewing.AppImage");
        let new = dir.join("downloaded");
        std::fs::write(&image, b"old").expect("old");
        std::fs::write(&new, b"new").expect("new");
        replace_file(&image, &new).expect("replace");
        assert_eq!(std::fs::read(&image).expect("read"), b"new");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = std::fs::metadata(&image)
                .expect("meta")
                .permissions()
                .mode();
            assert_eq!(mode & 0o111, 0o111, "executable");
        }
        // Nothing left beside it.
        let left: Vec<_> = std::fs::read_dir(&dir).expect("ls").flatten().collect();
        assert_eq!(left.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A tar.gz holding `tracewing.app` with one file saying `what`.
    fn app_archive(path: &Path, what: &str) {
        let file = std::fs::File::create(path).expect("archive");
        let gz = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
        let mut tar = tar::Builder::new(gz);
        let mut head = tar::Header::new_gnu();
        head.set_size(what.len() as u64);
        head.set_mode(0o755);
        head.set_cksum();
        tar.append_data(
            &mut head,
            "tracewing.app/Contents/MacOS/tracewing",
            what.as_bytes(),
        )
        .expect("append");
        tar.into_inner().expect("tar").finish().expect("gz");
    }

    #[test]
    fn an_app_bundle_is_swapped_once_checked_and_kept_when_refused() {
        let dir = temp("bundle");
        let bundle = dir.join("tracewing.app");
        std::fs::create_dir_all(bundle.join("Contents/MacOS")).expect("app");
        std::fs::write(bundle.join("Contents/MacOS/tracewing"), "old").expect("old");
        let archive = dir.join("release.app.tar.gz");
        app_archive(&archive, "new");
        // Refused: the running one stays as it was.
        let refused = replace_bundle(&bundle, &archive, |_, _| Err(io::Error::other("no")));
        assert!(refused.is_err());
        assert_eq!(
            std::fs::read_to_string(bundle.join("Contents/MacOS/tracewing")).expect("read"),
            "old"
        );
        // Accepted: swapped, and nothing left beside it.
        replace_bundle(&bundle, &archive, |new, old| {
            assert!(new.ends_with("tracewing.app") && old == bundle);
            Ok(())
        })
        .expect("swap");
        assert_eq!(
            std::fs::read_to_string(bundle.join("Contents/MacOS/tracewing")).expect("read"),
            "new"
        );
        let left: Vec<String> = std::fs::read_dir(&dir)
            .expect("ls")
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(left.len(), 2, "{left:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
