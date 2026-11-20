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

/// The Chromium database every Linux has, where `ca_install(Scope::User)`
/// always puts the CA.
#[cfg(target_os = "linux")]
fn shared_nssdb() -> String {
    format!(
        "sql:{}/.pki/nssdb",
        std::env::var("HOME").expect("no HOME in this container")
    )
}

/// How many certificates carry tracewing's nickname there, as `certutil`
/// lists them: by listing, not by asking for the nickname, because believing
/// that question's answer is what went wrong.
#[cfg(target_os = "linux")]
fn under_the_nickname() -> usize {
    let out = std::process::Command::new("certutil")
        .args(["-L", "-d", &shared_nssdb()])
        .output()
        .expect("run certutil");
    assert!(out.status.success(), "certutil -L: {}", said(&out));
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|line| line.starts_with("tracewing CA "))
        .count()
}

/// Puts a certificate under tracewing's nickname the way an older tracewing
/// did, which is the only way two can be there now.
#[cfg(target_os = "linux")]
fn add_under_the_nickname(cert_pem: &std::path::Path) {
    let out = std::process::Command::new("certutil")
        .args([
            "-A",
            "-d",
            &shared_nssdb(),
            "-t",
            "C,,",
            "-n",
            "tracewing CA",
            "-i",
            &cert_pem.display().to_string(),
        ])
        .output()
        .expect("run certutil");
    assert!(out.status.success(), "certutil -A: {}", said(&out));
}

