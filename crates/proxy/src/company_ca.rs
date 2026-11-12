//! tracewing's CA signed by a company's own authority, for machines where IT
//! lets no new root in. tracewing makes a certificate signing request (CSR)
//! for the key its CA already has; IT signs it as an intermediate CA, under a
//! root the company's machines trust already; tracewing then uses what comes
//! back in place of its own certificate, and sends it, with what leads to the
//! root, after each leaf. Nothing new is installed anywhere.
//!
//! The certificate it had is kept, as `ca-own.pem`, to go back to. The key is
//! the same throughout, kept where it was: in the keychain, under an entry
//! for each certificate, or in `ca.key`.

use crate::{
    ca::{CA_CERT_FILE, CA_CHAIN_FILE, CA_KEY_FILE, hostname, write_whole},
    ca_key::{self, KeyStore, OsKeychain, Secrets},
};
use anyhow::{Context, Result, bail};
use hudsucker::{
    rcgen::{
        BasicConstraints, CertificateParams, DistinguishedName, DnType, IsCa, KeyPair,
        KeyUsagePurpose, PublicKeyData,
    },
    rustls::pki_types::{CertificateDer, pem::PemObject},
};
use std::{fs, path::Path};
use x509_parser::prelude::{FromDer, X509Certificate};

/// tracewing's own certificate, kept while a company's is used.
const OWN_FILE: &str = "ca-own.pem";

/// Which CA tracewing uses, in words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaInfo {
    pub subject: String,
    pub issuer: String,
    /// `YYYY-MM-DD`.
    pub not_after: String,
    /// Signed by a company's authority, not by itself.
    pub company: bool,
    /// tracewing's own is kept, to go back to.
    pub can_go_back: bool,
}

/// Whether a certificate signed itself: tracewing's own CA does.
pub fn signed_itself(der: &CertificateDer<'_>) -> bool {
    X509Certificate::from_der(der)
        .map(|(_, cert)| cert.subject().as_raw() == cert.issuer().as_raw())
        .unwrap_or(true)
}

fn read_cert(path: &Path) -> Result<CertificateDer<'static>> {
    let pem = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    CertificateDer::from_pem_slice(pem.as_bytes())
        .with_context(|| format!("parse {}", path.display()))
}

/// The CA in `data_dir`, in words.
pub fn info(data_dir: &Path) -> Result<CaInfo> {
    let der = read_cert(&data_dir.join(CA_CERT_FILE))?;
    let (_, cert) = X509Certificate::from_der(&der).context("read the CA's certificate")?;
    Ok(CaInfo {
        subject: name(cert.subject()),
        issuer: name(cert.issuer()),
        not_after: day(&cert),
        company: !signed_itself(&der),
        can_go_back: data_dir.join(OWN_FILE).exists(),
    })
}

/// A name by its common name, or whole.
fn name(name: &x509_parser::x509::X509Name<'_>) -> String {
    name.iter_common_name()
        .next()
        .and_then(|cn| cn.as_str().ok())
        .map_or_else(|| name.to_string(), str::to_owned)
}

fn day(cert: &X509Certificate<'_>) -> String {
    let at = cert.validity().not_after.to_datetime();
    format!(
        "{:04}-{:02}-{:02}",
        at.year(),
        u8::from(at.month()),
        at.day()
    )
}

/// The CA's key, as its certificate now finds it.
fn key(data_dir: &Path, store: KeyStore, secrets: &dyn Secrets) -> Result<(KeyPair, String)> {
    let der = read_cert(&data_dir.join(CA_CERT_FILE))?;
    let (pem, _) = ca_key::load(&der, &data_dir.join(CA_KEY_FILE), store, secrets)?
        .context("the CA's key is in neither the keychain nor its file")?;
    let key = KeyPair::from_pem(&pem).context("read the CA's key")?;
    Ok((key, pem))
}

/// A certificate signing request for the CA's key, as an intermediate CA
/// that signs servers' certificates and no other CA's, in PEM.
pub fn request(data_dir: &Path, store: KeyStore) -> Result<String> {
    request_with(data_dir, store, &OsKeychain)
}

pub fn request_with(data_dir: &Path, store: KeyStore, secrets: &dyn Secrets) -> Result<String> {
    let (key, _) = key(data_dir, store, secrets)?;
    let mut params = CertificateParams::default();
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
    Ok(params.serialize_request(&key)?.pem()?)
}

/// Uses what IT sent back, a PEM file with the signed certificate first and
/// any that lead to the root after it, in place of the CA's certificate. It
/// must be for the CA's key, a CA, and good today. From the next start.
pub fn use_signed(data_dir: &Path, pem: &str, store: KeyStore) -> Result<CaInfo> {
    use_signed_with(data_dir, pem, store, &OsKeychain)
}

