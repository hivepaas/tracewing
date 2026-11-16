//! The one truststore tracewing writes: Java's.
//!
//! Java is the awkward case in [capturing.md](../../../docs/capturing.md). It
//! reads neither the system's proxy setting nor the proxy variables, and it
//! trusts neither the system's certificate store nor `SSL_CERT_FILE`: it
//! keeps a store of its own, in its own format, and takes the path to it on
//! the command line. So a terminal through tracewing, which is the answer for
//! `curl`, Node and Python, does nothing at all for a Java program.
//!
//! What tracewing can do is hand over a store that is already right, and the
//! flags that point a JVM at it. Nothing of the user's is touched: not the
//! JDK's `cacerts`, which needs an administrator, is shared with everything
//! else on the machine, and is replaced by the next JDK.
//!
//! ## What a JVM will and will not read, found by trying it
//!
//! Against a real JDK 21 and a real TLS server, with tracewing's authority as
//! the only thing that should have been trusted:
//!
//! | What was handed to `-Djavax.net.ssl.trustStore` | What the JVM did |
//! |---|---|
//! | nothing (the JDK's own `cacerts`) | refused: no certification path |
//! | the authority as a PEM file | **crashed before the request**, building its `SSLContext` |
//! | a PKCS#12 from `openssl pkcs12 -nokeys` | refused: *the trustAnchors parameter must be non-empty* |
//! | the same, with an empty password | refused, the same way |
//! | a PKCS#12 from `keytool -importcert` | trusted it |
//! | a PKCS#12 from [`p12_keystore`], as below | trusted it |
//!
//! Two of those are worth keeping in mind, because both fail **quietly** —
//! not with "this file is wrong" but with an empty store, which looks to
//! whoever sees it like tracewing's authority being rejected:
//!
//! - A PKCS#12 holding a certificate is not a Java truststore. The entry has
//!   to be a *trustedCertEntry*, which is an Oracle attribute
//!   (`2.16.840.1.113894.746875.1.1`) on the bag; `openssl pkcs12 -nokeys`
//!   writes the certificate without it, and the JDK then finds no anchors.
//!   [`p12_keystore`] writes it, which is why it is the dependency.
//! - **The password cannot be empty.** An empty one is accepted by the JDK
//!   and leaves the store looking empty, whether or not
//!   `trustStorePassword` is given. So [`PASSWORD`] is a real word. A
//!   truststore holds nothing secret — public certificates, which the
//!   servers themselves hand out — so the password is integrity, not
//!   secrecy, and it is the JDK's own conventional one.
//!
//! `-Djavax.net.ssl.trustStoreType=pkcs12` is not needed by a JDK of 9 or
//! later, which reads PKCS#12 by default, and is included because 8 does not.
//!
//! ## And the flag that made all of it useless
//!
//! With the store right and both proxy flags set, a JVM asking for
//! `https://localhost:10443` still refused — and not because of the store. A
//! JVM's `http.nonProxyHosts` defaults to
//! `localhost|127.*|[::1]|0.0.0.0|[::0]`, so it went straight to the server
//! and refused *that server's* certificate, which looks exactly like
//! tracewing's being rejected. [`flags`] empties it. Running the whole chain
//! — a real JVM, through a real `tracewing capture`, to a real TLS server —
//! is what found it; the store was already provably right by then.

use anyhow::{Context, Result};
use hudsucker::rustls::pki_types::{CertificateDer, pem::PemObject};
use p12_keystore::{Certificate, KeyStore, KeyStoreEntry, Pkcs12ImportPolicy};
use sha2::{Digest, Sha256};
use std::{
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
};

/// The store, in the data directory beside the authority it holds.
pub const TRUSTSTORE: &str = "java-truststore.p12";

/// What the store is written with. Not empty: an empty password leaves a
/// store the JDK reads as having nothing in it. `changeit` is the password
/// the JDK's own `cacerts` ships with, so it is the one somebody reading the
/// flags is least surprised by.
pub const PASSWORD: &str = "changeit";

