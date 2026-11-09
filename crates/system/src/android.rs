//! Android over ADB: an emulator, or a phone plugged in with USB debugging on.
//!
//! The proxy reaches the device through `adb reverse`, so the device's own
//! `127.0.0.1:port` is the proxy here, and neither has to be on the same
//! network. The device's proxy setting is the global `http_proxy`.
//!
//! The certificate goes in the system's store where `adb root` works and
//! Android is 13 or older, as on the emulator's Google APIs images: a tmpfs
//! over `/system/etc/security/cacerts`, which the next restart takes away.
//! Elsewhere it is copied to Downloads for the user to install from Settings,
//! as a user certificate: Chrome trusts those, apps only if they say so.
//!
//! A device left with the proxy set has no network once tracewing is gone, so
//! the devices set up are written down, and put back when tracewing closes or,
//! after a crash, at its next start.

use anyhow::{Context, Result, bail};
use md5::{Digest, Md5};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const JOURNAL_FILE: &str = "android.journal";
/// Where the certificate goes for the user to install.
const DOWNLOADS_PATH: &str = "/sdcard/Download/tracewing.crt";
const STAGED_PATH: &str = "/data/local/tmp/tracewing.crt";
const CACERTS: &str = "/system/etc/security/cacerts";
/// Android 14, which reads system certificates from an APEX instead.
const SDK_APEX_CERTS: u32 = 34;

/// `adb`, on the path or in the SDK where Android Studio puts it.
pub fn find_adb() -> Option<PathBuf> {
    let exe = if cfg!(windows) { "adb.exe" } else { "adb" };
    let on_path = std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(exe))
            .find(|path| path.is_file())
    });
    if on_path.is_some() {
        return on_path;
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    let mut sdks: Vec<PathBuf> = ["ANDROID_HOME", "ANDROID_SDK_ROOT"]
        .into_iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .collect();
    if let Some(home) = &home {
        sdks.push(home.join("Android/Sdk"));
        sdks.push(home.join("Library/Android/sdk"));
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        sdks.push(PathBuf::from(local).join("Android/Sdk"));
    }
    sdks.into_iter()
        .map(|sdk| sdk.join("platform-tools").join(exe))
        .find(|path| path.is_file())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    pub serial: String,
    /// `device` when it can be used; `unauthorized` until the prompt on it is
    /// accepted; `offline` while it starts.
    pub state: String,
    /// What it calls itself, as `Pixel 8`.
    pub model: String,
    pub emulator: bool,
}

impl Device {
    pub fn ready(&self) -> bool {
        self.state == "device"
    }
}

/// From `adb devices -l`.
pub fn parse_devices(text: &str) -> Vec<Device> {
    text.lines()
        .skip_while(|line| !line.starts_with("List of devices"))
        .skip(1)
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let serial = words.next()?.to_owned();
            let state = words.next()?.to_owned();
            let mut model = String::new();
            let mut product = String::new();
            for word in words {
                if let Some(value) = word.strip_prefix("model:") {
                    model = value.replace('_', " ");
                } else if let Some(value) = word.strip_prefix("product:") {
                    product = value.replace('_', " ");
                }
            }
            if model.is_empty() {
                model = if product.is_empty() {
                    serial.clone()
                } else {
                    product
                };
            }
            Some(Device {
                emulator: serial.starts_with("emulator-"),
                serial,
                state,
                model,
            })
        })
        .collect()
}

pub fn devices(adb: &Path) -> Result<Vec<Device>> {
    let out = run(adb, &["devices", "-l"])?;
    Ok(parse_devices(&out))
}

/// How the certificate was left on a device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trust {
    /// Among the system's, until the device restarts.
    System,
    /// In Downloads, with Settings open for the user to install it.
    Downloads,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetUp {
    /// The Android API level, as 34 for Android 14.
    pub sdk: u32,
    pub trust: Trust,
}

