//! How this copy of the app was installed, as far as can be told from where
//! it runs: a copy the app can replace itself (an AppImage, an `.app`, one put
//! there by its installer on Windows), one put somewhere by hand that it can
//! only point to the download of, one from the release's own `.deb` or `.rpm`
//! that the system's package tool installs the next of, or one something else
//! updates (a package manager, a store), which a replacement would leave
//! thinking it has the old one.

use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Install {
    /// An AppImage, at this path: replaced by the release's.
    AppImage(PathBuf),
    /// An app bundle on macOS, this `.app`: replaced whole.
    MacBundle(PathBuf),
    /// Put there by tracewing's installer on Windows: the next one installs
    /// over it.
    WindowsInstaller,
    /// Put there by hand (a binary run from a folder): the app says where
    /// the release is, and the user puts it there.
    Manual,
    /// From the release's own package (`/usr/bin/tracewing`): this tool
    /// installs the next one, as the user says.
    Package(PackageTool),
    /// Updated by something else: what, and the command that does it, if one.
    Managed {
        by: &'static str,
        command: Option<&'static str>,
    },
}

/// What installs a release's package on this system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackageTool {
    Apt,
    Dnf,
    Zypper,
}

impl PackageTool {
    pub fn name(self) -> &'static str {
        match self {
            PackageTool::Apt => "apt",
            PackageTool::Dnf => "dnf",
            PackageTool::Zypper => "zypper",
        }
    }
}

/// Where the release's `.deb` and `.rpm` put the binary. No distribution
/// packages tracewing, so a copy here is the release's own.
const PACKAGE_EXE: &str = "/usr/bin/tracewing";

/// Which tool installed the package at [`PACKAGE_EXE`]: dpkg keeps a list of a
/// package's files; for an `.rpm`, the tool the system has. `None`: neither,
/// so it was put there by hand.
fn package_tool(exists: impl Fn(&str) -> bool) -> Option<PackageTool> {
    if exists("/var/lib/dpkg/info/tracewing.list") {
        Some(PackageTool::Apt)
    } else if exists("/usr/bin/zypper") {
        Some(PackageTool::Zypper)
    } else if exists("/usr/bin/dnf") {
        Some(PackageTool::Dnf)
    } else {
        None
    }
}

impl Install {
    /// The app can download a release and install it itself.
    pub fn updates_itself(&self) -> bool {
        matches!(
            self,
            Install::AppImage(_) | Install::MacBundle(_) | Install::WindowsInstaller
        )
    }
}

/// The `.app` an executable inside one belongs to (`X.app/Contents/MacOS/x`).
fn bundle_of(exe: &Path) -> Option<PathBuf> {
    let bundle = exe.parent()?.parent()?.parent()?;
    let macos = exe.parent()?.file_name()? == "MacOS";
    let contents = exe.parent()?.parent()?.file_name()? == "Contents";
    let app = bundle.extension().is_some_and(|e| e == "app");
    (macos && contents && app).then(|| bundle.to_path_buf())
}