/// Writes the store if what is there is not already right, and says where it
/// is.
///
/// Lazily, and checked rather than assumed: the file is read back and the
/// certificates in it compared with the ones that would be written. Written
/// once and never again, a new authority would leave the old file silently
/// wrong — and it fails inside somebody's JVM, which is the worst place to
/// debug it. Rewritten every time the authority changes, it would write a
/// file for everybody who never touches Java.
pub fn truststore(ca_pem: &Path, data_dir: &Path) -> Result<PathBuf> {
    let path = data_dir.join(TRUSTSTORE);
    let wanted = trusted_roots(ca_pem)?;
    if let Ok(already) = fs::read(&path)
        && let Ok(store) = KeyStore::from_pkcs12(&already, PASSWORD, Pkcs12ImportPolicy::Strict)
        && same_certificates(&store, &wanted)
    {
        return Ok(path);
    }
    let mut store = KeyStore::new();
    for (i, der) in wanted.iter().enumerate() {
        let cert = Certificate::from_der(der)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("read a certificate for the Java truststore")?;
        // Each entry needs a name of its own, and the name is what `keytool
        // -list` shows: tracewing's first, since that is the one somebody is
        // looking for when they check.
        let alias = if i == 0 {
            "tracewing".to_owned()
        } else {
            format!("root-{i}")
        };
        store.add_entry(&alias, KeyStoreEntry::Certificate(cert));
    }
    let written = store
        .writer(PASSWORD)
        .write()
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("write the Java truststore")?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).with_context(|| format!("make {}", dir.display()))?;
    }
    fs::write(&path, written).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

/// What the flags a JVM is given look like, with the port and the path in
/// them.
///
/// Both schemes, because a program that asks for `http://` reads
/// `http.proxy*` and nothing else, and a line that only sets the HTTPS one
/// leaves plain requests going straight out.
///
/// **And `http.nonProxyHosts` emptied**, which is the flag that is easy to
/// leave out and makes the rest useless for the commonest case. A JVM's
/// default is `localhost|127.*|[::1]|0.0.0.0|[::0]`, so a program talking to
/// a server on this computer — which is most of what anybody debugs — goes
/// straight past the proxy however the other flags are set, and the only
/// sign of it is HTTPS failing against that server's own certificate rather
/// than tracewing's. Found by running a JVM through tracewing and watching
/// nothing arrive. The terminal's `NO_PROXY=''` is the same decision for the
/// same reason.
pub fn flags(proxy: SocketAddr, truststore: &Path) -> String {
    let (host, port) = (proxy.ip(), proxy.port());
    format!(
        "-Dhttps.proxyHost={host} -Dhttps.proxyPort={port} \\\n  \
         -Dhttp.proxyHost={host} -Dhttp.proxyPort={port} \\\n  \
         -Dhttp.nonProxyHosts= \\\n  \
         -Djavax.net.ssl.trustStore={store} \\\n  \
         -Djavax.net.ssl.trustStoreType=pkcs12 \\\n  \
         -Djavax.net.ssl.trustStorePassword={PASSWORD}",
        store = truststore.display()
    )
}

/// tracewing's authority, and then this machine's own roots.
///
/// The machine's roots are in there for the same reason the terminal's bundle
/// has them: a host tracewing is told not to decrypt is answered by the real
/// server, with the real server's certificate, and a store holding only
/// tracewing's authority would fail on exactly those hosts. tracewing's comes
/// first so that `keytool -list` shows it at the top.
fn trusted_roots(ca_pem: &Path) -> Result<Vec<Vec<u8>>> {
    let pem = fs::read(ca_pem).with_context(|| format!("read {}", ca_pem.display()))?;
    let mut ders: Vec<Vec<u8>> = CertificateDer::pem_slice_iter(&pem)
        .filter_map(Result::ok)
        .map(|cert| cert.to_vec())
        .collect();
    anyhow::ensure!(!ders.is_empty(), "no certificate in {}", ca_pem.display());
    // Some roots may not load; the rest still serve.
    for cert in rustls_native_certs::load_native_certs().certs {
        ders.push(cert.to_vec());
    }
    Ok(ders)
}

/// Whether a store holds these certificates and no others, by what they are
/// rather than by what they are called.
fn same_certificates(store: &KeyStore, wanted: &[Vec<u8>]) -> bool {
    let mut there: Vec<[u8; 32]> = store
        .entries()
        .filter_map(|(_, entry)| match entry {
            KeyStoreEntry::Certificate(cert) => Some(fingerprint(cert.as_der())),
            _ => None,
        })
        .collect();
    let mut asked: Vec<[u8; 32]> = wanted.iter().map(|der| fingerprint(der)).collect();
    there.sort_unstable();
    asked.sort_unstable();
    there == asked
}

