//! S6, the keychain: the CA's key, the WireGuard keys and a relay's password
//! go into the system's keychain — on Linux the Secret Service — rather than
//! a file, and come back, in this process and in another.
//!
//! Changes this computer's keychain. See `tracewing_e2e`.

use std::path::Path;
#[cfg(target_os = "linux")]
use std::process::Command;
use tracewing_e2e::{Capture, Origin, only_where_allowed, scratch};
use tracewing_proxy::{
    ca::LocalCa,
    ca_key::{KeyPlace, KeyStore, OsKeychain, Secrets, entry_name},
    crypto_provider,
    relay::RelaySettings,
    wireguard_key,
};

/// What the keychain holds under tracewing's name, read with the keychain's
/// own tool rather than tracewing's.
#[cfg(target_os = "linux")]
fn listed() -> String {
    let out = Command::new("secret-tool")
        .args(["search", "--all", "service", "tracewing"])
        .output()
        .expect("run secret-tool");
    // It says what it found on stderr and the secrets on stdout.
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// Takes everything of tracewing's out of the keychain, however the test ends.
struct Forget;

impl Drop for Forget {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        let _ = Command::new("secret-tool")
            .args(["clear", "service", "tracewing"])
            .status();
    }
}

fn ca_key_name(dir: &Path) -> String {
    let der = tracewing_system::ca_der(&dir.join("ca.pem")).unwrap();
    entry_name(&der)
}

#[test]
fn secrets_go_into_the_keychain_and_come_back() {
    only_where_allowed!();
    let _forget = Forget;
    let dir = scratch("keychain");

    // The CA's key, as the window keeps it.
    let data = dir.join("data");
    std::fs::create_dir_all(&data).unwrap();
    let ca = LocalCa::load_or_create(&data, crypto_provider(), KeyStore::System).unwrap();
    assert!(
        !data.join("ca.key").exists(),
        "the key went to a file, so the keychain refused it"
    );
    let name = ca_key_name(&data);
    assert!(
        OsKeychain.get(&name).unwrap().is_some(),
        "{name} is not in the keychain"
    );
    #[cfg(target_os = "linux")]
    assert!(
        listed().contains(&name),
        "secret-tool does not see {name}: {}",
        listed()
    );
    drop(ca);

    // Another process finds it there rather than making another CA: the
    // command line, with its key not in a file.
    let pem = std::fs::read(data.join("ca.pem")).unwrap();
    let origin = Origin::start(&dir);
    let capture = Capture::start_keeping_the_key_in(&data, &origin, KeyStore::System);
    assert_eq!(
        std::fs::read(capture.ca_pem()).unwrap(),
        pem,
        "the command line made a CA of its own"
    );
    assert!(!data.join("ca.key").exists());
    drop(capture);

    // The WireGuard keys: kept, then the same keys again.
    let file = dir.join("wireguard.key");
    let (pair, place) = wireguard_key::load_or_make(&file, KeyStore::System, &OsKeychain).unwrap();
    assert_eq!(place, KeyPlace::Keychain);
    assert!(!file.exists());
    let (again, _) = wireguard_key::load_or_make(&file, KeyStore::System, &OsKeychain).unwrap();
    assert_eq!(
        again.phone_public(),
        pair.phone_public(),
        "other keys the second time"
    );

    // A relay's password: out of relay.toml, and back when it is read.
    let relay = RelaySettings {
        proxy: "http://alice:s3cret-e2e@corp.example:3128".to_owned(),
        bypass: String::new(),
    };
    let place = relay
        .save_kept(&dir, KeyStore::System, &OsKeychain)
        .unwrap();
    assert_eq!(place, Some(KeyPlace::Keychain));
    let written = std::fs::read_to_string(dir.join("relay.toml")).unwrap();
    assert!(
        !written.contains("s3cret-e2e"),
        "the password stayed in the file: {written}"
    );
    let (read, _) = RelaySettings::load_kept(&dir, KeyStore::System, &OsKeychain).unwrap();
    assert_eq!(read.proxy, relay.proxy);

    std::fs::remove_dir_all(&dir).ok();
}
