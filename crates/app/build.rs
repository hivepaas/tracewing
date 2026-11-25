//! Puts the app's icon and its name into tracewing.exe, as packwing and
//! shellwing do. Windows shows a program's icon from its resources (Explorer,
//! the Start menu, the taskbar, Alt+Tab), and GPUI loads the window's icon
//! from resource 1. Built on Windows only.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(windows)]
    windows_resources();
}

#[cfg(windows)]
fn windows_resources() {
    // The installer's icon as well (packaging/windows/installer.nsi).
    const ICON: &str = "../../packaging/icons/tracewing.ico";
    println!("cargo:rerun-if-changed={ICON}");
    let mut res = winresource::WindowsResource::new();
    res.set_icon_with_id(ICON, "1")
        // Task Manager shows the description as the program's name.
        .set("FileDescription", "tracewing")
        .set("ProductName", "tracewing");
    if let Err(e) = res.compile() {
        panic!("cannot put the icon into tracewing.exe: {e}");
    }
}
