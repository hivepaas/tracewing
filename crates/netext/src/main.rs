//! The system extension's program: started by macOS, never by hand.

#[cfg(target_os = "macos")]
fn main() {
    tracewing_netext::provider::run()
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("tracewing-netext is tracewing's network extension for macOS");
    std::process::exit(1);
}