fn run(adb: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new(adb)
        .args(args)
        .output()
        .with_context(|| format!("run {}", adb.display()))?;
    if !out.status.success() {
        let said = String::from_utf8_lossy(&out.stderr);
        let said = if said.trim().is_empty() {
            String::from_utf8_lossy(&out.stdout)
        } else {
            said
        };
        bail!("adb {}: {}", args.join(" "), said.trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn on(adb: &Path, serial: &str, args: &[&str]) -> Result<String> {
    let mut all = vec!["-s", serial];
    all.extend_from_slice(args);
    run(adb, &all)
}

fn shell(adb: &Path, serial: &str, command: &str) -> Result<String> {
    on(adb, serial, &["shell", command])
}

/// The name Android gives a certificate in its store: OpenSSL's old subject
/// hash, the first four bytes of the subject's MD5, as a little-endian number.
pub fn store_name(cert_der: &[u8]) -> Result<String> {
    let (_, cert) = x509_parser::parse_x509_certificate(cert_der)
        .map_err(|e| anyhow::anyhow!("read the certificate: {e}"))?;
    let digest = Md5::digest(cert.subject().as_raw());
    let hash = u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]]);
    Ok(format!("{hash:08x}.0"))
}

/// Puts the system's certificates on a tmpfs, with ours among them. The mount
/// goes at the next restart, and the image underneath is never touched.
fn system_install_script(name: &str) -> String {
    format!(
        "set -e; \
         rm -rf /data/local/tmp/tracewing-cacerts; \
         mkdir -p /data/local/tmp/tracewing-cacerts; \
         cp {CACERTS}/* /data/local/tmp/tracewing-cacerts/; \
         mount -t tmpfs tmpfs {CACERTS}; \
         cp /data/local/tmp/tracewing-cacerts/* {CACERTS}/; \
         cp {STAGED_PATH} {CACERTS}/{name}; \
         chown root:root {CACERTS}/*; \
         chmod 644 {CACERTS}/*; \
         chcon u:object_r:system_file:s0 {CACERTS}/* 2>/dev/null || true; \
         rm -rf /data/local/tmp/tracewing-cacerts"
    )
}

fn is_root(adb: &Path, serial: &str) -> bool {
    shell(adb, serial, "id -u").is_ok_and(|id| id.trim() == "0")
}

/// Points the device at the proxy on `port` here, and leaves the certificate
/// trusted, or ready for the user to trust.
pub fn set_up(
    adb: &Path,
    serial: &str,
    port: u16,
    ca_pem: &Path,
    ca_der: &[u8],
    data_dir: &Path,
) -> Result<SetUp> {
    let sdk: u32 = shell(adb, serial, "getprop ro.build.version.sdk")?
        .trim()
        .parse()
        .context("read the device's Android version")?;
    let pem = ca_pem.display().to_string();

    let mut trust = Trust::Downloads;
    if sdk < SDK_APEX_CERTS {
        let was_root = is_root(adb, serial);
        // Production builds refuse, which is an answer, not a failure.
        let rooted = was_root
            || (run(adb, &["-s", serial, "root"]).is_ok()
                && on(adb, serial, &["wait-for-device"]).is_ok()
                && is_root(adb, serial));
        if rooted {
            on(adb, serial, &["push", &pem, STAGED_PATH])?;
            let name = store_name(ca_der)?;
            shell(adb, serial, &system_install_script(&name))
                .context("put the certificate among the system's")?;
            trust = Trust::System;
            if !was_root {
                // As it was found. adbd starts again, which `reverse` below needs.
                let _ = on(adb, serial, &["unroot"]);
                on(adb, serial, &["wait-for-device"])?;
            }
        }
    }
    if trust == Trust::Downloads {
        on(adb, serial, &["push", &pem, DOWNLOADS_PATH])?;
        let _ = shell(
            adb,
            serial,
            "am start -a android.settings.SECURITY_SETTINGS",
        );
    }

    let tcp = format!("tcp:{port}");
    let ours = format!("127.0.0.1:{port}");
    let now = shell(adb, serial, "settings get global http_proxy")?;
    let previous = match read_journal(data_dir)
        .into_iter()
        .find(|e| e.serial == serial)
    {
        // Set up again before it was put back: what it had before the first time.
        Some(entry) => entry.previous,
        None => proxy_setting(&now).filter(|p| *p != ours),
    };
    write_journal(
        data_dir,
        serial,
        Some(Entry {
            serial: serial.to_owned(),
            port,
            previous,
        }),
    )?;
    on(adb, serial, &["reverse", &tcp, &tcp])?;
    shell(
        adb,
        serial,
        &format!("settings put global http_proxy {ours}"),
    )?;
    Ok(SetUp { sdk, trust })
}

