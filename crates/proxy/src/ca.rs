//! The local certificate authority.
//!
//! Each install gets its own root CA, created on first run and kept in the
//! data directory. Leaf certificates are issued on the fly per host, signed by
//! that root, and cached in memory.
//!
//! Differences from hudsucker's `RcgenAuthority`:
//! - leaves are valid for 30 days, not 365 (Cronet rejects long-lived leaves
//!   with `ERR_CERT_VALIDITY_TOO_LONG`);
//! - leaves use their own key, so the CA key never signs a TLS handshake;
//! - IP hosts get an IP SAN instead of a DNS SAN;
//! - leaves name a CRL that the proxy serves itself: curl on Windows (Schannel)
//!   refuses a certificate whose revocation it has no way to check, with
//!   `CRYPT_E_NO_REVOCATION_CHECK`.

use anyhow::{Context, Result};
use http::uri::Authority;
use hudsucker::{
    certificate_authority::CertificateAuthority,
    rcgen::{
        BasicConstraints, CertificateParams, CertificateRevocationListParams, CrlDistributionPoint,
        DistinguishedName, DnType, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyIdMethod, KeyPair,
        KeyUsagePurpose, SanType, SerialNumber,
    },
    rustls::{
        ServerConfig,
        crypto::CryptoProvider,
        pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
    },
};
use std::{
    collections::HashMap,
    fs,
    io::Write,
    net::IpAddr,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use time::{Duration, OffsetDateTime};
use tracing::debug;

const CA_CERT_FILE: &str = "ca.pem";
const CA_KEY_FILE: &str = "ca.key";
/// The path the proxy answers itself, for clients that ask it directly.
pub const CRL_PATH: &str = "/tracewing.crl";

const CA_VALIDITY: Duration = Duration::days(3650);
const LEAF_VALIDITY: Duration = Duration::days(30);
/// Leaves start a day in the past so devices with a slow clock accept them.
const LEAF_BACKDATE: Duration = Duration::days(1);
/// Cached configs are dropped well before their leaf expires.
const CACHE_FOR: std::time::Duration = std::time::Duration::from_secs(7 * 24 * 60 * 60);
const CACHE_MAX: usize = 10_000;

pub struct LocalCa {
    issuer: Issuer<'static, KeyPair>,
    cert_path: PathBuf,
    leaf_key: KeyPair,
    leaf_key_der: PrivateKeyDer<'static>,
    crl_url: Option<String>,
    provider: Arc<CryptoProvider>,
    cache: Mutex<HashMap<String, (Instant, Arc<ServerConfig>)>>,
}

impl LocalCa {
    /// Loads the root CA from `dir`, creating it on first use.
    pub fn load_or_create(dir: &Path, provider: CryptoProvider) -> Result<Self> {
        let cert_path = dir.join(CA_CERT_FILE);
        let key_path = dir.join(CA_KEY_FILE);

        let issuer = if cert_path.exists() && key_path.exists() {
            let key = KeyPair::from_pem(&fs::read_to_string(&key_path)?)
                .with_context(|| format!("parse {}", key_path.display()))?;
            Issuer::from_ca_cert_pem(&fs::read_to_string(&cert_path)?, key)
                .with_context(|| format!("parse {}", cert_path.display()))?
        } else {
            fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
            create_root(&cert_path, &key_path)?
        };

        let leaf_key = KeyPair::generate().context("generate leaf key")?;
        let leaf_key_der = PrivatePkcs8KeyDer::from(leaf_key.serialize_der()).into();

        Ok(Self {
            issuer,
            cert_path,
            leaf_key,
            leaf_key_der,
            crl_url: None,
            provider: Arc::new(provider),
            cache: Mutex::new(HashMap::new()),
        })
    }

    /// Names the proxy's own CRL in every leaf from now on.
    pub fn serve_crl_at(&mut self, addr: std::net::SocketAddr) {
        self.crl_url = Some(format!("http://{addr}{CRL_PATH}"));
    }

    /// An empty CRL: nothing this CA issued is revoked.
    pub fn crl_der(&self) -> Result<Vec<u8>> {
        let now = OffsetDateTime::now_utc();
        let crl = CertificateRevocationListParams {
            this_update: now - LEAF_BACKDATE,
            next_update: now + LEAF_VALIDITY,
            crl_number: next_serial(),
            issuing_distribution_point: None,
            revoked_certs: Vec::new(),
            key_identifier_method: KeyIdMethod::Sha256,
        }
        .signed_by(&self.issuer)
        .context("sign the CRL")?;
        Ok(crl.der().to_vec())
    }

    /// The root certificate users install and trust.
    pub fn cert_path(&self) -> &Path {
        &self.cert_path
    }

    pub fn issue_leaf(&self, host: &str) -> Result<CertificateDer<'static>> {
        let now = OffsetDateTime::now_utc();
        let mut params = CertificateParams::default();
        params.serial_number = Some(next_serial());
        params.not_before = now - LEAF_BACKDATE;
        params.not_after = now + LEAF_VALIDITY;

        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, host);
        params.distinguished_name = dn;
        params.subject_alt_names.push(match host.parse::<IpAddr>() {
            Ok(ip) => SanType::IpAddress(ip),
            Err(_) => SanType::DnsName(host.try_into()?),
        });
        params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        params.use_authority_key_identifier_extension = true;
        if let Some(url) = &self.crl_url {
            params.crl_distribution_points = vec![CrlDistributionPoint {
                uris: vec![url.clone()],
            }];
        }

        Ok(params.signed_by(&self.leaf_key, &self.issuer)?.into())
    }

    fn server_config(&self, host: &str) -> Result<Arc<ServerConfig>> {
        let leaf = self.issue_leaf(host)?;
        let mut config = ServerConfig::builder_with_provider(Arc::clone(&self.provider))
            .with_safe_default_protocol_versions()?
            .with_no_client_auth()
            .with_single_cert(vec![leaf], self.leaf_key_der.clone_key())?;
        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        Ok(Arc::new(config))
    }
}

