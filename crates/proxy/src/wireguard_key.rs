//! Where the WireGuard keys are kept.
//!
//! Two of them, and the second is the surprising one. WireGuard has no
//! passwords: each end holds a private key and knows the other's public one.
//! So tracewing makes **both** pairs — its own, and the phone's — because the
//! alternative is asking somebody to generate a key on their phone and type
//! the public half of it in. The phone's private key goes into the
//! configuration it is given; both are kept here so that configuration can be
//! shown again, rather than once and never after.
//!
//! What holding them means: whoever has tracewing's private key can join the
//! tunnel as tracewing and read what the phone sends; whoever has the phone's
//! can pretend to be the phone. Neither is the root CA key — that one makes
//! every HTTPS site on this machine look real — but both are credentials, so
//! they go where the CA's key goes: the system's keychain, and a file
//! readable by this user only where there is no keychain, with the reason
//! said rather than swallowed. The mechanism is [`crate::ca_key`]'s
//! [`Secrets`] trait, which is why none of it is written twice here.
//!
//! The file is the fallback and not the normal case, and it is the same trade
//! `ca.key` makes: a machine with no Secret Service — a server, CI, a bare
//! container — would otherwise have no WireGuard at all.

use crate::ca_key::{KeyPlace, KeyStore, Secrets};
use crate::wireguard::{key_from, new_key};
use anyhow::{Context, Result};
use boringtun::x25519::{PublicKey, StaticSecret};
use std::{fs, path::Path};

/// The keychain entry these live under. One entry for both keys, because
/// they are useless apart: a pair that does not match is a tunnel that
/// cannot be built.
const ENTRY: &str = "WireGuard keys";

/// Both ends' private keys.
pub struct Pair {
    /// This machine's.
    pub ours: StaticSecret,
    /// The phone's, which tracewing made on its behalf and puts in the
    /// configuration it hands over.
    pub phone: StaticSecret,
}

impl Pair {
    /// A new pair, from the operating system's randomness.
    pub fn new() -> Result<Self> {
        Ok(Self {
            ours: new_key()?,
            phone: new_key()?,
        })
    }

    /// The phone's public key, which is the peer tracewing lets in.
    pub fn phone_public(&self) -> PublicKey {
        PublicKey::from(&self.phone)
    }

    /// Both keys, as two lines of base64. Named rather than numbered so that
    /// a file somebody opens says which is which, and so that a line added
    /// later does not change what the old ones mean.
    fn written(&self) -> String {
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD;
        format!(
            "ours = {}\nphone = {}\n",
            b64.encode(self.ours.as_bytes()),
            b64.encode(self.phone.as_bytes())
        )
    }

    fn read(text: &str) -> Result<Self> {
        let mut ours = None;
        let mut phone = None;
        for line in text.lines() {
            let Some((name, value)) = line.split_once('=') else {
                continue;
            };
            match name.trim() {
                "ours" => ours = Some(key_from(value)?),
                "phone" => phone = Some(key_from(value)?),
                _ => {}
            }
        }
        let ours = ours.context("no `ours` key in the WireGuard keys")?;
        let phone = phone.context("no `phone` key in the WireGuard keys")?;
        Ok(Self {
            ours: StaticSecret::from(ours),
            phone: StaticSecret::from(phone),
        })
    }
}

/// The keys, made and kept if there are none yet, and where they ended up.
///
/// `path` is the file they fall back to. It is written only when the keychain
/// cannot hold them, and [`KeyPlace::File`] carries why — a caller that shows
/// the user where a key lives has something to say either way.
pub fn load_or_make(
    path: &Path,
    store: KeyStore,
    secrets: &dyn Secrets,
) -> Result<(Pair, KeyPlace)> {
    // The keychain first whichever way `store` points: the app may have put
    // them there, and a tool reading the same data directory must find them
    // rather than make a second pair the phone has never heard of.
    if let Ok(Some(text)) = secrets.get(ENTRY) {
        return Ok((Pair::read(&text)?, KeyPlace::Keychain));
    }
    if path.exists() {
        let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        return Ok((Pair::read(&text)?, KeyPlace::File { why: None }));
    }

    let pair = Pair::new()?;
    let place = keep(&pair, path, store, secrets)?;
    Ok((pair, place))
}