/// A proxy from `settings get global http_proxy`, which says `null` or `:0`
/// for none.
fn proxy_setting(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty() && text != "null" && text != ":0").then(|| text.to_owned())
}

/// Puts the device's proxy back as it was before it was set up, and stops
/// sending its traffic here. The certificate stays: it does no harm, and a
/// system one goes at the next restart anyway.
pub fn undo(adb: &Path, serial: &str, port: u16, data_dir: &Path) -> Result<()> {
    let entry = read_journal(data_dir)
        .into_iter()
        .find(|e| e.serial == serial);
    let port = entry.as_ref().map_or(port, |e| e.port);
    match entry.and_then(|e| e.previous) {
        Some(previous) => {
            shell(
                adb,
                serial,
                &format!("settings put global http_proxy {previous}"),
            )?;
        }
        None => {
            // `:0` is what Settings itself writes for none, and is noticed at
            // once; deleting the key alone takes a restart to be. Then the key
            // goes too, as it was.
            shell(adb, serial, "settings put global http_proxy :0")?;
            shell(adb, serial, "settings delete global http_proxy")?;
        }
    }
    let _ = on(
        adb,
        serial,
        &["reverse", "--remove", &format!("tcp:{port}")],
    );
    write_journal(data_dir, serial, None)
}

/// Puts back the devices a crashed run left pointed at the proxy, those that
/// are connected; the others stay written down for the next time.
pub fn recover(adb: &Path, data_dir: &Path) -> Result<usize> {
    let ready: Vec<String> = devices(adb)?
        .into_iter()
        .filter(Device::ready)
        .map(|d| d.serial)
        .collect();
    let mut put_back = 0;
    for entry in read_journal(data_dir) {
        if ready.contains(&entry.serial) && undo(adb, &entry.serial, entry.port, data_dir).is_ok() {
            put_back += 1;
        }
    }
    Ok(put_back)
}

/// A device pointed at the proxy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub serial: String,
    pub port: u16,
    /// The proxy it had before, to put back.
    pub previous: Option<String>,
}

/// The devices left pointed at the proxy. One line each: the serial, the
/// port, and the proxy before or `-`, none of which has a space.
pub fn read_journal(data_dir: &Path) -> Vec<Entry> {
    fs::read_to_string(data_dir.join(JOURNAL_FILE))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let mut words = line.split(' ');
            let serial = words.next()?.to_owned();
            let port = words.next()?.parse().ok()?;
            let previous = words.next().filter(|p| *p != "-").map(str::to_owned);
            Some(Entry {
                serial,
                port,
                previous,
            })
        })
        .collect()
}