pub fn use_signed_with(
    data_dir: &Path,
    pem: &str,
    store: KeyStore,
    secrets: &dyn Secrets,
) -> Result<CaInfo> {
    let certs: Vec<CertificateDer<'static>> = CertificateDer::pem_slice_iter(pem.as_bytes())
        .collect::<Result<_, _>>()
        .context("this is not a PEM file of certificates")?;
    let Some((signed, rest)) = certs.split_first() else {
        bail!("there is no certificate in it");
    };
    let (key, key_pem) = key(data_dir, store, secrets)?;
    let (_, cert) = X509Certificate::from_der(signed).context("read the certificate")?;
    if cert.public_key().raw != key.subject_public_key_info().as_slice() {
        bail!(
            "this certificate is not for tracewing's key: it may answer another request, or \
             another computer's"
        );
    }
    if !cert
        .basic_constraints()
        .ok()
        .flatten()
        .is_some_and(|b| b.value.ca)
    {
        bail!("it was not signed as a CA: ask for an intermediate CA certificate");
    }
    if !cert.validity().is_valid() {
        bail!("it is not valid today: it runs until {}", day(&cert));
    }
    if signed_itself(signed) {
        bail!("it is signed by itself, not by the company's authority");
    }
    let cert_path = data_dir.join(CA_CERT_FILE);
    // tracewing's own, the first time, to go back to.
    let own = data_dir.join(OWN_FILE);
    if !own.exists() && signed_itself(&read_cert(&cert_path)?) {
        fs::copy(&cert_path, &own).with_context(|| format!("keep {}", own.display()))?;
    }
    // The key, where the new certificate will look for it.
    if !data_dir.join(CA_KEY_FILE).exists() {
        ca_key::save(
            signed,
            &key_pem,
            &data_dir.join(CA_KEY_FILE),
            store,
            secrets,
        )?;
    }
    write_whole(&cert_path, pem_of(std::slice::from_ref(signed)).as_bytes())?;
    let chain = data_dir.join(CA_CHAIN_FILE);
    if rest.is_empty() {
        let _ = fs::remove_file(&chain);
    } else {
        write_whole(&chain, pem_of(rest).as_bytes())?;
    }
    info(data_dir)
}

/// Goes back to tracewing's own CA, from the next start.
pub fn use_own(data_dir: &Path) -> Result<CaInfo> {
    let own = data_dir.join(OWN_FILE);
    if !own.exists() {
        bail!("tracewing's own certificate was not kept");
    }
    let text = fs::read(&own).with_context(|| format!("read {}", own.display()))?;
    write_whole(&data_dir.join(CA_CERT_FILE), &text)?;
    let _ = fs::remove_file(data_dir.join(CA_CHAIN_FILE));
    fs::remove_file(&own).with_context(|| format!("remove {}", own.display()))?;
    info(data_dir)
}

