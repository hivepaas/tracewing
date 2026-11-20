//! S2, the system proxy: turned on, the desktop's own setting names
//! tracewing; turned off, it is what it was, a proxy of the user's own
//! included. Read with the desktop's own tools rather than tracewing's reader,
//! so the reader is checked too.
//!
//! Changes this computer's proxy setting. See `tracewing_e2e`.

#![cfg(target_os = "linux")]

use std::net::SocketAddr;
use tracewing_e2e::{
    desktop::{names_tracewing, setting, the_users_own},
    free_port, only_where_allowed, scratch,
};

#[test]
fn the_system_proxy_names_tracewing_and_comes_back_as_it_was() {
    only_where_allowed!();
    let dir = scratch("system-proxy");
    the_users_own();
    let before = setting();
    let addr = SocketAddr::from(([127, 0, 0, 1], free_port()));

    let taken = tracewing_system::take_over(&dir, addr, false).unwrap();
    let during = setting();
    assert!(names_tracewing(&during, addr.port()), "{during:?}");

    taken.release().unwrap();
    assert_eq!(setting(), before, "not put back as it was");
    std::fs::remove_dir_all(&dir).ok();
}
