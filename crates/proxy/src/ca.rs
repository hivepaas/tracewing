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

use crate::ca_key::{self, KeyPlace, KeyStore, OsKeychain, Secrets};
use anyhow::{Context, Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use http::uri::Authority;
use hudsucker::{
    certificate_authority::CertificateAuthority,
    rcgen::{
        BasicConstraints, CertificateParams, CertificateRevocationListParams, CrlDistributionPoint,
        DistinguishedName, DnType, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyIdMethod, KeyPair,
        KeyUsagePurpose, PublicKeyData, SanType, SerialNumber,
    },
    rustls::{
        ServerConfig,
        crypto::CryptoProvider,
        pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, pem::PemObject},
        server::{ClientHello, ResolvesServerCert},
        sign::CertifiedKey,
    },
};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    net::IpAddr,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use time::{Duration, OffsetDateTime};
use tracing::{debug, warn};

pub(crate) const CA_CERT_FILE: &str = "ca.pem";
pub(crate) const CA_KEY_FILE: &str = "ca.key";
/// The certificates between a CA a company signed and the company's root,
/// sent after it: see [`crate::company_ca`].
pub(crate) const CA_CHAIN_FILE: &str = "ca-chain.pem";
/// The path the proxy answers itself, for clients that ask it directly.
pub const CRL_PATH: &str = "/tracewing.crl";

const CA_VALIDITY: Duration = Duration::days(3650);
const LEAF_VALIDITY: Duration = Duration::days(30);
/// Leaves start a day in the past so devices with a slow clock accept them.
const LEAF_BACKDATE: Duration = Duration::days(1);
/// Cached configs are dropped well before their leaf expires.
const CACHE_FOR: std::time::Duration = std::time::Duration::from_secs(7 * 24 * 60 * 60);
const CACHE_MAX: usize = 10_000;
/// The name a leaf is issued for when the host asked for cannot be one, so
/// that the client sees a certificate for another name instead of a
/// connection cut off.
const STAND_IN_HOST: &str = "unnamed.tracewing.invalid";

/// What the leaf is for. The certificate is the same either way; what differs
/// is what is offered over it, and QUIC offers less: `h3` alone, because that
/// is the only thing tracewing could read over QUIC, and TLS 1.3 alone,
/// because QUIC has no other version. The two are cached apart so that one
/// does not hand the other a config whose ALPN its client never asked for.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Over {
    /// TCP: HTTP/2 or HTTP/1.1, over any version of TLS a client still has.
    Tcp,
    /// QUIC: HTTP/3 over TLS 1.3, and nothing else.
    Quic,
}

impl Over {
    fn alpn(self) -> Vec<Vec<u8>> {
        match self {
            Self::Tcp => vec![b"h2".to_vec(), b"http/1.1".to_vec()],
            Self::Quic => vec![b"h3".to_vec()],
        }
    }
}

/// The configs issued so far, by host and by what they are offered over.
type Issued = HashMap<(Over, String), (Instant, Arc<ServerConfig>)>;

/// The leaves issued so far, by host. Apart from [`Issued`] because the two
/// are asked for in different ways and at different moments: a config per
/// host, before TLS starts, for a `CONNECT`; a leaf in the middle of a
/// handshake, for QUIC. Both are kept because making either again costs the
/// key parsing that neither should pay per connection.
type Leaves = HashMap<String, (Instant, Arc<CertifiedKey>)>;

pub struct LocalCa {
    issuer: Issuer<'static, KeyPair>,
    cert_path: PathBuf,
    leaf_key: KeyPair,
    leaf_key_der: PrivateKeyDer<'static>,
    crl_url: Option<String>,
    provider: Arc<CryptoProvider>,
    cache: Mutex<Issued>,
    leaves: Mutex<Leaves>,
    key_place: KeyPlace,
    /// Sent after each leaf, for a CA a company signed: itself, then those
    /// that lead to the company's root. Empty for tracewing's own.
    chain: Vec<CertificateDer<'static>>,
    /// Where the secrets of connections from apps are written, for Wireshark.
    key_log: Option<Arc<dyn hudsucker::rustls::KeyLog>>,
}