fn fingerprint(der: &[u8]) -> [u8; 32] {
    Sha256::digest(der).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("tracewing-java-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Two authorities, as certificates rather than as something generated:
    /// the same bytes every run, and no dependency a test alone would need.
    const ONE: &str = "-----BEGIN CERTIFICATE-----
MIIDGzCCAgOgAwIBAgIUEnEOluG9HQiyGcg9vI/iVWl0T98wDQYJKoZIhvcNAQEL
BQAwHTEbMBkGA1UEAwwSdHJhY2V3aW5nIHRlc3Qgb25lMB4XDTI2MTExNTE4MTgy
NloXDTM2MTExMjE4MTgyNlowHTEbMBkGA1UEAwwSdHJhY2V3aW5nIHRlc3Qgb25l
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAvbKGnmW0EMxt6+4V0bVq
l1pM+cky4XK7k2Q80kzoifHJmdgEkLg+ADoRxcb+O55IAgmGUxxpdtfK3YvrkTev
0k5O/sNOkvC4yMUFy+5LosK1yPoItCKIxvArADpM2vFT8bPGSy7y1l58Z3GjxFM8
pvPR1Wue2Gync39Fgz3aBiT5ba02ZsXtUvZkIZLL5SRLaEHn9JSnOHvBMgB9fCGv
UNUIfIJBhFlBSnoDzgE9z9QC1qkJ+2YByNYbJMNFKy89KDvWvZg35ZVU1VTsp7li
p65X0PF85GPdM4uEfdjuiJJ2pl8odn/6JXPvhC4uyeLBrCGGgYcFmmgpoXoA3+1P
uwIDAQABo1MwUTAdBgNVHQ4EFgQUQoN2AxY7+bqzUBg+Lbn0LyS53igwHwYDVR0j
BBgwFoAUQoN2AxY7+bqzUBg+Lbn0LyS53igwDwYDVR0TAQH/BAUwAwEB/zANBgkq
hkiG9w0BAQsFAAOCAQEALFeBXrChP3TV1tNgIz+DrfeM2K3bUl5jVUUlqEHCDuzq
awMhzoK6tfJOlAGxFjn60a0yz+HCfrv86tDjNdrD9pBG6Co2v+jm5YPRxuw3FM/w
ZWGfY5CJc/7wAA3cjy0evVMuImydQ2PgEgqlEWojB1XkfR2huZwO934uA141Vxmz
d7p/cuW+z3n+ZgA58u2y51TF0y8JMT7wZzUSWPtX50hbBJICsZT3nYxOlvwwTf/t
3mXkpY0nbro96pCFFOfTb0+lXFByjIn2m2RmJUrzrBchkkJdc8YyFbemq2Veoj3g
FfmgqrIn6rQ54OXkTRhpSzqUwgcUJcqBZvLzqKqshg==
-----END CERTIFICATE-----\n";
    const TWO: &str = "-----BEGIN CERTIFICATE-----
MIIDGzCCAgOgAwIBAgIUFaWtNzBjKyrrIeHTesw5OF7cZaAwDQYJKoZIhvcNAQEL
BQAwHTEbMBkGA1UEAwwSdHJhY2V3aW5nIHRlc3QgdHdvMB4XDTI2MTExNTE4MTgy
NloXDTM2MTExMjE4MTgyNlowHTEbMBkGA1UEAwwSdHJhY2V3aW5nIHRlc3QgdHdv
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAlKtJZy0OJ2gL+dZdFjXt
nWclm2l6HNEeUK21p7dcVP9qa0obTGKeiVnd/SxzT02hCSUTFFWP1jRrgzedCXGN
hE/jB6IsyCsCXNZpyi1ggLjaTTZWTvUoSYEM7YVjGwNPJKx75FX1oTk0zZEiaiXc
HyDh5IiuMfoP6TZvHtC1p4mWkLmGIKZ1WJkTgUf0hud7DhMBihg6qyw+0cEI03Ub
1ZuISlSJ9wJw52TOsN3ZQEzsN49pyo/ar5oP/aDUeIpTP0i9+8Z4gu4SGRYIvP6K
K36tvWjoZNCuZWhhmRBMReji4PEobEhtNKoX99wr7V9v6EVesF/2esuMNRpZiZwT
iwIDAQABo1MwUTAdBgNVHQ4EFgQUKnl+rKdth6uikkMQY433Km8JBW4wHwYDVR0j
BBgwFoAUKnl+rKdth6uikkMQY433Km8JBW4wDwYDVR0TAQH/BAUwAwEB/zANBgkq
hkiG9w0BAQsFAAOCAQEAkzJChu6ifJb9R1NQjJqwBc9o9iU7tS7OemS+3Q/wCHrz
VBkvFi5CmHxV/QyjyNDnMipgWL+rlW1GOo3gH996XKWmhUD+Nz/xpTwtZi9OlDhM
xXU4QSzW8ROwoVLt4e0QuXShm2vp1NeMCRf8KRWAsoJxyPxzjonsaZdydxKk6Zie
I3zI38HaEZhHe+UKtCj0vzHZ0LHN31NMiNdfwheLoVvH5wFIhoFghjIdNURElpZh
uxrzxv/ZbOoIkf9d6Ui2AEFfiIPDj8rfK5DbJ2y58C9ZnkAaMVrM7me1k/DXPDUZ
ZIltBuTHIZK52FMrC7EvLjvwCR7e8872ilECEV6p5g==
-----END CERTIFICATE-----\n";

    /// An authority to write a store from, written where one would be.
    fn authority(dir: &Path, name: &str) -> PathBuf {
        let pem = if name == "second" { TWO } else { ONE };
        let path = dir.join(format!("{name}.pem"));
        fs::write(&path, pem).unwrap();
        path
    }

    /// What `openssl pkcs12 -nokeys` gets wrong, and the whole reason this
    /// module exists: the entry has to be a trusted certificate, not just a
    /// certificate in a bag. A store without that reads as empty, and a JVM
    /// then says tracewing's authority was not trusted.
    #[test]
    fn the_store_holds_the_authority_as_a_trusted_certificate() {
        let dir = dir("trusted");
        let ca = authority(&dir, "tracewing-test");
        let path = truststore(&ca, &dir).unwrap();

        let store = KeyStore::from_pkcs12(
            &fs::read(&path).unwrap(),
            PASSWORD,
            Pkcs12ImportPolicy::Strict,
        )
        .unwrap();
        let certs: Vec<_> = store
            .entries()
            .filter_map(|(alias, entry)| match entry {
                KeyStoreEntry::Certificate(cert) => Some((alias.to_owned(), cert.clone())),
                _ => None,
            })
            .collect();
        assert!(
            certs.iter().any(|(alias, _)| alias == "tracewing"),
            "tracewing's own, under the name somebody looks for: {:?}",
            certs.iter().map(|(a, _)| a).collect::<Vec<_>>()
        );
        // Ours, by what it is rather than by its name.
        let ours = CertificateDer::pem_slice_iter(&fs::read(&ca).unwrap())
            .next()
            .unwrap()
            .unwrap()
            .to_vec();
        assert!(
            certs
                .iter()
                .any(|(_, cert)| fingerprint(cert.as_der()) == fingerprint(&ours)),
            "the authority that was asked for is in there"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// Lazily, and checked: asked twice it is not written twice, and an
    /// authority that changed is written again. Without the second half a new
    /// authority leaves a store that is silently wrong, which fails inside
    /// somebody's JVM.
    #[test]
    fn it_is_written_once_and_again_when_the_authority_changes() {
        let dir = dir("again");
        let ca = authority(&dir, "first");
        let path = truststore(&ca, &dir).unwrap();
        let first = fs::read(&path).unwrap();

        let again = truststore(&ca, &dir).unwrap();
        assert_eq!(again, path);
        assert_eq!(
            fs::read(&again).unwrap(),
            first,
            "the same authority, so the same bytes: nothing was rewritten"
        );

        // A new authority in the same file, as making a new one leaves.
        let second = authority(&dir, "second");
        fs::copy(&second, &ca).unwrap();
        truststore(&ca, &dir).unwrap();
        let after = fs::read(&path).unwrap();
        assert_ne!(
            after, first,
            "the authority changed, so the store was written again"
        );
        let store = KeyStore::from_pkcs12(&after, PASSWORD, Pkcs12ImportPolicy::Strict).unwrap();
        let new_ca = CertificateDer::pem_slice_iter(&fs::read(&second).unwrap())
            .next()
            .unwrap()
            .unwrap()
            .to_vec();
        assert!(
            store.entries().any(|(_, entry)| match entry {
                KeyStoreEntry::Certificate(cert) =>
                    fingerprint(cert.as_der()) == fingerprint(&new_ca),
                _ => false,
            }),
            "and holds the new one"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// A host tracewing is told not to decrypt is answered by the real
    /// server, so the store carries this machine's roots as well as
    /// tracewing's — the same reason the terminal's bundle does.
    #[test]
    fn the_machines_own_roots_are_in_there_too() {
        let dir = dir("roots");
        let ca = authority(&dir, "only-ours");
        let path = truststore(&ca, &dir).unwrap();
        let store = KeyStore::from_pkcs12(
            &fs::read(&path).unwrap(),
            PASSWORD,
            Pkcs12ImportPolicy::Strict,
        )
        .unwrap();
        let count = store
            .entries()
            .filter(|(_, entry)| matches!(entry, KeyStoreEntry::Certificate(_)))
            .count();
        let native = rustls_native_certs::load_native_certs().certs.len();
        assert_eq!(
            count,
            native + 1,
            "tracewing's authority and {native} of this machine's roots"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// The flags name both schemes and carry the password: a line missing any
    /// of the three leaves somebody with a JVM that cannot connect and
    /// nothing saying why.
    #[test]
    fn the_flags_say_the_port_the_path_and_the_password() {
        let flags = flags(
            "127.0.0.1:8080".parse().unwrap(),
            Path::new("/data/java-truststore.p12"),
        );
        assert!(flags.contains("-Dhttps.proxyPort=8080"), "{flags}");
        assert!(flags.contains("-Dhttp.proxyPort=8080"), "plain HTTP too");
        assert!(
            flags.contains("-Djavax.net.ssl.trustStore=/data/java-truststore.p12"),
            "{flags}"
        );
        assert!(
            flags.contains(&format!("trustStorePassword={PASSWORD}")),
            "an empty password reads as an empty store: {flags}"
        );
        assert!(
            flags.contains("-Dhttp.nonProxyHosts="),
            "without this a JVM talking to a server on this computer goes \
             straight past the proxy, which is most of what anybody debugs: {flags}"
        );
    }

    /// A PEM with nothing in it is said, rather than written out as an empty
    /// store for a JVM to refuse later.
    #[test]
    fn an_authority_that_is_not_there_is_said() {
        let dir = dir("empty");
        let empty = dir.join("nothing.pem");
        fs::write(&empty, "not a certificate\n").unwrap();
        let problem = truststore(&empty, &dir).unwrap_err().to_string();
        assert!(problem.contains("no certificate"), "{problem}");
        let _ = fs::remove_dir_all(&dir);
    }

    /// Writes a store and asks a real `keytool` what is in it. Needs a JDK,
    /// so it is `#[ignore]`d and CI only compiles it:
    ///
    /// ```sh
    /// JAVA_HOME=~/.local/jdk-21 \
    ///   cargo test -p tracewing-system --release java::tests::keytool -- --ignored
    /// ```
    ///
    /// This is the one that would have caught the `openssl pkcs12` mistake,
    /// and the only one here that asks the thing that actually has to be
    /// convinced. `trustedCertEntry` is what it prints for an entry a JVM
    /// will trust; `PrivateKeyEntry`, or no entries at all, is what the
    /// wrong store prints.
    #[test]
    #[ignore = "needs a JDK: JAVA_HOME, or keytool on the PATH"]
    fn keytool_calls_it_a_trusted_certificate_entry() {
        let dir = dir("keytool");
        let ca = authority(&dir, "tracewing-test");
        let path = truststore(&ca, &dir).unwrap();
        let keytool = std::env::var("JAVA_HOME")
            .map(|home| PathBuf::from(home).join("bin/keytool"))
            .unwrap_or_else(|_| PathBuf::from("keytool"));
        let out = std::process::Command::new(&keytool)
            .args(["-list", "-v", "-storepass", PASSWORD, "-keystore"])
            .arg(&path)
            .output()
            .unwrap_or_else(|e| panic!("run {}: {e}", keytool.display()));
        let said = String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success(), "keytool: {said}");
        assert!(
            said.contains("trustedCertEntry"),
            "a JVM will not trust what it does not call a trusted certificate entry: {said}"
        );
        assert!(said.contains("Alias name: tracewing"), "{said}");
        let _ = fs::remove_dir_all(&dir);
    }
}
