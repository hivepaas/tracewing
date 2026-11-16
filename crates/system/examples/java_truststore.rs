//! Writes the Java truststore for a data directory and prints the flags that
//! point a JVM at it, so the whole chain can be tried against a real JVM:
//!
//! ```sh
//! cargo run -p tracewing-system --release --example java_truststore -- <data dir> <proxy>
//! ```
//!
//! The window does this itself when the wizard's Java step is drawn. This is
//! for trying it by hand, which is the only way to see what a JVM makes of
//! it: the unit tests ask `p12-keystore` what it wrote, and the `#[ignore]`d
//! one asks `keytool`, but neither is a program connecting through tracewing.
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let dir = std::path::PathBuf::from(&args[1]);
    let proxy: std::net::SocketAddr = args[2].parse()?;
    let store = tracewing_system::java::truststore(&dir.join("ca.pem"), &dir)?;
    println!("{}", tracewing_system::java::flags(proxy, &store));
    Ok(())
}
