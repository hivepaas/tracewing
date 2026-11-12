//! Client certificates, for servers that ask the client for one (mutual TLS):
//! a certificate and its key, from PEM files, for the hosts a pattern names.
//! tracewing shows it in the app's place, as the app's own TLS is with
//! tracewing. The files stay where they are; `client-certs.toml` says where.

use anyhow::{Context, Result};
use hudsucker::rustls::{
    crypto::CryptoProvider,
    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
    sign::CertifiedKey,
};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use crate::rules::{pattern_fits, plain_host};

const FILE: &str = "client-certs.toml";

/// One certificate, and the hosts it is shown to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientCert {
    /// A host, or `*.example.com` for it and everything under it.
    pub host: String,
    /// A PEM file with the certificate, and those that issued it after it.
    pub cert: PathBuf,
    /// A PEM file with its private key; `None` when the certificate's file
    /// holds it too.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<PathBuf>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ClientCertSettings {
    pub certs: Vec<ClientCert>,
}

impl ClientCertSettings {
    /// None, or what `client-certs.toml` in `data_dir` says.
    pub fn load(data_dir: &Path) -> Result<Self> {
        let path = data_dir.join(FILE);
        if !path.exists() {
            return Ok(Self::default());
        }
        let text =
            std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("read {}", path.display()))
    }

    pub fn save(&self, data_dir: &Path) -> Result<()> {
        let path = data_dir.join(FILE);
        std::fs::write(&path, toml::to_string(self)?)
            .with_context(|| format!("write {}", path.display()))
    }
}

/// Reads a certificate and its key, and checks that they go together.
pub fn read(cert: &ClientCert, provider: &CryptoProvider) -> Result<Arc<CertifiedKey>, String> {
    let chain: Vec<CertificateDer<'static>> = CertificateDer::pem_file_iter(&cert.cert)
        .map_err(|e| format!("{}: {e}", cert.cert.display()))?
        .collect::<Result<_, _>>()
        .map_err(|e| format!("{}: {e}", cert.cert.display()))?;
    if chain.is_empty() {
        return Err(format!("{}: no certificate in it", cert.cert.display()));
    }
    let key_path = cert.key.as_deref().unwrap_or(&cert.cert);
    let text = std::fs::read_to_string(key_path).unwrap_or_default();
    if text.contains("ENCRYPTED") {
        return Err(format!(
            "{}: the key has a passphrase, which tracewing cannot use yet. Save it without one, as `openssl pkey -in key.pem -out plain.pem` does",
            key_path.display()
        ));
    }
    let key = PrivateKeyDer::from_pem_file(key_path)
        .map_err(|e| format!("{}: no private key in it ({e})", key_path.display()))?;
    CertifiedKey::from_der(chain, key, provider)
        .map(Arc::new)
        .map_err(|e| format!("the key does not go with the certificate: {e}"))
}

struct Loaded {
    host: String,
    key: Result<Arc<CertifiedKey>, String>,
}

/// The certificates in use: read when set, so a connection only looks one up.
/// Changes apply to the next connection.
pub struct ClientCerts {
    provider: Arc<CryptoProvider>,
    loaded: RwLock<Vec<Loaded>>,
}

impl std::fmt::Debug for ClientCerts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientCerts")
            .field("count", &self.lock().len())
            .finish()
    }
}

impl Default for ClientCerts {
    fn default() -> Self {
        Self {
            provider: Arc::new(crate::crypto_provider()),
            loaded: RwLock::default(),
        }
    }
}

impl ClientCerts {
    pub fn new(settings: &ClientCertSettings) -> Self {
        let certs = Self::default();
        certs.set(settings);
        certs
    }

    /// Reads them all again, for the next connections.
    pub fn set(&self, settings: &ClientCertSettings) {
        let loaded = settings
            .certs
            .iter()
            .map(|cert| Loaded {
                host: cert.host.trim().to_ascii_lowercase(),
                key: read(cert, &self.provider),
            })
            .collect();
        *self.loaded.write().unwrap_or_else(|e| e.into_inner()) = loaded;
    }

