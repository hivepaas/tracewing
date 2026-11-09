//! The authorities this computer trusts are trusted for servers too. Here
//! `SSL_CERT_FILE` stands in for the system's store, as it does on every OS;
//! a process of its own, since the variable is the whole process's.

use tracewing_proxy::{ca::LocalCa, crypto_provider, upstream_roots};

#[test]
fn the_computers_authorities_are_trusted_with_the_public_ones() {
    let dir = std::env::temp_dir().join(format!("tracewing-os-roots-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let ca = LocalCa::load_or_create(&dir, crypto_provider(), Default::default()).unwrap();
    // SAFETY: the only test in this process, before any thread reads them.
    // Cargo sets both on Linux, for the tests it runs.
    unsafe {
        std::env::set_var("SSL_CERT_FILE", ca.cert_path());
        std::env::remove_var("SSL_CERT_DIR");
    }

    let public = upstream_roots(&[], false).unwrap();
    let with_computers = upstream_roots(&[], true).unwrap();
    assert_eq!(with_computers.len(), public.len() + 1);
    let ours = |roots: &hudsucker::rustls::RootCertStore| {
        roots.roots.iter().any(|anchor| {
            anchor
                .subject
                .as_ref()
                .windows(b"tracewing".len())
                .any(|w| w == b"tracewing")
        })
    };
    assert!(ours(&with_computers));
    assert!(!ours(&public));
    std::fs::remove_dir_all(&dir).ok();
}
