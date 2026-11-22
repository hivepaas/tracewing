//! S8 through the window on Windows: the local way in left on, the window
//! started, a chosen program's HTTPS recorded through the adapter, and the
//! adapter gone with the window. See `tracewing_e2e::local_window`.
//!
//! With the program under test (`TRACEWING_E2E_BIN`), a build, whose
//! `wintun.dll` is the one `WINTUN_DLL` names (`e2e_windows.ps1 -Wintun`).
//! The installed package's own, beside it, is `installer.rs`'s.
//!
//! Puts an adapter on this computer while it runs. See `tracewing_e2e`.
#![cfg(windows)]

use tracewing_e2e::{local_window, only_where_allowed, program, scratch};

/// Whether this process is an administrator's, which the adapter needs: the
/// window would otherwise say so and bring nothing up.
fn elevated() -> bool {
    std::process::Command::new("whoami")
        .arg("/groups")
        .output()
        .is_ok_and(|out| String::from_utf8_lossy(&out.stdout).contains("S-1-16-12288"))
}

#[test]
fn the_window_left_on_records_a_chosen_program_through_the_adapter() {
    only_where_allowed!();
    let program = program();
    if local_window::wintun_for(&program).is_none() {
        eprintln!(
            "skipped: no wintun.dll beside the program and no WINTUN_DLL (e2e_windows.ps1 -Wintun)"
        );
        return;
    }
    if !elevated() {
        eprintln!("skipped: not an administrator, and the adapter needs one");
        return;
    }
    let dir = scratch("local-window");
    local_window::a_chosen_program_is_recorded_through_the_window(&program, &dir);
    std::fs::remove_dir_all(&dir).ok();
}
