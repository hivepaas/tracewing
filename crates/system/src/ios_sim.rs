//! iOS simulators on this Mac: the certificate put among the root
//! certificates each trusts, with `xcrun simctl keychain … add-root-cert`. A
//! simulator goes through the Mac's own proxy setting, so that is all it
//! needs; the certificate stays until the simulator is erased.

use anyhow::{Context, Result, bail};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

/// A simulator that is running.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Simulator {
    pub udid: String,
    pub name: String,
    /// As `iOS 18.2`.
    pub runtime: String,
}

/// `xcrun`, which comes with Xcode and its command line tools. Only a Mac
/// has it.
pub fn find_xcrun() -> Option<PathBuf> {
    let path = PathBuf::from("/usr/bin/xcrun");
    (cfg!(target_os = "macos") && path.exists()).then_some(path)
}

fn run(xcrun: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new(xcrun)
        .args(args)
        .output()
        .with_context(|| format!("run {}", xcrun.display()))?;
    if !out.status.success() {
        let said = String::from_utf8_lossy(&out.stderr);
        bail!("xcrun {}: {}", args.join(" "), said.trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// `com.apple.CoreSimulator.SimRuntime.iOS-18-2` as `iOS 18.2`.
fn runtime_name(key: &str) -> String {
    let short = key.rsplit('.').next().unwrap_or(key);
    match short.split_once('-') {
        Some((os, version)) => format!("{os} {}", version.replace('-', ".")),
        None => short.to_owned(),
    }
}

/// The running simulators in what `simctl list devices booted -j` says.
pub fn parse_booted(json: &str) -> Result<Vec<Simulator>> {
    let all: serde_json::Value = serde_json::from_str(json).context("read simctl's list")?;
    let mut found = Vec::new();
    let Some(runtimes) = all.get("devices").and_then(|d| d.as_object()) else {
        return Ok(found);
    };
    for (runtime, devices) in runtimes {
        for device in devices.as_array().into_iter().flatten() {
            let text = |key: &str| device.get(key).and_then(|v| v.as_str()).unwrap_or_default();
            if text("state") != "Booted" {
                continue;
            }
            found.push(Simulator {
                udid: text("udid").to_owned(),
                name: text("name").to_owned(),
                runtime: runtime_name(runtime),
            });
        }
    }
    found.sort_by(|a, b| (&a.runtime, &a.name).cmp(&(&b.runtime, &b.name)));
    Ok(found)
}

/// The simulators running now.
pub fn booted(xcrun: &Path) -> Result<Vec<Simulator>> {
    parse_booted(&run(xcrun, &["simctl", "list", "devices", "booted", "-j"])?)
}

/// Puts the certificate among the simulator's trusted roots. Apps started
/// after trust it; one already open may need opening again.
pub fn trust(xcrun: &Path, udid: &str, ca_pem: &Path) -> Result<()> {
    let pem = ca_pem.display().to_string();
    run(xcrun, &["simctl", "keychain", udid, "add-root-cert", &pem])
        .context("put the certificate among the simulator's")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTED: &str = r#"{
      "devices" : {
        "com.apple.CoreSimulator.SimRuntime.iOS-18-2" : [
          { "udid" : "A1", "name" : "iPhone 16", "state" : "Booted", "isAvailable" : true },
          { "udid" : "A2", "name" : "iPhone 16 Pro", "state" : "Shutdown" }
        ],
        "com.apple.CoreSimulator.SimRuntime.iOS-17-5" : [
          { "udid" : "B1", "name" : "iPad Air", "state" : "Booted" }
        ],
        "com.apple.CoreSimulator.SimRuntime.watchOS-11-0" : []
      }
    }"#;

    #[test]
    fn only_the_running_simulators_are_listed() {
        assert_eq!(
            parse_booted(LISTED).unwrap(),
            [
                Simulator {
                    udid: "B1".into(),
                    name: "iPad Air".into(),
                    runtime: "iOS 17.5".into(),
                },
                Simulator {
                    udid: "A1".into(),
                    name: "iPhone 16".into(),
                    runtime: "iOS 18.2".into(),
                },
            ]
        );
        assert!(parse_booted(r#"{"devices":{}}"#).unwrap().is_empty());
        assert!(parse_booted("not json").is_err());
    }

    /// `xcrun` as a script that answers as simctl does and says what it was
    /// asked, so that the commands are checked on any Unix.
    #[cfg(unix)]
    #[test]
    fn the_commands_are_simctls() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("ios-sim-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let asked = dir.join("asked");
        let fake = dir.join("xcrun");
        std::fs::write(
            &fake,
            format!(
                "#!/bin/sh\necho \"$@\" >> {asked}\n\
                 case \"$2\" in list) cat <<'EOF'\n{LISTED}\nEOF\n;; esac\n",
                asked = asked.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(booted(&fake).unwrap().len(), 2);
        trust(&fake, "A1", Path::new("/data/ca.pem")).unwrap();
        assert_eq!(
            std::fs::read_to_string(&asked).unwrap(),
            "simctl list devices booted -j\nsimctl keychain A1 add-root-cert /data/ca.pem\n"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