impl LocalCa {
    /// Loads the root CA from `dir`, creating it on first use, its key kept as
    /// `store` says.
    pub fn load_or_create(dir: &Path, provider: CryptoProvider, store: KeyStore) -> Result<Self> {
        Self::load_with(dir, provider, store, &OsKeychain)
    }

    /// The same, with the keychain given: tests give one in memory.
    pub fn load_with(
        dir: &Path,
        provider: CryptoProvider,
        store: KeyStore,
        secrets: &dyn Secrets,
    ) -> Result<Self> {
        let cert_path = dir.join(CA_CERT_FILE);
        let key_path = dir.join(CA_KEY_FILE);

        let (issuer, key_place) = if cert_path.exists() {
            let cert_pem = fs::read_to_string(&cert_path)
                .with_context(|| format!("read {}", cert_path.display()))?;
            let cert_der = CertificateDer::from_pem_slice(cert_pem.as_bytes())
                .with_context(|| format!("parse {}", cert_path.display()))?;
            // A certificate without its key is never replaced unasked: it may be
            // trusted on this machine and on phones.
            let Some((key_pem, place)) = ca_key::load(&cert_der, &key_path, store, secrets)? else {
                bail!(
                    "the key of {} is in neither the keychain nor {}. Move ca.pem away to make a \
                     new certificate, then trust that one instead",
                    cert_path.display(),
                    key_path.display()
                );
            };
            let key = KeyPair::from_pem(&key_pem).context("parse the CA's key")?;
            let issuer = Issuer::from_ca_cert_pem(&cert_pem, key)
                .with_context(|| format!("parse {}", cert_path.display()))?;
            (issuer, place)
        } else {
            fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
            create_root(&cert_path, &key_path, store, secrets)?
        };

        let leaf_key = KeyPair::generate().context("generate leaf key")?;
        let leaf_key_der = PrivatePkcs8KeyDer::from(leaf_key.serialize_der()).into();
        let chain = chain_of(dir, &cert_path)?;

        Ok(Self {
            issuer,
            cert_path,
            leaf_key,
            leaf_key_der,
            crl_url: None,
            provider: Arc::new(provider),
            cache: Mutex::new(HashMap::new()),
            leaves: Mutex::default(),
            key_place,
            chain,
            key_log: None,
        })
    }

    /// What each leaf is sent with: see `chain`.
    pub fn chain(&self) -> &[CertificateDer<'static>] {
        &self.chain
    }

    /// Where the CA's key is kept.
    pub fn key_place(&self) -> &KeyPlace {
        &self.key_place
    }

    /// The SHA-256 of the leaves' public key, in base64: what Chromium's
    /// `--ignore-certificate-errors-spki-list` takes. Every leaf shares the key,
    /// and the key is new each time the proxy starts.
    pub fn leaf_spki_sha256(&self) -> String {
        BASE64.encode(Sha256::digest(self.leaf_key.subject_public_key_info()))
    }

    /// The CA's own certificate, for a device to download and trust.
    pub fn cert_der(&self) -> Result<Vec<u8>> {
        let pem = fs::read_to_string(&self.cert_path)
            .with_context(|| format!("read {}", self.cert_path.display()))?;
        let der = CertificateDer::from_pem_slice(pem.as_bytes())
            .with_context(|| format!("parse {}", self.cert_path.display()))?;
        Ok(der.to_vec())
    }