/// A tracewing CA from another data directory, in a browser's store under
/// tracewing's own nickname: it is not this CA, installing replaces it,
/// uninstalling this one leaves it alone and says it is there.
///
/// **Measured with certutil before any of this was written.** `-A` under a
/// nickname a certificate of other content already carries leaves both and
/// exits 0; the nickname then names whichever went in last; `-D` takes out
/// that one alone; and `-L -n NICKNAME` succeeds for whatever is there. So
/// tracewing said the browsers trusted a CA whose leaves they were rejecting,
/// installing again left two, and uninstall took out one of them. The same
/// shape as #376 on Windows, where this user's root store was asked through a
/// view that also showed the machine's.
///
/// Also measured, and why no loop takes certificates out: NSS keeps one copy
/// of a given certificate. The same bytes added twice under one nickname
/// leave one; only certificates of different content accumulate.
#[cfg(target_os = "linux")]
#[test]
fn a_tracewing_ca_of_another_data_directory_is_not_this_one_and_is_left_alone() {
    only_where_allowed!();
    let dir = scratch("trust-older");
    let theirs = LocalCa::load_or_create(&dir.join("theirs"), crypto_provider(), KeyStore::File)
        .expect("another data directory's CA");
    let mine = LocalCa::load_or_create(&dir.join("mine"), crypto_provider(), KeyStore::File)
        .expect("this data directory's CA");
    let pem_theirs = theirs.cert_path().to_path_buf();
    let pem_mine = mine.cert_path().to_path_buf();
    let _untrust_theirs = Untrust(pem_theirs.clone());
    let _untrust_mine = Untrust(pem_mine.clone());
    assert_ne!(
        std::fs::read(&pem_theirs).unwrap(),
        std::fs::read(&pem_mine).unwrap(),
        "two CAs of the same name have to be two certificates for any of this to mean anything"
    );
    let uninstall = |data: &str| {
        let out = tracewing()
            .arg("uninstall")
            .arg("--data-dir")
            .arg(dir.join(data))
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", said(&out));
        String::from_utf8_lossy(&out.stdout).into_owned()
    };

    // Theirs in, as that data directory's tracewing put it there.
    tracewing_system::ca_install(&pem_theirs, Scope::User).unwrap();
    assert_eq!(
        under_the_nickname(),
        1,
        "installing for this user put nothing in the database every Linux has"
    );

    // It answers to tracewing's nickname and is not this CA.
    let places = ca_locations(&pem_mine).unwrap();
    assert!(
        !places.iter().any(|place| place.scope == Scope::User),
        "a CA that is not this one was reported as this one's: {places:?}"
    );

    // Asked to take this CA out of the browsers' stores directly, and not
    // through `ca_remove`, it still takes out only this CA. `ca_remove` would
    // not have come this far -- `ca_locations` does not report this one -- so
    // without an ask of its own nothing reaches the choice made here.
    tracewing_system::ca_uninstall(&pem_mine, Scope::User)
        .expect("taking out a CA that is not there is not a failure");
    assert_eq!(
        under_the_nickname(),
        1,
        "taking out this CA took another data directory's with it"
    );

    // Uninstalling this one leaves theirs where it is, and says it is there.
    // Taking it out on the strength of the shared name was tried, and
    // `uninstall --data-dir <a throwaway folder>` took this computer's own
    // anchor file out from under the real install.
    let said_it = uninstall("mine");
    assert_eq!(
        under_the_nickname(),
        1,
        "uninstalling this CA took another data directory's out: {said_it}"
    );
    assert!(
        said_it.contains("from another data directory is still trusted"),
        "uninstall said nothing about the CA it left trusted: {said_it}"
    );

    // The pair an older tracewing left: two installs, the second not
    // replacing the first. This CA last, so the nickname names it.
    add_under_the_nickname(&pem_mine);
    assert_eq!(under_the_nickname(), 2, "certutil did not leave two");
    assert!(
        ca_locations(&pem_mine)
            .unwrap()
            .iter()
            .any(|place| place.scope == Scope::User),
        "this CA is the one the nickname names and was not reported"
    );

    // Uninstall takes this one out and stops: theirs is not ours to take.
    let said_it = uninstall("mine");
    assert_eq!(
        under_the_nickname(),
        1,
        "uninstall did not leave exactly the other data directory's CA: {said_it}"
    );
    assert_eq!(
        ca_locations(&pem_mine).unwrap(),
        vec![],
        "this CA is out of the database and was still reported"
    );
    assert!(
        said_it.contains("from another data directory is still trusted"),
        "uninstall said nothing about the CA it left trusted: {said_it}"
    );

    // Installing replaces what the nickname names rather than sitting beside
    // it: the slot holds one CA, and it is this one.
    tracewing_system::ca_install(&pem_mine, Scope::User).unwrap();
    assert_eq!(
        under_the_nickname(),
        1,
        "installing left two certificates under one nickname"
    );
    assert_eq!(
        ca_locations(&pem_theirs).unwrap(),
        vec![],
        "the other CA is out of the database and was still reported"
    );

    // And out: nothing of tracewing's left, and nothing said about an older
    // one, because there is none.
    let said_it = uninstall("mine");
    assert_eq!(
        under_the_nickname(),
        0,
        "uninstall left a CA of ours: {said_it}"
    );
    assert!(
        !said_it.contains("another data directory"),
        "uninstall spoke of an older CA with none there: {said_it}"
    );
    assert!(
        uninstall("mine").contains("no store on this computer trusts"),
        "uninstall with nothing left said the wrong thing"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// The same on Windows, where each store is asked for this CA by its bytes,
/// and another data directory's is found by the subject every tracewing CA
/// has: both in this user's store, the other in the machine's too.
#[cfg(windows)]
#[test]
fn a_tracewing_ca_of_another_data_directory_is_left_alone_and_said() {
    only_where_allowed!();
    let dir = scratch("trust-older");
    let theirs = LocalCa::load_or_create(&dir.join("theirs"), crypto_provider(), KeyStore::File)
        .expect("another data directory's CA");
    let mine = LocalCa::load_or_create(&dir.join("mine"), crypto_provider(), KeyStore::File)
        .expect("this data directory's CA");
    let pem_theirs = theirs.cert_path().to_path_buf();
    let pem_mine = mine.cert_path().to_path_buf();
    let _untrust_theirs = Untrust(pem_theirs.clone());
    let _untrust_mine = Untrust(pem_mine.clone());
    let uninstall = |data: &str| {
        let out = tracewing()
            .arg("uninstall")
            .arg("--data-dir")
            .arg(dir.join(data))
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", said(&out));
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    let scopes = |pem: &std::path::Path| -> Vec<Scope> {
        ca_locations(pem)
            .unwrap()
            .into_iter()
            .map(|place| place.scope)
            .collect()
    };

    tracewing_system::ca_install(&pem_theirs, Scope::User).unwrap();
    tracewing_system::ca_install(&pem_theirs, Scope::Machine).unwrap();
    tracewing_system::ca_install(&pem_mine, Scope::User).unwrap();
    assert_eq!(scopes(&pem_mine), vec![Scope::User]);

    // This one out, theirs left in both stores, and said to be there.
    let said_it = uninstall("mine");
    assert_eq!(scopes(&pem_mine), vec![], "{said_it}");
    assert_eq!(
        scopes(&pem_theirs),
        vec![Scope::Machine, Scope::User],
        "uninstalling this CA took another data directory's out: {said_it}"
    );
    assert!(
        said_it.contains("from another data directory is still trusted"),
        "uninstall said nothing about the CA it left trusted: {said_it}"
    );

    // Theirs out by its own data directory: nothing of tracewing's is left,
    // and nothing is said about an older one.
    let said_it = uninstall("theirs");
    assert_eq!(scopes(&pem_theirs), vec![], "{said_it}");
    assert!(
        !said_it.contains("another data directory"),
        "uninstall spoke of an older CA with none there: {said_it}"
    );
    std::fs::remove_dir_all(&dir).ok();
}
