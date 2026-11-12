//! Android set up on a real device or emulator: the certificate among the
//! system's, where apps look, on Android 14 and later too. Only with a device
//! named, as `TRACEWING_ANDROID_SERIAL=emulator-5554`, and `adb` that can
//! root it (the emulator's Google APIs images can); otherwise it passes.
//!
//! `TRACEWING_ANDROID_SERIAL=emulator-5580 cargo test --release -p tracewing-system --test android_device -- --nocapture`

use std::{path::Path, process::Command};
use tracewing_proxy::{
    ca::LocalCa,
    ca_key::{KeyStore, MemoryKeychain},
};
use tracewing_system::android::{self, Trust};

/// The device's own port for the proxy, sent back here by `adb reverse`;
/// nothing listens on it here, so it takes none of the tests' ports.
const PORT: u16 = 18_090;

fn shell(adb: &Path, serial: &str, command: &str) -> String {
    let out = Command::new(adb)
        .args(["-s", serial, "shell", command])
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// Whether the app of that package, running, sees the certificate where
/// Android 14 and later read the system's.
fn app_sees(adb: &Path, serial: &str, package: &str, name: &str) -> bool {
    let pid = shell(adb, serial, &format!("pidof {package}"));
    assert!(!pid.is_empty(), "{package} runs");
    let pid = pid.split_whitespace().next().unwrap();
    shell(
        adb,
        serial,
        &format!(
            "nsenter --mount=/proc/{pid}/ns/mnt -- ls /apex/com.android.conscrypt/cacerts/{name}"
        ),
    )
    .contains(name)
}

#[test]
fn the_certificate_goes_where_the_devices_apps_look() {
    let Ok(serial) = std::env::var("TRACEWING_ANDROID_SERIAL") else {
        eprintln!("no device named: TRACEWING_ANDROID_SERIAL");
        return;
    };
    let _ = tracewing_proxy::crypto_provider().install_default();
    let adb = android::find_adb().expect("adb");
    let dir = std::env::temp_dir().join(format!("tracewing-android-device-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let ca = LocalCa::load_with(
        &dir,
        tracewing_proxy::crypto_provider(),
        KeyStore::File,
        &MemoryKeychain::default(),
    )
    .unwrap();
    let der = ca.cert_der().unwrap();
    let name = android::store_name(&der).unwrap();

    let done = android::set_up(&adb, &serial, PORT, ca.cert_path(), &der, &dir).unwrap();
    eprintln!("Android API {}: {:?}", done.sdk, done.trust);
    assert_eq!(
        done.trust,
        Trust::System,
        "rooted, it goes among the system's"
    );
    assert_eq!(
        shell(&adb, &serial, "settings get global http_proxy"),
        "127.0.0.1:18090"
    );

    if done.sdk >= 34 {
        // Root again, to look into apps' namespaces.
        let _ = Command::new(&adb).args(["-s", &serial, "root"]).output();
        let _ = Command::new(&adb)
            .args(["-s", &serial, "wait-for-device"])
            .output();
        assert!(
            app_sees(&adb, &serial, "com.android.systemui", &name),
            "an app that was running before"
        );
        // One started after: from zygote, which has it.
        let _ = shell(&adb, &serial, "am force-stop com.android.settings");
        shell(
            &adb,
            &serial,
            "am start -W -n com.android.settings/.Settings",
        );
        assert!(
            app_sees(&adb, &serial, "com.android.settings", &name),
            "an app started after"
        );
        let _ = Command::new(&adb).args(["-s", &serial, "unroot"]).output();
        let _ = Command::new(&adb)
            .args(["-s", &serial, "wait-for-device"])
            .output();
    }

    // Set up again: still there, not mounted twice.
    let again = android::set_up(&adb, &serial, PORT, ca.cert_path(), &der, &dir).unwrap();
    assert_eq!(again.trust, Trust::System);

    android::undo(&adb, &serial, PORT, &dir).unwrap();
    let proxy = shell(&adb, &serial, "settings get global http_proxy");
    assert!(
        proxy == "null" || proxy == ":0" || proxy.is_empty(),
        "{proxy}"
    );
    std::fs::remove_dir_all(&dir).ok();
}