/// How the app at `exe` was installed; `var` reads the environment.
pub fn install_of(exe: &Path, var: impl Fn(&str) -> Option<String>) -> Install {
    let managed = |by, command| Install::Managed { by, command };
    if var("FLATPAK_ID").is_some() {
        return managed("Flatpak", Some("flatpak update"));
    }
    if var("SNAP").is_some() {
        return managed("Snap", Some("sudo snap refresh tracewing"));
    }
    // An AppImage runs from a mount of itself; it names its file.
    if let Some(image) = var("APPIMAGE").filter(|p| !p.is_empty()) {
        return Install::AppImage(PathBuf::from(image));
    }
    let p = exe.to_string_lossy().replace('\\', "/").to_lowercase();
    if p.contains("/cellar/") || p.contains("/homebrew/") || p.contains("/linuxbrew/") {
        return managed("Homebrew", Some("brew upgrade tracewing"));
    }
    if p.contains("/scoop/apps/") {
        return managed("Scoop", Some("scoop update tracewing"));
    }
    if p.contains("/windowsapps/") {
        return managed("the Microsoft Store", None);
    }
    if let Some(bundle) = bundle_of(exe) {
        if bundle.join("Contents/_MASReceipt").exists() {
            return managed("the App Store", None);
        }
        return Install::MacBundle(bundle);
    }
    // Where tracewing's installer puts it: for the user, or for everyone.
    if p.ends_with(".exe")
        && (p.contains("/appdata/local/programs/tracewing/")
            || p.contains("/program files/tracewing/"))
    {
        return Install::WindowsInstaller;
    }
    if p == PACKAGE_EXE
        && let Some(tool) = package_tool(|f| Path::new(f).exists())
    {
        return Install::Package(tool);
    }
    // Under /usr (not /usr/local): a package of the system's.
    if p.starts_with("/usr/") && !p.starts_with("/usr/local/") {
        return managed("the system's package manager", None);
    }
    Install::Manual
}

#[cfg(test)]
mod tests {
    use super::*;

    fn install(path: &str, vars: &[(&str, &str)]) -> Install {
        let vars: Vec<(String, String)> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        install_of(&PathBuf::from(path), move |k| {
            vars.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone())
        })
    }

    #[test]
    fn what_updates_the_app_is_told_by_where_it_runs() {
        assert_eq!(install("/home/me/bin/tracewing", &[]), Install::Manual);
        assert_eq!(
            install(
                "/tmp/.mount_x/usr/bin/tracewing",
                &[("APPIMAGE", "/home/me/Apps/tracewing.AppImage")]
            ),
            Install::AppImage("/home/me/Apps/tracewing.AppImage".into())
        );
        assert_eq!(
            install("/Applications/tracewing.app/Contents/MacOS/tracewing", &[]),
            Install::MacBundle("/Applications/tracewing.app".into())
        );
        assert_eq!(
            install(
                r"C:\Users\me\AppData\Local\Programs\tracewing\tracewing.exe",
                &[]
            ),
            Install::WindowsInstaller
        );
        assert!(matches!(
            install("/app/bin/tracewing", &[("FLATPAK_ID", "dev.tracewing")]),
            Install::Managed { by: "Flatpak", .. }
        ));
        assert!(matches!(
            install("/opt/homebrew/Cellar/tracewing/0.2.0/bin/tracewing", &[]),
            Install::Managed {
                by: "Homebrew",
                command: Some("brew upgrade tracewing")
            }
        ));
        assert!(matches!(
            install(
                r"C:\Users\me\scoop\apps\tracewing\current\tracewing.exe",
                &[]
            ),
            Install::Managed { by: "Scoop", .. }
        ));
        assert!(matches!(
            install("/usr/lib/tracewing/tracewing", &[]),
            Install::Managed {
                by: "the system's package manager",
                ..
            }
        ));
        assert_eq!(install("/usr/local/bin/tracewing", &[]), Install::Manual);
        assert!(Install::AppImage("/x".into()).updates_itself());
        assert!(!Install::Package(PackageTool::Apt).updates_itself());
        assert!(!Install::Manual.updates_itself());
    }

    #[test]
    fn the_tool_of_the_release_s_package_is_told_by_the_system() {
        let tool = |have: &[&str]| {
            let have: Vec<String> = have.iter().map(|s| s.to_string()).collect();
            package_tool(|f| have.iter().any(|h| h == f))
        };
        assert_eq!(
            tool(&["/var/lib/dpkg/info/tracewing.list", "/usr/bin/dnf"]),
            Some(PackageTool::Apt)
        );
        assert_eq!(tool(&["/usr/bin/zypper"]), Some(PackageTool::Zypper));
        assert_eq!(tool(&["/usr/bin/dnf"]), Some(PackageTool::Dnf));
        // dpkg is there, but has no such package: put there by hand.
        assert_eq!(tool(&["/usr/bin/dpkg"]), None);
    }
}