    /// The certificate to show `host`, if one is for it: the first that fits.
    pub fn for_host(&self, host: &str) -> Option<Arc<CertifiedKey>> {
        let host = plain_host(host);
        self.lock()
            .iter()
            .filter(|loaded| pattern_fits(&loaded.host, &host))
            .find_map(|loaded| loaded.key.as_ref().ok().cloned())
    }

    /// Why each that could not be read was not, in the settings' order.
    pub fn problems(&self) -> Vec<Option<String>> {
        self.lock()
            .iter()
            .map(|loaded| loaded.key.as_ref().err().cloned())
            .collect()
    }

    fn lock(&self) -> std::sync::RwLockReadGuard<'_, Vec<Loaded>> {
        self.loaded.read().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hudsucker::rcgen::{CertificateParams, KeyPair};

    fn pem_pair(name: &str) -> (String, String) {
        let key = KeyPair::generate().unwrap();
        let cert = CertificateParams::new(vec![name.to_owned()])
            .unwrap()
            .self_signed(&key)
            .unwrap();
        (cert.pem(), key.serialize_pem())
    }

    #[test]
    fn a_certificate_is_shown_to_the_hosts_it_is_for() {
        let dir = std::env::temp_dir().join(format!("client-certs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let (cert, key) = pem_pair("me");
        std::fs::write(dir.join("me.pem"), &cert).unwrap();
        std::fs::write(dir.join("me.key"), &key).unwrap();
        std::fs::write(dir.join("both.pem"), format!("{cert}{key}")).unwrap();
        let (_, other_key) = pem_pair("other");
        std::fs::write(dir.join("other.key"), &other_key).unwrap();
        std::fs::write(
            dir.join("locked.key"),
            "-----BEGIN ENCRYPTED PRIVATE KEY-----\nAAAA\n-----END ENCRYPTED PRIVATE KEY-----\n",
        )
        .unwrap();

        let settings = ClientCertSettings {
            certs: vec![
                ClientCert {
                    host: "api.corp.dev".into(),
                    cert: dir.join("me.pem"),
                    key: Some(dir.join("me.key")),
                },
                ClientCert {
                    host: "*.Bank.dev".into(),
                    cert: dir.join("both.pem"),
                    key: None,
                },
                ClientCert {
                    host: "wrong.dev".into(),
                    cert: dir.join("me.pem"),
                    key: Some(dir.join("other.key")),
                },
                ClientCert {
                    host: "locked.dev".into(),
                    cert: dir.join("me.pem"),
                    key: Some(dir.join("locked.key")),
                },
                ClientCert {
                    host: "gone.dev".into(),
                    cert: dir.join("missing.pem"),
                    key: None,
                },
            ],
        };
        let certs = ClientCerts::new(&settings);
        assert!(certs.for_host("api.corp.dev:443").is_some());
        assert!(certs.for_host("API.corp.dev").is_some());
        assert!(certs.for_host("corp.dev").is_none());
        assert!(certs.for_host("bank.dev").is_some(), "the domain itself");
        assert!(certs.for_host("pay.eu.bank.dev").is_some());
        assert!(certs.for_host("wrong.dev").is_none());

        let problems = certs.problems();
        assert_eq!(problems[..2], [None, None]);
        assert!(problems[2].as_deref().unwrap().contains("does not go with"));
        assert!(problems[3].as_deref().unwrap().contains("passphrase"));
        assert!(problems[4].as_deref().unwrap().contains("missing.pem"));

        // Kept, and read back.
        settings.save(&dir).unwrap();
        assert_eq!(ClientCertSettings::load(&dir).unwrap(), settings);
        certs.set(&ClientCertSettings::default());
        assert!(certs.for_host("api.corp.dev").is_none());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