    /// Writes the secrets of connections from apps there, while it is on.
    pub fn log_keys_to(&mut self, key_log: Arc<dyn hudsucker::rustls::KeyLog>) {
        self.key_log = Some(key_log);
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

    fn server_config(&self, host: &str, over: Over) -> Result<Arc<ServerConfig>> {
        let leaf = self.issue_leaf(host)?;
        // QUIC is TLS 1.3 and nothing else, and a config that still offers
        // 1.2 is one `quinn` refuses outright rather than negotiates around.
        let versions: &[&hudsucker::rustls::SupportedProtocolVersion] = match over {
            Over::Tcp => hudsucker::rustls::ALL_VERSIONS,
            Over::Quic => &[&hudsucker::rustls::version::TLS13],
        };
        let mut config = ServerConfig::builder_with_provider(Arc::clone(&self.provider))
            .with_protocol_versions(versions)?
            .with_no_client_auth()
            .with_single_cert(
                std::iter::once(leaf)
                    .chain(self.chain.iter().cloned())
                    .collect(),
                self.leaf_key_der.clone_key(),
            )?;
        config.alpn_protocols = over.alpn();
        if let Some(key_log) = &self.key_log {
            config.key_log = Arc::clone(key_log);
        }
        Ok(Arc::new(config))
    }
}

/// The CA, for more than one listener at once: they share its leaves.
#[derive(Clone)]
pub struct SharedCa(pub Arc<LocalCa>);

impl CertificateAuthority for SharedCa {
    async fn gen_server_config(&self, authority: &Authority) -> Arc<ServerConfig> {
        self.0.gen_server_config(authority).await
    }
}

impl CertificateAuthority for LocalCa {
    async fn gen_server_config(&self, authority: &Authority) -> Arc<ServerConfig> {
        // IPv6 authorities keep their brackets in `host()`.
        let host = authority
            .host()
            .trim_start_matches('[')
            .trim_end_matches(']');

        self.config_for(host, Over::Tcp)
    }
}

impl LocalCa {
    /// The server side of TLS for `host`, from the cache while it is fresh.
    /// A host no certificate can be issued for gets one for a stand-in name,
    /// which its client refuses as it would any wrong name; nothing panics.
    /// What reaches here is ASCII already: rustls takes only DNS names from
    /// SNI, and a `CONNECT` whose host is not ASCII is refused before.
    /// One config for a QUIC endpoint, whose certificate is chosen as each
    /// client asks.
    ///
    /// TCP tells the proxy where a connection is going before TLS starts --
    /// a `CONNECT` names the host -- so [`Self::config_for`] can hand rustls
    /// a config already holding that host's leaf. QUIC says nothing first:
    /// the name is inside the ClientHello of the Initial packet, by which
    /// time the endpoint is listening and its config is made. So the
    /// endpoint holds this one, and [`Self::certified_for`] answers per
    /// handshake. See `docs/design/http3.md`.
    pub fn quic_config(self: &Arc<Self>) -> Result<Arc<ServerConfig>> {
        let mut config = ServerConfig::builder_with_provider(Arc::clone(&self.provider))
            // QUIC is TLS 1.3 alone, and `quinn` refuses a config that still
            // offers 1.2 rather than negotiating around it.
            .with_protocol_versions(&[&hudsucker::rustls::version::TLS13])?
            .with_no_client_auth()
            .with_cert_resolver(Arc::new(MintsAsAsked(Arc::clone(self))));
        config.alpn_protocols = Over::Quic.alpn();
        // 1-RTT only, on purpose: early data a proxy replays is a request
        // sent twice, and nothing here is ready to answer for that.
        config.max_early_data_size = 0;
        if let Some(key_log) = &self.key_log {
            config.key_log = Arc::clone(key_log);
        }
        Ok(Arc::new(config))
    }

    /// The leaf for `host` and its key, from the cache while it is fresh.
    /// A host no certificate can be issued for gets the stand-in, as
    /// [`Self::config_for`] does and for the same reason: a client refusing a
    /// certificate for the wrong name is an answer, and a handshake cut off
    /// without one is not.
    pub fn certified_for(&self, host: &str) -> Arc<CertifiedKey> {
        if let Some((issued, leaf)) = self.leaves.lock().unwrap().get(host)
            && issued.elapsed() < CACHE_FOR
        {
            return Arc::clone(leaf);
        }
        debug!(host, "issuing leaf certificate for a handshake");
        let made = self.certified(host).or_else(|e| {
            warn!(
                host,
                "no certificate for this host, a stand-in instead: {e:#}"
            );
            self.certified(STAND_IN_HOST)
        });
        let leaf = Arc::new(made.expect("a certificate for a fixed, valid name is always issued"));
        let mut leaves = self.leaves.lock().unwrap();
        if leaves.len() >= CACHE_MAX {
            leaves.clear();
        }
        leaves.insert(host.to_owned(), (Instant::now(), Arc::clone(&leaf)));
        leaf
    }

