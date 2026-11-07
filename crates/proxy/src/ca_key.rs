//! Where the root CA's private key is kept.
//!
//! Whoever holds that key can make any HTTPS site look real to this machine, so
//! the app keeps it in the system's keychain: Keychain Services on macOS,
//! Credential Manager on Windows, the Secret Service (GNOME Keyring, KWallet) on
//! Linux. Where there is none, as on a server or in CI, it stays in `ca.key`,
//! readable by this user only, and says why.
//!
//! An entry is named by the certificate's fingerprint, so CAs in different data
//! directories never meet, and a key from before the keychain moves there the
//! first time it is read.

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::Path, sync::Mutex};

const SERVICE: &str = "tracewing";

/// Where new keys go, and old ones move to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyStore {
    /// `ca.key` beside the certificate. For tests and tools, which must not
    /// write to the user's keychain.
    #[default]
    File,
    /// The system's keychain, or `ca.key` where there is none.
    System,
}

/// Where the key is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyPlace {
    Keychain,
    /// In `ca.key`, and why not in the keychain, if it was meant to be.
    File {
        why: Option<String>,
    },
}

/// A keychain, by entry name. Strings, since a PEM key is one.
pub trait Secrets {
    fn get(&self, name: &str) -> Result<Option<String>, String>;
    fn set(&self, name: &str, secret: &str) -> Result<(), String>;
}

/// The system's.
pub struct OsKeychain;

impl Secrets for OsKeychain {
    fn get(&self, name: &str) -> Result<Option<String>, String> {
        let entry = keyring::Entry::new(SERVICE, name).map_err(|e| e.to_string())?;
        match entry.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    fn set(&self, name: &str, secret: &str) -> Result<(), String> {
        keyring::Entry::new(SERVICE, name)
            .and_then(|entry| entry.set_password(secret))
            .map_err(|e| e.to_string())
    }
}

/// In memory, for tests. `broken` stands for a machine with no keychain.
#[derive(Default)]
pub struct MemoryKeychain {
    pub entries: Mutex<Vec<(String, String)>>,
    pub broken: bool,
}

impl Secrets for MemoryKeychain {
    fn get(&self, name: &str) -> Result<Option<String>, String> {
        if self.broken {
            return Err("no keychain here".into());
        }
        let entries = self.entries.lock().map_err(|e| e.to_string())?;
        Ok(entries
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, s)| s.clone()))
    }

    fn set(&self, name: &str, secret: &str) -> Result<(), String> {
        if self.broken {
            return Err("no keychain here".into());
        }
        let mut entries = self.entries.lock().map_err(|e| e.to_string())?;
        entries.retain(|(n, _)| n != name);
        entries.push((name.to_owned(), secret.to_owned()));
        Ok(())
    }
}

/// The keychain entry for the key of the certificate `cert_der`.
pub fn entry_name(cert_der: &[u8]) -> String {
    let digest = Sha256::digest(cert_der);
    let hex: String = digest[..8].iter().map(|b| format!("{b:02x}")).collect();
    format!("root CA key {hex}")
}

/// The key of an existing certificate, in PEM, and where it is. A key still in
/// `ca.key` moves to the keychain when `store` says to, and the file goes once
/// the keychain gives the key back. `None` if it is nowhere.
pub fn load(
    cert_der: &[u8],
    key_path: &Path,
    store: KeyStore,
    secrets: &dyn Secrets,
) -> Result<Option<(String, KeyPlace)>> {
    let name = entry_name(cert_der);
    let read_file =
        || fs::read_to_string(key_path).with_context(|| format!("read {}", key_path.display()));
    match store {
        KeyStore::File => {
            if key_path.exists() {
                return Ok(Some((read_file()?, KeyPlace::File { why: None })));
            }
            // The app may keep it in the keychain; a tool on the same folder
            // must find it there rather than think it lost.
            Ok(secrets
                .get(&name)
                .ok()
                .flatten()
                .map(|pem| (pem, KeyPlace::Keychain)))
        }
        KeyStore::System => {
            let why = match secrets.get(&name) {
                Ok(Some(pem)) => return Ok(Some((pem, KeyPlace::Keychain))),
                Ok(None) => None,
                Err(e) => Some(e),
            };
            if !key_path.exists() {
                // Another process may have just moved it.
                return Ok(secrets
                    .get(&name)
                    .ok()
                    .flatten()
                    .map(|pem| (pem, KeyPlace::Keychain)));
            }
            let pem = read_file()?;
            if let Some(why) = why {
                return Ok(Some((pem, KeyPlace::File { why: Some(why) })));
            }
            let place = match move_to_keychain(&name, &pem, key_path, secrets) {
                Ok(()) => KeyPlace::Keychain,
                Err(why) => KeyPlace::File { why: Some(why) },
            };
            Ok(Some((pem, place)))
        }
    }
}