/// Writes `pair` where `store` says, and says where that was.
fn keep(pair: &Pair, path: &Path, store: KeyStore, secrets: &dyn Secrets) -> Result<KeyPlace> {
    let text = pair.written();
    let why = match store {
        KeyStore::File => None,
        KeyStore::System => match secrets.set(ENTRY, &text).and_then(|()| secrets.get(ENTRY)) {
            // Read back before it is trusted: a keychain that takes a secret
            // and loses it would otherwise be found out by a phone that
            // cannot connect, much later.
            Ok(Some(back)) if back == text => return Ok(KeyPlace::Keychain),
            Ok(_) => Some("the keychain did not give the keys back".to_owned()),
            Err(e) => Some(e),
        },
    };
    write_file(path, &text)?;
    Ok(KeyPlace::File { why })
}

/// The file, readable by this user and nobody else.
fn write_file(path: &Path, text: &str) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).with_context(|| format!("make {}", dir.display()))?;
    }
    fs::write(path, text).with_context(|| format!("write {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // After the write, not before: the bytes are already there, and a
        // mode set first would be undone by a `write` that creates the file.
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .with_context(|| format!("set the mode of {}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ca_key::MemoryKeychain;

    fn dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("tracewing-wg-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn keys_are_made_once_and_found_again() {
        let dir = dir("twice");
        let path = dir.join("wireguard.key");
        let keychain = MemoryKeychain::default();

        let (first, place) = load_or_make(&path, KeyStore::System, &keychain).unwrap();
        assert_eq!(place, KeyPlace::Keychain);
        assert!(
            !path.exists(),
            "the keychain held them, so no file was written"
        );

        let (again, place) = load_or_make(&path, KeyStore::System, &keychain).unwrap();
        assert_eq!(place, KeyPlace::Keychain);
        assert_eq!(
            again.ours.as_bytes(),
            first.ours.as_bytes(),
            "the same keys, or the phone would have to be set up again"
        );
        assert_eq!(again.phone.as_bytes(), first.phone.as_bytes());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_two_keys_are_not_the_same_key() {
        let pair = Pair::new().unwrap();
        assert_ne!(
            pair.ours.as_bytes(),
            pair.phone.as_bytes(),
            "both ends holding one key is not a tunnel, it is a mirror"
        );
    }

    /// A machine with no keychain still gets a tunnel, and the file says so.
    #[test]
    fn with_no_keychain_they_go_in_a_file_and_it_says_why() {
        let dir = dir("nokeychain");
        let path = dir.join("wireguard.key");
        let broken = MemoryKeychain {
            broken: true,
            ..Default::default()
        };

        let (made, place) = load_or_make(&path, KeyStore::System, &broken).unwrap();
        match place {
            KeyPlace::File { why: Some(why) } => {
                assert!(why.contains("no keychain here"), "{why}")
            }
            other => panic!("the file, with a reason, not {other:?}"),
        }
        assert!(path.exists());

        let (again, place) = load_or_make(&path, KeyStore::System, &broken).unwrap();
        assert_eq!(place, KeyPlace::File { why: None }, "found, not made again");
        assert_eq!(again.phone.as_bytes(), made.phone.as_bytes());
        fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn the_file_is_this_users_alone() {
        use std::os::unix::fs::PermissionsExt;
        let dir = dir("mode");
        let path = dir.join("wireguard.key");
        let broken = MemoryKeychain {
            broken: true,
            ..Default::default()
        };
        load_or_make(&path, KeyStore::System, &broken).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "a private key is not for the rest of the machine"
        );
        fs::remove_dir_all(&dir).ok();
    }

    /// A tool pointed at the same data directory finds what the app kept,
    /// instead of making a second pair the phone has never heard of.
    #[test]
    fn a_tool_that_writes_files_still_reads_the_keychain() {
        let dir = dir("tool");
        let path = dir.join("wireguard.key");
        let keychain = MemoryKeychain::default();
        let (app, _) = load_or_make(&path, KeyStore::System, &keychain).unwrap();

        let (tool, place) = load_or_make(&path, KeyStore::File, &keychain).unwrap();
        assert_eq!(place, KeyPlace::Keychain);
        assert_eq!(tool.ours.as_bytes(), app.ours.as_bytes());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn what_is_written_is_what_is_read() {
        let pair = Pair::new().unwrap();
        let back = Pair::read(&pair.written()).unwrap();
        assert_eq!(back.ours.as_bytes(), pair.ours.as_bytes());
        assert_eq!(back.phone.as_bytes(), pair.phone.as_bytes());

        // Half a pair is not a pair, and is said rather than guessed at.
        let only_ours = format!("ours = {}\n", {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode(pair.ours.as_bytes())
        });
        assert!(Pair::read(&only_ours).is_err());
        assert!(Pair::read("").is_err());
        assert!(Pair::read("ours = nonsense\nphone = nonsense\n").is_err());
    }
}