    fn certified(&self, host: &str) -> Result<CertifiedKey> {
        let leaf = self.issue_leaf(host)?;
        let chain = std::iter::once(leaf)
            .chain(self.chain.iter().cloned())
            .collect();
        Ok(CertifiedKey::from_der(
            chain,
            self.leaf_key_der.clone_key(),
            &self.provider,
        )?)
    }

    pub fn config_for(&self, host: &str, over: Over) -> Arc<ServerConfig> {
        let key = (over, host.to_owned());
        if let Some((issued, config)) = self.cache.lock().unwrap().get(&key)
            && issued.elapsed() < CACHE_FOR
        {
            return Arc::clone(config);
        }

        debug!(host, ?over, "issuing leaf certificate");
        let config = match self.server_config(host, over) {
            Ok(config) => config,
            Err(e) => {
                warn!(
                    host,
                    "no certificate for this host, a stand-in instead: {e:#}"
                );
                // Not kept: the next try may do better, and the cache holds
                // only what is right for its host.
                return self
                    .server_config(STAND_IN_HOST, over)
                    .expect("a certificate for a fixed, valid name is always issued");
            }
        };

        let mut cache = self.cache.lock().unwrap();
        if cache.len() >= CACHE_MAX {
            cache.clear();
        }
        cache.insert(key, (Instant::now(), Arc::clone(&config)));
        config
    }
}

/// The certificate for whichever name a client asked for, minted as it asks.
/// A client that sent no name at all gets the stand-in rather than nothing:
/// see [`LocalCa::certified_for`].
struct MintsAsAsked(Arc<LocalCa>);

// `ResolvesServerCert` asks for `Debug`, and a CA prints as itself and no
// more: its key is in here.
impl std::fmt::Debug for MintsAsAsked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the local CA, minting as asked")
    }
}

impl ResolvesServerCert for MintsAsAsked {
    fn resolve(&self, hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        Some(
            self.0
                .certified_for(hello.server_name().unwrap_or(STAND_IN_HOST)),
        )
    }
}

/// The chain a CA a company signed is sent with; none for one that signed
/// itself.
fn chain_of(dir: &Path, cert_path: &Path) -> Result<Vec<CertificateDer<'static>>> {
    let pem =
        fs::read_to_string(cert_path).with_context(|| format!("read {}", cert_path.display()))?;
    let own = CertificateDer::from_pem_slice(pem.as_bytes())
        .with_context(|| format!("parse {}", cert_path.display()))?;
    if crate::company_ca::signed_itself(&own) {
        return Ok(Vec::new());
    }
    let mut chain = vec![own];
    let more = dir.join(CA_CHAIN_FILE);
    if more.exists() {
        for cert in CertificateDer::pem_file_iter(&more)
            .with_context(|| format!("read {}", more.display()))?
        {
            chain.push(cert.with_context(|| format!("parse {}", more.display()))?);
        }
    }
    Ok(chain)
}

fn create_root(
    cert_path: &Path,
    key_path: &Path,
    store: KeyStore,
    secrets: &dyn Secrets,
) -> Result<(Issuer<'static, KeyPair>, KeyPlace)> {
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

    // The key first: whoever sees the certificate can then find it.
    let place = ca_key::save(cert.der(), &key.serialize_pem(), key_path, store, secrets)?;
    write_whole(cert_path, cert.pem().as_bytes())?;

    Ok((Issuer::new(params, key), place))
}