/// Writes `entry` for `serial`, or with `None` forgets it.
fn write_journal(data_dir: &Path, serial: &str, entry: Option<Entry>) -> Result<()> {
    let mut entries = read_journal(data_dir);
    entries.retain(|e| e.serial != serial);
    entries.extend(entry);
    let path = data_dir.join(JOURNAL_FILE);
    if entries.is_empty() {
        match fs::remove_file(&path) {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e).with_context(|| format!("remove {}", path.display())),
        }
    }
    fs::create_dir_all(data_dir)?;
    let text: String = entries
        .iter()
        .map(|e| {
            let previous = e.previous.as_deref().unwrap_or("-");
            format!("{} {} {previous}\n", e.serial, e.port)
        })
        .collect();
    let tmp = path.with_extension("journal.tmp");
    fs::write(&tmp, text).with_context(|| format!("write {}", tmp.display()))?;
    fs::rename(&tmp, &path).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hudsucker::rustls::pki_types::{CertificateDer, pem::PemObject};

    #[test]
    fn devices_are_read_as_adb_lists_them() {
        let listed = "* daemon started successfully\n\
            List of devices attached\n\
            emulator-5554          device product:sdk_gphone64_x86_64 model:sdk_gphone64_x86_64 device:emu64xa transport_id:1\n\
            R58M12345AB            device usb:1-1 product:beyond1lteeea model:SM_G973F device:beyond1 transport_id:2\n\
            0123456789ABCDEF       unauthorized usb:1-2 transport_id:3\n\
            \n";
        let devices = parse_devices(listed);
        assert_eq!(devices.len(), 3);
        assert_eq!(devices[0].model, "sdk gphone64 x86 64");
        assert!(devices[0].emulator && devices[0].ready());
        assert_eq!(devices[1].model, "SM G973F");
        assert!(!devices[1].emulator);
        assert_eq!(devices[2].state, "unauthorized");
        assert_eq!(devices[2].model, "0123456789ABCDEF");
        assert!(!devices[2].ready());
        assert!(parse_devices("List of devices attached\n\n").is_empty());
    }

    #[test]
    fn a_certificate_is_named_as_openssl_names_it() {
        let pem = include_bytes!("../testdata/isrg-root-x1.pem");
        let der = CertificateDer::from_pem_slice(pem).unwrap();
        // `openssl x509 -subject_hash_old` says so.
        assert_eq!(store_name(&der).unwrap(), "6187b673.0");
    }

    #[test]
    fn the_system_store_is_covered_not_changed() {
        let script = system_install_script("6187b673.0");
        assert!(script.starts_with("set -e;"));
        assert!(script.contains("mount -t tmpfs tmpfs /system/etc/security/cacerts"));
        assert!(
            script.contains(
                "cp /data/local/tmp/tracewing.crt /system/etc/security/cacerts/6187b673.0"
            )
        );
        assert!(!script.contains("remount"));
    }

    #[test]
    fn the_journal_keeps_each_device_once_with_what_it_had() {
        let dir = std::env::temp_dir().join(format!("tracewing-android-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let entry = |serial: &str, port, previous: Option<&str>| Entry {
            serial: serial.to_owned(),
            port,
            previous: previous.map(str::to_owned),
        };
        write_journal(
            &dir,
            "emulator-5554",
            Some(entry("emulator-5554", 9090, None)),
        )
        .unwrap();
        write_journal(
            &dir,
            "R58M",
            Some(entry("R58M", 9090, Some("10.0.0.1:3128"))),
        )
        .unwrap();
        write_journal(
            &dir,
            "emulator-5554",
            Some(entry("emulator-5554", 9191, None)),
        )
        .unwrap();
        assert_eq!(
            read_journal(&dir),
            [
                entry("R58M", 9090, Some("10.0.0.1:3128")),
                entry("emulator-5554", 9191, None),
            ]
        );
        write_journal(&dir, "R58M", None).unwrap();
        write_journal(&dir, "emulator-5554", None).unwrap();
        assert!(read_journal(&dir).is_empty());
        assert!(!dir.join(JOURNAL_FILE).exists());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn no_proxy_is_said_in_two_ways() {
        assert_eq!(proxy_setting("null\n"), None);
        assert_eq!(proxy_setting(":0"), None);
        assert_eq!(proxy_setting(""), None);
        assert_eq!(
            proxy_setting("10.0.0.1:3128\n").as_deref(),
            Some("10.0.0.1:3128")
        );
    }
}
