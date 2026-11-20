//! S1, trust: the CA put where this computer looks, for its user and for the
//! machine, checked the way the OS checks, used by a program that trusts only
//! what the system trusts, and taken out again by `tracewing uninstall`.
//!
//! Changes this computer's trust stores. See `tracewing_e2e`.

use tracewing_e2e::{Capture, Origin, curl_through, only_where_allowed, said, scratch, tracewing};
use tracewing_proxy::{ca::LocalCa, ca_key::KeyStore, crypto_provider};
use tracewing_system::{Scope, ca_locations, ca_remove, verify_ca};

/// Takes the CA out of every store, however the test ends.
struct Untrust(std::path::PathBuf);

impl Drop for Untrust {
    fn drop(&mut self) {
        let _ = ca_remove(&self.0);
    }
}

#[test]
fn the_authority_is_trusted_where_the_system_looks_and_taken_out_again() {
    only_where_allowed!();
    let dir = scratch("trust");
    let origin = Origin::start(&dir);
    let data = dir.join("data");
    let capture = Capture::start(&data, &origin);
    let pem = capture.ca_pem();
    let _untrust = Untrust(pem.clone());
    let url = format!("https://localhost:{}/", origin.port);
    let ca = LocalCa::load_or_create(&data, crypto_provider(), KeyStore::File).unwrap();

    // Nothing trusts it yet, so the proxy's certificate is refused.
    assert_eq!(
        ca_locations(&pem).unwrap(),
        vec![],
        "trusted before it was put anywhere"
    );
    assert!(verify_ca(&ca, crypto_provider()).is_err());
    let out = curl_through(capture.port, &url);
    assert!(
        !out.status.success(),
        "curl took a certificate nobody trusts: {}",
        said(&out)
    );

    // In, as Set up puts it: this user's stores, then the machine's.
    tracewing_system::ca_install(&pem, Scope::User).unwrap();
    tracewing_system::ca_install(&pem, Scope::Machine).unwrap();
    let places = ca_locations(&pem).unwrap();
    for scope in [Scope::User, Scope::Machine] {
        assert!(
            places.iter().any(|place| place.scope == scope),
            "not in a store for {scope:?}: {places:?}"
        );
    }
    verify_ca(&ca, crypto_provider()).expect("the OS's verifier refuses the CA it now holds");
    let out = curl_through(capture.port, &url);
    assert!(
        out.status.success(),
        "curl refused the proxy: {}",
        said(&out)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "ok", "{}", said(&out));

    // Out again, as before removing tracewing.
    let out = tracewing()
        .arg("uninstall")
        .arg("--data-dir")
        .arg(&data)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", said(&out));
    assert_eq!(ca_locations(&pem).unwrap(), vec![], "{}", said(&out));
    assert!(
        verify_ca(&ca, crypto_provider()).is_err(),
        "the OS still trusts it"
    );
    let out = curl_through(capture.port, &url);
    assert!(!out.status.success(), "curl still takes it: {}", said(&out));

    drop(capture);
    std::fs::remove_dir_all(&dir).ok();
}