/// Writes a file that is either not there or whole: written beside it,
/// then moved in place. A browser or a test that sees the certificate as it is
/// made would otherwise read it half written, or empty, and trust nothing.
pub(crate) fn write_whole(path: &Path, contents: &[u8]) -> Result<()> {
    let mut part = path.as_os_str().to_owned();
    part.push(".part");
    let part = PathBuf::from(part);
    fs::write(&part, contents).with_context(|| format!("write {}", part.display()))?;
    fs::rename(&part, path).with_context(|| format!("move {} in place", path.display()))
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

pub(crate) fn hostname() -> String {
    fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("COMPUTERNAME").ok())
        .unwrap_or_else(|| "local".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_written_whole_leaves_nothing_beside_it() {
        let dir = std::env::temp_dir().join(format!("tracewing-ca-whole-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ca.pem");
        write_whole(&path, b"whole").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"whole");
        let names: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["ca.pem"]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_host_no_certificate_can_be_issued_for_gets_a_stand_in() {
        let dir =
            std::env::temp_dir().join(format!("tracewing-ca-stand-in-{}", std::process::id()));
        let ca = LocalCa::load_or_create(&dir, crate::crypto_provider(), KeyStore::File).unwrap();
        // A DNS name in a certificate is ASCII: punycode, for a name that is not.
        assert!(ca.issue_leaf("bücher.example").is_err());
        assert!(ca.issue_leaf("xn--bcher-kva.example").is_ok());
        let config = ca.config_for("bücher.example", Over::Tcp);
        assert!(!config.alpn_protocols.is_empty(), "a config all the same");
        assert!(
            !ca.cache
                .lock()
                .unwrap()
                .contains_key(&(Over::Tcp, "bücher.example".to_owned())),
            "not kept"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    /// What a leaf is offered over, which is less for QUIC: `h3` alone, and
    /// TLS 1.3 alone, because QUIC has neither another protocol tracewing
    /// could read nor another version of TLS. The certificate itself is the
    /// same one, and the two are kept apart in the cache -- without that, a
    /// client that asked over TCP could be handed `h3` and refuse it.
    #[test]
    fn a_leaf_for_quic_offers_http3_and_tls13_alone() {
        let dir = std::env::temp_dir().join(format!("tracewing-ca-quic-{}", std::process::id()));
        let ca = LocalCa::load_or_create(&dir, crate::crypto_provider(), KeyStore::File).unwrap();

        let tcp = ca.config_for("api.example.com", Over::Tcp);
        assert_eq!(tcp.alpn_protocols, [b"h2".to_vec(), b"http/1.1".to_vec()]);
        let quic = ca.config_for("api.example.com", Over::Quic);
        assert_eq!(quic.alpn_protocols, [b"h3".to_vec()]);
        assert!(
            !Arc::ptr_eq(&tcp, &quic),
            "one host, two answers: the TCP config is not handed to QUIC"
        );
        // Both are issued by `issue_leaf`, so a host is one certificate
        // however it is reached; what that leaf says is asserted on its own
        // in `a_leaf_lasts_thirty_days_and_names_its_host_as_it_is`, because
        // a `ServerConfig` does not hand its certificate back out.

        // Each from the cache the second time, and each its own entry.
        assert!(Arc::ptr_eq(
            &quic,
            &ca.config_for("api.example.com", Over::Quic)
        ));
        assert!(Arc::ptr_eq(
            &tcp,
            &ca.config_for("api.example.com", Over::Tcp)
        ));
        assert_eq!(ca.cache.lock().unwrap().len(), 2);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_leaf_lasts_thirty_days_and_names_its_host_as_it_is() {
        use x509_parser::extensions::GeneralName;
        let dir = std::env::temp_dir().join(format!("tracewing-ca-leaf-{}", std::process::id()));
        let ca = LocalCa::load_or_create(&dir, crate::crypto_provider(), KeyStore::File).unwrap();
        for (host, ip) in [
            ("api.example.com", false),
            ("10.0.0.1", true),
            ("::1", true),
        ] {
            let leaf = ca.issue_leaf(host).unwrap();
            let (_, cert) = x509_parser::parse_x509_certificate(&leaf).unwrap();
            let validity = cert.validity();
            let days = (validity.not_after.timestamp() - validity.not_before.timestamp()) / 86_400;
            assert_eq!(days, 31, "30 days on, from a day back: {host}");
            let names = cert.subject_alternative_name().unwrap().unwrap();
            let name = &names.value.general_names[0];
            match (name, ip) {
                (GeneralName::IPAddress(bytes), true) => {
                    assert_eq!(
                        bytes.len(),
                        if host.contains(':') { 16 } else { 4 },
                        "{host}"
                    )
                }
                (GeneralName::DNSName(dns), false) => assert_eq!(*dns, host),
                other => panic!("{host}: {other:?}"),
            }
        }
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_host_that_is_not_ascii_does_not_get_this_far() {
        // What hudsucker issues for comes from SNI, which rustls checks, or
        // from a CONNECT's authority, which does not parse.
        assert!(Authority::try_from("bücher.example").is_err());
        assert!(Authority::try_from("bücher.example:443").is_err());
        assert!(hudsucker::rustls::pki_types::ServerName::try_from("bücher.example").is_err());
        assert!(Authority::try_from("xn--bcher-kva.example:443").is_ok());
    }

    #[test]
    fn the_spki_hash_is_that_of_the_leaves() {
        let dir = std::env::temp_dir().join(format!("tracewing-ca-spki-{}", std::process::id()));
        let ca = LocalCa::load_or_create(&dir, crate::crypto_provider(), KeyStore::File).unwrap();
        for host in ["example.com", "10.0.0.1"] {
            let leaf = ca.issue_leaf(host).unwrap();
            let (_, cert) = x509_parser::parse_x509_certificate(&leaf).unwrap();
            let spki = cert.tbs_certificate.subject_pki.raw;
            assert_eq!(ca.leaf_spki_sha256(), BASE64.encode(Sha256::digest(spki)));
        }
        let again =
            LocalCa::load_or_create(&dir, crate::crypto_provider(), KeyStore::File).unwrap();
        assert_ne!(
            again.leaf_spki_sha256(),
            ca.leaf_spki_sha256(),
            "a new start, a new leaf key"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_key_lives_in_the_keychain_and_the_ca_comes_back_whole() {
        let dir = std::env::temp_dir().join(format!("tracewing-ca-chain-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let chain = crate::ca_key::MemoryKeychain::default();
        let made =
            LocalCa::load_with(&dir, crate::crypto_provider(), KeyStore::System, &chain).unwrap();
        assert_eq!(made.key_place(), &KeyPlace::Keychain);
        assert!(!dir.join(CA_KEY_FILE).exists(), "no key on disk");
        let cert = fs::read(dir.join(CA_CERT_FILE)).unwrap();

        let again =
            LocalCa::load_with(&dir, crate::crypto_provider(), KeyStore::System, &chain).unwrap();
        assert_eq!(
            fs::read(dir.join(CA_CERT_FILE)).unwrap(),
            cert,
            "the same CA"
        );
        let leaf = again.issue_leaf("example.com").unwrap();
        let (_, parsed) = x509_parser::parse_x509_certificate(&leaf).unwrap();
        let (_, root) = x509_parser::pem::parse_x509_pem(&cert).unwrap();
        let root = root.parse_x509().unwrap();
        assert!(
            parsed.verify_signature(Some(root.public_key())).is_ok(),
            "signed by the key the keychain gave back"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_certificate_whose_key_is_gone_is_not_replaced() {
        let dir = std::env::temp_dir().join(format!("tracewing-ca-lost-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let chain = crate::ca_key::MemoryKeychain::default();
        LocalCa::load_with(&dir, crate::crypto_provider(), KeyStore::System, &chain).unwrap();
        let cert = fs::read(dir.join(CA_CERT_FILE)).unwrap();

        let empty = crate::ca_key::MemoryKeychain::default();
        let error = LocalCa::load_with(&dir, crate::crypto_provider(), KeyStore::System, &empty)
            .err()
            .expect("no key, no CA");
        assert!(
            format!("{error:#}").contains("neither the keychain nor"),
            "{error:#}"
        );
        assert_eq!(
            fs::read(dir.join(CA_CERT_FILE)).unwrap(),
            cert,
            "left as it was"
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