fn pem_of(certs: &[CertificateDer<'_>]) -> String {
    use base64::Engine as _;
    let mut out = String::new();
    for cert in certs {
        out.push_str("-----BEGIN CERTIFICATE-----\n");
        let text = base64::engine::general_purpose::STANDARD.encode(cert.as_ref());
        for line in text.as_bytes().chunks(64) {
            out.push_str(std::str::from_utf8(line).unwrap_or_default());
            out.push('\n');
        }
        out.push_str("-----END CERTIFICATE-----\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ca::LocalCa, ca_key::MemoryKeychain};
    use hudsucker::{
        rcgen::{CertificateSigningRequestParams, Issuer},
        rustls::{
            RootCertStore,
            client::{WebPkiServerVerifier, danger::ServerCertVerifier},
            pki_types::{ServerName, UnixTime},
        },
    };
    use std::sync::Arc;

    /// A company's root, and an intermediate under it that signs requests.
    struct Company {
        root: CertificateDer<'static>,
        intermediate_pem: String,
        signer: Issuer<'static, KeyPair>,
    }

    fn company() -> Company {
        let root_key = KeyPair::generate().unwrap();
        let mut root = CertificateParams::default();
        root.distinguished_name
            .push(DnType::CommonName, "Example Corp Root");
        root.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        root.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        let root_cert = root.self_signed(&root_key).unwrap();
        let root_issuer = Issuer::new(root, root_key);
        let key = KeyPair::generate().unwrap();
        let mut middle = CertificateParams::default();
        middle
            .distinguished_name
            .push(DnType::CommonName, "Example Corp Issuing CA");
        middle.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        middle.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        let middle_cert = middle.signed_by(&key, &root_issuer).unwrap();
        Company {
            root: root_cert.der().clone(),
            intermediate_pem: middle_cert.pem(),
            signer: Issuer::new(middle, key),
        }
    }

    fn dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("company-ca-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Whether a machine that trusts only the company's root takes a leaf
    /// for `host` as `ca` sends it.
    fn company_machine_takes(ca: &LocalCa, root: &CertificateDer<'static>, host: &str) -> bool {
        let mut roots = RootCertStore::empty();
        roots.add(root.clone()).unwrap();
        let verifier = WebPkiServerVerifier::builder_with_provider(
            Arc::new(roots),
            Arc::new(crate::crypto_provider()),
        )
        .build()
        .unwrap();
        let leaf = ca.issue_leaf(host).unwrap();
        verifier
            .verify_server_cert(
                &leaf,
                ca.chain(),
                &ServerName::try_from(host.to_owned()).unwrap(),
                &[],
                UnixTime::now(),
            )
            .is_ok()
    }

    #[test]
    fn a_ca_the_company_signs_is_trusted_where_its_root_is() {
        for store in [KeyStore::System, KeyStore::File] {
            let dir = dir(&format!("{store:?}"));
            let keychain = MemoryKeychain::default();
            let provider = crate::crypto_provider;
            let own = LocalCa::load_with(&dir, provider(), store, &keychain).unwrap();
            let corp = company();
            assert!(!company_machine_takes(&own, &corp.root, "api.example.com"));
            assert!(!info(&dir).unwrap().company);

            // The request, as IT gets it, signed as IT signs it.
            let csr = request_with(&dir, store, &keychain).unwrap();
            assert!(csr.starts_with("-----BEGIN CERTIFICATE REQUEST-----"));
            let asked = CertificateSigningRequestParams::from_pem(&csr).unwrap();
            assert_eq!(
                asked.params.is_ca,
                IsCa::Ca(BasicConstraints::Constrained(0))
            );
            let signed = asked.signed_by(&corp.signer).unwrap();
            let sent_back = format!("{}{}", signed.pem(), corp.intermediate_pem);

            let now = use_signed_with(&dir, &sent_back, store, &keychain).unwrap();
            assert!(now.company && now.can_go_back);
            assert_eq!(now.issuer, "Example Corp Issuing CA");
            let company_ca = LocalCa::load_with(&dir, provider(), store, &keychain).unwrap();
            assert_eq!(company_ca.chain().len(), 2, "itself, then the intermediate");
            assert!(company_machine_takes(
                &company_ca,
                &corp.root,
                "api.example.com"
            ));

            // Back to its own: nothing of the company's is sent.
            let back = use_own(&dir).unwrap();
            assert!(!back.company && !back.can_go_back);
            let own_again = LocalCa::load_with(&dir, provider(), store, &keychain).unwrap();
            assert!(own_again.chain().is_empty());
            assert_eq!(own_again.cert_der().unwrap(), own.cert_der().unwrap());
            fs::remove_dir_all(&dir).unwrap();
        }
    }

    #[test]
    fn what_cannot_be_used_is_refused_and_nothing_changes() {
        let dir = dir("refused");
        let keychain = MemoryKeychain::default();
        LocalCa::load_with(&dir, crate::crypto_provider(), KeyStore::File, &keychain).unwrap();
        let before = fs::read(dir.join(CA_CERT_FILE)).unwrap();
        let corp = company();
        let refused = |pem: &str| {
            use_signed_with(&dir, pem, KeyStore::File, &keychain)
                .unwrap_err()
                .to_string()
        };

        // For another key: another computer's request.
        let other_key = KeyPair::generate().unwrap();
        let mut other = CertificateParams::default();
        other.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
        let other_csr = other.serialize_request(&other_key).unwrap().pem().unwrap();
        let wrong_key = CertificateSigningRequestParams::from_pem(&other_csr)
            .unwrap()
            .signed_by(&corp.signer)
            .unwrap();
        assert!(refused(&wrong_key.pem()).contains("not for tracewing's key"));

        // Signed, but not as a CA.
        let csr = request_with(&dir, KeyStore::File, &keychain).unwrap();
        let mut asked = CertificateSigningRequestParams::from_pem(&csr).unwrap();
        asked.params.is_ca = IsCa::NoCa;
        let not_ca = asked.signed_by(&corp.signer).unwrap();
        assert!(refused(&not_ca.pem()).contains("not signed as a CA"));

        assert!(refused("hello").contains("no certificate"));
        assert_eq!(fs::read(dir.join(CA_CERT_FILE)).unwrap(), before);
        assert!(!dir.join(OWN_FILE).exists());
        assert!(use_own(&dir).is_err(), "nothing to go back to");
        fs::remove_dir_all(&dir).unwrap();
    }
}
