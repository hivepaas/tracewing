//! tracewing.com wears the app's icon too, so that it looks like the
//! program it is in Explorer. Built on Windows only.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(windows)]
    {
        const ICON: &str = "../../packaging/icons/tracewing.ico";
        println!("cargo:rerun-if-changed={ICON}");
        let mut res = winresource::WindowsResource::new();
        res.set_icon_with_id(ICON, "1")
            .set("FileDescription", "tracewing (command line)")
            .set("ProductName", "tracewing");
        if let Err(e) = res.compile() {
            panic!("cannot put the icon into tracewing.com: {e}");
        }
    }
}