fn move_to_keychain(
    name: &str,
    pem: &str,
    key_path: &Path,
    secrets: &dyn Secrets,
) -> Result<(), String> {
    secrets.set(name, pem)?;
    match secrets.get(name)? {
        Some(back) if back == pem => {}
        _ => return Err("the keychain did not give the key back".into()),
    }
    fs::remove_file(key_path).map_err(|e| format!("remove {}: {e}", key_path.display()))
}

/// Keeps the key of a new certificate, before the certificate is written: who
/// sees the certificate can then find its key.
pub fn save(
    cert_der: &[u8],
    pem: &str,
    key_path: &Path,
    store: KeyStore,
    secrets: &dyn Secrets,
) -> Result<KeyPlace> {
    let why = match store {
        KeyStore::File => None,
        KeyStore::System => {
            let name = entry_name(cert_der);
            match secrets.set(&name, pem).and_then(|()| secrets.get(&name)) {
                Ok(Some(back)) if back == pem => return Ok(KeyPlace::Keychain),
                Ok(_) => Some("the keychain did not give the key back".to_owned()),
                Err(e) => Some(e),
            }
        }
    };
    write_private(key_path, pem.as_bytes())?;
    Ok(KeyPlace::File { why })
}

/// Writes a file only the current user can read.
fn write_private(path: &Path, contents: &[u8]) -> Result<()> {
    if path.exists() {
        bail!("{} is there already", path.display());
    }
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut file = options
        .open(path)
        .with_context(|| format!("create {}", path.display()))?;
    file.write_all(contents)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CERT: &[u8] = b"a certificate";
    const PEM: &str = "-----BEGIN PRIVATE KEY-----\nkey\n-----END PRIVATE KEY-----\n";

    fn dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("tracewing-key-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_new_key_goes_to_the_keychain_and_no_file() {
        let dir = dir("new");
        let key = dir.join("ca.key");
        let chain = MemoryKeychain::default();
        let place = save(CERT, PEM, &key, KeyStore::System, &chain).unwrap();
        assert_eq!(place, KeyPlace::Keychain);
        assert!(!key.exists());
        assert_eq!(
            load(CERT, &key, KeyStore::System, &chain).unwrap(),
            Some((PEM.to_owned(), KeyPlace::Keychain))
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn with_no_keychain_it_stays_in_a_file_and_says_why() {
        let dir = dir("none");
        let key = dir.join("ca.key");
        let chain = MemoryKeychain {
            broken: true,
            ..Default::default()
        };
        let place = save(CERT, PEM, &key, KeyStore::System, &chain).unwrap();
        assert_eq!(
            place,
            KeyPlace::File {
                why: Some("no keychain here".into())
            }
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&key).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        let (pem, place) = load(CERT, &key, KeyStore::System, &chain).unwrap().unwrap();
        assert_eq!(pem, PEM);
        assert!(matches!(place, KeyPlace::File { why: Some(_) }));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_key_from_before_moves_to_the_keychain() {
        let dir = dir("move");
        let key = dir.join("ca.key");
        let chain = MemoryKeychain::default();
        save(CERT, PEM, &key, KeyStore::File, &chain).unwrap();
        assert!(key.exists());
        assert_eq!(
            load(CERT, &key, KeyStore::System, &chain).unwrap(),
            Some((PEM.to_owned(), KeyPlace::Keychain))
        );
        assert!(!key.exists(), "the file goes once the keychain has it");
        assert_eq!(chain.get(&entry_name(CERT)).unwrap().as_deref(), Some(PEM));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn tools_never_write_to_the_keychain_but_find_the_key_there() {
        let dir = dir("tools");
        let key = dir.join("ca.key");
        let chain = MemoryKeychain::default();
        let place = save(CERT, PEM, &key, KeyStore::File, &chain).unwrap();
        assert_eq!(place, KeyPlace::File { why: None });
        assert!(chain.entries.lock().unwrap().is_empty());

        // The app moved it; a tool on the same folder still finds it.
        load(CERT, &key, KeyStore::System, &chain).unwrap();
        assert_eq!(
            load(CERT, &key, KeyStore::File, &chain).unwrap(),
            Some((PEM.to_owned(), KeyPlace::Keychain))
        );
        assert_eq!(
            load(b"another", &key, KeyStore::File, &chain).unwrap(),
            None
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn entries_are_named_by_the_certificate() {
        assert_ne!(entry_name(b"one"), entry_name(b"two"));
        assert!(entry_name(b"one").starts_with("root CA key "));
        assert_eq!(entry_name(b"one").len(), "root CA key ".len() + 16);
    }
}
