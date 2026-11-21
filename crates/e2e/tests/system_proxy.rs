//! S2, the system proxy: turned on, the desktop's own setting names
//! tracewing; turned off, it is what it was, a proxy of the user's own
//! included. Read with the desktop's own tools rather than tracewing's reader,
//! so the reader is checked too.
//!
//! Changes this computer's proxy setting. See `tracewing_e2e`.

#![cfg(any(target_os = "linux", windows, target_os = "macos"))]

use tracewing_e2e::{
    desktop::{names_tracewing, setting, the_users_own},
    free_port,
    helper::Helper,
    only_where_allowed, scratch,
};

#[test]
fn the_system_proxy_names_tracewing_and_comes_back_as_it_was() {
    only_where_allowed!();
    let dir = scratch("system-proxy");
    let _put_back = the_users_own();
    let before = setting();
    let port = free_port();

    // tracewing takes it, as the window does when the switch goes on.
    let app = Helper::system_proxy(&dir, port, true);
    let during = setting();
    assert!(names_tracewing(&during, port), "{during:?}");

    // And lets it go, as the window does when it closes.
    let ended = app.let_go();
    assert!(ended.success(), "letting go failed: {ended:?}");
    assert_eq!(setting(), before, "not put back as it was");
    std::fs::remove_dir_all(&dir).ok();
}

/// A Mac where nobody ever set a proxy: no server on any service. Letting go,
/// tracewing clears the server it wrote rather than leaving itself there,
/// turned off. The other two write their whole setting back either way.
#[cfg(target_os = "macos")]
#[test]
fn a_mac_without_a_proxy_has_none_after() {
    only_where_allowed!();
    let dir = scratch("system-proxy-none");
    let _put_back = tracewing_e2e::desktop::none_set();
    let before = setting();
    let port = free_port();

    let app = Helper::system_proxy(&dir, port, true);
    let during = setting();
    assert!(names_tracewing(&during, port), "{during:?}");

    let ended = app.let_go();
    assert!(ended.success(), "letting go failed: {ended:?}");
    assert_eq!(setting(), before, "not put back as it was");
    std::fs::remove_dir_all(&dir).ok();
}
