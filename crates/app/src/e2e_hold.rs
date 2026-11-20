//! `tracewing e2e-hold`, which nobody types: what crates/e2e's tests run in
//! place of the window, to see what happens to the system proxy or the block
//! on UDP 443 when the program holding it goes, however it goes.
//!
//! It has to be this program, not the test's own: what the window arms behind
//! the proxy — the watchdog, the Windows scheduled task, the systemd timer,
//! the macOS agent — and the elevated helper behind the Windows block are all
//! started as `std::env::current_exe()`, so a test that took the proxy itself
//! would leave its own program behind it, not tracewing.
//!
//! `e2e-hold system-proxy --data-dir D --port P [--no-watchdog]` takes the
//! system proxy for 127.0.0.1:P as the window does. `e2e-hold quic-block
//! --data-dir D` blocks UDP 443 as the window does. Either says [`HELD`] on
//! its own line once it holds it, then holds it until its input ends, and lets
//! it go; killed instead, it leaves what it held to what stands behind it.

use anyhow::{Context, Result, bail};
use std::{
    io::{Read, Write},
    net::SocketAddr,
    path::PathBuf,
};
use tracewing_system as system;

/// What it says once it holds what it was asked to.
pub const HELD: &str = "tracewing-e2e: held";

/// Runs it, from the arguments after `e2e-hold`.
pub fn run(args: &[String]) -> Result<()> {
    let mut what = None;
    let mut data_dir = None;
    let mut port = None;
    let mut watchdog = true;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--data-dir" => data_dir = rest.next().map(PathBuf::from),
            "--port" => port = rest.next().map(|p| p.parse::<u16>()).transpose()?,
            "--no-watchdog" => watchdog = false,
            other if what.is_none() => what = Some(other.to_owned()),
            other => bail!("e2e-hold: {other}?"),
        }
    }
    let data_dir = data_dir.context("e2e-hold needs --data-dir")?;
    let release: Box<dyn FnOnce() -> Result<()>> = match what.as_deref() {
        Some("system-proxy") => {
            let port = port.context("e2e-hold system-proxy needs --port")?;
            let taken = system::take_over(
                &data_dir,
                SocketAddr::from(([127, 0, 0, 1], port)),
                watchdog,
            )?;
            Box::new(move || taken.release().map(drop))
        }
        Some("quic-block") => {
            let block = system::block_quic(&data_dir)?;
            Box::new(move || block.release())
        }
        _ => bail!("e2e-hold: system-proxy or quic-block"),
    };
    let mut out = std::io::stdout();
    writeln!(out, "{HELD}")?;
    out.flush()?;
    // Until whoever started it lets go of its input.
    let _ = std::io::stdin().read_to_end(&mut Vec::new());
    release()
}