impl CertificateAuthority for LocalCa {
    async fn gen_server_config(&self, authority: &Authority) -> Arc<ServerConfig> {
        // IPv6 authorities keep their brackets in `host()`.
        let host = authority
            .host()
            .trim_start_matches('[')
            .trim_end_matches(']');

        if let Some((issued, config)) = self.cache.lock().unwrap().get(host)
            && issued.elapsed() < CACHE_FOR
        {
            return Arc::clone(config);
        }

        debug!(host, "issuing leaf certificate");
        let config = self
            .server_config(host)
            .unwrap_or_else(|e| panic!("issue certificate for {host}: {e:#}"));

        let mut cache = self.cache.lock().unwrap();
        if cache.len() >= CACHE_MAX {
            cache.clear();
        }
        cache.insert(host.to_owned(), (Instant::now(), Arc::clone(&config)));
        config
    }
}

fn create_root(cert_path: &Path, key_path: &Path) -> Result<Issuer<'static, KeyPair>> {
    let now = OffsetDateTime::now_utc();
    let mut params = CertificateParams::default();
    params.serial_number = Some(next_serial());
    params.not_before = now - Duration::days(1);
    params.not_after = now + CA_VALIDITY;

    let mut dn = DistinguishedName::new();
    dn.push(DnType::OrganizationName, "tracewing");
    dn.push(DnType::CommonName, format!("tracewing CA ({})", hostname()));
    params.distinguished_name = dn;
    params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
    params.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
        KeyUsagePurpose::DigitalSignature,
    ];

    let key = KeyPair::generate().context("generate CA key")?;
    let cert = params.self_signed(&key).context("self-sign CA")?;

    write_private(key_path, key.serialize_pem().as_bytes())?;
    fs::write(cert_path, cert.pem()).with_context(|| format!("write {}", cert_path.display()))?;

    Ok(Issuer::new(params, key))
}

/// Writes a file only the current user can read.
fn write_private(path: &Path, contents: &[u8]) -> Result<()> {
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

/// A positive serial that is unique per process and across restarts.
fn next_serial() -> SerialNumber {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&nanos.to_be_bytes());
    bytes[8..].copy_from_slice(&COUNTER.fetch_add(1, Ordering::Relaxed).to_be_bytes());
    bytes[0] &= 0x7f;
    SerialNumber::from_slice(&bytes)
}

fn hostname() -> String {
    fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("COMPUTERNAME").ok())
        .unwrap_or_else(|| "local".to_owned())
}
